//! Check emitted boundaries of a generated plane and cylinder sharing an edge.
//! No model input or sampled model data. Run with --check to enforce the contract.

use anyhow::{Context, Result, ensure};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{FaceMesh, face};
use serde_json::{Value, json};
use std::collections::BTreeMap;

const PLANE: usize = 0;
const CYLINDER: usize = 1;
const FIRST: f64 = 0.2;
const LAST: f64 = 1.3;
const HEIGHT: f64 = 0.7;
const BOTTOM: f64 = -0.5;
const GEOMETRY_EPS: f64 = 1e-7;

#[derive(Clone, Copy, PartialEq)]
enum Variant {
    Agree,
    Reverse,
    SwapFaces,
    EndpointDisagreement,
    InteriorDisagreement,
    OffPlane,
}

fn ptr(index: Option<usize>) -> Value {
    json!({"pointer_token": index.map_or(0, |i| i+1), "offset": 0})
}

fn face_fields(surface: usize, ring: usize) -> Value {
    json!({"m_pSurf":ptr(Some(surface)), "m_pFirstLoop":ptr(Some(ring)),
        "m_faceRegions":[], "m_faceFlags_v9":0, "m_renderStyleId":0})
}

fn ring_fields(owner: usize, start: usize) -> Value {
    json!({"m_pFace":ptr(Some(owner)), "m_next":ptr(Some(start)),
        "m_nextLoop":ptr(None), "m_open":false})
}

fn edge(owners: [Option<usize>; 2], next: [Option<usize>; 2], pairs: &[[[f64; 2]; 2]]) -> Value {
    let uv = |pair| json!({"uv":pair});
    json!({"m_pFace":owners.map(ptr), "m_next":next.map(ptr),
        "m_firstAndLastEdgePnts":[uv(pairs[0]),uv(pairs[pairs.len()-1])],
        "m_interiorEdgePnts":pairs[1..pairs.len()-1].iter().copied().map(uv).collect::<Vec<_>>()})
}

fn side(owner: usize, next: usize, start: [f64; 2], end: [f64; 2]) -> Value {
    edge(
        [Some(owner), None],
        [Some(next), None],
        &[[start, [0., 0.]], [end, [0., 0.]]],
    )
}

fn build(slope: f64, variant: Variant) -> Result<ObjectGraph> {
    let plane = |theta: f64| [theta.cos(), theta.sin()];
    let cyl = |theta: f64| [theta, HEIGHT + slope * theta.cos()];
    let center = (FIRST + LAST) / 2.;
    let plane_first = if variant == Variant::EndpointDisagreement {
        plane(FIRST + 0.0004)
    } else {
        plane(FIRST)
    };
    let mut common = [
        [plane_first, cyl(FIRST)],
        [plane(center), cyl(center)],
        [plane(LAST), cyl(LAST)],
    ];
    if variant == Variant::InteriorDisagreement {
        // Both samples lie on the same analytic intersection, at different positions.
        common[1][0] = plane(center + 0.0004);
    }
    if variant == Variant::OffPlane {
        // The cylinder's middle sample stays on its cylinder and leaves the plane.
        common[1][1][1] += 0.0004;
    }
    if variant == Variant::Reverse {
        common.reverse();
    }
    let mut owners = [Some(PLANE), Some(CYLINDER)];
    let mut next = [Some(7), Some(9)];
    if variant == Variant::SwapFaces {
        owners.swap(0, 1);
        next.swap(0, 1);
        for pair in &mut common {
            pair.swap(0, 1);
        }
    }
    let records = [
        ("Face", face_fields(2, 4)),
        ("Face", face_fields(3, 5)),
        (
            "Plane",
            json!({"m_origin":[0.,0.,HEIGHT], "m_xVec":[1.,0.,slope],
            "m_yVec":[0.,1.,0.], "m_orientFlag":true}),
        ),
        (
            "CylSurf",
            json!({"m_center":[0.,0.,0.], "m_radius":1.,
            "m_xVec":[1.,0.,0.], "m_yVec":[0.,1.,0.], "m_zVec":[0.,0.,1.], "m_orientFlag":true,
            "m_Envelope":{"m_corners":[[FIRST,BOTTOM],[LAST,1.2]]}}),
        ),
        ("EdgeLoop", ring_fields(PLANE, 6)),
        ("EdgeLoop", ring_fields(CYLINDER, 6)),
        ("Edge", edge(owners, next, &common)),
        ("Edge", side(PLANE, 8, plane(LAST), [0., 0.])),
        ("Edge", side(PLANE, 4, [0., 0.], plane_first)),
        ("Edge", side(CYLINDER, 10, cyl(FIRST), [FIRST, BOTTOM])),
        ("Edge", side(CYLINDER, 11, [FIRST, BOTTOM], [LAST, BOTTOM])),
        ("Edge", side(CYLINDER, 5, [LAST, BOTTOM], cyl(LAST))),
    ];
    let objects=records.into_iter().enumerate().map(|(i,(class,fields))|
        json!({"class_tag":0,"class_name":class,"token":i+1,"start":0,"fields_end":0,"fields":fields})
    ).collect::<Vec<_>>();
    Ok(serde_json::from_value(
        json!({"consumed_bytes":0,"objects":objects,"edges":[]}),
    )?)
}

type Point = [f64; 3];
type Segment = [Point; 2];

fn angle(p: Point) -> f64 {
    p[1].atan2(p[0])
}

fn distance(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}

fn boundary(mesh: &FaceMesh) -> Result<Vec<Segment>> {
    let mut counts = BTreeMap::new();
    for &[i, j, k] in &mesh.triangles {
        for (a, b) in [(i, j), (j, k), (k, i)] {
            *counts.entry((a.min(b), a.max(b))).or_insert(0) += 1;
        }
    }
    let on_cap = |p: Point| (p[0].hypot(p[1]) - 1.).abs() < GEOMETRY_EPS && p[2] > 0.4;
    let mut edges = Vec::new();
    for ((i, j), count) in counts {
        let mut pair = [mesh.vertices[i as usize], mesh.vertices[j as usize]];
        if count == 1
            && pair.into_iter().all(on_cap)
            && (angle(pair[0]) - angle(pair[1])).abs() > 1e-9
        {
            pair.sort_by(|a, b| angle(*a).total_cmp(&angle(*b)));
            edges.push(pair);
        }
    }
    edges.sort_by(|a, b| angle(a[0]).total_cmp(&angle(b[0])));
    ensure!(!edges.is_empty(), "no cap boundary triangles");
    ensure!((angle(edges[0][0]) - FIRST).abs() < GEOMETRY_EPS);
    ensure!((angle(edges.last().unwrap()[1]) - LAST).abs() < GEOMETRY_EPS);
    ensure!(
        edges
            .windows(2)
            .all(|p| distance(p[0][1], p[1][0]) < GEOMETRY_EPS),
        "cap path has a gap"
    );
    Ok(edges)
}

fn point_segment(point: Point, [a, b]: Segment) -> f64 {
    let ab: Point = std::array::from_fn(|i| b[i] - a[i]);
    let along = (0..3).map(|i| (point[i] - a[i]) * ab[i]).sum::<f64>()
        / ab.iter().map(|v| v * v).sum::<f64>();
    distance(
        point,
        std::array::from_fn(|i| a[i] + along.clamp(0., 1.) * ab[i]),
    )
}

fn path_gap(from: &[Segment], to: &[Segment]) -> f64 {
    from.iter()
        .flat_map(|&[a, b]| {
            (0..=8).map(move |i| std::array::from_fn(|k| a[k] + (b[k] - a[k]) * (i as f64 / 8.)))
        })
        .map(|p| {
            to.iter()
                .map(|&edge| point_segment(p, edge))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0., f64::max)
}

fn run(label: &str, slope: f64, variant: Variant) -> Result<(bool, Value)> {
    let graph = build(slope, variant)?;
    let planar = face(&graph, PLANE);
    if matches!(
        variant,
        Variant::EndpointDisagreement | Variant::InteriorDisagreement | Variant::OffPlane
    ) {
        let wanted = if variant == Variant::OffPlane {
            "shared curved edge leaves adjacent plane"
        } else {
            "shared edge surface samples disagree"
        };
        let error = planar.expect_err("an unsupported sample should be refused");
        ensure!(
            error.to_string().contains(wanted),
            "{label} refused for {error}"
        );
        return Ok((true, json!({"case":label,"expected_refusal":wanted})));
    }
    let plane = boundary(&planar?.context("empty plane")?)?;
    let cylinder_mesh = face(&graph, CYLINDER)?.context("empty cylinder")?;
    let cyl = boundary(&cylinder_mesh)?;
    let cylinder_trim_vertices = cylinder_mesh.trim_uv.iter().filter(|p| p[1] > 0.4).count();
    let max_residual = |edges: &[Segment]| {
        edges
            .iter()
            .flatten()
            .map(|&[x, _, z]| (z - HEIGHT - slope * x).abs() / (1. + slope * slope).sqrt())
            .fold(0., f64::max)
    };
    let same = plane.len() == cyl.len()
        && plane
            .iter()
            .zip(&cyl)
            .all(|(a, b)| (0..2).all(|i| distance(a[i], b[i]) <= GEOMETRY_EPS));
    let passed = same && max_residual(&plane) <= GEOMETRY_EPS && max_residual(&cyl) <= GEOMETRY_EPS;
    Ok((
        passed,
        json!({"case":label,"agreed":passed,"same_ordered_segments":same,
        "plane_segments":plane.len(),"cylinder_segments":cyl.len(),
        "cylinder_trim_segments_before_strip_clipping":cylinder_trim_vertices - 1,
        "plane_max_distance_to_plane_ft":max_residual(&plane),
        "cylinder_max_distance_to_plane_ft":max_residual(&cyl),
        "sampled_plane_to_cylinder_path_gap_ft":path_gap(&plane,&cyl),
        "sampled_cylinder_to_plane_path_gap_ft":path_gap(&cyl,&plane)}),
    ))
}

fn main() -> Result<()> {
    let mut passed = true;
    for (label, slope, variant) in [
        ("level", 0., Variant::Agree),
        ("oblique", 0.4, Variant::Agree),
        ("reverse_stored_order", 0.4, Variant::Reverse),
        ("swap_face_slots", 0.4, Variant::SwapFaces),
        ("endpoint_disagreement", 0.4, Variant::EndpointDisagreement),
        ("interior_disagreement", 0.4, Variant::InteriorDisagreement),
        ("off_plane", 0.4, Variant::OffPlane),
    ] {
        let (holds, evidence) = run(label, slope, variant).with_context(|| label.to_owned())?;
        passed &= holds;
        println!("{evidence}");
    }
    ensure!(
        passed || !std::env::args().any(|arg| arg == "--check"),
        "one or more emitted shared boundaries violate the contract"
    );
    Ok(())
}
