//! #2098: do a body's faces meet along an edge a plane shares with a parametric face?
//!
//! FACT under test: where an edge a plane shares with a ruled surface, a surface of
//! revolution or a cone runs across both of the curved face's parameters, the plane's
//! side is drawn through the points where the plane cuts the curved surface
//! (`native_saved_mesh::refined_plane_edge`, #1706) and, before #2098, the curved face's
//! side straight in its parameters between the edge's saved points, so the two faces part
//! by the edge's bow. Run on the commit before #2098 and on #2098, it says how many bodies
//! hold such an edge, how far their faces part, and what the change costs in triangles.
//!
//! For every element the export writes (and the shared symbols its saved graphics name), a
//! body whose graphics hold such an edge is meshed as the export meshes it
//! (`graphics_with_resolver`); each face's open boundary (an edge of one of its triangles no
//! other of its triangles has) is set against every other face of the body. It prints:
//!   body owner=<ElementId> crossing=<n> faces=<n> triangles=<n> boundary=<n>
//!     over_0.0001=<n> over_0.001=<n> largest=<ft> refused=<why|->
//!   - `crossing`: the plane-to-parametric edges whose curved side changes both parameters,
//!     then `crossing_by=RuledSurf:<n>,SurfRev:<n>,ConeSurf:<n>`, the same by the curved
//!     side's class;
//!   - `over_*` and `largest`: boundary vertices farther than that from every other face.
//!
//! then, for a meshed body, a line per pair of surface classes holding a vertex over 0.001 ft:
//!   far owner=<ElementId> own=<class> nearest=<class> vertices=<n> largest=<ft>
//!   - `own`: the surface of the vertex's face; `nearest`: that of the face nearest it.
//!
//! and last `total bodies=<n> refused=<n> triangles=<n> over_0.001=<n> largest=<ft>`.
//! ElementIds, counts and distances in feet only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2098_face_cracks -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{self, Record};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::{graphics_with_resolver, pointer};

type V3 = [f64; 3];
/// A mesh edge by its two ends, rounded to a millionth of a foot, the lower first.
type EdgeKey = ([i64; 3], [i64; 3]);
/// Counts by the curved side's surface class, in the order of `CURVED`.
type ByClass = [usize; 3];
/// Vertices far from every other face, by their face's and the nearest face's surface class.
type Far = BTreeMap<(String, String), (usize, f64)>;

const CURVED: [&str; 3] = ["RuledSurf", "SurfRev", "ConeSurf"];

fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

fn instance_symbols(graph: &ObjectGraph) -> Vec<u64> {
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

fn uv(v: &Fv) -> Option<[f64; 2]> {
    let a = v.as_array().filter(|a| a.len() == 2)?;
    Some([a[0].as_f64()?, a[1].as_f64()?])
}

/// The class of the surface of the face on side `side` of the edge at `ei`.
fn surface_class(g: &ObjectGraph, ei: usize, side: usize) -> Option<&str> {
    let faces = g.objects[ei].fields.get("m_pFace")?.as_array()?;
    let face = pointer(g, ei, faces.get(side)?).ok()??;
    let si = pointer(g, face, g.objects[face].fields.get("m_pSurf")?).ok()??;
    Some(g.objects[si].class_name.as_str())
}

/// The plane-to-parametric edges of `g` whose curved side changes both parameters, by the
/// curved side's class in the order of `CURVED`.
fn crossing_edges(g: &ObjectGraph) -> ByClass {
    let mut n = [0; 3];
    for (ei, object) in g.objects.iter().enumerate() {
        if object.class_name != "Edge" {
            continue;
        }
        for side in 0..2 {
            let (Some(own), Some(other)) =
                (surface_class(g, ei, side), surface_class(g, ei, 1 - side))
            else {
                continue;
            };
            let Some(class) = CURVED.iter().position(|c| *c == own) else {
                continue;
            };
            if other != "Plane" {
                continue;
            }
            let mut points = Vec::new();
            if let Some(ends) = object
                .fields
                .get("m_firstAndLastEdgePnts")
                .and_then(Fv::as_array)
            {
                points.extend(
                    ends.iter()
                        .filter_map(|p| p.get("uv").and_then(|u| u.get(side)).and_then(uv)),
                );
            }
            if let Some(inner) = object
                .fields
                .get("m_interiorEdgePnts")
                .and_then(Fv::as_array)
            {
                points.extend(
                    inner
                        .iter()
                        .filter_map(|p| p.get("uv").and_then(|u| u.get(side)).and_then(uv)),
                );
            }
            let fixed = |k: usize| points.iter().all(|p| (p[k] - points[0][k]).abs() < 1e-7);
            if points.len() >= 2 && !fixed(0) && !fixed(1) {
                n[class] += 1;
            }
        }
    }
    n
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

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
        let t = dot(ap, n) / nn;
        let foot: V3 = std::array::from_fn(|k| p[k] - t * n[k]);
        let v2 = sub(foot, a);
        let (d00, d01, d11, d20, d21) = (
            dot(ab, ab),
            dot(ab, ac),
            dot(ac, ac),
            dot(v2, ab),
            dot(v2, ac),
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

/// One body's faces: triangles, open-boundary vertices, and their gaps to other faces.
struct Body {
    faces: usize,
    triangles: usize,
    gaps: Vec<f64>,
    /// The boundary vertices more than 0.001 ft from every other face, by the surface class
    /// of their own face and of the face nearest them: how many, and the farthest.
    far: Far,
}

/// The class of the surface of the face a primitive was meshed from: its `object_index` is
/// the `Geometry`, and the face is the one of that object's `m_pFaces` with its tag.
fn face_surface(
    graph: &ObjectGraph,
    kept: &BTreeMap<u64, ObjectGraph>,
    p: &rvt::research::native_saved_mesh::Primitive,
) -> String {
    let g = p
        .source_owner_id
        .and_then(|id| kept.get(&id))
        .unwrap_or(graph);
    g.objects
        .get(p.object_index)
        .and_then(|o| o.fields.get("m_pFaces"))
        .and_then(Fv::as_array)
        .and_then(|list| {
            list.iter()
                .filter_map(|f| pointer(g, p.object_index, f).ok().flatten())
                .find(|i| {
                    g.objects[*i]
                        .fields
                        .get("m_GInfo")
                        .and_then(|info| info.get("m_tag"))
                        .and_then(Fv::as_i64)
                        == Some(p.face_tag)
                })
        })
        .and_then(|fi| {
            let si = pointer(g, fi, g.objects[fi].fields.get("m_pSurf")?).ok()??;
            // #2102: say which faces hold two trim loops, to tell band_mesh's faces apart.
            let first = g.objects[fi]
                .fields
                .get("m_pFirstLoop")
                .and_then(|l| pointer(g, fi, l).ok().flatten());
            let second = first.and_then(|l| {
                g.objects[l]
                    .fields
                    .get("m_nextLoop")
                    .and_then(|n| pointer(g, l, n).ok().flatten())
            });
            let loops = if second.is_some() { "+2loops" } else { "" };
            Some(format!("{}{loops}", g.objects[si].class_name))
        })
        .unwrap_or_else(|| "-".to_string())
}

fn body(graph: &ObjectGraph, kept: &BTreeMap<u64, ObjectGraph>) -> Result<Body, String> {
    let meshes = graphics_with_resolver(graph, &|id| kept.get(&id));
    if let Some(why) = meshes.diagnostics.first() {
        return Err(why.chars().take(60).collect());
    }
    let faces: Vec<_> = meshes
        .primitives
        .iter()
        .filter(|p| !p.hidden && !p.filtered)
        .collect();
    let triangles = faces.iter().map(|f| f.triangles.len()).sum();
    let classes: Vec<String> = faces.iter().map(|f| face_surface(graph, kept, f)).collect();
    let mut gaps = Vec::new();
    let mut far: Far = BTreeMap::new();
    let mut pairs: BTreeMap<(String, String), (usize, usize, usize, usize)> = BTreeMap::new();
    for (fi, face) in faces.iter().enumerate() {
        let key = |i: u32| face.vertices[i as usize].map(|x| (x * 1e6).round() as i64);
        let mut uses: BTreeMap<EdgeKey, usize> = BTreeMap::new();
        for t in &face.triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                let (ka, kb) = (key(a), key(b));
                *uses
                    .entry(if ka < kb { (ka, kb) } else { (kb, ka) })
                    .or_default() += 1;
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
            let mut nearest = fi;
            for (gi, other) in faces.iter().enumerate() {
                if gi == fi {
                    continue;
                }
                for t in &other.triangles {
                    let [a, b, c] = t.map(|i| other.vertices[i as usize]);
                    let d = triangle_distance(p, a, b, c);
                    if d < best {
                        best = d;
                        nearest = gi;
                    }
                }
            }
            // #2104: whether this boundary point is one the nearest face also holds.
            if nearest != fi {
                let shared = faces[nearest]
                    .vertices
                    .iter()
                    .any(|q| (0..3).map(|j| (q[j] - p[j]).powi(2)).sum::<f64>().sqrt() <= 1e-6);
                let entry = pairs
                    .entry((classes[fi].clone(), classes[nearest].clone()))
                    .or_default();
                entry.0 += 1;
                if shared {
                    entry.1 += 1;
                }
                if best > 1e-3 {
                    entry.2 += 1;
                    if shared {
                        entry.3 += 1;
                    }
                }
            }
            if best > 1e-3 {
                let entry = far
                    .entry((classes[fi].clone(), classes[nearest].clone()))
                    .or_default();
                entry.0 += 1;
                entry.1 = entry.1.max(best);
            }
            gaps.push(best);
        }
    }
    for ((own, near), (n, shared, over, over_shared)) in &pairs {
        if *over > 0 {
            println!(
                "pair own={own} nearest={near} boundary={n} shared={shared} over_0.001={over} over_0.001_shared={over_shared}"
            );
        }
    }
    Ok(Body {
        faces: faces.len(),
        triangles,
        gaps,
        far,
    })
}

/// The cylinder faces of `g` with two or more trim loops (#2102 selects the bodies with one).
fn two_loop_cylinders(g: &ObjectGraph) -> usize {
    g.objects
        .iter()
        .enumerate()
        .filter(|(_, o)| o.class_name == "Face")
        .filter(|(fi, o)| {
            let surface = o
                .fields
                .get("m_pSurf")
                .and_then(|p| pointer(g, *fi, p).ok().flatten());
            let first = o
                .fields
                .get("m_pFirstLoop")
                .and_then(|p| pointer(g, *fi, p).ok().flatten());
            let second = first.and_then(|l| {
                g.objects[l]
                    .fields
                    .get("m_nextLoop")
                    .and_then(|p| pointer(g, l, p).ok().flatten())
            });
            surface.is_some_and(|si| g.objects[si].class_name == "CylSurf") && second.is_some()
        })
        .count()
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_2098_face_cracks MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let model = rvt::ifc::RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        rvt::ifc::ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            rvt::ifc::entities::IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut options = native_document::Options {
        selected_ids: ids,
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    // Pass one: each graph's own crossing edges and the symbols it names, keeping no graph.
    let mut crossing_of: BTreeMap<u64, ByClass> = BTreeMap::new();
    let mut symbols_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut owners: BTreeSet<u64> = BTreeSet::new();
    let mut two_loop_of: BTreeMap<u64, usize> = BTreeMap::new();
    let mut note = |r: &Record,
                    crossing_of: &mut BTreeMap<u64, ByClass>,
                    symbols_of: &mut BTreeMap<u64, Vec<u64>>| {
        if let Some(graph) = &r.graph {
            crossing_of.insert(r.identity.element_id, crossing_edges(graph));
            symbols_of.insert(r.identity.element_id, instance_symbols(graph));
            two_loop_of.insert(r.identity.element_id, two_loop_cylinders(graph));
        }
    };
    native_document::extract_graphs(&mut rf, &options, |r| {
        owners.insert(r.identity.element_id);
        note(&r, &mut crossing_of, &mut symbols_of);
        Ok(())
    })?;
    for _ in 0..16 {
        let wanted: BTreeSet<u64> = symbols_of
            .values()
            .flatten()
            .filter(|id| !crossing_of.contains_key(id))
            .copied()
            .collect();
        if wanted.is_empty() {
            break;
        }
        options.selected_ids = wanted;
        native_document::extract_graphs(&mut rf, &options, |r| {
            note(&r, &mut crossing_of, &mut symbols_of);
            Ok(())
        })?;
    }
    // Every symbol a graph reaches, itself included.
    let closure = |id: u64| -> BTreeSet<u64> {
        let mut seen = BTreeSet::from([id]);
        let mut stack = vec![id];
        while let Some(next) = stack.pop() {
            for symbol in symbols_of.get(&next).into_iter().flatten() {
                if seen.insert(*symbol) {
                    stack.push(*symbol);
                }
            }
        }
        seen
    };
    let mut crossing_total: BTreeMap<u64, ByClass> = BTreeMap::new();
    let mut wanted: BTreeSet<u64> = BTreeSet::new();
    for owner in &owners {
        let reach = closure(*owner);
        let mut crossing: ByClass = [0; 3];
        for id in &reach {
            for (k, n) in crossing_of
                .get(id)
                .copied()
                .unwrap_or([0; 3])
                .iter()
                .enumerate()
            {
                crossing[k] += n;
            }
        }
        let two_loop: usize = reach.iter().filter_map(|id| two_loop_of.get(id)).sum();
        if two_loop > 0 {
            let graphs: Vec<u64> = reach
                .iter()
                .filter(|id| two_loop_of.get(id).is_some_and(|n| *n > 0))
                .copied()
                .collect();
            println!("reach owner={owner} two_loop_graphs={graphs:?}");
        }
        if crossing.iter().sum::<usize>() > 0 || two_loop > 0 {
            crossing_total.insert(*owner, crossing);
            wanted.extend(reach);
        }
    }
    // Pass two: the graphs of the bodies with such an edge and of every symbol they reach.
    let mut kept: BTreeMap<u64, ObjectGraph> = BTreeMap::new();
    if !wanted.is_empty() {
        options.selected_ids = wanted;
        native_document::extract_graphs(&mut rf, &options, |r| {
            if let Some(graph) = &r.graph {
                kept.insert(r.identity.element_id, graph.clone());
            }
            Ok(())
        })?;
    }
    let (mut bodies, mut refused, mut triangles, mut over_total) = (0, 0, 0usize, 0);
    let mut largest_total = 0.0_f64;
    for (owner, by_class) in &crossing_total {
        let Some(graph) = kept.get(owner) else {
            continue;
        };
        let crossing = format!(
            "{} crossing_by=RuledSurf:{},SurfRev:{},ConeSurf:{}",
            by_class.iter().sum::<usize>(),
            by_class[0],
            by_class[1],
            by_class[2]
        );
        bodies += 1;
        match body(graph, &kept) {
            Ok(b) => {
                let over = |bar: f64| b.gaps.iter().filter(|g| **g > bar).count();
                let largest = b.gaps.iter().copied().fold(0.0_f64, f64::max);
                triangles += b.triangles;
                over_total += over(1e-3);
                largest_total = largest_total.max(largest);
                println!(
                    "body owner={owner} crossing={crossing} faces={} triangles={} boundary={} over_0.0001={} over_0.001={} largest={largest:.6} refused=-",
                    b.faces,
                    b.triangles,
                    b.gaps.len(),
                    over(1e-4),
                    over(1e-3)
                );
                for ((own, near), (n, most)) in &b.far {
                    println!(
                        "far owner={owner} own={own} nearest={near} vertices={n} largest={most:.6}"
                    );
                }
            }
            Err(why) => {
                refused += 1;
                println!(
                    "body owner={owner} crossing={crossing} faces=0 triangles=0 boundary=0 over_0.0001=0 over_0.001=0 largest=0 refused={}",
                    why.replace(' ', "_")
                );
            }
        }
    }
    println!(
        "total bodies={bodies} refused={refused} triangles={triangles} over_0.001={over_total} largest={largest_total:.6}"
    );
    Ok(())
}
