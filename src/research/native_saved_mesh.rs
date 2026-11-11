//! Tessellation of bounded saved planar and cylindrical BRep faces. No application or vendor runtime.
use crate::native_parameters::ObjectGraph;
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceMesh {
    pub face_index: usize,
    pub face_tag: i64,
    pub render_style_id: i64,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub normal: [f64; 3],
    #[serde(default)]
    pub normals: Vec<[f64; 3]>,
    #[serde(default)]
    pub analytic_surface: Option<AnalyticCylinder>,
    #[serde(default)]
    pub trim_uv: Vec<[f64; 2]>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticCylinder {
    pub center: [f64; 3],
    pub radius: f64,
    pub x_vec: [f64; 3],
    pub y_vec: [f64; 3],
    pub z_vec: [f64; 3],
    pub orient_flag: bool,
    pub u_range: [f64; 2],
    pub v_range: [f64; 2],
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CylinderTessellationProfile {
    pub chord_error: f64,
    pub max_v_edge: f64,
}
impl Default for CylinderTessellationProfile {
    fn default() -> Self {
        Self {
            chord_error: 0.001,
            max_v_edge: 1.0,
        }
    }
}
struct PointerIndex<'a> {
    graph: &'a ObjectGraph,
    lookup: &'a crate::native_parameters::PointerLookup,
}
impl<'a> PointerIndex<'a> {
    fn new(graph: &'a ObjectGraph) -> Self {
        Self {
            graph,
            lookup: graph.lookup(),
        }
    }
    fn resolve(&self, source: usize, v: &Value) -> Result<Option<usize>> {
        let g = self.graph;
        let t = v["pointer_token"]
            .as_u64()
            .context("missing pointer token")?;
        if t == 0 {
            return Ok(None);
        }
        let off = v["offset"].as_u64().context("pointer offset")? as usize;
        let edges = self
            .lookup
            .edges
            .get(&(source, off))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        if edges.len() == 1 {
            let e = &g.edges[edges[0]];
            let target = g
                .objects
                .get(e.target_object_index)
                .context("pointer target out of bounds")?;
            ensure!(
                e.pointer_token as u64 == t && target.class_tag == e.target_class_tag,
                "pointer edge metadata mismatch"
            );
            if let Some(tag) = v["class_tag"].as_u64() {
                ensure!(
                    tag == target.class_tag as u64,
                    "pointer target class mismatch"
                )
            }
            return Ok(Some(e.target_object_index));
        }
        ensure!(edges.is_empty(), "ambiguous pointer edge");
        ensure!(t != u32::MAX as u64, "unresolved anonymous pointer");
        let objs = self.lookup.tokens.get(&t).map(Vec::as_slice).unwrap_or(&[]);
        ensure!(objs.len() == 1, "unresolved or ambiguous token {t}");
        Ok(Some(objs[0]))
    }
}
pub fn pointer(g: &ObjectGraph, source: usize, v: &Value) -> Result<Option<usize>> {
    PointerIndex::new(g).resolve(source, v)
}
fn point<const N: usize>(v: &Value) -> Result<[f64; N]> {
    let a = v.as_array().context("point array")?;
    ensure!(a.len() == N, "point dimension");
    let mut p = [0.; N];
    for (i, x) in a.iter().enumerate() {
        p[i] = x.as_f64().context("point number")?;
        ensure!(p[i].is_finite(), "nonfinite point")
    }
    Ok(p)
}
/// The angle, in degrees, within which a stored facet normal must lie of its
/// facet's winding normal for [`normals_follow_winding`] (RE-342).
pub const NORMAL_WINDING_TOLERANCE_DEGREES: f64 = 0.1;
/// Whether `normals` holds one stored normal per facet of `facets` and every
/// one lies within [`NORMAL_WINDING_TOLERANCE_DEGREES`] of the winding normal
/// of its facet over `points`. A facet whose corners are collinear or out of
/// range, a stored normal of no length and one that is not a number make it
/// false, and so does a mesh of no facets, which has nothing that follows
/// anything. A mesh that binds its normals per facet (`m_normalsFlag` 2) is
/// read only where this holds (RE-342): the stored normal is never taken on
/// trust.
pub fn normals_follow_winding(
    points: &[[f64; 3]],
    facets: &[[u32; 3]],
    normals: &[[f64; 3]],
) -> bool {
    let limit = NORMAL_WINDING_TOLERANCE_DEGREES.to_radians().cos();
    !facets.is_empty()
        && facets.len() == normals.len()
        && facets.iter().zip(normals).all(|(facet, stored)| {
            let corner = |i: u32| points.get(i as usize).copied();
            let (Some(a), Some(b), Some(c)) =
                (corner(facet[0]), corner(facet[1]), corner(facet[2]))
            else {
                return false;
            };
            let winding = crate::vector_math::cross(
                crate::vector_math::sub(b, a),
                crate::vector_math::sub(c, a),
            );
            let winding_length = crate::vector_math::dot(winding, winding).sqrt();
            let stored_length = crate::vector_math::dot(*stored, *stored).sqrt();
            winding_length > 1e-12
                && stored_length > 1e-9
                && crate::vector_math::dot(winding, *stored) / (winding_length * stored_length)
                    >= limit
        })
}
fn close(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-7
}
fn cross(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn area(r: &[[f64; 2]]) -> f64 {
    (0..r.len())
        .map(|i| r[i][0] * r[(i + 1) % r.len()][1] - r[(i + 1) % r.len()][0] * r[i][1])
        .sum::<f64>()
        / 2.
}
fn retain_boundary_points(
    points: &[[f64; 2]],
    boundary_edges: &BTreeSet<(u32, u32)>,
    triangles: &mut Vec<[u32; 3]>,
) -> Result<()> {
    // Which points a triangle uses, kept as splits add them, instead of a
    // search of every triangle for every point.
    let mut used = vec![false; points.len()];
    for &vertex in triangles.iter().flatten() {
        if let Some(slot) = used.get_mut(vertex as usize) {
            *slot = true;
        }
    }
    for pi in 0..points.len() {
        if used[pi] {
            continue;
        }
        let p = points[pi];
        let mut split = None;
        for (ti, t) in triangles.iter().enumerate() {
            for edge in 0..3 {
                let a = t[edge] as usize;
                let b = t[(edge + 1) % 3] as usize;
                let c = t[(edge + 2) % 3];
                let edge_key = (
                    t[edge].min(t[(edge + 1) % 3]),
                    t[edge].max(t[(edge + 1) % 3]),
                );
                let boundary_line = boundary_edges.contains(&edge_key)
                    || boundary_edges.iter().any(|(u, v)| {
                        cross(points[*u as usize], points[*v as usize], points[a]).abs() <= 1e-8
                            && cross(points[*u as usize], points[*v as usize], points[b]).abs()
                                <= 1e-8
                            && cross(points[*u as usize], points[*v as usize], p).abs() <= 1e-8
                    });
                if !boundary_line {
                    continue;
                }
                let ab = [points[b][0] - points[a][0], points[b][1] - points[a][1]];
                let ap = [p[0] - points[a][0], p[1] - points[a][1]];
                let length2 = ab[0] * ab[0] + ab[1] * ab[1];
                let dot = ap[0] * ab[0] + ap[1] * ab[1];
                if length2 > 1e-24
                    && cross(points[a], points[b], p).abs() <= 1e-8
                    && dot > 1e-12
                    && dot < length2 - 1e-12
                {
                    split = Some((
                        ti,
                        [t[edge], pi as u32, c],
                        [pi as u32, t[(edge + 1) % 3], c],
                    ));
                    break;
                }
            }
            if split.is_some() {
                break;
            }
        }
        // A ring that repeats its first point as its last (a conduit's end
        // cap, its two samples 1e-12 ft apart) leaves the copy out of every
        // triangle: the point is there already, through its twin.
        if split.is_none() && (0..points.len()).any(|j| j != pi && used[j] && close(points[j], p)) {
            used[pi] = true;
            continue;
        }
        let Some((ti, first, second)) = split else {
            bail!("boundary sample was discarded by triangulation")
        };
        triangles[ti] = first;
        triangles.push(second);
        used[pi] = true;
    }
    Ok(())
}

/// How far the chord between two neighbouring points of a crossing edge may leave
/// the edge (feet): a quarter of the 0.001 ft bar the scorers hold meshes to (#1706).
const EDGE_SAGITTA: f64 = 2.5e-4;
/// The most times a span between two samples is halved. A halving takes off most of a
/// chord's deviation (a half, not three quarters, where the point found splits a steep
/// span unevenly), so a smooth edge settles in a dozen; one still above the bound at
/// the limit never settles.
const EDGE_REFINEMENT_DEPTH: u32 = 32;
/// The most points one edge may be given, so that a hostile file cannot ask for more.
const EDGE_POINT_BUDGET: usize = 100_000;

/// What a crossing edge's refinement may spend: halvings of one span, and points in all.
struct EdgeLimits {
    depth: u32,
    points: usize,
}
const EDGE_LIMITS: EdgeLimits = EdgeLimits {
    depth: EDGE_REFINEMENT_DEPTH,
    points: EDGE_POINT_BUDGET,
};
/// How close to the plane (feet) a point solved onto it must lie.
const EDGE_ON_PLANE: f64 = 1e-9;

/// The parameter an edge's samples run along, `0` for `u` and `1` for `v`, where
/// the other is constant to 1e-7 along all of them; None where they change both.
fn isoparametric_axis(samples: &[[f64; 2]]) -> Option<usize> {
    let (start, end) = (*samples.first()?, *samples.last()?);
    let axis = if (start[1] - end[1]).abs() < 1e-7 && (start[0] - end[0]).abs() > 1e-7 {
        0
    } else if (start[0] - end[0]).abs() < 1e-7 && (start[1] - end[1]).abs() > 1e-7 {
        1
    } else {
        return None;
    };
    samples
        .iter()
        .all(|uv| (uv[1 - axis] - start[1 - axis]).abs() < 1e-7)
        .then_some(axis)
}

/// The unit normal of a plane with the basis `x`, `y`.
fn unit_normal(x: [f64; 3], y: [f64; 3]) -> Result<[f64; 3]> {
    let normal = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let length = normal.iter().map(|v| v * v).sum::<f64>().sqrt();
    ensure!(
        length.is_finite() && length > 1e-12,
        "singular shared plane basis"
    );
    Ok(normal.map(|v| v / length))
}

fn point_distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).map(|k| (a[k] - b[k]).powi(2)).sum::<f64>().sqrt()
}

/// How far `p` is from the chord `a`-`b`, in space.
fn distance_to_chord(p: [f64; 3], a: [f64; 3], b: [f64; 3]) -> f64 {
    let ab: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
    let along = (0..3).map(|k| (p[k] - a[k]) * ab[k]).sum::<f64>();
    let length = (0..3).map(|k| ab[k] * ab[k]).sum::<f64>();
    let t = if length > 0. {
        (along / length).clamp(0., 1.)
    } else {
        0.
    };
    point_distance(p, std::array::from_fn(|k| a[k] + t * ab[k]))
}

/// The samples of an edge between a plane and a curved face that runs across both
/// of the curved face's parameters (a slanted plane through a cylinder: an
/// ellipse), with the points of the edge itself that keep the chord between
/// neighbours within [`EDGE_SAGITTA`] of the edge put between them (#1706).
///
/// The samples are the graph's, in the curved face's parameters; `evaluate` is
/// that surface. Between two samples the parameter that changes more is held at
/// its middle value and the other is solved for, by Newton's method, until the
/// point of the surface is within [`EDGE_ON_PLANE`] of the plane: a point of the
/// edge itself. Where it is farther from the chord than the bound it is put
/// between the samples and each half is looked at in turn, to
/// [`EDGE_REFINEMENT_DEPTH`] halvings. The chord is judged against the edge and
/// not against the surface, which a chord leaves by the surface's own curvature
/// wherever the strips of the curved face are cut finer than that, and an
/// interval that holds an extremum of the parameter solved for has the edge
/// outside the samples' own values on both sides. A point that does not reach the
/// plane, or lies farther from the middle of the chord than the chord is long (the
/// other branch of the intersection), or a span that is still above the bound after
/// the last halving, or more points than [`EDGE_POINT_BUDGET`], is an error: the
/// edge keeps being refused as it was. The plane's polygon and the cylinder's loop
/// both take this list (less the points beyond the cylinder's envelope, which both
/// leave out), so their vertices along the edge are the same.
fn refine_crossing_edge(
    evaluate: &dyn Fn([f64; 2]) -> Result<[f64; 3]>,
    plane_origin: [f64; 3],
    plane_normal: [f64; 3],
    samples: &[[f64; 2]],
) -> Result<Vec<[f64; 2]>> {
    refine_crossing_edge_within(evaluate, plane_origin, plane_normal, samples, &EDGE_LIMITS)
}

fn refine_crossing_edge_within(
    evaluate: &dyn Fn([f64; 2]) -> Result<[f64; 3]>,
    plane_origin: [f64; 3],
    plane_normal: [f64; 3],
    samples: &[[f64; 2]],
    limits: &EdgeLimits,
) -> Result<Vec<[f64; 2]>> {
    ensure!(samples.len() >= 2, "an edge needs two samples");
    let signed = |p: [f64; 3]| -> f64 {
        (0..3)
            .map(|k| (p[k] - plane_origin[k]) * plane_normal[k])
            .sum()
    };
    let mut out = vec![samples[0]];
    for pair in samples.windows(2) {
        bisect_crossing_edge(evaluate, &signed, limits, pair[0], pair[1], 0, &mut out)?;
        out.push(pair[1]);
    }
    Ok(out)
}

fn bisect_crossing_edge(
    evaluate: &dyn Fn([f64; 2]) -> Result<[f64; 3]>,
    signed: &dyn Fn([f64; 3]) -> f64,
    limits: &EdgeLimits,
    a: [f64; 2],
    b: [f64; 2],
    depth: u32,
    out: &mut Vec<[f64; 2]>,
) -> Result<()> {
    let (point_a, point_b) = (evaluate(a)?, evaluate(b)?);
    let chord = point_distance(point_a, point_b);
    ensure!(chord.is_finite(), "edge refinement left the surface");
    // Two samples closer than the bound hold the edge between them within it.
    if chord <= EDGE_SAGITTA {
        return Ok(());
    }
    let chord_middle: [f64; 3] = std::array::from_fn(|k| (point_a[k] + point_b[k]) / 2.);
    let middle = [(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.];
    // The parameter that changes more is held at its middle value and the other is
    // solved so that the point is in the plane: the point of the edge there.
    let solved = usize::from((b[0] - a[0]).abs() >= (b[1] - a[1]).abs());
    let along = |x: f64| -> [f64; 2] {
        let mut uv = middle;
        uv[solved] = x;
        uv
    };
    let height = |x: f64| -> Result<f64> { Ok(signed(evaluate(along(x))?)) };
    let mut x = middle[solved];
    let mut reached = false;
    for _ in 0..30 {
        let value = height(x)?;
        if value.abs() <= EDGE_ON_PLANE {
            reached = true;
            break;
        }
        let step = 1e-6 * (1. + x.abs());
        let slope = (height(x + step)? - height(x - step)?) / (2. * step);
        ensure!(
            slope.is_finite() && slope.abs() > 1e-9,
            "edge refinement found no slope"
        );
        x -= value / slope;
        ensure!(x.is_finite(), "edge refinement left the parameters");
    }
    ensure!(reached, "edge refinement did not reach the plane");
    let exact = along(x);
    let point = evaluate(exact)?;
    // The edge between the samples is no farther from the middle of their chord than
    // the chord is long; the other branch of the intersection is.
    ensure!(
        point_distance(point, chord_middle) <= chord + 1e-6,
        "edge refinement left its span"
    );
    let deviation = distance_to_chord(point, point_a, point_b);
    ensure!(deviation.is_finite(), "edge refinement left the surface");
    if deviation <= EDGE_SAGITTA {
        return Ok(());
    }
    ensure!(
        depth < limits.depth,
        "edge refinement is still above the bound after {} halvings",
        limits.depth
    );
    ensure!(
        out.len() < limits.points,
        "edge refinement point budget exceeded"
    );
    bisect_crossing_edge(evaluate, signed, limits, a, exact, depth + 1, out)?;
    out.push(exact);
    bisect_crossing_edge(evaluate, signed, limits, exact, b, depth + 1, out)
}

/// The cylinder side of [`refined_plane_edge`] (#1706): the loop of a cylinder
/// takes along an edge it shares with a plane, where that edge runs across both
/// its parameters, the points [`refine_crossing_edge`] gives, as the plane's
/// polygon does. Any other edge is as the graph stores it.
///
/// An error is not an edge to refuse here: the loop keeps the samples the graph
/// stored, as it did before, and the plane's side, which does refuse, decides
/// whether the element is drawn.
fn refined_curved_edge(
    g: &ObjectGraph,
    index: &PointerIndex<'_>,
    face: usize,
    edge_index: usize,
    side: usize,
    original: &[[f64; 2]],
) -> Result<Vec<[f64; 2]>> {
    if isoparametric_axis(original).is_some() {
        return Ok(original.to_vec());
    }
    let edge = &g.objects[edge_index];
    let faces = edge.fields["m_pFace"]
        .as_array()
        .context("edge face pair")?;
    ensure!(faces.len() == 2, "edge face pair length");
    let Some(other) = index.resolve(edge_index, &faces[1 - side])? else {
        return Ok(original.to_vec());
    };
    let Some(other_surface) = index.resolve(other, &g.objects[other].fields["m_pSurf"])? else {
        return Ok(original.to_vec());
    };
    let Some(own_surface) = index.resolve(face, &g.objects[face].fields["m_pSurf"])? else {
        return Ok(original.to_vec());
    };
    if g.objects[other_surface].class_name != "Plane"
        || g.objects[own_surface].class_name != "CylSurf"
    {
        return Ok(original.to_vec());
    }
    let plane = &g.objects[other_surface];
    let origin = point::<3>(&plane.fields["m_origin"])?;
    let normal = unit_normal(
        point::<3>(&plane.fields["m_xVec"])?,
        point::<3>(&plane.fields["m_yVec"])?,
    )?;
    let cylinder = &g.objects[own_surface];
    let center = point::<3>(&cylinder.fields["m_center"])?;
    let x_vec = point::<3>(&cylinder.fields["m_xVec"])?;
    let y_vec = point::<3>(&cylinder.fields["m_yVec"])?;
    let z_vec = point::<3>(&cylinder.fields["m_zVec"])?;
    let radius = cylinder.fields["m_radius"]
        .as_f64()
        .context("cylinder radius")?;
    let evaluate = move |uv: [f64; 2]| -> Result<[f64; 3]> {
        let (sn, cs) = uv[0].sin_cos();
        Ok(std::array::from_fn(|k| {
            center[k] + radius * (cs * x_vec[k] + sn * y_vec[k]) + uv[1] * z_vec[k]
        }))
    };
    let envelope = cylinder_envelope(&cylinder.fields["m_Envelope"]["m_corners"])?;
    Ok(within_envelope(
        refine_crossing_edge(&evaluate, origin, normal, original)?,
        original,
        envelope,
    ))
}

/// The parameter box of a cylinder surface, from its `m_Envelope` corners.
fn cylinder_envelope(corners: &Value) -> Result<([f64; 2], [f64; 2])> {
    let corners = corners.as_array().context("cylinder envelope")?;
    ensure!(corners.len() == 2, "cylinder envelope bounds");
    Ok((point::<2>(&corners[0])?, point::<2>(&corners[1])?))
}

/// The points of a refined crossing edge that a cylinder's loop and the plane's
/// polygon both take: the samples the graph gave, and the points of the edge
/// inside the cylinder's envelope. The envelope is made from the samples, so a
/// point of the edge between two of them can lie a sagitta beyond it, and
/// `cylinder_face` refuses a loop outside its envelope: both sides leave that
/// point out, so that they hold one list.
fn within_envelope(
    refined: Vec<[f64; 2]>,
    original: &[[f64; 2]],
    (low, high): ([f64; 2], [f64; 2]),
) -> Vec<[f64; 2]> {
    refined
        .into_iter()
        .filter(|uv| {
            original.contains(uv)
                || (0..2).all(|k| uv[k] >= low[k] - 1e-9 && uv[k] <= high[k] + 1e-9)
        })
        .collect()
}

// Refine a particular shared edge in its curved surface's parameter domain.
// The planar side is reconstructed from the same evaluated 3D positions, not
// from interpolation along the old polygon's chords.
fn refined_plane_edge(
    g: &ObjectGraph,
    index: &PointerIndex<'_>,
    face: usize,
    edge_index: usize,
    plane_side: usize,
    original: Vec<[f64; 2]>,
) -> Result<Vec<[f64; 2]>> {
    let edge = &g.objects[edge_index];
    let faces = edge.fields["m_pFace"]
        .as_array()
        .context("edge face pair")?;
    ensure!(faces.len() == 2, "edge face pair length");
    let other_side = 1 - plane_side;
    let Some(other) = index.resolve(edge_index, &faces[other_side])? else {
        return Ok(original);
    };
    let other_surface = index
        .resolve(other, &g.objects[other].fields["m_pSurf"])?
        .context("shared edge surface missing")?;
    let kind = g.objects[other_surface].class_name.as_str();
    if !matches!(kind, "CylSurf" | "SurfRev" | "RuledSurf" | "ConeSurf") {
        return Ok(original);
    }
    type SurfaceEval = Box<dyn Fn([f64; 2]) -> Result<[f64; 3]>>;
    let (u_values, v_values, evaluate): (Vec<f64>, Vec<f64>, SurfaceEval) = if kind == "CylSurf" {
        let profile = CylinderTessellationProfile::default();
        let mesh = cylinder_face(g, other, profile)?;
        let cylinder = mesh
            .analytic_surface
            .context("cylinder analytic metadata missing")?;
        let min: [f64; 2] = std::array::from_fn(|k| {
            mesh.trim_uv
                .iter()
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min)
        });
        let max: [f64; 2] = std::array::from_fn(|k| {
            mesh.trim_uv
                .iter()
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let step = (4.
            * (profile.chord_error / (2. * cylinder.radius))
                .min(1.)
                .sqrt()
                .asin())
        .min(std::f64::consts::FRAC_PI_2);
        let us = tessellation_axis(min[0], max[0], step, mesh.trim_uv.iter().map(|p| p[0]))?;
        let vs = tessellation_axis(
            min[1],
            max[1],
            profile.max_v_edge,
            mesh.trim_uv.iter().map(|p| p[1]),
        )?;
        let eval = Box::new(move |uv: [f64; 2]| {
            let (sn, cs) = uv[0].sin_cos();
            Ok(std::array::from_fn(|k| {
                cylinder.center[k]
                    + cylinder.radius * (cs * cylinder.x_vec[k] + sn * cylinder.y_vec[k])
                    + uv[1] * cylinder.z_vec[k]
            }))
        });
        (us, vs, eval)
    } else {
        let grid = crate::native_parametric_mesh::face_grid(g, other)?;
        (
            grid.u,
            grid.v,
            Box::new(move |uv: [f64; 2]| grid.surface.evaluate(uv[0], uv[1])),
        )
    };
    let ends = edge.fields["m_firstAndLastEdgePnts"]
        .as_array()
        .context("edge endpoints")?;
    ensure!(ends.len() == 2, "edge endpoint count");
    let mut samples = vec![point::<2>(&ends[0]["uv"][other_side])?];
    for sample in edge.fields["m_interiorEdgePnts"]
        .as_array()
        .context("edge interior samples")?
    {
        samples.push(point::<2>(&sample["uv"][other_side])?);
    }
    samples.push(point::<2>(&ends[1]["uv"][other_side])?);
    let start = samples[0];
    let end = *samples.last().unwrap();
    let axis = isoparametric_axis(&samples);
    let plane_index = index
        .resolve(face, &g.objects[face].fields["m_pSurf"])?
        .context("plane surface missing")?;
    let plane = &g.objects[plane_index];
    ensure!(
        plane.class_name == "Plane",
        "edge refinement requires a plane"
    );
    let origin = point::<3>(&plane.fields["m_origin"])?;
    let x = point::<3>(&plane.fields["m_xVec"])?;
    let y = point::<3>(&plane.fields["m_yVec"])?;
    let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(a, b)| a * b).sum::<f64>();
    let xx = dot(x, x);
    let xy = dot(x, y);
    let yy = dot(y, y);
    let det = xx * yy - xy * xy;
    ensure!(
        det.is_finite() && det > 1e-24,
        "singular shared plane basis"
    );
    let project = |uv: [f64; 2]| -> Result<[f64; 2]> {
        let q = evaluate(uv)?;
        let d = std::array::from_fn(|k| q[k] - origin[k]);
        let dx = dot(d, x);
        let dy = dot(d, y);
        let p = [(dx * yy - dy * xy) / det, (dy * xx - dx * xy) / det];
        let residual = (0..3)
            .map(|k| (origin[k] + p[0] * x[k] + p[1] * y[k] - q[k]).powi(2))
            .sum::<f64>();
        ensure!(
            residual <= 1e-14,
            "shared curved edge leaves adjacent plane"
        );
        Ok(p)
    };
    ensure!(
        samples.len() == original.len(),
        "shared edge sample count mismatch"
    );
    for (uv, expected) in samples.iter().zip(&original) {
        ensure!(
            close(project(*uv)?, *expected),
            "shared edge surface samples disagree"
        );
    }
    let Some(axis) = axis else {
        // An edge that runs across both parameters of the curved face (a slanted
        // plane through a cylinder is an ellipse in its parameters) has no grid
        // line to follow: the polygon takes the points of the edge itself that
        // keep its chords within the bar of the surface, the same points the
        // curved face's loop takes (#1706).
        let normal = unit_normal(x, y)?;
        let mut refined = refine_crossing_edge(evaluate.as_ref(), origin, normal, &samples)?;
        if kind == "CylSurf" {
            // The cylinder's loop leaves out the points beyond its envelope, and so
            // does the polygon, so that the two hold one list.
            refined = within_envelope(
                refined,
                &samples,
                cylinder_envelope(&g.objects[other_surface].fields["m_Envelope"]["m_corners"])?,
            );
        }
        return refined.into_iter().map(project).collect();
    };
    let knots = if axis == 0 { u_values } else { v_values };
    let lo = start[axis].min(end[axis]);
    let hi = start[axis].max(end[axis]);
    let mut values = knots
        .into_iter()
        .filter(|v| *v > lo + 1e-12 && *v < hi - 1e-12)
        .collect::<Vec<_>>();
    values.insert(0, lo);
    values.push(hi);
    if start[axis] > end[axis] {
        values.reverse();
    }
    values
        .into_iter()
        .map(|t| {
            let mut uv = start;
            uv[axis] = t;
            project(uv)
        })
        .collect()
}
fn inside(p: [f64; 2], r: &[[f64; 2]]) -> bool {
    let mut hit = false;
    for i in 0..r.len() {
        let a = r[i];
        let b = r[(i + 1) % r.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            hit = !hit
        }
    }
    hit
}
/// Which side of a plane-to-cylinder edge a loop's samples are refined on (#1706).
#[derive(Clone, Copy, PartialEq, Eq)]
enum EdgeSide {
    /// The loop of a plane: an edge it shares with a curved face follows that
    /// face's grid ([`refined_plane_edge`]).
    Plane,
    /// The loop of a cylinder: an edge it shares with a plane that crosses it
    /// takes the points the plane's side takes ([`refined_curved_edge`]).
    Cylinder,
}
fn ring(
    g: &ObjectGraph,
    index: &PointerIndex<'_>,
    face: usize,
    li: usize,
) -> Result<Vec<[f64; 2]>> {
    ring_impl(g, index, face, li, EdgeSide::Cylinder)
}
fn ring_impl(
    g: &ObjectGraph,
    index: &PointerIndex<'_>,
    face: usize,
    li: usize,
    refine: EdgeSide,
) -> Result<Vec<[f64; 2]>> {
    let l = &g.objects[li];
    ensure!(
        ["EdgeLoop", "EdgeLoopWithChainEnvelopes"].contains(&l.class_name.as_str()),
        "unsupported loop {}",
        l.class_name
    );
    ensure!(l.fields["m_open"] == false, "open face loop");
    ensure!(
        index.resolve(li, &l.fields["m_pFace"])? == Some(face),
        "loop owner mismatch"
    );
    let mut cur = index
        .resolve(li, &l.fields["m_next"])?
        .context("empty loop")?;
    let mut seen = BTreeSet::new();
    let mut segments = Vec::new();
    while cur != li {
        ensure!(seen.insert(cur), "edge chain cycle before loop sentinel");
        let e = &g.objects[cur];
        ensure!(e.class_name == "Edge", "nonedge loop member");
        let faces = e.fields["m_pFace"].as_array().context("edge face pair")?;
        let sides: Vec<_> = faces
            .iter()
            .enumerate()
            .filter_map(|(i, p)| match index.resolve(cur, p) {
                Ok(Some(f)) if f == face => Some(Ok(i)),
                Err(e) => Some(Err(e)),
                _ => None,
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(sides.len() == 1, "ambiguous edge face side");
        let side = sides[0];
        let endpoints = e.fields["m_firstAndLastEdgePnts"]
            .as_array()
            .context("edge endpoints")?;
        ensure!(endpoints.len() == 2, "edge endpoint count");
        let mut pts = vec![point::<2>(&endpoints[0]["uv"][side])?];
        for p in e.fields["m_interiorEdgePnts"]
            .as_array()
            .context("edge interior points")?
        {
            pts.push(point::<2>(&p["uv"][side])?)
        }
        pts.push(point::<2>(&endpoints[1]["uv"][side])?);
        pts = match refine {
            EdgeSide::Plane => refined_plane_edge(g, index, face, cur, side, pts)?,
            EdgeSide::Cylinder => {
                refined_curved_edge(g, index, face, cur, side, &pts).unwrap_or(pts)
            }
        };
        segments.push(pts);
        cur = index
            .resolve(cur, &e.fields["m_next"][side])?
            .context("null next edge")?;
    }
    // Saved circular loops can be represented by two connected arc edges
    // (typically two semicircles). A third edge is not a topology invariant;
    // closure, nonzero area, and connected ordering below remain mandatory.
    ensure!(segments.len() >= 2, "too few boundary edges");
    ensure!(
        segments.iter().all(|s: &Vec<_>| !s.is_empty()),
        "empty boundary edge"
    );
    let mut output = None;
    for reverse in [false, true] {
        let mut out = segments[0].clone();
        if reverse {
            out.reverse()
        };
        let mut valid = true;
        for s in &segments[1..] {
            let last = *out.last().unwrap();
            if close(last, s[0]) {
                out.extend_from_slice(&s[1..])
            } else if close(last, *s.last().unwrap()) {
                out.extend(s.iter().rev().skip(1).copied())
            } else {
                valid = false;
                break;
            }
        }
        if valid && close(out[0], *out.last().unwrap()) {
            out.pop();
            output = Some(out);
            break;
        }
    }
    let mut out = output.context("disconnected UV boundary")?;
    out.dedup_by(|a, b| close(*a, *b));
    ensure!(
        out.len() >= 3 && area(&out).abs() > 1e-12,
        "degenerate boundary"
    );
    Ok(out)
}

fn tessellation_axis(
    min: f64,
    max: f64,
    step: f64,
    knots: impl Iterator<Item = f64>,
) -> Result<Vec<f64>> {
    ensure!(
        min.is_finite() && max.is_finite() && step.is_finite() && step > 0. && max > min,
        "invalid tessellation axis"
    );
    let count = ((max - min) / step).ceil();
    ensure!(
        count.is_finite() && (1. ..=2_000_000.).contains(&count),
        "tessellation axis too large"
    );
    let n = count as usize;
    let mut values = (0..=n)
        .map(|i| min + (max - min) * i as f64 / n as f64)
        .collect::<Vec<_>>();
    values.extend(knots.filter(|v| v.is_finite() && *v > min && *v < max));
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    ensure!(values.len() >= 2, "degenerate tessellation axis");
    Ok(values)
}
const FACE_REGIONS: &str = "face regions need subdivision";
/// The most triangle pairs `plane_regions_tile` clips for one face.
const REGION_WORK: usize = 4_000_000;
type Triangle = [[f64; 2]; 3];
fn segment_distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let length = dx * dx + dy * dy;
    let t = if length > 0. {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / length).clamp(0., 1.)
    } else {
        0.
    };
    (p[0] - (a[0] + t * dx)).hypot(p[1] - (a[1] + t * dy))
}
/// Whether `p` lies inside `outline` or within `tolerance` (UV units) of it.
fn inside_or_on(p: [f64; 2], outline: &[[f64; 2]], tolerance: f64) -> bool {
    inside(p, outline)
        || (0..outline.len())
            .any(|i| segment_distance(p, outline[i], outline[(i + 1) % outline.len()]) <= tolerance)
}
/// `outline` as counter-clockwise triangles whose areas add up to its own, or
/// an error where they do not (a ring that crosses itself).
fn triangles_of(outline: &[[f64; 2]]) -> Result<Vec<Triangle>> {
    let holes: Vec<u32> = Vec::new();
    let mut indices = Vec::<u32>::new();
    earcut::Earcut::new().earcut(outline.iter().copied(), &holes, &mut indices);
    let mut triangles = Vec::new();
    let mut sum = 0.;
    for t in indices.chunks_exact(3) {
        ensure!(
            t.iter().all(|&k| (k as usize) < outline.len()),
            FACE_REGIONS
        );
        let mut triangle = [
            outline[t[0] as usize],
            outline[t[1] as usize],
            outline[t[2] as usize],
        ];
        let twice = cross(triangle[0], triangle[1], triangle[2]);
        if twice.abs() <= 1e-14 {
            continue;
        }
        if twice < 0. {
            triangle.swap(1, 2);
        }
        sum += twice.abs() / 2.;
        triangles.push(triangle);
    }
    let whole = area(outline).abs();
    ensure!(
        !triangles.is_empty() && (sum - whole).abs() <= 1e-7 * whole.max(1e-6),
        FACE_REGIONS
    );
    Ok(triangles)
}
/// The area `subject` shares with the counter-clockwise triangle `clip`:
/// Sutherland–Hodgman against its three sides.
fn shared_area(subject: &Triangle, clip: &Triangle) -> f64 {
    let mut polygon: Vec<[f64; 2]> = subject.to_vec();
    for (i, &a) in clip.iter().enumerate() {
        let b = clip[(i + 1) % 3];
        let side = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let input = std::mem::take(&mut polygon);
        for (j, &p) in input.iter().enumerate() {
            let q = input[(j + 1) % input.len()];
            let (sp, sq) = (side(p), side(q));
            if sp >= 0. {
                polygon.push(p);
            }
            if (sp >= 0.) != (sq >= 0.) {
                let t = sp / (sp - sq);
                polygon.push([p[0] + t * (q[0] - p[0]), p[1] + t * (q[1] - p[1])]);
            }
        }
        if polygon.len() < 3 {
            return 0.;
        }
    }
    area(&polygon).abs()
}
/// The area two sets of triangles share, in UV units squared, counting the
/// triangle pairs clipped in `work`.
fn shared_area_of(a: &[Triangle], b: &[Triangle], work: &mut usize) -> Result<f64> {
    *work = work.saturating_add(a.len().saturating_mul(b.len()));
    ensure!(*work <= REGION_WORK, FACE_REGIONS);
    Ok(a.iter()
        .map(|s| b.iter().map(|c| shared_area(s, c)).sum::<f64>())
        .sum())
}
/// Whether `regions`, rings in the parent's UV plane, tile `parent`, a single
/// ring (RE-329). `tolerance` is a length in UV units and, times the parent's
/// area, the tolerance on every area compared. Every region vertex lies inside
/// the parent or within `tolerance` of its outline; every region's area is
/// inside the parent (the area it shares with the parent's triangles equals
/// its own, which also refuses an edge that crosses a notch of a non-convex
/// parent); no two regions share area (so no region is listed twice); and the
/// regions' areas add up to the parent's. The overlap is measured as an area,
/// both outlines triangulated and each pair of triangles clipped, not by a
/// centroid test, because two non-convex tiles can overlap with neither
/// centroid inside the other, and not by bounding boxes, which say nothing of
/// a notch. A ring that crosses itself, or more regions or triangle pairs than
/// the bounds allow, does not tile.
fn regions_tile(parent: &[[f64; 2]], regions: &[&[[f64; 2]]], tolerance: f64) -> bool {
    if regions.len() > 256 {
        return false;
    }
    let parent_area = area(parent).abs();
    let area_tolerance = tolerance * parent_area.max(1e-6);
    let Ok(parent_triangles) = triangles_of(parent) else {
        return false;
    };
    let mut tiles: Vec<Vec<Triangle>> = Vec::new();
    let mut covered = 0.;
    let mut work = 0usize;
    for outline in regions {
        if !outline.iter().all(|&p| inside_or_on(p, parent, tolerance)) {
            return false;
        }
        let Ok(tile) = triangles_of(outline) else {
            return false;
        };
        let tile_area = area(outline).abs();
        let Ok(inside_area) = shared_area_of(&tile, &parent_triangles, &mut work) else {
            return false;
        };
        if (inside_area - tile_area).abs() > area_tolerance {
            return false;
        }
        covered += tile_area;
        tiles.push(tile);
    }
    for (i, a) in tiles.iter().enumerate() {
        for b in &tiles[i + 1..] {
            match shared_area_of(a, b, &mut work) {
                Ok(shared) if shared <= area_tolerance => {}
                _ => return false,
            }
        }
    }
    (covered - parent_area).abs() <= area_tolerance
}
/// A plane face whose regions are further plane faces on the same plane, each
/// with one loop and no regions of its own, listed once, that tile its single
/// outline (`regions_tile`) only has its area split (RE-329): the face is
/// drawn whole. Anything else keeps the face refused.
fn plane_regions_tile(
    g: &ObjectGraph,
    index: &PointerIndex<'_>,
    fi: usize,
    plane: usize,
    regions: &[Value],
) -> Result<()> {
    let basis = |si: usize| -> Result<[[f64; 3]; 3]> {
        let s = &g.objects[si];
        Ok([
            point::<3>(&s.fields["m_origin"])?,
            point::<3>(&s.fields["m_xVec"])?,
            point::<3>(&s.fields["m_yVec"])?,
        ])
    };
    let single_ring = |face: usize| -> Result<Vec<[f64; 2]>> {
        let li = index
            .resolve(face, &g.objects[face].fields["m_pFirstLoop"])?
            .context(FACE_REGIONS)?;
        ensure!(
            index
                .resolve(li, &g.objects[li].fields["m_nextLoop"])?
                .is_none(),
            FACE_REGIONS
        );
        ring(g, index, face, li)
    };
    let parent_basis = basis(plane)?;
    let parent = single_ring(fi)?;
    ensure!(regions.len() <= 256, FACE_REGIONS);
    let mut listed = BTreeSet::new();
    let mut outlines: Vec<Vec<[f64; 2]>> = Vec::new();
    for value in regions {
        let ri = index.resolve(fi, value)?.context(FACE_REGIONS)?;
        ensure!(listed.insert(ri), FACE_REGIONS);
        let region = &g.objects[ri];
        ensure!(
            region.class_name == "Face"
                && region.fields["m_faceRegions"]
                    .as_array()
                    .is_some_and(Vec::is_empty),
            FACE_REGIONS
        );
        let si = index
            .resolve(ri, &region.fields["m_pSurf"])?
            .context(FACE_REGIONS)?;
        ensure!(g.objects[si].class_name == "Plane", FACE_REGIONS);
        ensure!(
            basis(si)?
                .iter()
                .flatten()
                .zip(parent_basis.iter().flatten())
                .all(|(a, b)| (a - b).abs() <= 1e-9),
            FACE_REGIONS
        );
        outlines.push(single_ring(ri)?);
    }
    let rings: Vec<&[[f64; 2]]> = outlines.iter().map(Vec::as_slice).collect();
    ensure!(regions_tile(&parent, &rings, 1e-7), FACE_REGIONS);
    Ok(())
}
/// Returns no mesh for a face with no trimming boundary (an auxiliary surface).
pub fn face(g: &ObjectGraph, fi: usize) -> Result<Option<FaceMesh>> {
    face_indexed(g, &PointerIndex::new(g), fi)
}
fn face_indexed(g: &ObjectGraph, index: &PointerIndex<'_>, fi: usize) -> Result<Option<FaceMesh>> {
    let f = g.objects.get(fi).context("face index")?;
    ensure!(f.class_name == "Face", "not Face");
    let Some(mut li) = index.resolve(fi, &f.fields["m_pFirstLoop"])? else {
        return Ok(None);
    };
    let regions = f.fields["m_faceRegions"]
        .as_array()
        .context("face regions")?;
    let pi = index
        .resolve(fi, &f.fields["m_pSurf"])?
        .context("face surface")?;
    let p = &g.objects[pi];
    if p.class_name == "CylSurf" {
        return Ok(Some(cylinder_face(
            g,
            fi,
            CylinderTessellationProfile::default(),
        )?));
    }
    if matches!(p.class_name.as_str(), "SurfRev" | "RuledSurf" | "ConeSurf") {
        return Ok(Some(crate::native_parametric_mesh::face(g, fi)?));
    }
    ensure!(
        p.class_name == "Plane",
        "unsupported surface {}",
        p.class_name
    );
    if !regions.is_empty() {
        plane_regions_tile(g, index, fi, pi, regions)?;
    }
    let origin = point::<3>(&p.fields["m_origin"])?;
    let x = point::<3>(&p.fields["m_xVec"])?;
    let y = point::<3>(&p.fields["m_yVec"])?;
    let orient = p.fields["m_orientFlag"]
        .as_bool()
        .context("plane orientation")?
        ^ (f.fields["m_faceFlags_v9"].as_u64().context("face flags")? & 2 != 0);
    let mut normal = [
        x[1] * y[2] - x[2] * y[1],
        x[2] * y[0] - x[0] * y[2],
        x[0] * y[1] - x[1] * y[0],
    ];
    let norm = normal.iter().map(|v| v * v).sum::<f64>().sqrt();
    ensure!(norm > 1e-12, "singular plane");
    for v in &mut normal {
        *v /= if orient { norm } else { -norm }
    }
    let mut loops = Vec::new();
    let mut seen = BTreeSet::new();
    loop {
        ensure!(seen.insert(li), "loop-list cycle");
        loops.push(ring_impl(g, index, fi, li, EdgeSide::Plane)?);
        match index.resolve(li, &g.objects[li].fields["m_nextLoop"])? {
            Some(next) => li = next,
            None => break,
        }
    }
    // A face can contain multiple disconnected outer rings, with nested holes.
    let depths: Vec<_> = loops
        .iter()
        .enumerate()
        .map(|(i, r)| {
            loops
                .iter()
                .enumerate()
                .filter(|(j, s)| *j != i && inside(r[0], s))
                .count()
        })
        .collect();
    let mut vertices = Vec::new();
    let mut triangles = Vec::new();
    for (i, outer) in loops
        .iter()
        .enumerate()
        .filter(|(i, _)| depths[*i] % 2 == 0)
    {
        let mut pts = outer.clone();
        let mut holes = Vec::new();
        let mut boundary_edges = BTreeSet::new();
        let mut add_boundary_edges = |start: usize, end: usize| {
            for k in start..end {
                let a = k as u32;
                let b = (if k + 1 == end { start } else { k + 1 }) as u32;
                boundary_edges.insert((a.min(b), a.max(b)));
            }
        };
        add_boundary_edges(0, pts.len());
        let mut expected = area(outer).abs();
        for (j, h) in loops.iter().enumerate() {
            if depths[j] == depths[i] + 1 && inside(h[0], outer) {
                holes.push(pts.len() as u32);
                let start = pts.len();
                pts.extend_from_slice(h);
                add_boundary_edges(start, pts.len());
                expected -= area(h).abs()
            }
        }
        let mut ts = Vec::<u32>::new();
        earcut::Earcut::new().earcut(pts.iter().copied(), &holes, &mut ts);
        ensure!(
            !ts.is_empty() && ts.len().is_multiple_of(3),
            "triangulation failed"
        );
        let mut earcut_triangles = ts
            .chunks_exact(3)
            .map(|t| [t[0], t[1], t[2]])
            .collect::<Vec<_>>();
        ensure!(
            earcut_triangles
                .iter()
                .flatten()
                .all(|i| (*i as usize) < pts.len()),
            "triangle bounds"
        );
        // Earcut can retain collinear points only in zero-area triangles. Drop
        // those first so boundary restoration does not mistake them for usable
        // incident triangles and subsequently leave a T-junction behind.
        earcut_triangles.retain(|t| {
            cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]).abs() > 1e-12
        });
        retain_boundary_points(&pts, &boundary_edges, &mut earcut_triangles)?;
        let mut sum = 0.;
        let base = vertices.len() as u32;
        for t in &mut earcut_triangles {
            ensure!(
                t.iter().all(|v| (*v as usize) < pts.len()),
                "triangle bounds"
            );
            let a = cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
            sum += a.abs() / 2.;
            if a.abs() <= 1e-12 {
                continue;
            }
            if (a > 0.) != orient {
                t.swap(1, 2)
            }
            triangles.push([base + t[0], base + t[1], base + t[2]])
        }
        if (sum - expected).abs() > 1e-7 * expected.max(1.) {
            // DIAGNOSTIC (scratch branch, not for main): why the area check fails (#1732).
            let tag = f.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1);
            let summary: Vec<String> = loops
                .iter()
                .enumerate()
                .map(|(j, l)| format!("{j}:n{} a{:.6e} d{}", l.len(), area(l), depths[j]))
                .collect();
            let hole_loops: Vec<&Vec<[f64; 2]>> = loops
                .iter()
                .enumerate()
                .filter(|(j, h)| depths[*j] == depths[i] + 1 && inside(h[0], outer))
                .map(|(_, h)| h)
                .collect();
            let mut touch_outer = 0usize;
            let mut outside_outer = 0usize;
            let mut touch_holes = 0usize;
            for (a, h) in hole_loops.iter().enumerate() {
                for v in h.iter() {
                    if outer.iter().any(|o| close(*o, *v)) {
                        touch_outer += 1;
                    }
                    if !inside(*v, outer) {
                        outside_outer += 1;
                    }
                    for (b, g) in hole_loops.iter().enumerate() {
                        if a < b && g.iter().any(|o| close(*o, *v)) {
                            touch_holes += 1;
                        }
                    }
                }
            }
            let bbox = outer
                .iter()
                .fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |m, p| {
                    [
                        m[0].min(p[0]),
                        m[1].min(p[1]),
                        m[2].max(p[0]),
                        m[3].max(p[1]),
                    ]
                });
            eprintln!(
                "AREADIAG fi={fi} tag={tag} outer={i} loops=[{}] holes={} pts={} earcut_tris={} kept_tris={} sum={sum:e} expected={expected:e} diff={:e} touch_outer={touch_outer} outside_outer={outside_outer} touch_holes={touch_holes} bbox={bbox:?}",
                summary.join("; "),
                holes.len(),
                pts.len(),
                ts.len() / 3,
                earcut_triangles.len(),
                sum - expected,
            );
        }
        ensure!(
            (sum - expected).abs() <= 1e-7 * expected.max(1.),
            "triangulated area mismatch: {sum} vs {expected}"
        );
        for uv in pts {
            vertices.push(std::array::from_fn(|k| {
                origin[k] + uv[0] * x[k] + uv[1] * y[k]
            }))
        }
    }
    if triangles.is_empty() {
        bail!("no bounded triangles")
    }
    let vertex_count = vertices.len();
    Ok(Some(FaceMesh {
        face_index: fi,
        face_tag: f.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
        render_style_id: crate::native_metadata::identifier(&f.fields["m_renderStyleId"])
            .context("render style id")?,
        vertices,
        triangles,
        normal,
        normals: vec![normal; vertex_count],
        analytic_surface: None,
        trim_uv: Vec::new(),
    }))
}

/// Tessellate one Face whose saved surface is a bounded cylindrical surface.
/// The analytic surface and UV trim are retained alongside the mesh.
pub fn cylinder_face(
    g: &ObjectGraph,
    fi: usize,
    profile: CylinderTessellationProfile,
) -> Result<FaceMesh> {
    ensure!(
        profile.chord_error.is_finite() && profile.chord_error > 0.,
        "invalid cylinder chord error"
    );
    ensure!(
        profile.max_v_edge.is_finite() && profile.max_v_edge > 0.,
        "invalid cylinder V edge"
    );
    let index = PointerIndex::new(g);
    let face = g.objects.get(fi).context("face index")?;
    ensure!(face.class_name == "Face", "not Face");
    ensure!(
        face.fields["m_faceRegions"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "face regions need subdivision"
    );
    let surface_index = index
        .resolve(fi, &face.fields["m_pSurf"])?
        .context("face surface")?;
    let surface = &g.objects[surface_index];
    ensure!(
        surface.class_name == "CylSurf",
        "unsupported surface {}",
        surface.class_name
    );
    let center = point::<3>(&surface.fields["m_center"])?;
    let radius = surface.fields["m_radius"]
        .as_f64()
        .context("cylinder radius")?;
    ensure!(radius.is_finite() && radius > 0., "invalid cylinder radius");
    let x_vec = point::<3>(&surface.fields["m_xVec"])?;
    let y_vec = point::<3>(&surface.fields["m_yVec"])?;
    let z_vec = point::<3>(&surface.fields["m_zVec"])?;
    let dot = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
    let norm = |a: [f64; 3]| dot(a, a).sqrt();
    ensure!(
        (norm(x_vec) - 1.).abs() < 1e-7
            && (norm(y_vec) - 1.).abs() < 1e-7
            && (norm(z_vec) - 1.).abs() < 1e-7,
        "cylinder basis not unit"
    );
    ensure!(
        dot(x_vec, y_vec).abs() < 1e-7
            && dot(x_vec, z_vec).abs() < 1e-7
            && dot(y_vec, z_vec).abs() < 1e-7,
        "cylinder basis not orthogonal"
    );
    let cross_xy = [
        x_vec[1] * y_vec[2] - x_vec[2] * y_vec[1],
        x_vec[2] * y_vec[0] - x_vec[0] * y_vec[2],
        x_vec[0] * y_vec[1] - x_vec[1] * y_vec[0],
    ];
    // A left-handed basis is the same cylinder: only the surface's own
    // normal, ∂P/∂u × ∂P/∂v as on a plane, turns inward (#1247).
    let handed = if dot(cross_xy, z_vec) > 0. { 1. } else { -1. };
    let envelope = surface.fields["m_Envelope"]["m_corners"]
        .as_array()
        .context("cylinder envelope")?;
    ensure!(envelope.len() == 2, "cylinder envelope bounds");
    let u_range = [
        envelope[0][0].as_f64().context("u min")?,
        envelope[1][0].as_f64().context("u max")?,
    ];
    let v_range = [
        envelope[0][1].as_f64().context("v min")?,
        envelope[1][1].as_f64().context("v max")?,
    ];
    ensure!(
        u_range.iter().chain(v_range.iter()).all(|v| v.is_finite()),
        "nonfinite cylinder envelope"
    );
    ensure!(
        u_range[1] > u_range[0]
            && u_range[1] - u_range[0] <= std::f64::consts::TAU + 1e-7
            && v_range[1] > v_range[0],
        "invalid cylinder envelope or seam crossing"
    );
    let loop_index = index
        .resolve(fi, &face.fields["m_pFirstLoop"])?
        .context("cylinder trim loop")?;
    ensure!(
        index
            .resolve(loop_index, &g.objects[loop_index].fields["m_nextLoop"])?
            .is_none(),
        "multiple cylinder trim loops unsupported"
    );
    let trim_uv = ring(g, &index, fi, loop_index)?;
    ensure!(!trim_uv.is_empty(), "empty cylinder trim");
    let mut trim_min = [f64::INFINITY; 2];
    let mut trim_max = [f64::NEG_INFINITY; 2];
    for uv in &trim_uv {
        for k in 0..2 {
            trim_min[k] = trim_min[k].min(uv[k]);
            trim_max[k] = trim_max[k].max(uv[k]);
        }
    }
    ensure!(
        (trim_min[0] - trim_max[0]).abs() > 1e-12 && (trim_min[1] - trim_max[1]).abs() > 1e-12,
        "degenerate cylinder trim"
    );
    let tol = 1e-7;
    for (a, b) in trim_uv
        .iter()
        .zip(trim_uv.iter().cycle().skip(1))
        .take(trim_uv.len())
    {
        ensure!(
            a[0] >= u_range[0] - tol
                && a[0] <= u_range[1] + tol
                && a[1] >= v_range[0] - tol
                && a[1] <= v_range[1] + tol
                && b[0] >= u_range[0] - tol
                && b[0] <= u_range[1] + tol
                && b[1] >= v_range[0] - tol
                && b[1] <= v_range[1] + tol,
            "cylinder trim outside envelope"
        );
        ensure!(
            (b[0] - a[0]).hypot(b[1] - a[1]) > tol,
            "repeated cylinder trim vertex"
        );
    }
    // A loop that runs round its UV box, side by side, is meshed as a grid;
    // any other is cut into strips across U (#1247).
    let rectangular = trim_uv.len() >= 4 && {
        let mut covered = [0.; 4];
        let mut perimeter = 0.;
        let mut on_sides = true;
        for (a, b) in trim_uv
            .iter()
            .zip(trim_uv.iter().cycle().skip(1))
            .take(trim_uv.len())
        {
            let du = b[0] - a[0];
            let dv = b[1] - a[1];
            let length = du.hypot(dv);
            let side = if dv.abs() < tol && (a[1] - trim_min[1]).abs() < tol {
                Some(0)
            } else if du.abs() < tol && (a[0] - trim_max[0]).abs() < tol {
                Some(1)
            } else if dv.abs() < tol && (a[1] - trim_max[1]).abs() < tol {
                Some(2)
            } else if du.abs() < tol && (a[0] - trim_min[0]).abs() < tol {
                Some(3)
            } else {
                None
            };
            let Some(side) = side else {
                on_sides = false;
                break;
            };
            covered[side] += length;
            perimeter += length;
        }
        let expected_perimeter = 2. * ((trim_max[0] - trim_min[0]) + (trim_max[1] - trim_min[1]));
        on_sides
            && (perimeter - expected_perimeter).abs() <= tol * expected_perimeter.max(1.)
            && (covered[0] - (trim_max[0] - trim_min[0])).abs() <= tol
            && (covered[1] - (trim_max[1] - trim_min[1])).abs() <= tol
            && (covered[2] - (trim_max[0] - trim_min[0])).abs() <= tol
            && (covered[3] - (trim_max[1] - trim_min[1])).abs() <= tol
    };
    // Keep a full-turn trim from collapsing to coincident seam vertices when a
    // deliberately coarse profile is supplied. This is a geometric validity
    // bound, independent of the analytic chord-error policy.
    let sagitta_ratio = (profile.chord_error / (2. * radius)).min(1.);
    let angular_step = (4. * sagitta_ratio.sqrt().asin()).min(std::f64::consts::FRAC_PI_2);
    ensure!(
        angular_step.is_finite() && angular_step > 0.,
        "cylinder tessellation precision underflow"
    );
    let u_values = tessellation_axis(
        trim_min[0],
        trim_max[0],
        angular_step,
        trim_uv.iter().map(|p| p[0]),
    )
    .context("cylinder tessellation grid too large")?;
    let v_values = tessellation_axis(
        trim_min[1],
        trim_max[1],
        profile.max_v_edge,
        trim_uv.iter().map(|p| p[1]),
    )
    .context("cylinder tessellation grid too large")?;
    const MAX_GRID_VERTICES: usize = 2_000_000;
    ensure!(
        u_values.len() <= MAX_GRID_VERTICES && v_values.len() <= MAX_GRID_VERTICES,
        "cylinder tessellation grid too large"
    );
    let nu = u_values.len() - 1;
    let nv = v_values.len() - 1;
    let grid_vertices = (nu + 1)
        .checked_mul(nv + 1)
        .context("cylinder vertex count overflow")?;
    let grid_triangles = nu
        .checked_mul(nv)
        .and_then(|v| v.checked_mul(2))
        .context("cylinder triangle count overflow")?;
    ensure!(
        grid_vertices <= MAX_GRID_VERTICES && grid_triangles <= MAX_GRID_VERTICES * 2,
        "cylinder tessellation grid too large"
    );
    let orient = surface.fields["m_orientFlag"]
        .as_bool()
        .context("cylinder orientation")?
        ^ (face.fields["m_faceFlags_v9"]
            .as_u64()
            .context("face flags")?
            & 2
            != 0);
    let eval = |u: f64, v: f64| {
        let (s, c) = u.sin_cos();
        [
            center[0] + radius * (c * x_vec[0] + s * y_vec[0]) + v * z_vec[0],
            center[1] + radius * (c * x_vec[1] + s * y_vec[1]) + v * z_vec[1],
            center[2] + radius * (c * x_vec[2] + s * y_vec[2]) + v * z_vec[2],
        ]
    };
    let analytic_surface = Some(AnalyticCylinder {
        center,
        radius,
        x_vec,
        y_vec,
        z_vec,
        orient_flag: orient,
        u_range,
        v_range,
    });
    if !rectangular {
        let (vertices, normals, triangles) = strip_mesh(
            &trim_uv,
            &u_values,
            radius,
            orient,
            handed,
            eval,
            [x_vec, y_vec],
        )?;
        return Ok(FaceMesh {
            face_index: fi,
            face_tag: face.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
            render_style_id: crate::native_metadata::identifier(&face.fields["m_renderStyleId"])?,
            vertices,
            triangles,
            normal: normals[0],
            normals,
            analytic_surface,
            trim_uv,
        });
    }
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    for &v in v_values.iter().take(nv + 1) {
        for &u in u_values.iter().take(nu + 1) {
            vertices.push(eval(u, v));
            let (s, c) = u.sin_cos();
            let sign = if orient { handed } else { -handed };
            let n = [
                sign * (c * x_vec[0] + s * y_vec[0]),
                sign * (c * x_vec[1] + s * y_vec[1]),
                sign * (c * x_vec[2] + s * y_vec[2]),
            ];
            normals.push(n);
        }
    }
    let mut triangles = Vec::new();
    let row = nu + 1;
    for j in 0..nv {
        for i in 0..nu {
            let a = (j * row + i) as u32;
            let b = a + 1;
            let c = a + row as u32;
            let d = c + 1;
            if orient {
                triangles.extend([[a, b, d], [a, d, c]]);
            } else {
                triangles.extend([[a, d, b], [a, c, d]]);
            }
        }
    }
    Ok(FaceMesh {
        face_index: fi,
        face_tag: face.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
        render_style_id: crate::native_metadata::identifier(&face.fields["m_renderStyleId"])?,
        vertices,
        triangles,
        normal: normals[0],
        normals,
        analytic_surface,
        trim_uv,
    })
}

/// Vertices, normals and triangles of a cylinder face.
type CylinderStrips = (Vec<[f64; 3]>, Vec<[f64; 3]>, Vec<[u32; 3]>);

/// A cylinder face whose trim loop is not its UV box: the loop is clipped to
/// strips across U no wider than the grid's angular step, each strip earcut in
/// arc-length coordinates, so every edge stays within the chord bound while V,
/// along which the cylinder is straight, needs no division. The strips must
/// cover the loop's own area, or the loop is taken as self-intersecting.
fn strip_mesh(
    trim_uv: &[[f64; 2]],
    u_values: &[f64],
    radius: f64,
    orient: bool,
    handed: f64,
    eval: impl Fn(f64, f64) -> [f64; 3],
    [x_vec, y_vec]: [[f64; 3]; 2],
) -> Result<CylinderStrips> {
    // Each strip clips the whole loop: bound the work a hostile file can ask.
    ensure!(
        u_values.len().saturating_mul(trim_uv.len()) <= 20_000_000,
        "cylinder trim strip budget exceeded"
    );
    let sign = if orient { handed } else { -handed };
    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut triangles = Vec::new();
    let mut covered = 0.;
    for w in u_values.windows(2) {
        let piece = clip_u(&clip_u(trim_uv, w[0], true), w[1], false);
        if piece.len() < 3 {
            continue;
        }
        let pts: Vec<[f64; 2]> = piece.iter().map(|p| [p[0] * radius, p[1]]).collect();
        let mut ts = Vec::<u32>::new();
        earcut::Earcut::new().earcut(pts.iter().copied(), &[], &mut ts);
        let base = u32::try_from(vertices.len()).context("cylinder vertex count overflow")?;
        for p in &piece {
            vertices.push(eval(p[0], p[1]));
            let (s, c) = p[0].sin_cos();
            normals.push([
                sign * (c * x_vec[0] + s * y_vec[0]),
                sign * (c * x_vec[1] + s * y_vec[1]),
                sign * (c * x_vec[2] + s * y_vec[2]),
            ]);
        }
        for t in ts.chunks_exact(3) {
            ensure!(
                t.iter().all(|&i| (i as usize) < pts.len()),
                "triangle bounds"
            );
            let a = cross(pts[t[0] as usize], pts[t[1] as usize], pts[t[2] as usize]);
            if a.abs() <= 1e-12 {
                continue;
            }
            covered += a.abs() / 2.;
            let t = if (a > 0.) == orient {
                [t[0], t[1], t[2]]
            } else {
                [t[0], t[2], t[1]]
            };
            triangles.push([base + t[0], base + t[1], base + t[2]]);
        }
        ensure!(
            triangles.len() <= 4_000_000,
            "cylinder triangle count overflow"
        );
    }
    let scaled: Vec<[f64; 2]> = trim_uv.iter().map(|p| [p[0] * radius, p[1]]).collect();
    let expected = area(&scaled).abs();
    ensure!(
        !triangles.is_empty() && (covered - expected).abs() <= 1e-6 * expected.max(1e-6),
        "self-intersecting cylinder trim"
    );
    Ok((vertices, normals, triangles))
}

/// The part of `ring` on one side of `u = at`: `u >= at` where `lower`, else
/// `u <= at` (Sutherland–Hodgman against one line).
fn clip_u(ring: &[[f64; 2]], at: f64, lower: bool) -> Vec<[f64; 2]> {
    let keep = |p: [f64; 2]| if lower { p[0] >= at } else { p[0] <= at };
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
            let t = (at - a[0]) / (b[0] - a[0]);
            out.push([at, a[1] + t * (b[1] - a[1])]);
        }
    }
    out
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Primitive {
    pub source_owner_id: Option<u64>,
    pub object_index: usize,
    pub face_tag: i64,
    pub render_style_id: i64,
    pub material_id: Option<i64>,
    pub vertices: Vec<[f64; 3]>,
    pub normals: Vec<[f64; 3]>,
    pub triangles: Vec<[u32; 3]>,
    /// Whether a branch flagged not visible lies on the way to it. Only a
    /// decode that includes hidden branches ([`graphics_including_hidden`])
    /// holds any (RE-318).
    #[serde(default, skip_serializing_if = "is_false")]
    pub hidden: bool,
    /// Whether a filter that rejects the context lies on the way to it. Only
    /// a decode that includes filtered branches ([`graphics_including_filtered`])
    /// holds any (RE-318).
    #[serde(default, skip_serializing_if = "is_false")]
    pub filtered: bool,
}
fn is_false(value: &bool) -> bool {
    !*value
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct GraphicsMeshes {
    pub primitives: Vec<Primitive>,
    pub diagnostics: Vec<String>,
    pub unbounded_faces: usize,
    /// Boundary-empty cutout records recognized by an observed saved profile.
    /// Their source records remain in the native graph; no surface is invented.
    #[serde(default)]
    pub empty_trim_faces: usize,
    pub rejected_filters: usize,
    #[serde(default)]
    pub excluded_visibility_branches: usize,
    #[serde(default)]
    pub excluded_non_surface_branches: usize,
    #[serde(default)]
    pub profile_observations: Vec<String>,
}
/// Decode selected saved graphics. Positions and normals are in the owning
/// document frame; length units remain Revit internal feet.
///
/// This and the other entry points but [`graphics_pre2023`] refuse an
/// `InstanceInfo` with no `m_GRepId`, as they did before RE-342 (#1436 let
/// every route read it as the default representation; RE-342 section 9.8
/// narrowed that to [`graphics_pre2023`]). This is research API, outside the
/// semver promise.
pub fn graphics(g: &ObjectGraph) -> GraphicsMeshes {
    graphics_with_resolver(g, &|_| None)
}
pub fn graphics_with_resolver<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
) -> GraphicsMeshes {
    graphics_with_resolver_at_detail(g, resolver, 3)
}
/// Decode the explicit saved3D detail representation (native levels1,2,3).
pub fn graphics_with_resolver_at_detail<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
) -> GraphicsMeshes {
    graphics_impl(g, resolver, detail_level, false, false, false)
}
/// [`graphics_with_resolver_at_detail`] for the graphics of a file of Revit
/// 2015 to 2022, the releases no export reads saved graphics of: a
/// `FacetedTopology9` under normal binding 2 is read there where its stored
/// normals follow the winding ([`normals_follow_winding`], RE-342), a set
/// `m_bHasScale` on an `InstanceInfo` of the older schema with a rotation
/// matrix is walked through, and so is an `InstanceInfo` with no `m_GRepId`
/// (exactly `m_Trf`, `m_symbolId` and `m_cda`), which is the default
/// representation there (`native_graphics_traversal`). Every other reader of
/// this module refuses all three, and so does this one on any other release's
/// graphics, which the caller decides by the file's release. A research
/// route: `include_hidden` and `include_filtered` are as in
/// [`graphics_including_hidden`] and [`graphics_including_filtered`].
pub fn graphics_pre2023<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
    include_hidden: bool,
    include_filtered: bool,
) -> GraphicsMeshes {
    graphics_impl(
        g,
        resolver,
        detail_level,
        include_hidden,
        include_filtered,
        true,
    )
}
/// [`graphics_with_resolver_at_detail`], also decoding the branches whose
/// visibility flags hide them; each [`Primitive`] one lies on the way to is
/// marked [`Primitive::hidden`] (RE-318).
pub fn graphics_including_hidden<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
) -> GraphicsMeshes {
    graphics_impl(g, resolver, detail_level, true, false, false)
}
/// [`graphics_including_hidden`], also decoding the children of a filter
/// whose conditions reject the context or are not read; each [`Primitive`]
/// one lies on the way to is marked [`Primitive::filtered`] (RE-318).
pub fn graphics_including_filtered<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
) -> GraphicsMeshes {
    graphics_impl(g, resolver, detail_level, true, true, false)
}
fn graphics_impl<'a>(
    g: &'a ObjectGraph,
    resolver: &dyn Fn(u64) -> Option<&'a ObjectGraph>,
    detail_level: i64,
    include_hidden: bool,
    include_filtered: bool,
    pre2023: bool,
) -> GraphicsMeshes {
    let selection = if pre2023 {
        crate::native_graphics_traversal::select_graphics_pre2023_at_detail(
            g,
            resolver,
            detail_level,
            include_hidden || include_filtered,
            include_filtered,
        )
    } else if include_filtered {
        crate::native_graphics_traversal::select_graphics_including_filtered_at_detail(
            g,
            resolver,
            detail_level,
        )
    } else if include_hidden {
        crate::native_graphics_traversal::select_graphics_including_hidden_at_detail(
            g,
            resolver,
            detail_level,
        )
    } else {
        crate::native_graphics_traversal::select_graphics_with_resolver_at_detail(
            g,
            resolver,
            detail_level,
        )
    };
    let mut out = GraphicsMeshes {
        diagnostics: selection
            .diagnostics
            .iter()
            .map(|d| format!("object {}: {}", d.object_index, d.message))
            .collect(),
        rejected_filters: selection.rejected_filters,
        excluded_visibility_branches: selection.excluded_visibility_branches,
        excluded_non_surface_branches: selection.excluded_non_surface_branches,
        profile_observations: selection.profile_observations,
        ..Default::default()
    };
    let mut pointer_indices = BTreeMap::new();
    for selected in selection.selected {
        let g = match selected.source_owner_id {
            None => g,
            Some(id) => match resolver(id) {
                Some(graph) => graph,
                None => {
                    out.diagnostics
                        .push(format!("resolved graphics owner {id} disappeared"));
                    continue;
                }
            },
        };
        let index = pointer_indices
            .entry(selected.source_owner_id)
            .or_insert_with(|| PointerIndex::new(g));
        let oi = selected.object_index;
        let obj = &g.objects[oi];
        let mut decoded = Vec::new();
        let result = (|| -> Result<()> {
            if obj.class_name == "Geometry" {
                for ptr in obj.fields["m_pFaces"]
                    .as_array()
                    .context("geometry faces")?
                {
                    let fi = index.resolve(oi, ptr)?.context("null geometry face")?;
                    match face_indexed(g, index, fi)? {
                        Some(m) => {
                            let normals = if m.normals.len() == m.vertices.len() {
                                m.normals.clone()
                            } else {
                                vec![m.normal; m.vertices.len()]
                            };
                            decoded.push(Primitive {
                                source_owner_id: selected.source_owner_id,
                                object_index: oi,
                                face_tag: m.face_tag,
                                render_style_id: m.render_style_id,
                                material_id: None,
                                vertices: m.vertices,
                                normals,
                                triangles: m.triangles,
                                hidden: selected.hidden,
                                filtered: selected.filtered,
                            })
                        }
                        None => {
                            if crate::native_empty_faces::is_observed_empty_trim_face(g, fi) {
                                out.empty_trim_faces += 1;
                            } else {
                                out.unbounded_faces += 1;
                            }
                        }
                    }
                }
            } else if obj.class_name == "GPolyMesh" {
                let ti = index
                    .resolve(oi, &obj.fields["m_pFacetedTopology"])?
                    .context("faceted topology")?;
                let topology = &g.objects[ti];
                // FacetedTopology9 derives from FacetedTopology1, which declares
                // what FacetedTopology0 does (both derive from
                // FloatFacetedTopology and hold `m_facetsArr`), and adds one
                // more array, `m_edgeVisFlagsArr`, which is not read (RE-342).
                // The same fields serve: points, normals and facets. It is read
                // only in the graphics of a file of Revit 2015 to 2022
                // (`pre2023`, `graphics_pre2023`: no export reads those), under
                // normal binding 2 only, and only where every stored normal
                // follows the winding; every other class, binding and release
                // stays refused.
                let topology_class = topology.class_name.as_str();
                ensure!(
                    topology_class == "FacetedTopology0"
                        || (pre2023 && topology_class == "FacetedTopology9"),
                    "unsupported faceted topology"
                );
                let f = &topology.fields;
                let vertices = f["m_pointsArr"]
                    .as_array()
                    .context("mesh points")?
                    .iter()
                    .map(point::<3>)
                    .collect::<Result<Vec<_>>>()?;
                let normals = f["m_normalsArr"]
                    .as_array()
                    .context("mesh normals")?
                    .iter()
                    .map(point::<3>)
                    .collect::<Result<Vec<_>>>()?;
                ensure!(
                    matches!(
                        (topology_class, f["m_normalsFlag"].as_u64()),
                        ("FacetedTopology0", Some(0)) | ("FacetedTopology9", Some(2))
                    ),
                    "unsupported normal binding"
                );
                let triangles = f["m_facetsArr"]
                    .as_array()
                    .context("mesh facets")?
                    .iter()
                    .map(|v| -> Result<[u32; 3]> {
                        let a = v.as_array().context("facet array")?;
                        ensure!(a.len() == 3, "nontriangle facet");
                        let mut t = [0; 3];
                        for i in 0..3 {
                            t[i] = u32::try_from(a[i].as_u64().context("facet index")?)?;
                            ensure!((t[i] as usize) < vertices.len(), "facet index bounds")
                        }
                        Ok(t)
                    })
                    .collect::<Result<Vec<_>>>()?;
                if topology_class == "FacetedTopology9" {
                    ensure!(
                        normals_follow_winding(&vertices, &triangles, &normals),
                        "stored normals do not follow the winding"
                    );
                } else {
                    ensure!(
                        normals.is_empty() || normals.len() == triangles.len(),
                        "unsupported normal cardinality"
                    );
                }
                let facet_normals = !normals.is_empty();

                let mut expanded_vertices = Vec::new();
                let mut expanded_normals = Vec::new();
                let mut expanded_triangles = Vec::new();
                for (i, t) in triangles.iter().enumerate() {
                    let base = expanded_vertices.len() as u32;
                    let normal = if facet_normals {
                        normals[i]
                    } else {
                        let a = vertices[t[0] as usize];
                        let b = vertices[t[1] as usize];
                        let c = vertices[t[2] as usize];
                        let u: [f64; 3] = std::array::from_fn(|k| b[k] - a[k]);
                        let v: [f64; 3] = std::array::from_fn(|k| c[k] - a[k]);
                        [
                            u[1] * v[2] - u[2] * v[1],
                            u[2] * v[0] - u[0] * v[2],
                            u[0] * v[1] - u[1] * v[0],
                        ]
                    };
                    for idx in t {
                        expanded_vertices.push(vertices[*idx as usize]);
                        expanded_normals.push(normal);
                    }
                    expanded_triangles.push([base, base + 1, base + 2]);
                }
                let (vertices, normals, triangles) =
                    (expanded_vertices, expanded_normals, expanded_triangles);
                decoded.push(Primitive {
                    source_owner_id: selected.source_owner_id,
                    object_index: oi,
                    face_tag: obj.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
                    render_style_id: crate::native_metadata::identifier(
                        &obj.fields["m_interiorGStyleID"],
                    )
                    .context("polymesh style")?,
                    material_id: Some(
                        crate::native_metadata::identifier(&obj.fields["m_materialID"])
                            .context("polymesh material")?,
                    )
                    .filter(|v| *v >= 0),
                    vertices,
                    normals,
                    triangles,
                    hidden: selected.hidden,
                    filtered: selected.filtered,
                })
            }
            for mesh in &mut decoded {
                transform(mesh, selected.world_transform)?
            }
            Ok(())
        })();
        match result {
            Ok(()) => out.primitives.extend(decoded),
            Err(e) => out.diagnostics.push(format!("object {oi}: {e:#}")),
        }
    }
    out
}
fn transform(p: &mut Primitive, m: [[f64; 4]; 4]) -> Result<()> {
    let a = [m[0][0], m[1][0], m[2][0]];
    let b = [m[0][1], m[1][1], m[2][1]];
    let c = [m[0][2], m[1][2], m[2][2]];
    let cp = |a: [f64; 3], b: [f64; 3]| {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    };
    let bc = cp(b, c);
    let ca = cp(c, a);
    let ab = cp(a, b);
    let det = (0..3).map(|i| a[i] * bc[i]).sum::<f64>();
    ensure!(
        det.is_finite() && det.abs() > 1e-12,
        "singular graphics transform"
    );
    for v in &mut p.vertices {
        let old = *v;
        *v = std::array::from_fn(|i| m[i][3] + (0..3).map(|j| m[i][j] * old[j]).sum::<f64>());
        ensure!(
            v.iter().all(|x| x.is_finite()),
            "nonfinite transformed vertex"
        )
    }
    for n in &mut p.normals {
        let old = *n;
        *n = std::array::from_fn(|i| (bc[i] * old[0] + ca[i] * old[1] + ab[i] * old[2]) / det);
        let len = n.iter().map(|v| v * v).sum::<f64>().sqrt();
        ensure!(len > 1e-12 && len.is_finite(), "invalid transformed normal");
        for v in n {
            *v /= len
        }
    }
    if det < 0. {
        for t in &mut p.triangles {
            t.swap(1, 2)
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        EDGE_SAGITTA, EdgeLimits, distance_to_chord, isoparametric_axis, normals_follow_winding,
        refine_crossing_edge, refine_crossing_edge_within, regions_tile, within_envelope,
    };
    use anyhow::Result;
    use std::f64::consts::{FRAC_PI_2, PI};

    const TOLERANCE: f64 = 1e-7;

    // One facet whose winding normal is +z.
    fn follows(normal: [f64; 3]) -> bool {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        normals_follow_winding(&points, &[[0, 1, 2]], &[normal])
    }

    fn tilted(degrees: f64) -> [f64; 3] {
        let radians = degrees.to_radians();
        [radians.sin(), 0., radians.cos()]
    }

    #[test]
    fn a_stored_normal_opposite_to_the_winding_is_refused() {
        assert!(!follows([0., 0., -1.]));
    }

    #[test]
    fn a_stored_normal_tilted_past_the_tolerance_is_refused() {
        assert!(!follows(tilted(0.2)));
        assert!(!follows(tilted(-0.2)));
    }

    #[test]
    fn a_stored_normal_inside_the_tolerance_is_kept_whatever_its_length() {
        assert!(follows([0., 0., 1.]));
        assert!(follows([0., 0., 2.]));
        assert!(follows(tilted(0.05)));
    }

    // Its direction is the winding's exactly: only the length guard refuses it.
    #[test]
    fn a_stored_normal_too_short_to_have_a_direction_is_refused() {
        assert!(!follows([0., 0., 1e-10]));
        assert!(follows([0., 0., 1e-8]));
    }

    #[test]
    fn a_stored_normal_of_no_length_or_not_a_number_is_refused() {
        assert!(!follows([0., 0., 0.]));
        assert!(!follows([f64::NAN, 0., 1.]));
        assert!(!follows([f64::INFINITY, 0., 1.]));
    }

    #[test]
    fn a_facet_with_collinear_corners_has_no_winding_to_follow() {
        let points = [[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]];
        assert!(!normals_follow_winding(
            &points,
            &[[0, 1, 2]],
            &[[0., 0., 1.]]
        ));
    }

    #[test]
    fn a_facet_with_a_corner_out_of_range_is_refused() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        assert!(!normals_follow_winding(
            &points,
            &[[0, 1, 3]],
            &[[0., 0., 1.]]
        ));
    }

    #[test]
    fn fewer_normals_than_facets_are_refused() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        assert!(!normals_follow_winding(&points, &[[0, 1, 2]], &[]));
    }

    #[test]
    fn more_normals_than_facets_are_refused() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        assert!(!normals_follow_winding(
            &points,
            &[[0, 1, 2]],
            &[[0., 0., 1.], [0., 0., 1.]]
        ));
    }

    #[test]
    fn a_mesh_of_no_facets_follows_nothing() {
        assert!(!normals_follow_winding(&[], &[], &[]));
        assert!(!normals_follow_winding(&[[0., 0., 0.]], &[], &[]));
    }

    #[test]
    fn a_corner_that_is_not_finite_is_refused() {
        for bad in [f64::NAN, f64::INFINITY] {
            let points = [[0., 0., 0.], [1., 0., 0.], [0., bad, 0.]];
            assert!(!normals_follow_winding(
                &points,
                &[[0, 1, 2]],
                &[[0., 0., 1.]]
            ));
        }
    }

    #[test]
    fn a_facet_that_repeats_a_corner_has_no_winding_to_follow() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        for facet in [[0, 0, 1], [0, 1, 1], [2, 1, 2]] {
            assert!(!normals_follow_winding(&points, &[facet], &[[0., 0., 1.]]));
        }
    }

    // The winding normal of a facet with legs of `side` feet is `side * side`
    // long; below 1e-12 it is no direction at all.
    #[test]
    fn a_facet_too_small_to_have_a_winding_direction_is_refused() {
        let facet = |side: f64| {
            let points = [[0., 0., 0.], [side, 0., 0.], [0., side, 0.]];
            normals_follow_winding(&points, &[[0, 1, 2]], &[[0., 0., 1.]])
        };
        assert!(!facet(1e-7));
        assert!(facet(1e-5));
    }

    #[test]
    fn the_tolerance_is_a_tenth_of_a_degree() {
        assert!(follows(tilted(0.099)));
        assert!(!follows(tilted(0.101)));
    }

    #[test]
    fn one_bad_facet_among_good_ones_refuses_the_mesh() {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [1., 1., 0.]];
        let facets = [[0, 1, 2], [1, 3, 2]];
        assert!(normals_follow_winding(
            &points,
            &facets,
            &[[0., 0., 1.], [0., 0., 1.]]
        ));
        assert!(!normals_follow_winding(
            &points,
            &facets,
            &[[0., 0., 1.], [0., 0., -1.]]
        ));
    }

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
        vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
    }

    fn l_shape() -> Vec<[f64; 2]> {
        vec![[0., 0.], [2., 0.], [2., 1.], [1., 1.], [1., 2.], [0., 2.]]
    }

    fn tile(parent: &[[f64; 2]], regions: &[Vec<[f64; 2]>]) -> bool {
        let rings: Vec<&[[f64; 2]]> = regions.iter().map(Vec::as_slice).collect();
        regions_tile(parent, &rings, TOLERANCE)
    }

    // A region listed twice and the other half missing: the areas add up to the
    // parent's and both lie inside its bounding box, so only the overlap says no.
    #[test]
    fn a_region_listed_twice_with_the_other_half_missing_does_not_tile() {
        let left = rect(0., 0., 1., 1.);
        assert!(!tile(&rect(0., 0., 2., 1.), &[left.clone(), left]));
    }

    // 1.2 + 0.8 is the parent's area of 2, but [1.0, 1.2] is covered twice and
    // [1.8, 2.0] not at all.
    #[test]
    fn overlapping_regions_that_leave_a_gap_of_the_same_area_do_not_tile() {
        let parent = rect(0., 0., 2., 1.);
        assert!(!tile(
            &parent,
            &[rect(0., 0., 1.2, 1.), rect(1., 0., 1.8, 1.)]
        ));
    }

    #[test]
    fn a_gap_between_the_regions_does_not_tile() {
        let parent = rect(0., 0., 2., 1.);
        assert!(!tile(
            &parent,
            &[rect(0., 0., 1., 1.), rect(1., 0., 1.8, 1.)]
        ));
    }

    // The bottom bar (area 2) and a square in the notch (area 1) add up to the
    // L's area of 3 and lie in its bounding box, but the square is outside it.
    #[test]
    fn a_region_in_the_notch_of_an_l_shaped_parent_does_not_tile() {
        assert!(!tile(
            &l_shape(),
            &[rect(0., 0., 2., 1.), rect(1., 1., 2., 2.)]
        ));
    }

    #[test]
    fn a_region_past_the_parents_boundary_by_more_than_the_tolerance_does_not_tile() {
        let parent = rect(0., 0., 2., 1.);
        assert!(!tile(
            &parent,
            &[rect(0., 0., 1., 1.), rect(1., 0., 2.001, 1.)]
        ));
    }

    // A bow-tie's shoelace area is 0 though its triangles cover 0.5: a ring
    // that crosses itself must not tile, not even as its own region.
    #[test]
    fn a_self_crossing_parent_does_not_tile_even_with_itself_as_its_region() {
        let bow_tie = [[0., 0.], [1., 1.], [1., 0.], [0., 1.]];
        assert!(!tile(&bow_tie, &[bow_tie.to_vec()]));
    }

    #[test]
    fn a_region_over_the_boundary_within_the_tolerance_tiles() {
        let parent = rect(0., 0., 2., 1.);
        assert!(tile(
            &parent,
            &[rect(0., 0., 1., 1.), rect(1., 0., 2. + 5e-8, 1.)]
        ));
    }

    // The faces of r2025-12's windows: a parent and the two strips either side
    // of a line at v = 0.14583333333340992, edges 1e-15 apart.
    #[test]
    fn the_two_strips_of_a_window_face_tile() {
        let parent = [
            [-9.123137028441703e-16, 0.],
            [1.9166666666666732, 0.],
            [1.9166666666666732, 0.16666666666666102],
            [-9.123137028441703e-16, 0.16666666666666102],
        ];
        let strips = [
            rect(0., 0., 1.9166666666666723, 0.14583333333340992),
            rect(
                -9.123137028441703e-16,
                0.14583333333340992,
                1.9166666666666732,
                0.16666666666666102,
            ),
        ];
        assert!(tile(&parent, &strips));
    }

    #[test]
    fn the_two_arms_of_an_l_shaped_parent_tile_it() {
        assert!(tile(
            &l_shape(),
            &[rect(0., 0., 2., 1.), rect(0., 1., 1., 2.)]
        ));
    }

    // A cylinder of radius 2 about the vertical through (10, 20, 5): (u, v) is
    // the angle and the height above 5.
    fn cylinder(uv: [f64; 2]) -> Result<[f64; 3]> {
        Ok([10. + 2. * uv[0].cos(), 20. + 2. * uv[0].sin(), 5. + uv[1]])
    }
    // The same of radius 20.
    fn wide_cylinder(uv: [f64; 2]) -> Result<[f64; 3]> {
        Ok([10. + 20. * uv[0].cos(), 20. + 20. * uv[0].sin(), 5. + uv[1]])
    }
    // The slanted plane z = 5 + 0.5 (x - 10), by a point and a unit normal: it cuts the
    // cylinder along v = cos u.
    fn slanted_plane() -> ([f64; 3], [f64; 3]) {
        let length = (0.25f64 + 1.).sqrt();
        ([10., 20., 5.], [-0.5 / length, 0., 1. / length])
    }
    // The plane z = 5 + 3 (x - 10): it cuts the cylinder along v = 6 cos u, which is
    // steeper in v than in u over half of every turn, so the parameter solved for there is u.
    fn steep_plane() -> ([f64; 3], [f64; 3]) {
        let length = 10f64.sqrt();
        ([10., 20., 5.], [-3. / length, 0., 1. / length])
    }
    fn height_over(plane: ([f64; 3], [f64; 3]), p: [f64; 3]) -> f64 {
        (0..3)
            .map(|k| (p[k] - plane.0[k]) * plane.1[k])
            .sum::<f64>()
            .abs()
    }
    // The cut of the slanted plane across the cylinder from u = 0 to a quarter turn, in
    // `count` samples.
    fn cut(count: usize) -> Vec<[f64; 2]> {
        (0..count)
            .map(|i| {
                let u = FRAC_PI_2 * i as f64 / (count - 1) as f64;
                [u, u.cos()]
            })
            .collect()
    }
    // A full turn of a cut of height `scale` cos u in `count` samples, with a phase of
    // 0.37 so that no sample lands on an extremum of the cut.
    fn turn(count: usize, scale: f64) -> Vec<[f64; 2]> {
        (0..count)
            .map(|i| {
                let u = 2. * PI * (i as f64 + 0.37) / count as f64;
                [u, scale * u.cos()]
            })
            .collect()
    }
    // Every point of the true edge between two refined neighbours (v = scale cos u, taken
    // from the formula and not the solver) is within the bound of the chord that joins them.
    fn assert_chords_hold_the_edge(refined: &[[f64; 2]], scale: f64) {
        for pair in refined.windows(2) {
            let (a, b) = (cylinder(pair[0]).unwrap(), cylinder(pair[1]).unwrap());
            for step in 0..=400 {
                let u = pair[0][0] + (pair[1][0] - pair[0][0]) * step as f64 / 400.;
                let edge = cylinder([u, scale * u.cos()]).unwrap();
                assert!(
                    distance_to_chord(edge, a, b) <= EDGE_SAGITTA * 1.05 + 1e-9,
                    "{pair:?} at u = {u}"
                );
            }
        }
    }

    #[test]
    fn sparse_samples_of_a_slanted_cut_gain_points_on_the_cut() {
        let plane = slanted_plane();
        let coarse = cut(2);
        let refined = refine_crossing_edge(&cylinder, plane.0, plane.1, &coarse).unwrap();
        assert!(refined.len() > 2);
        for uv in &refined {
            assert!(height_over(plane, cylinder(*uv).unwrap()) < 1e-8, "{uv:?}");
        }
        assert_chords_hold_the_edge(&refined, 1.);
        // The samples the graph gave are kept, first and last.
        assert_eq!(refined[0], coarse[0]);
        assert_eq!(*refined.last().unwrap(), coarse[1]);
    }

    #[test]
    fn samples_already_within_the_bound_gain_nothing() {
        let plane = slanted_plane();
        let dense = cut(200);
        assert_eq!(
            refine_crossing_edge(&cylinder, plane.0, plane.1, &dense).unwrap(),
            dense
        );
        // Two samples closer than the bound hold the edge between them within it.
        let near = [[2., 2f64.cos()], [2.0001, 2.0001f64.cos()]];
        assert_eq!(
            refine_crossing_edge(&cylinder, plane.0, plane.1, &near).unwrap(),
            near.to_vec()
        );
    }

    #[test]
    fn a_turn_sampled_off_its_extrema_is_refined_where_the_edge_bulges_past_its_samples() {
        // A sample interval that holds the extremum of v has the edge past both its
        // samples, so the value solved for lies outside theirs: such an interval is the
        // rule on a closed ellipse, and a range test on that value refused every one.
        let plane = slanted_plane();
        for count in [8, 12, 24, 48] {
            let samples = turn(count, 1.);
            let refined = refine_crossing_edge(&cylinder, plane.0, plane.1, &samples).unwrap();
            assert!(refined.len() > samples.len(), "{count}");
            for uv in &refined {
                assert!(height_over(plane, cylinder(*uv).unwrap()) < 1e-8, "{uv:?}");
            }
            assert_chords_hold_the_edge(&refined, 1.);
        }
    }

    #[test]
    fn samples_straddling_an_extremum_gain_the_extremum() {
        let plane = slanted_plane();
        let samples = [[-PI / 12., (PI / 12.).cos()], [PI / 12., (PI / 12.).cos()]];
        let refined = refine_crossing_edge(&cylinder, plane.0, plane.1, &samples).unwrap();
        assert!(
            refined
                .iter()
                .any(|uv| uv[0].abs() < 1e-9 && (uv[1] - 1.).abs() < 1e-9),
            "{refined:?}"
        );
        assert_chords_hold_the_edge(&refined, 1.);
    }

    #[test]
    fn a_steep_cut_is_refined_and_holds_the_edge_where_u_is_the_parameter_solved_for() {
        let plane = steep_plane();
        for count in [8, 12, 24] {
            let samples = turn(count, 6.);
            let refined = refine_crossing_edge(&cylinder, plane.0, plane.1, &samples)
                .unwrap_or_else(|error| panic!("{count}: {error:#}"));
            for uv in &refined {
                assert!(height_over(plane, cylinder(*uv).unwrap()) < 1e-8, "{uv:?}");
            }
            assert_chords_hold_the_edge(&refined, 6.);
        }
    }

    #[test]
    fn a_quarter_turn_of_a_wide_cylinder_in_two_samples_is_refined_to_the_bound() {
        let plane = slanted_plane();
        let samples = [[0., 10.], [FRAC_PI_2, 0.]];
        let refined = refine_crossing_edge(&wide_cylinder, plane.0, plane.1, &samples).unwrap();
        assert!(refined.len() > 100);
        for uv in &refined {
            assert!(
                height_over(plane, wide_cylinder(*uv).unwrap()) < 1e-8,
                "{uv:?}"
            );
        }
    }

    #[test]
    fn a_point_of_the_plane_far_from_the_span_is_refused() {
        // The plane z = 100 meets the cylinder, a long way above the cut.
        let error =
            refine_crossing_edge(&cylinder, [10., 20., 100.], [0., 0., 1.], &cut(2)).unwrap_err();
        assert!(format!("{error:#}").contains("left its span"), "{error:#}");
    }

    #[test]
    fn a_span_still_above_the_bound_at_the_depth_limit_is_refused() {
        // The sparse quarter turn needs about a dozen halvings; two are not enough.
        let plane = slanted_plane();
        let limits = EdgeLimits {
            depth: 2,
            points: 1_000,
        };
        let error =
            refine_crossing_edge_within(&cylinder, plane.0, plane.1, &cut(2), &limits).unwrap_err();
        assert!(format!("{error:#}").contains("2 halvings"), "{error:#}");
        // With the limits of the rule it is refined.
        assert!(refine_crossing_edge(&cylinder, plane.0, plane.1, &cut(2)).is_ok());
    }

    #[test]
    fn an_edge_that_asks_for_more_points_than_the_budget_is_refused() {
        let plane = slanted_plane();
        let limits = EdgeLimits {
            depth: 32,
            points: 3,
        };
        let error =
            refine_crossing_edge_within(&cylinder, plane.0, plane.1, &cut(2), &limits).unwrap_err();
        assert!(format!("{error:#}").contains("point budget"), "{error:#}");
    }

    #[test]
    fn a_surface_that_gives_no_point_stops_the_refinement() {
        let plane = slanted_plane();
        let broken = |uv: [f64; 2]| -> Result<[f64; 3]> {
            if uv[0] > 0.7 && uv[0] < 0.8 {
                anyhow::bail!("no point")
            }
            cylinder(uv)
        };
        assert!(refine_crossing_edge(&broken, plane.0, plane.1, &cut(2)).is_err());
    }

    #[test]
    fn one_sample_is_no_edge_and_a_surface_of_no_numbers_is_refused() {
        let plane = slanted_plane();
        assert!(refine_crossing_edge(&cylinder, plane.0, plane.1, &cut(2)[..1]).is_err());
        let nan = |_: [f64; 2]| -> Result<[f64; 3]> { Ok([f64::NAN; 3]) };
        assert!(refine_crossing_edge(&nan, plane.0, plane.1, &cut(2)).is_err());
    }

    #[test]
    fn a_point_beyond_the_envelope_is_left_out_of_the_list_both_sides_take() {
        let original = [[0., 0.], [1., 1.]];
        let refined = vec![[0., 0.], [0.5, 0.4], [0.9, 1.0004], [1., 1.]];
        // The point at v = 1.0004 is past the envelope's top, 1; the samples stay
        // wherever they are, and a point inside stays.
        let kept = within_envelope(refined, &original, ([0., 0.], [1., 1.]));
        assert_eq!(kept, vec![[0., 0.], [0.5, 0.4], [1., 1.]]);
        let outside = within_envelope(
            vec![[0., 0.], [2., 2.]],
            &[[0., 0.], [2., 2.]],
            ([0., 0.], [1., 1.]),
        );
        assert_eq!(outside, vec![[0., 0.], [2., 2.]]);
    }

    #[test]
    fn an_edge_along_one_parameter_is_isoparametric_and_one_across_both_is_not() {
        assert_eq!(
            isoparametric_axis(&[[0., 1.], [0.5, 1.], [2., 1.]]),
            Some(0)
        );
        assert_eq!(
            isoparametric_axis(&[[3., 0.], [3., 0.5], [3., 2.]]),
            Some(1)
        );
        // Both change, the endpoints agree on neither or on both, or a sample
        // drifts off the constant parameter, and a single sample is no edge.
        assert_eq!(isoparametric_axis(&cut(5)), None);
        assert_eq!(isoparametric_axis(&[[0., 1.], [2., 1.], [0., 1.]]), None);
        assert_eq!(isoparametric_axis(&[[0., 1.], [0.5, 1.1], [2., 1.]]), None);
        assert_eq!(isoparametric_axis(&[[0., 1.]]), None);
        assert_eq!(isoparametric_axis(&[]), None);
    }
}
