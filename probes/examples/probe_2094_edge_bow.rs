//! #2094: does a face's boundary, drawn through the edge points the file saves, leave the
//! curved face it borders?
//!
//! FACT under test: on the BIM4LCA timber office's two stair flights that the spline
//! reader draws, the only Revit vertices more than 0.001 ft from rvt-rs's surface (two a
//! flight, at most 0.00125 ft) sit on a crease of Revit's body next to one of rvt-rs's
//! vertical faces (`tools/re/far_vertices_vs_ifc.py` on branch `measure/2094-far-vertices`,
//! Measure 44516271925), and a four times finer face grid leaves them unmoved (Measure
//! 44510586239). A face is trimmed by its loop's edges, each drawn through the points the
//! file saves for it (`m_firstAndLastEdgePnts`, `m_interiorEdgePnts`), with straight steps
//! between them in the face's own parameters. On a plane a straight step is a straight
//! chord in the model; on a curved face it follows the surface. Where a plane meets a
//! curved face along a curved edge, the two faces' boundaries part between saved points by
//! the curve's bow, which no face grid changes.
//!
//! For every `Edge` of the given elements' saved graphics (and the shared symbols they
//! name) whose two faces both read, the probe evaluates each saved point on both faces, and
//! the middle of every step between two saved points (in each face's own parameters) on
//! each face, and prints:
//!   edge owner=<ElementId> object=<n> faces=<surface class>/<surface class> points=<n>
//!     join=<ft> gap=<ft> at=<step>/<steps>
//!   - `join`: the farthest a saved point lies from itself as the other face places it;
//!   - `gap`: the farthest the two faces' middles of one step lie apart, the bow left
//!     between the two boundaries;
//!   only edges whose `gap` is over 1e-6 ft are listed, the farthest first, 40 at most;
//!   near target=<x,y,z> owner=<ElementId> object=<n> sides=<class:axis>/<class:axis>
//!     points=<n> side0=<ft> side1=<ft> gap=<ft>: for each Revit vertex that leaves
//!     rvt-rs's surface (`TARGETS`), the three edges whose sides pass nearest it, how far
//!     each face's side (saved points, 16 steps evaluated between) passes from it, and the
//!     axis each face's side runs along (`-` where it crosses both);
//!   mesh owner=<ElementId> faces=<n> diagnostics=<n> boundary_vertices=<n> over_0.0001=<n>
//!     over_0.001=<n> largest=<ft>: the two flights' faces meshed as the export meshes them
//!     (`graphics_with_resolver`), each face's open boundary vertices and how far each lies
//!     from every other face, then the eight farthest with where they are: faces that part
//!     along a shared edge show here as a gap. Each names its face as `geometry=<n>
//!     face=<n> surface=<class> chords=<n>` (the `Geometry` object it was meshed from, the
//!     face of it with the primitive's tag, and the spans between saved points its grid
//!     draws straight, `-` for a face with no grid), and a line per such face gives how
//!     many of the vertices over 0.001 ft are on it:
//!   mesh owner=<ElementId> over_0.001_on geometry=<n> face=<n> surface=<class>
//!     chords=<n> vertices=<n>;
//!   tally faces=<a>/<b> edges=<n> gap_over_0.001=<n> largest=<ft>
//! ElementIds, surface classes and distances in feet only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2094_edge_bow -- MODEL.rvt [ID ...]
//! With no ElementId it reads every element the export writes, and adds a line for each
//! element whose edges border a ruled face with a spline profile:
//!   spline owner=<ElementId> class=<IFC class> edges=<n> gap_over_0.001=<n> largest=<ft>
//! (an owner is the record the graphics are read from: a shared symbol's own id where an
//! element draws one).

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_parametric_surface::{Curve, ParametricSurface};
use rvt::research::native_saved_mesh::pointer;

type V3 = [f64; 3];

/// The Revit vertices `far_vertices_vs_ifc.py` places more than 0.001 ft from rvt-rs's
/// surface on the timber office's two flights (Measure 44518424477), in rvt-rs's frame.
const TARGETS: [V3; 4] = [
    [305.0318, 49.7848, 226.0539],
    [305.0318, 49.5977, 225.901],
    [157.7549, 49.7848, 226.0539],
    [157.7549, 49.5977, 225.901],
];

/// The two flights whose meshes are checked for faces that part along a shared edge.
const MESH_OWNERS: [u64; 2] = [3_558_983, 3_561_904];

/// The closest distance from `p` to the triangle `a`, `b`, `c`.
fn triangle_distance(p: V3, a: V3, b: V3, c: V3) -> f64 {
    let sub = |x: V3, y: V3| -> V3 { std::array::from_fn(|k| x[k] - y[k]) };
    let dot = |x: V3, y: V3| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let n: V3 = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    let nn = dot(n, n);
    if nn > 0. {
        // Barycentric coordinates of the foot of the perpendicular.
        let t = dot(ap, n) / nn;
        let foot: V3 = std::array::from_fn(|k| p[k] - t * n[k]);
        let (v0, v1, v2) = (ab, ac, sub(foot, a));
        let (d00, d01, d11, d20, d21) = (
            dot(v0, v0),
            dot(v0, v1),
            dot(v1, v1),
            dot(v2, v0),
            dot(v2, v1),
        );
        let den = d00 * d11 - d01 * d01;
        if den.abs() > 0. {
            let v = (d11 * d20 - d01 * d21) / den;
            let w = (d00 * d21 - d01 * d20) / den;
            if v >= 0. && w >= 0. && v + w <= 1. {
                return (t * t * nn).sqrt();
            }
        }
    }
    segment_distance(p, a, b)
        .min(segment_distance(p, b, c))
        .min(segment_distance(p, c, a))
}

/// Sub-steps evaluated between two saved points of an edge on each face.
const SUBSTEPS: usize = 16;

fn segment_distance(p: V3, a: V3, b: V3) -> f64 {
    let ab: V3 = std::array::from_fn(|k| b[k] - a[k]);
    let ap: V3 = std::array::from_fn(|k| p[k] - a[k]);
    let len2: f64 = ab.iter().map(|x| x * x).sum();
    let t = if len2 > 0. {
        (ap.iter().zip(ab).map(|(x, y)| x * y).sum::<f64>() / len2).clamp(0., 1.)
    } else {
        0.
    };
    dist(p, std::array::from_fn(|k| a[k] + t * ab[k]))
}

fn polyline_distance(p: V3, line: &[V3]) -> f64 {
    line.windows(2)
        .map(|w| segment_distance(p, w[0], w[1]))
        .fold(f64::INFINITY, f64::min)
}

/// Whether every point of a face's side of an edge holds one parameter fixed: the
/// axis that runs, or `-` for an edge that runs across both.
fn iso_axis(side: &[[f64; 2]]) -> &'static str {
    let fixed = |k: usize| side.iter().all(|p| (p[k] - side[0][k]).abs() < 1e-7);
    match (fixed(0), fixed(1)) {
        (false, true) => "u",
        (true, false) => "v",
        _ => "-",
    }
}

fn uv(v: &Fv) -> Option<[f64; 2]> {
    let a = v.as_array().filter(|a| a.len() == 2)?;
    let p = [a[0].as_f64()?, a[1].as_f64()?];
    p.iter().all(|x| x.is_finite()).then_some(p)
}

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

fn instance_symbols(record: &Record) -> Vec<u64> {
    let Some(graph) = &record.graph else {
        return Vec::new();
    };
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            object
                .fields
                .get("m_symbolId")
                .and_then(identifier)
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// One edge's two faces' boundaries compared.
struct Bow {
    /// Whether one of its faces is a ruled face with a spline profile.
    hermite: bool,
    owner: u64,
    object: usize,
    faces: String,
    points: usize,
    join: f64,
    gap: f64,
    at: (usize, usize),
    /// Each face's side of the edge, the saved points with `SUBSTEPS` between, in the model.
    lines: [Vec<V3>; 2],
    axes: String,
}

/// A face's surface, as far as the probe evaluates one: a plane by its origin and two axes
/// (as `native_saved_mesh` reads one), else the parametric reader's surface.
enum Surface {
    Plane { origin: V3, x: V3, y: V3 },
    Parametric(Box<ParametricSurface>),
}

impl Surface {
    fn bounds(&self) -> [[f64; 2]; 2] {
        match self {
            Surface::Plane { .. } => [[f64::NEG_INFINITY; 2], [f64::INFINITY; 2]],
            Surface::Parametric(s) => s.bounds(),
        }
    }
    fn evaluate(&self, u: f64, v: f64) -> Result<V3> {
        match self {
            Surface::Plane { origin, x, y } => {
                Ok(std::array::from_fn(|k| origin[k] + u * x[k] + v * y[k]))
            }
            Surface::Parametric(s) => s.evaluate(u, v),
        }
    }
}

fn triple(v: &Fv) -> Option<V3> {
    let a = v.as_array().filter(|a| a.len() == 3)?;
    Some([a[0].as_f64()?, a[1].as_f64()?, a[2].as_f64()?])
}

/// The face at `face`'s surface and its class.
fn surface_of(g: &ObjectGraph, face: usize) -> Result<(Surface, String)> {
    let f = &g.objects[face];
    let si = pointer(g, face, &f.fields["m_pSurf"])?.context("face surface")?;
    let s = &g.objects[si];
    let class = s.class_name.clone();
    if class == "Plane" {
        let read = |name: &str| s.fields.get(name).and_then(triple).context("plane field");
        return Ok((
            Surface::Plane {
                origin: read("m_origin")?,
                x: read("m_xVec")?,
                y: read("m_yVec")?,
            },
            class,
        ));
    }
    Ok((
        Surface::Parametric(Box::new(ParametricSurface::read(g, si)?)),
        class,
    ))
}

/// What `edge_bow` reads of one edge.
struct EdgeRead {
    hermite: bool,
    faces: String,
    points: usize,
    join: f64,
    gap: f64,
    at: (usize, usize),
    lines: [Vec<V3>; 2],
    axes: String,
}

fn edge_bow(g: &ObjectGraph, ei: usize) -> Result<Option<EdgeRead>> {
    let e = &g.objects[ei];
    let faces = e.fields["m_pFace"].as_array().context("edge faces")?;
    if faces.len() != 2 {
        return Ok(None);
    }
    let (Some(f0), Some(f1)) = (pointer(g, ei, &faces[0])?, pointer(g, ei, &faces[1])?) else {
        return Ok(None);
    };
    let (s0, c0) = surface_of(g, f0)?;
    let (s1, c1) = surface_of(g, f1)?;
    let ends = e.fields["m_firstAndLastEdgePnts"]
        .as_array()
        .context("edge ends")?;
    let interior = e.fields["m_interiorEdgePnts"]
        .as_array()
        .context("edge interior")?;
    let mut points: Vec<&Fv> = vec![&ends[0]];
    points.extend(interior.iter());
    points.push(&ends[1]);
    let mut sides: [Vec<[f64; 2]>; 2] = [Vec::new(), Vec::new()];
    for p in &points {
        for (side, out) in sides.iter_mut().enumerate() {
            out.push(uv(&p["uv"][side]).context("edge uv")?);
        }
    }
    // Into each surface's parameter range, as the mesher clamps them (within 1e-7).
    for (side, surface) in sides.iter_mut().zip([&s0, &s1]) {
        let env = surface.bounds();
        for p in side.iter_mut() {
            for k in 0..2 {
                anyhow::ensure!(
                    p[k] >= env[0][k] - 1e-7 && p[k] <= env[1][k] + 1e-7,
                    "edge point outside its surface"
                );
                p[k] = p[k].clamp(env[0][k], env[1][k]);
            }
        }
    }
    let mut join = 0.0_f64;
    for k in 0..points.len() {
        let a = s0.evaluate(sides[0][k][0], sides[0][k][1])?;
        let b = s1.evaluate(sides[1][k][0], sides[1][k][1])?;
        join = join.max(dist(a, b));
    }
    let mut gap = 0.0_f64;
    let mut at = (0, points.len() - 1);
    for k in 0..points.len() - 1 {
        let mid = |s: &Surface, side: &Vec<[f64; 2]>| {
            s.evaluate(
                (side[k][0] + side[k + 1][0]) / 2.0,
                (side[k][1] + side[k + 1][1]) / 2.0,
            )
        };
        let d = dist(mid(&s0, &sides[0])?, mid(&s1, &sides[1])?);
        if d > gap {
            gap = d;
            at = (k, points.len() - 1);
        }
    }
    // Each face's side of the edge in the model: its saved points, and `SUBSTEPS` evaluated
    // between each two on that face's own surface.
    let line = |s: &Surface, side: &Vec<[f64; 2]>| -> Result<Vec<V3>> {
        let mut out = Vec::new();
        for k in 0..side.len() - 1 {
            for j in 0..SUBSTEPS {
                let t = j as f64 / SUBSTEPS as f64;
                out.push(s.evaluate(
                    side[k][0] + t * (side[k + 1][0] - side[k][0]),
                    side[k][1] + t * (side[k + 1][1] - side[k][1]),
                )?);
            }
        }
        let last = side[side.len() - 1];
        out.push(s.evaluate(last[0], last[1])?);
        Ok(out)
    };
    let lines = [line(&s0, &sides[0])?, line(&s1, &sides[1])?];
    let axes = format!(
        "{}:{}/{}:{}",
        c0,
        iso_axis(&sides[0]),
        c1,
        iso_axis(&sides[1])
    );
    let mut pair = [c0, c1];
    pair.sort();
    let spline = |s: &Surface| {
        matches!(s, Surface::Parametric(p) if matches!(
            p.as_ref(),
            ParametricSurface::RuledSurf { profile1, profile2, .. }
                if matches!(profile1, Curve::Hermite { .. }) || matches!(profile2, Curve::Hermite { .. })
        ))
    };
    Ok(Some(EdgeRead {
        hermite: spline(&s0) || spline(&s1),
        faces: format!("{}/{}", pair[0], pair[1]),
        points: points.len(),
        join,
        gap,
        at,
        lines,
        axes,
    }))
}

fn scan(owner: u64, record: &Record, bows: &mut Vec<Bow>, refused: &mut BTreeMap<String, usize>) {
    let Some(graph) = &record.graph else {
        return;
    };
    for (ei, object) in graph.objects.iter().enumerate() {
        if object.class_name != "Edge" {
            continue;
        }
        match edge_bow(graph, ei) {
            Ok(Some(read)) => bows.push(Bow {
                hermite: read.hermite,
                owner,
                object: ei,
                faces: read.faces,
                points: read.points,
                join: read.join,
                gap: read.gap,
                at: read.at,
                lines: read.lines,
                axes: read.axes,
            }),
            Ok(None) => {}
            Err(error) => {
                let message: String = format!("{error:#}").chars().take(60).collect();
                *refused.entry(message).or_default() += 1;
            }
        }
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .filter(|a| !a.starts_with("--"))
        .collect();
    let path = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2094_edge_bow MODEL.rvt [ID ...]"))?;
    let mut ids: BTreeSet<u64> = args[1..].iter().filter_map(|a| a.parse().ok()).collect();
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    // With no ElementId given, every element the export writes, with its IFC class.
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    let model = rvt::ifc::RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        rvt::ifc::ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    for entity in &model.entities {
        if let rvt::ifc::entities::IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                classes.insert(id, ifc_type.clone());
            }
        }
    }
    if ids.is_empty() {
        ids = classes.keys().copied().collect();
    }
    let mut options = native_document::Options {
        selected_ids: ids.clone(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut bows = Vec::new();
    let mut refused = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut wanted = BTreeSet::new();
    // The two flights' own graphs, and the symbols they name, kept for the mesh check.
    let mut kept: BTreeMap<u64, ObjectGraph> = BTreeMap::new();
    let mut kept_symbols: BTreeSet<u64> = BTreeSet::new();
    native_document::extract_graphs(&mut rf, &options, |r| {
        seen.insert(r.identity.element_id);
        wanted.extend(instance_symbols(&r));
        if MESH_OWNERS.contains(&r.identity.element_id) {
            kept_symbols.extend(instance_symbols(&r));
            if let Some(graph) = &r.graph {
                kept.insert(r.identity.element_id, graph.clone());
            }
        }
        scan(r.identity.element_id, &r, &mut bows, &mut refused);
        Ok(())
    })?;
    for _ in 0..16 {
        wanted.retain(|id| !seen.contains(id));
        if wanted.is_empty() {
            break;
        }
        seen.extend(&wanted);
        options.selected_ids = std::mem::take(&mut wanted);
        let mut found = Vec::new();
        native_document::extract_graphs(&mut rf, &options, |r| {
            found.extend(instance_symbols(&r));
            if kept_symbols.contains(&r.identity.element_id) {
                if let Some(graph) = &r.graph {
                    kept.insert(r.identity.element_id, graph.clone());
                }
            }
            scan(r.identity.element_id, &r, &mut bows, &mut refused);
            Ok(())
        })?;
        wanted.extend(found);
    }
    println!(
        "file elements={} records_read={} edges={} refused={}",
        ids.len(),
        seen.len(),
        bows.len(),
        refused.values().sum::<usize>()
    );
    for (why, n) in &refused {
        println!("refused {n} {why}");
    }
    bows.sort_by(|a, b| b.gap.total_cmp(&a.gap));
    for bow in bows.iter().filter(|b| b.gap > 1e-6).take(40) {
        println!(
            "edge owner={} object={} faces={} points={} join={:.6} gap={:.6} at={}/{}",
            bow.owner, bow.object, bow.faces, bow.points, bow.join, bow.gap, bow.at.0, bow.at.1
        );
    }
    // The edges nearest each Revit vertex that leaves rvt-rs's surface: how far each
    // face's side of the edge passes from it.
    for target in TARGETS {
        let mut near: Vec<(f64, f64, &Bow)> = bows
            .iter()
            .map(|b| {
                (
                    polyline_distance(target, &b.lines[0]),
                    polyline_distance(target, &b.lines[1]),
                    b,
                )
            })
            .collect();
        near.sort_by(|a, b| a.0.min(a.1).total_cmp(&b.0.min(b.1)));
        for (d0, d1, b) in near.iter().take(3) {
            println!(
                "near target={:.4},{:.4},{:.4} owner={} object={} sides={} points={} side0={d0:.6} side1={d1:.6} gap={:.6}",
                target[0], target[1], target[2], b.owner, b.object, b.axes, b.points, b.gap
            );
        }
    }
    // Whether the flights' faces, meshed as the export meshes them, meet along their shared
    // edges: each face's open boundary (an edge of one of its triangles that no other of
    // its triangles has), and how far each of its vertices lies from every other face.
    for owner in MESH_OWNERS {
        let Some(graph) = kept.get(&owner) else {
            println!("mesh owner={owner} not read");
            continue;
        };
        let meshes =
            rvt::research::native_saved_mesh::graphics_with_resolver(graph, &|id| kept.get(&id));
        let faces: Vec<&rvt::research::native_saved_mesh::Primitive> = meshes
            .primitives
            .iter()
            .filter(|p| !p.hidden && !p.filtered)
            .collect();
        let mut gaps: Vec<(f64, V3, usize)> = Vec::new();
        for (fi, face) in faces.iter().enumerate() {
            let key = |i: u32| face.vertices[i as usize].map(|x| (x * 1e6).round() as i64);
            let mut uses: BTreeMap<([i64; 3], [i64; 3]), usize> = BTreeMap::new();
            for t in &face.triangles {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    let (ka, kb) = (key(a), key(b));
                    let edge = if ka < kb { (ka, kb) } else { (kb, ka) };
                    *uses.entry(edge).or_default() += 1;
                }
            }
            let mut boundary: BTreeSet<[i64; 3]> = BTreeSet::new();
            for ((a, b), n) in &uses {
                if *n == 1 {
                    boundary.insert(*a);
                    boundary.insert(*b);
                }
            }
            for k in &boundary {
                let p: V3 = k.map(|x| x as f64 / 1e6);
                let mut best = f64::INFINITY;
                for (gi, other) in faces.iter().enumerate() {
                    if gi == fi {
                        continue;
                    }
                    for t in &other.triangles {
                        let [a, b, c] = t.map(|i| other.vertices[i as usize]);
                        best = best.min(triangle_distance(p, a, b, c));
                    }
                }
                gaps.push((best, p, fi));
            }
        }
        // The face a gap's vertex is on: its object, its surface's class, and how many
        // spans between saved points its grid draws straight (`-` where it has no grid).
        // A primitive's `object_index` is the `Geometry` it was meshed from; its face is
        // the one of that object's `m_pFaces` whose tag is the primitive's.
        let describe = |fi: usize| -> String {
            let face = faces[fi];
            let g = face
                .source_owner_id
                .and_then(|id| kept.get(&id))
                .unwrap_or(graph);
            let face_index = g
                .objects
                .get(face.object_index)
                .and_then(|o| o.fields.get("m_pFaces"))
                .and_then(|list| list.as_array())
                .and_then(|list| {
                    list.iter()
                        .filter_map(|p| pointer(g, face.object_index, p).ok().flatten())
                        .find(|i| {
                            g.objects[*i].fields["m_GInfo"]["m_tag"].as_i64() == Some(face.face_tag)
                        })
                });
            let Some(fx) = face_index else {
                return format!("geometry={} face=- surface=- chords=-", face.object_index);
            };
            let surface = g.objects[fx]
                .fields
                .get("m_pSurf")
                .and_then(|s| pointer(g, fx, s).ok().flatten())
                .and_then(|si| g.objects.get(si))
                .map_or("-", |s| s.class_name.as_str());
            let chords = rvt::research::native_parametric_mesh::face_grid(g, fx)
                .map_or_else(|_| "-".to_string(), |grid| grid.chords.len().to_string());
            format!(
                "geometry={} face={fx} surface={surface} chords={chords}",
                face.object_index
            )
        };
        gaps.sort_by(|a, b| b.0.total_cmp(&a.0));
        let over = |bar: f64| gaps.iter().filter(|g| g.0 > bar).count();
        println!(
            "mesh owner={owner} faces={} diagnostics={} boundary_vertices={} over_0.0001={} over_0.001={} largest={:.6}",
            faces.len(),
            meshes.diagnostics.len(),
            gaps.len(),
            over(1e-4),
            over(1e-3),
            gaps.first().map_or(0., |g| g.0)
        );
        for (d, p, fi) in gaps.iter().take(8) {
            println!(
                "mesh owner={owner} gap={d:.6} at={:.4},{:.4},{:.4} {}",
                p[0],
                p[1],
                p[2],
                describe(*fi)
            );
        }
        // Where the vertices over 0.001 ft lie, by their face's surface and spans.
        let mut by_face: BTreeMap<String, usize> = BTreeMap::new();
        for (_, _, fi) in gaps.iter().filter(|g| g.0 > 1e-3) {
            *by_face.entry(describe(*fi)).or_default() += 1;
        }
        for (face, n) in &by_face {
            println!("mesh owner={owner} over_0.001_on {face} vertices={n}");
        }
    }
    // Per element, the edges that border a ruled face with a spline profile: how many,
    // how many bow more than 0.001 ft from their saved points' straight lines, the most.
    let mut owners: BTreeMap<u64, (usize, usize, f64)> = BTreeMap::new();
    for bow in bows.iter().filter(|b| b.hermite) {
        let entry = owners.entry(bow.owner).or_default();
        entry.0 += 1;
        entry.1 += usize::from(bow.gap > 0.001);
        entry.2 = entry.2.max(bow.gap);
    }
    for (owner, (n, over, largest)) in &owners {
        println!(
            "spline owner={owner} class={} edges={n} gap_over_0.001={over} largest={largest:.6}",
            classes.get(owner).map_or("-", String::as_str)
        );
    }
    let mut tally: BTreeMap<&str, (usize, usize, f64)> = BTreeMap::new();
    for bow in &bows {
        let entry = tally.entry(bow.faces.as_str()).or_default();
        entry.0 += 1;
        entry.1 += usize::from(bow.gap > 0.001);
        entry.2 = entry.2.max(bow.gap);
    }
    for (faces, (n, over, largest)) in &tally {
        println!("tally faces={faces} edges={n} gap_over_0.001={over} largest={largest:.6}");
    }
    Ok(())
}
