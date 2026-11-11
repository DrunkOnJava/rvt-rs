//! Bounded UV-grid meshing for decoded parametric surfaces.

use crate::native_parameters::ObjectGraph;
use crate::native_parametric_surface::ParametricSurface;
use crate::native_saved_mesh::FaceMesh;
use crate::vector_math::distance;
use anyhow::{Context, Result, ensure};

#[derive(Debug, Clone)]
pub struct ParametricMesh {
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

fn uv_point(v: &serde_json::Value) -> Result<[f64; 2]> {
    let a = v.as_array().context("UV array")?;
    ensure!(a.len() == 2, "UV dimension");
    let p = [
        a[0].as_f64().context("UV number")?,
        a[1].as_f64().context("UV number")?,
    ];
    ensure!(p.iter().all(|x| x.is_finite()), "nonfinite UV");
    Ok(p)
}

/// The grid a `Face` is meshed on: its surface, the knots along `u` and `v`, the
/// trim loop in the surface's parameters and the parameter rectangle the face
/// spans. Public for `probe_1683_ruled_grids`, which prints what sets a ruled
/// face's cells.
#[derive(Debug)]
pub struct FaceGrid {
    pub surface: ParametricSurface,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
    /// The points of every trim loop, edge after edge: the knots' source.
    pub trim: Vec<[f64; 2]>,
    pub bounds: [[f64; 2]; 2],
    /// Whether the face's one trim loop is the rectangle `bounds`, so that the grid's
    /// cells are the face's; then `loops` is empty.
    pub rectangular: bool,
    /// Where it is not, each trim loop as a closed polygon in the surface's
    /// parameters (the first is the outer one, the others holes), its last point not
    /// repeated; the face is the part of the grid inside them (#1706).
    pub loops: Vec<Vec<[f64; 2]>>,
}

// A face whose trim is the complete rectangle of its parameters (including analytic
// collapsed poles) is meshed on its grid; merely lying on the rectangle's perimeter
// is insufficient. Any other trim, or several loops, is cut into the grid's cells
// (`trimmed_cells`, #1706).
//
// The component comes back with whether `main`'s guard, which refused one with
// an edge naming fewer than two faces anywhere in it, would have refused it:
// such a component is drawn only because the guard was narrowed (#1683).
fn ruled_surface_component(
    g: &ObjectGraph,
    start_face: usize,
) -> Result<(std::collections::BTreeSet<usize>, bool)> {
    // The saved graph does not consistently materialize Edge -> Face pointer
    // records in g.edges.  Resolve the authoritative m_pFace pair on each
    // Edge instead; pointer() also covers token-only references.  Malformed
    // disconnected edges are ignored while building this component index so
    // they cannot taint an otherwise resolvable ruled-surface component.
    let mut adjacency: std::collections::BTreeMap<usize, std::collections::BTreeSet<usize>> =
        std::collections::BTreeMap::new();
    let mut invalid_by_face = std::collections::BTreeSet::new();
    for (edge_index, edge) in g
        .objects
        .iter()
        .enumerate()
        .filter(|(_, object)| object.class_name == "Edge")
    {
        let Some(faces) = edge
            .fields
            .get("m_pFace")
            .and_then(|value| value.as_array())
        else {
            continue;
        };
        let resolved = faces
            .iter()
            .map(|value| {
                crate::native_saved_mesh::pointer(g, edge_index, value)
                    .ok()
                    .flatten()
            })
            .collect::<Vec<_>>();
        let valid_faces = resolved
            .iter()
            .filter_map(|target| *target)
            .filter(|target| {
                g.objects
                    .get(*target)
                    .is_some_and(|object| object.class_name == "Face")
            })
            .collect::<Vec<_>>();
        if faces.len() != 2 || valid_faces.len() != 2 {
            invalid_by_face.extend(valid_faces);
            continue;
        }
        let first = valid_faces[0];
        let second = valid_faces[1];
        adjacency.entry(first).or_default().insert(second);
        adjacency.entry(second).or_default().insert(first);
    }
    let mut component = std::collections::BTreeSet::from([start_face]);
    let mut queue = std::collections::VecDeque::from([start_face]);
    while let Some(face) = queue.pop_front() {
        for &other_face in adjacency.get(&face).into_iter().flatten() {
            if component.insert(other_face) {
                queue.push_back(other_face);
            }
        }
    }
    // An edge that names fewer than two faces leaves a face's neighbours unknown
    // only where it borders a ruled face. The shared grid sizes the ruled faces
    // of a component alike, a ruled face's neighbours are the faces its own
    // edges name, and an edge of a plane or a cylinder that names one face (a
    // boundary of the faces the graph holds, as on four roofs of Snowdon Towers
    // Architectural, #1683) cannot join two ruled faces.
    let ruled = |face: usize| {
        g.objects
            .get(face)
            .and_then(|object| object.fields.get("m_pSurf"))
            .and_then(|value| {
                crate::native_saved_mesh::pointer(g, face, value)
                    .ok()
                    .flatten()
            })
            .and_then(|surface| g.objects.get(surface))
            .is_some_and(|surface| surface.class_name == "RuledSurf")
    };
    if let Some(face) = component
        .iter()
        .find(|face| ruled(**face) && invalid_by_face.contains(face))
    {
        anyhow::bail!("malformed ruled-surface edge is connected to face {face}");
    }
    let refused_before = component.iter().any(|face| invalid_by_face.contains(face));
    Ok((component, refused_before))
}

/// The most trim loops a face may hold.
const MAX_TRIM_LOOPS: usize = 64;

/// The samples of each edge of the loop at `li` of the face `fi`, edge by edge, in the
/// face's parameters and clamped to the surface's envelope `env`. A loop has at least
/// `least` edges.
fn loop_segments(
    g: &ObjectGraph,
    fi: usize,
    li: usize,
    env: [[f64; 2]; 2],
    least: usize,
) -> Result<Vec<Vec<[f64; 2]>>> {
    use crate::native_saved_mesh::pointer;
    let l = &g.objects[li];
    ensure!(
        ["EdgeLoop", "EdgeLoopWithChainEnvelopes"].contains(&l.class_name.as_str()),
        "unsupported trim loop"
    );
    ensure!(l.fields["m_open"] == false, "open trim loop");
    ensure!(
        pointer(g, li, &l.fields["m_pFace"])? == Some(fi),
        "loop owner mismatch"
    );
    let mut cur = pointer(g, li, &l.fields["m_next"])?.context("empty trim")?;
    let mut seen = std::collections::BTreeSet::new();
    let mut segments = Vec::new();
    while cur != li {
        ensure!(seen.insert(cur), "trim edge cycle");
        let e = &g.objects[cur];
        ensure!(e.class_name == "Edge", "nonedge trim member");
        let faces = e.fields["m_pFace"].as_array().context("edge face pair")?;
        ensure!(faces.len() == 2, "edge face pair length");
        let mut side = None;
        for (i, p) in faces.iter().enumerate() {
            if pointer(g, cur, p)? == Some(fi) {
                ensure!(side.is_none(), "ambiguous face side");
                side = Some(i);
            }
        }
        let side = side.context("edge does not belong to face")?;
        let ends = e.fields["m_firstAndLastEdgePnts"]
            .as_array()
            .context("edge endpoints")?;
        ensure!(ends.len() == 2, "edge endpoint count");
        let mut points = vec![uv_point(&ends[0]["uv"][side])?];
        for p in e.fields["m_interiorEdgePnts"]
            .as_array()
            .context("edge interior points")?
        {
            points.push(uv_point(&p["uv"][side])?);
        }
        points.push(uv_point(&ends[1]["uv"][side])?);
        segments.push(points);
        cur = pointer(g, cur, &e.fields["m_next"][side])?.context("null trim next")?;
    }
    ensure!(segments.len() >= least, "too few parametric edges");
    ensure!(
        segments.iter().all(|s: &Vec<_>| !s.is_empty()),
        "empty parametric edge"
    );
    for segment in &mut segments {
        for p in segment {
            for k in 0..2 {
                ensure!(
                    p[k] >= env[0][k] - 1e-7 && p[k] <= env[1][k] + 1e-7,
                    "trim outside surface envelope"
                );
                p[k] = p[k].clamp(env[0][k], env[1][k]);
            }
        }
    }
    Ok(segments)
}

/// Whether the edges of a loop join end to end in model coordinates, either way
/// round, and close.
fn loop_closes(surface: &ParametricSurface, segments: &[Vec<[f64; 2]>]) -> Result<bool> {
    for reverse in [false, true] {
        let first = if reverse {
            *segments[0].last().unwrap()
        } else {
            segments[0][0]
        };
        let mut last = if reverse {
            segments[0][0]
        } else {
            *segments[0].last().unwrap()
        };
        let mut valid = true;
        for seg in &segments[1..] {
            let p = surface.evaluate(last[0], last[1])?;
            let a = seg[0];
            let b = *seg.last().unwrap();
            if distance(p, surface.evaluate(a[0], a[1])?) < 1e-7 {
                last = b;
            } else if distance(p, surface.evaluate(b[0], b[1])?) < 1e-7 {
                last = a;
            } else {
                valid = false;
                break;
            }
        }
        if valid
            && distance(
                surface.evaluate(first[0], first[1])?,
                surface.evaluate(last[0], last[1])?,
            ) < 1e-7
        {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether the one loop of a face is the rectangle `min`..`max` of its own
/// parameters: every edge along one of its sides, and the sides covered whole (a
/// revolution's side may collapse to a pole). `Ok(false)` where an edge crosses
/// the rectangle; an error where edges lie on the sides and do not cover them.
fn trim_is_rectangle(
    surface: &ParametricSurface,
    segments: &[Vec<[f64; 2]>],
    min: [f64; 2],
    max: [f64; 2],
) -> Result<bool> {
    let mut sides: [Vec<[f64; 2]>; 4] = std::array::from_fn(|_| Vec::new());
    for seg in segments {
        for pair in seg.windows(2) {
            let a = pair[0];
            let b = pair[1];
            if (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12 {
                continue;
            }
            let (side, axis) = if (a[1] - min[1]).abs() < 1e-7 && (b[1] - min[1]).abs() < 1e-7 {
                (0, 0)
            } else if (a[0] - max[0]).abs() < 1e-7 && (b[0] - max[0]).abs() < 1e-7 {
                (1, 1)
            } else if (a[1] - max[1]).abs() < 1e-7 && (b[1] - max[1]).abs() < 1e-7 {
                (2, 0)
            } else if (a[0] - min[0]).abs() < 1e-7 && (b[0] - min[0]).abs() < 1e-7 {
                (3, 1)
            } else {
                return Ok(false);
            };
            sides[side].push([a[axis].min(b[axis]), a[axis].max(b[axis])]);
        }
    }
    for (side, intervals) in sides.iter_mut().enumerate() {
        let axis = if side % 2 == 0 { 0 } else { 1 };
        if intervals.is_empty() {
            // A revolution's horizontal UV edge can collapse to a pole. The
            // half-turn distance is exactly twice the profile's radial norm.
            ensure!(
                axis == 0 && matches!(surface, ParametricSurface::SurfRev { .. }),
                "missing rectangular trim side"
            );
            let v = if side == 0 { min[1] } else { max[1] };
            let a = surface.evaluate(min[0], v)?;
            let b = surface.evaluate((min[0] + max[0]) * 0.5, v)?;
            let c = surface.evaluate(max[0], v)?;
            ensure!(
                distance(a, b) < 1e-9 && distance(a, c) < 1e-9,
                "missing noncollapsed trim side"
            );
            continue;
        }
        intervals.sort_by(|a, b| a[0].total_cmp(&b[0]));
        let mut end = min[axis];
        for p in intervals {
            ensure!(
                (p[0] - end).abs() < 1e-7,
                "overlapping or incomplete trim perimeter"
            );
            end = p[1];
        }
        ensure!((end - max[axis]).abs() < 1e-7, "incomplete trim perimeter");
    }
    Ok(true)
}

/// A loop as a closed polygon in the surface's parameters: each edge's samples in the
/// direction that continues the one before, and the last point (the first again)
/// dropped. A join that meets in model coordinates but not in the parameters (the
/// seam of a revolution, a pole) is refused.
fn loop_polygon(surface: &ParametricSurface, segments: &[Vec<[f64; 2]>]) -> Result<Vec<[f64; 2]>> {
    let same =
        |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).abs() <= 1e-9 && (a[1] - b[1]).abs() <= 1e-9;
    for reverse in [false, true] {
        let mut ring: Vec<[f64; 2]> = if reverse {
            segments[0].iter().rev().copied().collect()
        } else {
            segments[0].clone()
        };
        let mut joined = true;
        for seg in &segments[1..] {
            let last = *ring.last().unwrap();
            let p = surface.evaluate(last[0], last[1])?;
            let (a, b) = (seg[0], *seg.last().unwrap());
            if distance(p, surface.evaluate(a[0], a[1])?) < 1e-7 {
                if !same(last, a) {
                    joined = false;
                    break;
                }
                ring.extend(seg.iter().skip(1).copied());
            } else if distance(p, surface.evaluate(b[0], b[1])?) < 1e-7 {
                if !same(last, b) {
                    joined = false;
                    break;
                }
                ring.extend(seg.iter().rev().skip(1).copied());
            } else {
                joined = false;
                break;
            }
        }
        if !joined {
            continue;
        }
        let (first, last) = (ring[0], *ring.last().unwrap());
        if same(first, last)
            && distance(
                surface.evaluate(first[0], first[1])?,
                surface.evaluate(last[0], last[1])?,
            ) < 1e-7
        {
            ring.pop();
            ensure!(ring.len() >= 3, "a trim loop needs three points");
            return Ok(ring);
        }
    }
    anyhow::bail!("a trim loop does not close in the surface's parameters")
}

/// The grid the face at `fi` is meshed on, or why it is not meshed (a trim loop
/// that does not close, a ruled face whose neighbours the graph does not resolve,
/// a surface that is not read). A face whose one loop is the rectangle of its
/// parameters is meshed on the grid; any other loop, or several, is cut into its
/// cells.
pub fn face_grid(g: &ObjectGraph, fi: usize) -> Result<FaceGrid> {
    use crate::native_parametric_surface::Curve;
    use crate::native_saved_mesh::pointer;
    let f = g.objects.get(fi).context("face index")?;
    ensure!(f.class_name == "Face", "not Face");
    ensure!(
        f.fields["m_faceRegions"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "face regions need subdivision"
    );
    let si = pointer(g, fi, &f.fields["m_pSurf"])?.context("face surface")?;
    let surface = ParametricSurface::read(g, si)?;
    let env = surface.bounds();
    // Every loop of the face, the first the outer one: [loop][edge][sample] in the
    // surface's parameters.
    let mut loops: Vec<Vec<Vec<[f64; 2]>>> = Vec::new();
    let mut seen_loops = std::collections::BTreeSet::new();
    let mut next = Some(pointer(g, fi, &f.fields["m_pFirstLoop"])?.context("face loop")?);
    while let Some(li) = next {
        ensure!(seen_loops.insert(li), "trim loop cycle");
        ensure!(loops.len() < MAX_TRIM_LOOPS, "too many trim loops");
        let segments = loop_segments(g, fi, li, env, if loops.is_empty() { 2 } else { 1 })?;
        ensure!(
            loop_closes(&surface, &segments)?,
            "disconnected parametric trim in model coordinates"
        );
        loops.push(segments);
        next = pointer(g, li, &g.objects[li].fields["m_nextLoop"])?;
    }
    let trim = loops
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let min = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let max = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    ensure!(
        max[0] > min[0] && max[1] > min[1],
        "degenerate parametric domain"
    );
    let rectangular = loops.len() == 1 && trim_is_rectangle(&surface, &loops[0], min, max)?;
    let polygons = if rectangular {
        Vec::new()
    } else {
        loops
            .iter()
            .map(|segments| loop_polygon(&surface, segments))
            .collect::<Result<Vec<_>>>()?
    };
    const CHORD: f64 = 0.001;
    let length = |p: [f64; 3]| p.iter().map(|x| x * x).sum::<f64>().sqrt();
    let angular = |r: f64| {
        (4. * (CHORD / (2. * r.max(CHORD))).min(1.).sqrt().asin()).min(std::f64::consts::FRAC_PI_2)
    };
    let (du, dv) = match &surface {
        ParametricSurface::SurfRev { profile, .. } => match profile {
            Curve::Line {
                origin, direction, ..
            } => {
                let radius = [min[1], max[1]]
                    .into_iter()
                    .map(|v| (origin[0] + v * direction[0]).hypot(origin[1] + v * direction[1]))
                    .fold(0., f64::max);
                (angular(radius), 1. / length(*direction))
            }
            Curve::Arc { center, radius, .. } => (
                angular(center[0].hypot(center[1]) + 2. * radius),
                angular(*radius),
            ),
            // The second derivative of `c + a cos t x + b sin t y` is at most
            // max(a, b) long, so a step in `t` bows no further from its chord
            // than the same step on a circle of that radius.
            Curve::Ellipse {
                center,
                x_len,
                y_len,
                ..
            } => (
                angular(center[0].hypot(center[1]) + 2. * x_len.max(*y_len)),
                angular(x_len.max(*y_len)),
            ),
        },
        ParametricSurface::ConeSurf { half_angle, .. } => {
            // The generators are straight: only the angle needs a chord bound.
            let far = min[1].abs().max(max[1].abs());
            (angular(far * half_angle.sin()), 1.)
        }
        ParametricSurface::RuledSurf {
            profile1, profile2, ..
        } => {
            // A face whose neighbours the graph does not resolve is refused: what
            // is drawn from it would hold a hole the graph does not show.
            let (component, narrowed) = ruled_surface_component(g, fi)?;
            if !narrowed {
                // A component `main`'s guard accepted keeps the grid it always
                // had, one normalized grid across its ruled patches, so that no
                // export drawn before changes (#1683).
                //
                // A shared normalized grid across ruled patches preserves their
                // common straight generator edges even when profile lengths differ.
                let mut n = 1.0_f64;
                for i in component.into_iter().filter(|i| {
                    g.objects
                        .get(*i)
                        .and_then(|face| face.fields.get("m_pSurf"))
                        .is_some()
                }) {
                    let Some(surface_index) = pointer(g, i, &g.objects[i].fields["m_pSurf"])?
                    else {
                        continue;
                    };
                    if g.objects[surface_index].class_name != "RuledSurf" {
                        continue;
                    }
                    let s = ParametricSurface::read(g, surface_index)?;
                    let ParametricSurface::RuledSurf {
                        profile1, profile2, ..
                    } = s
                    else {
                        continue;
                    };
                    // Each profile's chord (its end less its start) and the
                    // normalized steps its own curvature needs: one per foot
                    // along a line, and the angular step of its largest radius
                    // across an arc or an ellipse.
                    let profile_steps = |c: &Curve| -> ([f64; 3], f64) {
                        let r = c.range();
                        let chord = match c {
                            Curve::Line { direction, .. } => {
                                std::array::from_fn(|k| direction[k] * (r[1] - r[0]))
                            }
                            _ => {
                                let (a, b) = (c.eval(r[0]), c.eval(r[1]));
                                std::array::from_fn(|k| b[k] - a[k])
                            }
                        };
                        let steps = match c {
                            Curve::Line { .. } => length(chord).ceil(),
                            Curve::Arc { radius, .. } => {
                                ((r[1] - r[0]).abs() / angular(*radius)).ceil()
                            }
                            Curve::Ellipse { x_len, y_len, .. } => {
                                ((r[1] - r[0]).abs() / angular(x_len.max(*y_len))).ceil()
                            }
                        };
                        (chord, steps)
                    };
                    let (da, na) = profile_steps(&profile1);
                    let (db, nb) = profile_steps(&profile2);
                    let mixed = length(std::array::from_fn(|k| da[k] - db[k]));
                    n = n.max((mixed / CHORD).sqrt().ceil()).max(na).max(nb);
                }
                (1. / n, 1. / n)
            } else {
                // Each face takes the grid its own surface needs. Its generators are
                // straight, so a neighbour's finer grid along a shared generator
                // adds vertices on the same line and opens no gap (#1683: one grid
                // for the whole component gave a roof of 23 ruled faces 1.1 million
                // triangles where Revit's own body has 1,259). The normalized steps
                // are the angular step of the largest radius across an arc or an
                // ellipse, none along a line, and the twist: a surface between two
                // lines leaves its corners' plane by the corners' second difference
                // along the plane's normal, so a face whose four corners lie in one
                // plane is flat whatever its profiles' lengths, and any other keeps
                // the whole difference, as before.
                let range_steps = |c: &Curve| -> f64 {
                    let r = c.range();
                    match c {
                        Curve::Line { .. } => 1.,
                        Curve::Arc { radius, .. } => {
                            ((r[1] - r[0]).abs() / angular(*radius)).ceil()
                        }
                        Curve::Ellipse { x_len, y_len, .. } => {
                            ((r[1] - r[0]).abs() / angular(x_len.max(*y_len))).ceil()
                        }
                    }
                };
                let corner = |u: f64, v: f64| surface.evaluate(u, v);
                let c00 = corner(env[0][0], env[0][1])?;
                let c10 = corner(env[1][0], env[0][1])?;
                let c01 = corner(env[0][0], env[1][1])?;
                let c11 = corner(env[1][0], env[1][1])?;
                let twist: [f64; 3] = std::array::from_fn(|k| c00[k] - c10[k] - c01[k] + c11[k]);
                let a: [f64; 3] = std::array::from_fn(|k| c10[k] - c00[k]);
                let b: [f64; 3] = std::array::from_fn(|k| c01[k] - c00[k]);
                let normal = [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ];
                let normal_length = length(normal);
                let flat = normal_length > 0.
                    && (twist.iter().zip(normal).map(|(t, n)| t * n).sum::<f64>() / normal_length)
                        .abs()
                        < 1e-8;
                let twisted = if flat { 0. } else { length(twist) };
                // `u` runs along the profiles, `v` between them. The profiles' arcs
                // ask for steps along `u` alone, and the twist for `twisted / CHORD`
                // cells in all, however they divide between the two: a face of a long
                // arc between two straight generators needs its arc's steps and one
                // cell across, not as many across as along.
                //
                // Between two straight profiles the surface is a bilinear patch, and
                // cut into `n_u * n_v` cells of two triangles it leaves them by
                // `twisted / (4 * n_u * n_v)` at most (at a cell's middle), so a
                // quarter of the cells hold the bar (#1683: the roofs' faces of 11 to
                // 24 ft of twist asked 4 times that). A curved profile adds its own
                // bow, up to the bar, to the twist's, so such a face keeps the margin.
                let straight = matches!(profile1, Curve::Line { .. })
                    && matches!(profile2, Curve::Line { .. });
                let cells = twisted / if straight { 4. * CHORD } else { CHORD };
                let n_u = cells
                    .sqrt()
                    .ceil()
                    .max(range_steps(profile1))
                    .max(range_steps(profile2))
                    .max(1.);
                let n_v = (cells / n_u).ceil().max(1.);
                (1. / n_u, 1. / n_v)
            }
        }
    };
    let axis = |k: usize, step: f64| -> Result<Vec<f64>> {
        let count = ((max[k] - min[k]) / step).ceil().max(1.);
        ensure!(
            count.is_finite() && count <= 10000.,
            "parametric axis budget exceeded"
        );
        let count = count as usize;
        let mut a = (0..=count)
            .map(|i| min[k] + (max[k] - min[k]) * i as f64 / count as f64)
            .collect::<Vec<_>>();
        a.extend(trim.iter().map(|p| p[k]));
        a.sort_by(f64::total_cmp);
        a.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        Ok(a)
    };
    Ok(FaceGrid {
        u: axis(0, du)?,
        v: axis(1, dv)?,
        surface,
        trim,
        bounds: [min, max],
        rectangular,
        loops: polygons,
    })
}

pub fn face(g: &ObjectGraph, fi: usize) -> Result<FaceMesh> {
    let grid = face_grid(g, fi)?;
    let f = &g.objects[fi];
    let flags = f.fields["m_faceFlags_v9"].as_u64().context("face flags")?;
    let orient = flags & 2 == 0;
    let mesh = if grid.rectangular {
        tessellate_uv(
            &grid.surface,
            &grid.u,
            &grid.v,
            grid.bounds,
            orient,
            2_000_000,
        )?
    } else {
        tessellate_trimmed(
            &grid.surface,
            &grid.loops,
            &grid.u,
            &grid.v,
            orient,
            2_000_000,
        )?
    };
    Ok(FaceMesh {
        face_index: fi,
        face_tag: f.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
        render_style_id: crate::native_metadata::identifier(&f.fields["m_renderStyleId"])?,
        normal: mesh.normals[0],
        vertices: mesh.vertices,
        normals: mesh.normals,
        triangles: mesh.triangles,
        analytic_surface: None,
        trim_uv: grid.trim,
    })
}

pub fn tessellate(
    surface: &ParametricSurface,
    u_segments: usize,
    v_segments: usize,
    max_vertices: usize,
) -> Result<ParametricMesh> {
    ensure!(
        u_segments > 0 && v_segments > 0 && max_vertices > 3 && max_vertices <= u32::MAX as usize,
        "invalid parametric mesh budget"
    );
    let e = surface.bounds();
    let u_count = u_segments
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("U grid overflow"))?;
    let v_count = v_segments
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("V grid overflow"))?;
    let grid_count = u_count
        .checked_mul(v_count)
        .ok_or_else(|| anyhow::anyhow!("parametric mesh vertex count overflow"))?;
    ensure!(
        grid_count <= max_vertices,
        "parametric mesh vertex budget exceeded"
    );
    ensure!(
        grid_count <= u32::MAX as usize,
        "parametric mesh exceeds index range"
    );
    let mut us = Vec::with_capacity(u_count);
    let mut vs = Vec::with_capacity(v_count);
    for i in 0..=u_segments {
        us.push(e[0][0] + (e[1][0] - e[0][0]) * (i as f64 / u_segments as f64));
    }
    for j in 0..=v_segments {
        vs.push(e[0][1] + (e[1][1] - e[0][1]) * (j as f64 / v_segments as f64));
    }
    tessellate_uv(surface, &us, &vs, e, true, max_vertices)
}

pub fn tessellate_uv(
    surface: &ParametricSurface,
    u_knots: &[f64],
    v_knots: &[f64],
    trim: [[f64; 2]; 2],
    orient: bool,
    max_vertices: usize,
) -> Result<ParametricMesh> {
    ensure!(
        u_knots.len() >= 2
            && v_knots.len() >= 2
            && max_vertices > 3
            && max_vertices <= u32::MAX as usize,
        "invalid parametric mesh budget"
    );
    ensure!(
        u_knots.iter().all(|x| x.is_finite())
            && v_knots.iter().all(|x| x.is_finite())
            && u_knots.windows(2).all(|w| w[1] > w[0])
            && v_knots.windows(2).all(|w| w[1] > w[0]),
        "knots must be finite and strictly increasing"
    );
    ensure!(
        trim.iter().flatten().all(|x| x.is_finite())
            && trim[1][0] > trim[0][0]
            && trim[1][1] > trim[0][1],
        "invalid trim rectangle"
    );
    let bounds = surface.bounds();
    ensure!(
        trim[0][0] >= bounds[0][0]
            && trim[1][0] <= bounds[1][0]
            && trim[0][1] >= bounds[0][1]
            && trim[1][1] <= bounds[1][1],
        "trim outside surface envelope"
    );
    let (Some(u_first), Some(u_last), Some(v_first), Some(v_last)) = (
        u_knots.first(),
        u_knots.last(),
        v_knots.first(),
        v_knots.last(),
    ) else {
        anyhow::bail!("empty knot vector");
    };
    ensure!(
        *u_first >= trim[0][0]
            && *u_last <= trim[1][0]
            && *v_first >= trim[0][1]
            && *v_last <= trim[1][1],
        "knots outside trim rectangle"
    );
    let count = u_knots
        .len()
        .checked_mul(v_knots.len())
        .ok_or_else(|| anyhow::anyhow!("parametric mesh vertex count overflow"))?;
    ensure!(
        count <= max_vertices && count <= u32::MAX as usize,
        "parametric mesh vertex budget exceeded"
    );
    let mut vertices = Vec::with_capacity(count);
    let mut normals = Vec::with_capacity(count);
    for &v in v_knots {
        for &u in u_knots {
            vertices.push(surface.evaluate(u, v)?);
            let mut n = surface.normal(u, v)?;
            if !orient {
                for value in &mut n {
                    *value = -*value;
                }
            }
            normals.push(n);
        }
    }
    let cells = u_knots
        .len()
        .checked_sub(1)
        .zip(v_knots.len().checked_sub(1))
        .and_then(|(u, v)| u.checked_mul(v))
        .ok_or_else(|| anyhow::anyhow!("parametric mesh cell count overflow"))?;
    let mut triangles = Vec::with_capacity(
        cells
            .checked_mul(2)
            .ok_or_else(|| anyhow::anyhow!("parametric triangle count overflow"))?,
    );
    let surface_orient = surface.orientation();
    let winding = surface_orient == orient;
    let stride = u_knots.len();
    for j in 0..v_knots.len() - 1 {
        for i in 0..u_knots.len() - 1 {
            let a = (j * stride + i) as u32;
            let b = a + 1;
            let c = ((j + 1) * stride + i + 1) as u32;
            let d = ((j + 1) * stride + i) as u32;
            let area = |x: u32, y: u32, z: u32| {
                let p = vertices[x as usize];
                let q = vertices[y as usize];
                let r = vertices[z as usize];
                let u = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
                let v = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
                (u[1] * v[2] - u[2] * v[1])
                    .hypot((u[2] * v[0] - u[0] * v[2]).hypot(u[0] * v[1] - u[1] * v[0]))
            };
            let collapsed = |x: u32, y: u32| {
                let p = vertices[x as usize];
                let q = vertices[y as usize];
                let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= 1e-20
            };
            let add_triangle =
                |x: u32, y: u32, z: u32, triangles: &mut Vec<[u32; 3]>| -> Result<()> {
                    let triangle_area = area(x, y, z);
                    if triangle_area <= 1e-12 {
                        ensure!(
                            collapsed(x, y) || collapsed(y, z) || collapsed(z, x),
                            "noncollapsed degenerate parametric triangle"
                        );
                    } else {
                        triangles.push(if winding { [x, y, z] } else { [x, z, y] });
                    }
                    Ok(())
                };
            add_triangle(a, b, c, &mut triangles)?;
            add_triangle(a, c, d, &mut triangles)?;
        }
    }
    ensure!(
        !triangles.is_empty(),
        "parametric surface produced no triangles"
    );
    Ok(ParametricMesh {
        vertices,
        normals,
        triangles,
    })
}

/// How close (in parameter units) a trim point must be to a knot, and two parameters
/// to be one.
const KNOT_TOLERANCE: f64 = 1e-9;
/// Work a hostile file can ask of `trimmed_cells`: strips times trim segments.
const STRIP_BUDGET: usize = 20_000_000;
/// The most triangles a trimmed face may come to.
const TRIANGLE_BUDGET: usize = 4_000_000;

/// The points of a trimmed face in (u, v) and its triangles, by their indices.
type Cells = (Vec<[f64; 2]>, Vec<[u32; 3]>);

/// One trim segment, oriented so that `u0 < u1`.
#[derive(Clone, Copy)]
struct Segment {
    u0: f64,
    v0: f64,
    u1: f64,
    v1: f64,
}

impl Segment {
    /// Where the segment is at `u`, exactly at its ends.
    fn at(&self, u: f64) -> f64 {
        if (u - self.u0).abs() <= KNOT_TOLERANCE {
            self.v0
        } else if (u - self.u1).abs() <= KNOT_TOLERANCE {
            self.v1
        } else {
            self.v0 + (u - self.u0) * (self.v1 - self.v0) / (self.u1 - self.u0)
        }
    }
}

fn on_knots(knots: &[f64], x: f64) -> bool {
    let i = knots.partition_point(|k| *k < x - KNOT_TOLERANCE);
    knots
        .get(i)
        .is_some_and(|k| (*k - x).abs() <= KNOT_TOLERANCE)
}

/// The part of the convex polygon `ring` on one side of `v = at`: `v >= at` where
/// `lower`, else `v <= at` (Sutherland-Hodgman against one line).
fn clip_v(ring: &[[f64; 2]], at: f64, lower: bool) -> Vec<[f64; 2]> {
    let keep = |p: [f64; 2]| if lower { p[1] >= at } else { p[1] <= at };
    let mut out = Vec::new();
    for (&a, &b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if keep(a) {
            out.push(a);
        }
        if keep(a) != keep(b) {
            let t = (at - a[1]) / (b[1] - a[1]);
            out.push([a[0] + t * (b[0] - a[0]), at]);
        }
    }
    out
}

/// The signed area of a polygon.
fn polygon_area(ring: &[[f64; 2]]) -> f64 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| (a[0] * b[1] - b[0] * a[1]) / 2.0)
        .sum()
}

/// Whether `p` is inside `ring` (even-odd).
fn inside_polygon(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut odd = false;
    for (&a, &b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
        {
            odd = !odd;
        }
    }
    odd
}

/// The area the loops enclose, by even-odd: each loop counts for its own area, added
/// when an even number of the other loops contain it and taken away when an odd
/// number do, whichever way it turns.
fn enclosed_area(loops: &[Vec<[f64; 2]>]) -> f64 {
    loops
        .iter()
        .enumerate()
        .map(|(i, ring)| {
            // A vertex that is not on another loop says which loops contain this one.
            let probe = ring
                .iter()
                .copied()
                .find(|p| {
                    loops.iter().enumerate().all(|(j, other)| {
                        j == i
                            || other
                                .iter()
                                .zip(other.iter().cycle().skip(1))
                                .take(other.len())
                                .all(|(a, b)| point_to_segment(*p, *a, *b) > KNOT_TOLERANCE)
                    })
                })
                .unwrap_or(ring[0]);
            let depth = loops
                .iter()
                .enumerate()
                .filter(|(j, other)| *j != i && inside_polygon(other, probe))
                .count();
            let area = polygon_area(ring).abs();
            if depth.is_multiple_of(2) { area } else { -area }
        })
        .sum()
}

fn point_to_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length).clamp(0., 1.)
    } else {
        0.
    };
    (p[0] - (a[0] + t * dx)).hypot(p[1] - (a[1] + t * dy))
}

/// The cells of a face whose trim is not the rectangle of its grid: the region
/// inside `loops` (every loop closed and its last point not repeated; the holes
/// are loops inside the first), cut by the grid's `u_knots` into strips and by
/// `v_knots` across them. Every point of every loop must be on the knots, so
/// that within a strip each trim segment runs from one side to the other and
/// the strip's region is a set of trapezoids between them, which the `v_knots`
/// cut into convex pieces that are fanned into triangles, counter-clockwise in
/// (u, v). The triangles must cover the area the loops enclose, or the loops
/// cross each other or themselves. Returns the points in (u, v) and the
/// triangles (#1706).
fn trimmed_cells(loops: &[Vec<[f64; 2]>], u_knots: &[f64], v_knots: &[f64]) -> Result<Cells> {
    ensure!(
        !loops.is_empty() && loops.iter().all(|ring| ring.len() >= 3),
        "a trim loop needs three points"
    );
    let increasing = |knots: &[f64]| {
        knots.len() >= 2
            && knots.iter().all(|k| k.is_finite())
            && knots.windows(2).all(|w| w[1] > w[0])
    };
    ensure!(
        increasing(u_knots) && increasing(v_knots),
        "knots must be finite and strictly increasing"
    );
    let mut segments = Vec::new();
    for ring in loops {
        for (&a, &b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            ensure!(
                a.iter().chain(&b).all(|x| x.is_finite()),
                "nonfinite trim point"
            );
            ensure!(
                on_knots(u_knots, a[0]) && on_knots(v_knots, a[1]),
                "a trim point is not on the grid's knots"
            );
            // A segment along a strip's edge bounds no strip.
            if (b[0] - a[0]).abs() <= KNOT_TOLERANCE {
                continue;
            }
            let (p, q) = if a[0] < b[0] { (a, b) } else { (b, a) };
            segments.push(Segment {
                u0: p[0],
                v0: p[1],
                u1: q[0],
                v1: q[1],
            });
        }
    }
    let enclosed = enclosed_area(loops);
    ensure!(
        (u_knots.len() - 1).saturating_mul(segments.len()) <= STRIP_BUDGET,
        "trim strip budget exceeded"
    );
    let mut points: Vec<[f64; 2]> = Vec::new();
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut index: std::collections::HashMap<(i64, i64), u32> = std::collections::HashMap::new();
    let mut vertex = |p: [f64; 2], points: &mut Vec<[f64; 2]>| -> Result<u32> {
        let key = ((p[0] * 1e9).round() as i64, (p[1] * 1e9).round() as i64);
        if let Some(&i) = index.get(&key) {
            return Ok(i);
        }
        let i = u32::try_from(points.len()).context("trim vertex count overflow")?;
        points.push(p);
        index.insert(key, i);
        Ok(i)
    };
    let mut covered = 0.0;
    for w in u_knots.windows(2) {
        let (left, right) = (w[0], w[1]);
        // Every point is a knot, so a segment that meets the strip crosses all of it.
        let mut active: Vec<&Segment> = segments
            .iter()
            .filter(|s| s.u0 <= left + KNOT_TOLERANCE && s.u1 >= right - KNOT_TOLERANCE)
            .collect();
        if active.is_empty() {
            continue;
        }
        let middle = (left + right) / 2.0;
        active.sort_by(|a, b| a.at(middle).total_cmp(&b.at(middle)));
        ensure!(
            active.len().is_multiple_of(2),
            "a trim loop is open or crosses itself"
        );
        let mut previous_top = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for pair in active.chunks_exact(2) {
            let (low_left, low_right) = (pair[0].at(left), pair[0].at(right));
            let (high_left, high_right) = (pair[1].at(left), pair[1].at(right));
            ensure!(
                high_left >= low_left - KNOT_TOLERANCE
                    && high_right >= low_right - KNOT_TOLERANCE
                    && low_left >= previous_top.0 - KNOT_TOLERANCE
                    && low_right >= previous_top.1 - KNOT_TOLERANCE,
                "trim segments cross inside a strip"
            );
            previous_top = (high_left, high_right);
            if high_left - low_left <= KNOT_TOLERANCE && high_right - low_right <= KNOT_TOLERANCE {
                continue;
            }
            let trapezoid = [
                [left, low_left],
                [right, low_right],
                [right, high_right],
                [left, high_left],
            ];
            // Cut across at every knot of v inside the trapezoid.
            let low = low_left.min(low_right);
            let high = high_left.max(high_right);
            let mut cuts = vec![low];
            cuts.extend(
                v_knots
                    .iter()
                    .copied()
                    .filter(|v| *v > low + KNOT_TOLERANCE && *v < high - KNOT_TOLERANCE),
            );
            cuts.push(high);
            for band in cuts.windows(2) {
                let mut piece = clip_v(&clip_v(&trapezoid, band[0], true), band[1], false);
                piece.dedup_by(|a, b| (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12);
                if piece.len() > 1 && {
                    let (a, b) = (piece[0], piece[piece.len() - 1]);
                    (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12
                } {
                    piece.pop();
                }
                if piece.len() < 3 {
                    continue;
                }
                let first = vertex(piece[0], &mut points)?;
                for pair in piece[1..].windows(2) {
                    let area = ((pair[0][0] - piece[0][0]) * (pair[1][1] - piece[0][1])
                        - (pair[1][0] - piece[0][0]) * (pair[0][1] - piece[0][1]))
                        / 2.0;
                    if area <= 0.0 {
                        continue;
                    }
                    covered += area;
                    let b = vertex(pair[0], &mut points)?;
                    let c = vertex(pair[1], &mut points)?;
                    triangles.push([first, b, c]);
                }
                ensure!(
                    triangles.len() <= TRIANGLE_BUDGET,
                    "trim triangle count overflow"
                );
            }
        }
    }
    ensure!(
        !triangles.is_empty() && (covered - enclosed).abs() <= 1e-9 * enclosed.abs().max(1e-9),
        "the trim loops cross each other or themselves"
    );
    Ok((points, triangles))
}

/// The face's cells inside its trim loops (`trimmed_cells`), on the surface. The
/// triangles turn the way the grid's do for the same orientation.
fn tessellate_trimmed(
    surface: &ParametricSurface,
    loops: &[Vec<[f64; 2]>],
    u_knots: &[f64],
    v_knots: &[f64],
    orient: bool,
    max_vertices: usize,
) -> Result<ParametricMesh> {
    let (points, cells) = trimmed_cells(loops, u_knots, v_knots)?;
    ensure!(
        points.len() <= max_vertices && points.len() <= u32::MAX as usize,
        "parametric mesh vertex budget exceeded"
    );
    let mut vertices = Vec::with_capacity(points.len());
    let mut normals = Vec::with_capacity(points.len());
    for p in &points {
        vertices.push(surface.evaluate(p[0], p[1])?);
        let mut n = surface.normal(p[0], p[1])?;
        if !orient {
            for value in &mut n {
                *value = -*value;
            }
        }
        normals.push(n);
    }
    let winding = surface.orientation() == orient;
    let mut triangles = Vec::with_capacity(cells.len());
    for [a, b, c] in cells {
        let (p, q, r) = (
            vertices[a as usize],
            vertices[b as usize],
            vertices[c as usize],
        );
        let u = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
        let v = [r[0] - p[0], r[1] - p[1], r[2] - p[2]];
        let area = (u[1] * v[2] - u[2] * v[1])
            .hypot((u[2] * v[0] - u[0] * v[2]).hypot(u[0] * v[1] - u[1] * v[0]));
        if area <= 1e-12 {
            let collapsed = |x: [f64; 3], y: [f64; 3]| {
                let d = [y[0] - x[0], y[1] - x[1], y[2] - x[2]];
                d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= 1e-20
            };
            let dist = |x: [f64; 3], y: [f64; 3]| {
                ((y[0] - x[0]).powi(2) + (y[1] - x[1]).powi(2) + (y[2] - x[2]).powi(2)).sqrt()
            };
            let kind = match surface {
                ParametricSurface::SurfRev { .. } => "SurfRev",
                ParametricSurface::ConeSurf { .. } => "ConeSurf",
                ParametricSurface::RuledSurf { .. } => "RuledSurf",
            };
            // The refusal texts are stored with their digits scrubbed: write them as letters.
            let detail = format!(
                "{kind} area {area:.3e} edges {:.3e} {:.3e} {:.3e} uv {:.9?} {:.9?} {:.9?} of {} cells",
                dist(p, q),
                dist(q, r),
                dist(r, p),
                points[a as usize],
                points[b as usize],
                points[c as usize],
                points.len()
            );
            let letters: String = detail
                .chars()
                .map(|ch| {
                    if ch.is_ascii_digit() {
                        (b'a' + (ch as u8 - b'0')) as char
                    } else {
                        ch
                    }
                })
                .collect();
            ensure!(
                collapsed(p, q) || collapsed(q, r) || collapsed(r, p),
                "DETAIL {letters}"
            );
        } else {
            triangles.push(if winding { [a, b, c] } else { [a, c, b] });
        }
    }
    ensure!(
        !triangles.is_empty(),
        "parametric surface produced no triangles"
    );
    Ok(ParametricMesh {
        vertices,
        normals,
        triangles,
    })
}

#[cfg(test)]
mod tests {
    use super::{inside_polygon, trimmed_cells};

    fn triangle_area(points: &[[f64; 2]], t: &[u32; 3]) -> f64 {
        let (a, b, c) = (
            points[t[0] as usize],
            points[t[1] as usize],
            points[t[2] as usize],
        );
        ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])) / 2.0
    }

    /// Every triangle turns counter-clockwise and lies inside the loops, and their
    /// areas add up to `expected`.
    fn check(
        loops: &[Vec<[f64; 2]>],
        u: &[f64],
        v: &[f64],
        expected: f64,
    ) -> (Vec<[f64; 2]>, Vec<[u32; 3]>) {
        let (points, triangles) = trimmed_cells(loops, u, v).unwrap();
        let mut total = 0.0;
        for t in &triangles {
            assert!(t.iter().all(|i| (*i as usize) < points.len()));
            let area = triangle_area(&points, t);
            assert!(area > 0.0, "{t:?} turns clockwise");
            total += area;
            let middle = [
                (points[t[0] as usize][0] + points[t[1] as usize][0] + points[t[2] as usize][0])
                    / 3.0,
                (points[t[0] as usize][1] + points[t[1] as usize][1] + points[t[2] as usize][1])
                    / 3.0,
            ];
            let inside = loops
                .iter()
                .filter(|ring| inside_polygon(ring, middle))
                .count()
                % 2
                == 1;
            assert!(inside, "{t:?} lies outside the loops at {middle:?}");
        }
        assert!(
            (total - expected).abs() < 1e-9,
            "{total} against {expected}"
        );
        (points, triangles)
    }

    #[test]
    fn an_l_shape_is_covered_once_whichever_way_it_turns() {
        let ccw = vec![vec![
            [0., 0.],
            [2., 0.],
            [2., 1.],
            [1., 1.],
            [1., 2.],
            [0., 2.],
        ]];
        check(&ccw, &[0., 1., 2.], &[0., 1., 2.], 3.0);
        let cw = vec![vec![
            [0., 0.],
            [0., 2.],
            [1., 2.],
            [1., 1.],
            [2., 1.],
            [2., 0.],
        ]];
        check(&cw, &[0., 1., 2.], &[0., 1., 2.], 3.0);
    }

    #[test]
    fn a_hole_is_left_out_whichever_way_it_turns() {
        let outer = vec![[0., 0.], [3., 0.], [3., 3.], [0., 3.]];
        let against = vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]];
        let along = vec![[1., 1.], [2., 1.], [2., 2.], [1., 2.]];
        let knots = [0., 1., 2., 3.];
        check(&[outer.clone(), against], &knots, &knots, 8.0);
        check(&[outer, along], &knots, &knots, 8.0);
        let ring = vec![
            vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
            vec![[2., 1.], [1., 2.], [2., 3.], [3., 2.]],
        ];
        let knots = [0., 1., 2., 3., 4.];
        check(&ring, &knots, &knots, 14.0);
    }

    #[test]
    fn a_diagonal_trim_is_cut_at_the_knots_it_crosses() {
        let triangle = vec![vec![[0., 0.], [2., 0.], [0., 2.]]];
        let (points, _) = check(&triangle, &[0., 1., 2.], &[0., 1., 2.], 2.0);
        // The diagonal crosses u = 1 at v = 1 and v = 1 at u = 1: one point, not two.
        assert!(
            points
                .iter()
                .any(|p| (p[0] - 1.0).abs() < 1e-12 && (p[1] - 1.0).abs() < 1e-12)
        );
    }

    #[test]
    fn a_strip_with_two_bars_gets_two_trapezoids() {
        // A C opening to the right: right of u = 1 the strips hold a bar below and a bar above.
        let c = vec![vec![
            [0., 0.],
            [3., 0.],
            [3., 1.],
            [1., 1.],
            [1., 2.],
            [3., 2.],
            [3., 3.],
            [0., 3.],
        ]];
        check(&c, &[0., 1., 3.], &[0., 1., 2., 3.], 7.0);
    }

    #[test]
    fn two_loops_that_share_an_edge_make_one_rectangle() {
        let two = vec![
            vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            vec![[1., 0.], [2., 0.], [2., 1.], [1., 1.]],
        ];
        check(&two, &[0., 1., 2.], &[0., 1.], 2.0);
    }

    #[test]
    fn a_rectangle_is_cut_at_every_knot() {
        let r = vec![vec![[0., 0.], [3., 0.], [3., 2.], [0., 2.]]];
        let (_, triangles) = check(&r, &[0., 1., 2., 3.], &[0., 1., 2.], 6.0);
        assert_eq!(triangles.len(), 12);
    }

    #[test]
    fn a_loop_that_crosses_itself_is_refused() {
        let bowtie = vec![vec![[0., 0.], [2., 2.], [2., 0.], [0., 2.]]];
        assert!(trimmed_cells(&bowtie, &[0., 2.], &[0., 2.]).is_err());
    }

    #[test]
    fn a_point_off_the_knots_a_nonfinite_point_and_bad_knots_are_refused() {
        let off = vec![vec![[0., 0.], [2., 0.], [2.5, 2.], [0., 2.]]];
        assert!(trimmed_cells(&off, &[0., 2.], &[0., 2.]).is_err());
        let nan = vec![vec![[0., 0.], [1., 0.], [f64::NAN, 1.]]];
        assert!(trimmed_cells(&nan, &[0., 1.], &[0., 1.]).is_err());
        let square = vec![vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]];
        assert!(trimmed_cells(&square, &[0.], &[0., 1.]).is_err());
        assert!(trimmed_cells(&square, &[1., 0.], &[0., 1.]).is_err());
        assert!(trimmed_cells(&[vec![[0., 0.], [1., 0.]]], &[0., 1.], &[0., 1.]).is_err());
        assert!(trimmed_cells(&[], &[0., 1.], &[0., 1.]).is_err());
    }
}
