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

pub(crate) struct FaceGrid {
    pub surface: ParametricSurface,
    pub u: Vec<f64>,
    pub v: Vec<f64>,
    pub trim: Vec<[f64; 2]>,
    pub bounds: [[f64; 2]; 2],
    /// The trim loop in UV, in order, where it does not run round its UV box
    /// (RE-302); `None` for a box, meshed as the grid.
    pub ring: Option<Vec<[f64; 2]>>,
}

// Only complete rectangular boundaries (including analytic collapsed poles)
// are accepted. Merely lying on the rectangle's perimeter is insufficient.
fn ruled_surface_component(
    g: &ObjectGraph,
    start_face: usize,
) -> Result<std::collections::BTreeSet<usize>> {
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
    if let Some(face) = component.iter().find(|face| invalid_by_face.contains(face)) {
        anyhow::bail!("malformed ruled-surface edge is connected to face {face}");
    }
    Ok(component)
}

/// Whether a trim's edges close in model coordinates, and its loop in UV, in
/// order, where it is continuous in UV (no seam).
fn order_loop(
    surface: &ParametricSurface,
    segments: &[Vec<[f64; 2]>],
) -> Result<(bool, Option<Vec<[f64; 2]>>)> {
    let mut closes = false;
    // The loop in UV, in order, where it is continuous in UV (no seam).
    let mut ordered: Option<Vec<[f64; 2]>> = None;
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
        let mut path: Vec<[f64; 2]> = if reverse {
            segments[0].iter().rev().copied().collect()
        } else {
            segments[0].clone()
        };
        let mut continuous = true;
        let mut valid = true;
        for seg in &segments[1..] {
            let p = surface.evaluate(last[0], last[1])?;
            let a = seg[0];
            let b = *seg.last().unwrap();
            let next: Vec<[f64; 2]> = if distance(p, surface.evaluate(a[0], a[1])?) < 1e-7 {
                last = b;
                seg.clone()
            } else if distance(p, surface.evaluate(b[0], b[1])?) < 1e-7 {
                last = a;
                seg.iter().rev().copied().collect()
            } else {
                valid = false;
                break;
            };
            let joint = path.last().copied().unwrap_or(next[0]);
            continuous &=
                (joint[0] - next[0][0]).abs() < 1e-7 && (joint[1] - next[0][1]).abs() < 1e-7;
            path.extend_from_slice(&next[1..]);
        }
        if valid
            && distance(
                surface.evaluate(first[0], first[1])?,
                surface.evaluate(last[0], last[1])?,
            ) < 1e-7
        {
            closes = true;
            if continuous && path.len() >= 4 {
                let (f, l) = (path[0], path[path.len() - 1]);
                if (f[0] - l[0]).abs() < 1e-7 && (f[1] - l[1]).abs() < 1e-7 {
                    path.pop();
                    ordered = Some(path);
                }
            }
            break;
        }
    }
    Ok((closes, ordered))
}

pub(crate) fn face_grid(g: &ObjectGraph, fi: usize) -> Result<FaceGrid> {
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
    let li = pointer(g, fi, &f.fields["m_pFirstLoop"])?.context("face loop")?;
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
    ensure!(
        pointer(g, li, &l.fields["m_nextLoop"])?.is_none(),
        "multiple parametric trim loops unsupported"
    );
    let mut cur = pointer(g, li, &l.fields["m_next"])?.context("empty trim")?;
    let mut seen = std::collections::BTreeSet::new();
    let mut segments = Vec::new();
    let mut edges = Vec::new();
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
        edges.push((cur, side));
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
    ensure!(segments.len() >= 2, "too few parametric edges");
    ensure!(
        segments.iter().all(|s: &Vec<_>| !s.is_empty()),
        "empty parametric edge"
    );
    let env = surface.bounds();
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
    let (closes, ordered) = order_loop(&surface, &segments)?;
    ensure!(closes, "disconnected parametric trim in model coordinates");
    let trim = segments.iter().flatten().copied().collect::<Vec<_>>();
    let min = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let max = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    ensure!(
        max[0] > min[0] && max[1] > min[1],
        "degenerate parametric domain"
    );
    let mut sides: [Vec<[f64; 2]>; 4] = std::array::from_fn(|_| Vec::new());
    let mut rectangular = true;
    'sides: for seg in &segments {
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
                rectangular = false;
                break 'sides;
            };
            sides[side].push([a[axis].min(b[axis]), a[axis].max(b[axis])]);
        }
    }
    // A loop that does not run round its UV box is meshed from its own
    // outline (RE-302), where it is continuous in UV.
    let ring = if rectangular {
        None
    } else {
        Some(ordered.context("nonrectangular parametric trim")?)
    };
    for (side, intervals) in sides.iter_mut().enumerate().filter(|_| rectangular) {
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
            Curve::Ellipse {
                center,
                x_len,
                y_len,
                ..
            } => {
                let radius = x_len.max(*y_len);
                (
                    angular(center[0].hypot(center[1]) + 2. * radius),
                    angular(radius),
                )
            }
        },
        ParametricSurface::ConeSurf { half_angle, .. } => {
            // The generators are straight: only the angle needs a chord bound.
            let far = min[1].abs().max(max[1].abs());
            (angular(far * half_angle.sin()), 1.)
        }
        ParametricSurface::RuledSurf { .. } => {
            // A shared normalized grid across ruled patches preserves their
            // common straight generator edges even when profile lengths differ.
            // Straight profiles take one count for both directions; curved
            // ones (RE-302) take U from their bend and V from their twist.
            let mut n = 1.0_f64;
            let mut n_v = 1.0_f64;
            let component = ruled_surface_component(g, fi)?;
            for i in component.into_iter().filter(|i| {
                g.objects
                    .get(*i)
                    .and_then(|face| face.fields.get("m_pSurf"))
                    .is_some()
            }) {
                let Some(surface_index) = pointer(g, i, &g.objects[i].fields["m_pSurf"])? else {
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
                if let (
                    Curve::Line {
                        direction: a,
                        range: ra,
                        ..
                    },
                    Curve::Line {
                        direction: b,
                        range: rb,
                        ..
                    },
                ) = (&profile1, &profile2)
                {
                    let da = std::array::from_fn(|k| a[k] * (ra[1] - ra[0]));
                    let db = std::array::from_fn(|k| b[k] * (rb[1] - rb[0]));
                    let mixed = length(std::array::from_fn(|k| da[k] - db[k]));
                    let lines = (mixed / CHORD)
                        .sqrt()
                        .ceil()
                        .max(length(da).ceil())
                        .max(length(db).ceil());
                    n = n.max(lines);
                    n_v = n_v.max(lines);
                    continue;
                }
                // A curved profile (an arc or an ellipse) gets as many U steps
                // as keep each chord within CHORD of it at its largest radius.
                // The generators are straight, so V needs steps only for the
                // twist between the two profiles: the largest difference of
                // their tangents over the patch, sampled.
                let turns = |c: &Curve| {
                    c.bend_radius().map_or(0., |radius| {
                        let [a, b] = c.range();
                        ((b - a).abs() / angular(radius)).ceil()
                    })
                };
                let n_u = turns(&profile1)
                    .max(turns(&profile2))
                    .max(profile1.span_length().ceil())
                    .max(profile2.span_length().ceil())
                    .max(1.);
                let [a1, b1] = profile1.range();
                let [a2, b2] = profile2.range();
                let twist = (0..=64)
                    .map(|k| {
                        let s = f64::from(k) / 64.;
                        let d1 = profile1.derivative(a1 + s * (b1 - a1));
                        let d2 = profile2.derivative(a2 + s * (b2 - a2));
                        length(std::array::from_fn(|i| {
                            d1[i] * (b1 - a1) - d2[i] * (b2 - a2)
                        }))
                    })
                    .fold(0., f64::max);
                n = n.max(n_u);
                n_v = n_v.max((twist / (CHORD * n_u)).ceil());
            }
            (1. / n, 1. / n_v)
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
    let u = axis(0, du)?;
    let v = axis(1, dv)?;
    // A trimmed face is cut on the true course of each edge it shares with a
    // plane, its grid kept from the samples (#1247).
    let ring = match ring {
        None => None,
        Some(ring) => {
            let point = |p: [f64; 2]| surface.evaluate(p[0], p[1]);
            let refined = segments
                .iter()
                .zip(&edges)
                .map(|(segment, &(edge, side))| {
                    crate::native_saved_mesh::curved_edge_points(
                        g,
                        edge,
                        side,
                        segment.clone(),
                        [Some(u.as_slice()), Some(v.as_slice())],
                        &point,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            if refined == segments {
                Some(ring)
            } else {
                Some(
                    order_loop(&surface, &refined)?
                        .1
                        .context("nonrectangular parametric trim")?,
                )
            }
        }
    };
    Ok(FaceGrid {
        u,
        v,
        surface,
        trim,
        bounds: [min, max],
        ring,
    })
}

pub fn face(g: &ObjectGraph, fi: usize) -> Result<FaceMesh> {
    let grid = face_grid(g, fi)?;
    let f = &g.objects[fi];
    let flags = f.fields["m_faceFlags_v9"].as_u64().context("face flags")?;
    let mesh = match &grid.ring {
        Some(ring) => tessellate_trimmed(
            &grid.surface,
            &grid.u,
            &grid.v,
            ring,
            flags & 2 == 0,
            4_000_000,
        )?,
        None => tessellate_uv(
            &grid.surface,
            &grid.u,
            &grid.v,
            grid.bounds,
            flags & 2 == 0,
            2_000_000,
        )?,
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

/// The part of `ring` on one side of `p[axis] = at`: at or above it where
/// `lower`, else at or below it (Sutherland–Hodgman against one line), with a
/// vertex a cut repeats dropped.
fn clip_axis(ring: &[[f64; 2]], axis: usize, at: f64, lower: bool) -> Vec<[f64; 2]> {
    let keep = |p: [f64; 2]| if lower { p[axis] >= at } else { p[axis] <= at };
    let mut out: Vec<[f64; 2]> = Vec::new();
    let mut push = |p: [f64; 2]| {
        if out.last() != Some(&p) {
            out.push(p);
        }
    };
    for (&a, &b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        if keep(a) {
            push(a);
        }
        if keep(a) != keep(b) {
            let t = (at - a[axis]) / (b[axis] - a[axis]);
            let mut p = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            p[axis] = at;
            push(p);
        }
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Twice the signed area of the UV triangle `a b c`.
fn cross_uv(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

/// Twice the signed area of a UV polygon.
fn double_area(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum()
}

/// The UV range of `ring` along `axis`.
fn extent(ring: &[[f64; 2]], axis: usize) -> (f64, f64) {
    ring.iter()
        .map(|p| p[axis])
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), x| {
            (lo.min(x), hi.max(x))
        })
}

/// The triangles of one grid cell's piece of a trim loop, as indices into
/// `piece`: a fan where the piece is convex, else earcut.
fn triangulate_piece(piece: &[[f64; 2]], sign: f64) -> Vec<[u32; 3]> {
    let n = piece.len();
    let convex =
        (0..n).all(|i| sign * cross_uv(piece[i], piece[(i + 1) % n], piece[(i + 2) % n]) >= -1e-15);
    if convex {
        return (1..n as u32 - 1).map(|k| [0, k, k + 1]).collect();
    }
    let mut ts = Vec::<u32>::new();
    earcut::Earcut::new().earcut(piece.iter().copied(), &[], &mut ts);
    ts.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect()
}

/// A parametric face trimmed by a loop that is not its UV box (RE-302): the
/// loop is clipped to each grid cell it covers (Sutherland–Hodgman against the
/// cell's four lines, a column of cells first), so each piece lies in one cell
/// of the grid the rectangular path would draw, and is fanned where convex,
/// else earcut. A cell the loop covers whole gives its two triangles, as the
/// rectangular path's, and the pieces' corners are shared across cells. The
/// pieces must cover the loop's own UV area, or the loop is taken as
/// self-intersecting.
pub fn tessellate_trimmed(
    surface: &ParametricSurface,
    u_knots: &[f64],
    v_knots: &[f64],
    ring: &[[f64; 2]],
    orient: bool,
    max_triangles: usize,
) -> Result<ParametricMesh> {
    // Each cell clips a column of the loop: bound the work a hostile file can ask.
    const MAX_CLIP_WORK: usize = 200_000_000;
    ensure!(
        ring.len() >= 3
            && u_knots.len() >= 2
            && v_knots.len() >= 2
            && ring.iter().flatten().all(|x| x.is_finite()),
        "invalid trimmed parametric face"
    );
    let winding = surface.orientation() == orient;
    let expected = double_area(ring).abs() / 2.;
    ensure!(expected > 1e-15, "degenerate parametric trim");
    let mut covered = 0.;
    let mut work = 0usize;
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();
    let mut shared = std::collections::HashMap::<[u64; 2], u32>::new();
    let windows = |knots: &[f64], (lo, hi): (f64, f64)| {
        let first = knots.partition_point(|&k| k <= lo).saturating_sub(1);
        let last = knots.partition_point(|&k| k < hi).min(knots.len() - 1);
        first..last.max(first)
    };
    for i in windows(u_knots, extent(ring, 0)) {
        work = work.saturating_add(ring.len());
        ensure!(
            work <= MAX_CLIP_WORK,
            "parametric trim clip budget exceeded"
        );
        let column = clip_axis(
            &clip_axis(ring, 0, u_knots[i], true),
            0,
            u_knots[i + 1],
            false,
        );
        if column.len() < 3 || double_area(&column).abs() <= 1e-15 {
            continue;
        }
        for j in windows(v_knots, extent(&column, 1)) {
            work = work.saturating_add(column.len());
            ensure!(
                work <= MAX_CLIP_WORK,
                "parametric trim clip budget exceeded"
            );
            let piece = clip_axis(
                &clip_axis(&column, 1, v_knots[j], true),
                1,
                v_knots[j + 1],
                false,
            );
            let double = double_area(&piece);
            if piece.len() < 3 || double.abs() <= 1e-15 {
                continue;
            }
            let mut index = Vec::with_capacity(piece.len());
            for p in &piece {
                let key = [(p[0] + 0.).to_bits(), (p[1] + 0.).to_bits()];
                let at = match shared.get(&key) {
                    Some(&at) => at,
                    None => {
                        let at = u32::try_from(vertices.len())
                            .context("parametric vertex count overflow")?;
                        vertices.push(surface.evaluate(p[0], p[1])?);
                        let mut n = surface.normal(p[0], p[1])?;
                        if !orient {
                            for value in &mut n {
                                *value = -*value;
                            }
                        }
                        normals.push(n);
                        shared.insert(key, at);
                        at
                    }
                };
                index.push(at);
            }
            for t in triangulate_piece(&piece, double.signum()) {
                ensure!(
                    t.iter().all(|&k| (k as usize) < piece.len()),
                    "triangle bounds"
                );
                let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
                let area = cross_uv(piece[a], piece[b], piece[c]);
                if area.abs() <= 1e-15 {
                    continue;
                }
                covered += area.abs() / 2.;
                triangles.push(if (area > 0.) == winding {
                    [index[a], index[b], index[c]]
                } else {
                    [index[a], index[c], index[b]]
                });
            }
            ensure!(
                triangles.len() <= max_triangles,
                "parametric triangle count overflow"
            );
        }
    }
    ensure!(
        !triangles.is_empty() && (covered - expected).abs() <= 1e-6 * expected.max(1e-9),
        "self-intersecting parametric trim"
    );
    Ok(ParametricMesh {
        vertices,
        normals,
        triangles,
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
