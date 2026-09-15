//! Bounded UV-grid meshing for decoded parametric surfaces.

use crate::native_parameters::ObjectGraph;
use crate::native_parametric_surface::ParametricSurface;
use crate::native_saved_mesh::FaceMesh;
use anyhow::{Context, Result, ensure};

#[derive(Debug, Clone)]
pub struct ParametricMesh {
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
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
    ensure!(segments.len() >= 2, "too few parametric edges");
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
    let mut closes = false;
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
            closes = true;
            break;
        }
    }
    ensure!(closes, "disconnected parametric trim in model coordinates");
    let trim = segments.iter().flatten().copied().collect::<Vec<_>>();
    let min = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min));
    let max = std::array::from_fn(|k| trim.iter().map(|p| p[k]).fold(f64::NEG_INFINITY, f64::max));
    ensure!(
        max[0] > min[0] && max[1] > min[1],
        "degenerate parametric domain"
    );
    let mut sides: [Vec<[f64; 2]>; 4] = std::array::from_fn(|_| Vec::new());
    for seg in &segments {
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
                anyhow::bail!("nonrectangular parametric trim");
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
        },
        ParametricSurface::RuledSurf { .. } => {
            // A shared normalized grid across ruled patches preserves their
            // common straight generator edges even when profile lengths differ.
            let mut n = 1.0_f64;
            let component = ruled_surface_component(g, fi)?;
            for i in component.into_iter().filter(|i| {
                g.objects
                    .get(*i)
                    .and_then(|face| face.fields.get("m_pSurf"))
                    .is_some()
            }) {
                let Some(surface_index) =
                    crate::native_saved_mesh::pointer(g, i, &g.objects[i].fields["m_pSurf"])?
                else {
                    continue;
                };
                if g.objects[surface_index].class_name != "RuledSurf" {
                    continue;
                }
                let s = ParametricSurface::read(g, surface_index)?;
                let ParametricSurface::RuledSurf {
                    profile1:
                        Curve::Line {
                            direction: a,
                            range: ra,
                            ..
                        },
                    profile2:
                        Curve::Line {
                            direction: b,
                            range: rb,
                            ..
                        },
                    ..
                } = s
                else {
                    anyhow::bail!("nonlinear ruled profiles need a curvature policy")
                };
                let da = std::array::from_fn(|k| a[k] * (ra[1] - ra[0]));
                let db = std::array::from_fn(|k| b[k] * (rb[1] - rb[0]));
                let mixed = length(std::array::from_fn(|k| da[k] - db[k]));
                n = n
                    .max((mixed / CHORD).sqrt().ceil())
                    .max(length(da).ceil())
                    .max(length(db).ceil());
            }
            (1. / n, 1. / n)
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
    })
}

pub fn face(g: &ObjectGraph, fi: usize) -> Result<FaceMesh> {
    let grid = face_grid(g, fi)?;
    let f = &g.objects[fi];
    let flags = f.fields["m_faceFlags_v9"].as_u64().context("face flags")?;
    let mesh = tessellate_uv(
        &grid.surface,
        &grid.u,
        &grid.v,
        grid.bounds,
        flags & 2 == 0,
        2_000_000,
    )?;
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
    ensure!(
        u_knots.first().unwrap() >= &trim[0][0]
            && u_knots.last().unwrap() <= &trim[1][0]
            && v_knots.first().unwrap() >= &trim[0][1]
            && v_knots.last().unwrap() <= &trim[1][1],
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
        .unwrap()
        .checked_mul(v_knots.len().checked_sub(1).unwrap())
        .ok_or_else(|| anyhow::anyhow!("parametric mesh cell count overflow"))?;
    let mut triangles = Vec::with_capacity(
        cells
            .checked_mul(2)
            .ok_or_else(|| anyhow::anyhow!("parametric triangle count overflow"))?,
    );
    let surface_orient = match surface {
        ParametricSurface::SurfRev { orient, .. } | ParametricSurface::RuledSurf { orient, .. } => {
            *orient
        }
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_parameters::{GraphObject, ObjectGraph};
    use crate::native_parametric_surface::{Curve, ParametricSurface};
    use serde_json::json;

    fn pole_surface() -> ParametricSurface {
        ParametricSurface::SurfRev {
            center: [0., 0., 0.],
            x: [1., 0., 0.],
            y: [0., 1., 0.],
            z: [0., 0., 1.],
            envelope: [[0., 0.], [std::f64::consts::TAU, 1.]],
            orient: true,
            profile: Curve::line([0., 0., 1.], [1., 0., -1.], [0., 1.]),
        }
    }

    #[test]
    fn ruled_component_ignores_disconnected_malformed_surface() {
        let object = |class_name: &str, token: u32, fields: serde_json::Value| GraphObject {
            class_tag: 1,
            class_name: class_name.into(),
            token,
            start: 0,
            fields_end: 0,
            fields,
        };
        let mut graph = ObjectGraph {
            consumed_bytes: 1,
            objects: vec![
                object("Face", 10, json!({})),
                object("Face", 11, json!({})),
                object("Face", 12, json!({})),
                object(
                    "Edge",
                    20,
                    json!({"m_pFace":[{"pointer_token":10,"offset":8},{"pointer_token":11,"offset":12}]}),
                ),
                object(
                    "Edge",
                    21,
                    json!({"m_pFace":[{"pointer_token":99,"offset":8},{"pointer_token":100,"offset":12}]}),
                ),
            ],
            // Deliberately empty: these are token-only Edge.m_pFace refs.
            edges: vec![],
        };
        let component = ruled_surface_component(&graph, 0).unwrap();
        assert_eq!(component.into_iter().collect::<Vec<_>>(), vec![0, 1]);
        graph.objects.push(object(
            "Edge",
            22,
            json!({"m_pFace":[{"pointer_token":10,"offset":8},{"pointer_token":999,"offset":12}]}),
        ));
        let error = ruled_surface_component(&graph, 0).unwrap_err();
        assert!(error.to_string().contains("malformed ruled-surface edge"));
    }

    #[test]
    fn pole_collapse_drops_zero_area_triangles_and_keeps_cap() {
        let mesh = tessellate(&pole_surface(), 8, 2, 100).unwrap();
        assert!(!mesh.triangles.is_empty());
        assert!(mesh.triangles.len() < 8 * 2 * 2);
    }

    fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }

    fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }

    fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    #[test]
    fn face_and_surface_orientation_each_change_winding_once() {
        for surface_orient in [true, false] {
            let mut surface = pole_surface();
            if let ParametricSurface::SurfRev { orient, .. } = &mut surface {
                *orient = surface_orient;
            }
            for face_flip in [true, false] {
                let mesh = tessellate_uv(
                    &surface,
                    &[0., 1., 2.],
                    &[0., 0.5, 1.],
                    [[0., 0.], [2., 1.]],
                    face_flip,
                    20,
                )
                .unwrap();
                assert!(!mesh.triangles.is_empty());
                for triangle in &mesh.triangles {
                    let a = mesh.vertices[triangle[0] as usize];
                    let b = mesh.vertices[triangle[1] as usize];
                    let c = mesh.vertices[triangle[2] as usize];
                    let geometric = cross(sub(b, a), sub(c, a));
                    let normal = mesh.normals[triangle[0] as usize];
                    assert!(
                        dot(geometric, normal) > 1e-12,
                        "surface_orient={surface_orient}, face_flip={face_flip}, geometric={geometric:?}, normal={normal:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn trim_and_budget_are_rejected_before_allocation() {
        let surface = pole_surface();
        assert!(
            tessellate_uv(
                &surface,
                &[0., 1.],
                &[0., 1.],
                [[-1., 0.], [2., 1.]],
                true,
                10
            )
            .is_err()
        );
        assert!(tessellate(&surface, 10, 10, 4).is_err());
        assert!(
            tessellate_uv(
                &surface,
                &[0., 0., 1.],
                &[0., 1.],
                [[0., 0.], [1., 1.]],
                true,
                10
            )
            .is_err()
        );
        assert!(
            tessellate_uv(
                &surface,
                &[0., f64::NAN],
                &[0., 1.],
                [[0., 0.], [1., 1.]],
                true,
                10
            )
            .is_err()
        );
        assert!(
            tessellate_uv(
                &surface,
                &[0., 1.],
                &[0., 1.],
                [[0., f64::NAN], [1., 1.]],
                true,
                10
            )
            .is_err()
        );
    }
}
