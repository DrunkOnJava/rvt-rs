//! RE-422 (#1683): what the saved graph of a roof that keeps its level block
//! holds at the edge `ruled_surface_component` refuses.
//!
//! FACT under test: four roofs of Autodesk's Snowdon Towers Architectural
//! (Revit 2024; ElementIds 1926040, 1975810, 1985275 and 2112673) keep the
//! level block of their sketch outline because one edge of the ruled-surface
//! component their faces make has an `m_pFace` that is `Face + none`
//! (RE-420, #1683): one `Face` on a side and nothing on the other. This
//! prints, for each of them, the classes of the graph's objects, the surface
//! class of every `Face`, the shapes of the `m_pFace` of every `Edge`, the
//! components the faces make through the edges that hold two faces (the
//! component the guard builds), and the first edges that hold less, with the
//! face they border, that face's surface class and the number of edges that
//! touch it.
//!
//! Output, one line per fact:
//!   roof <id> objects=<n> <class>=<n> ...
//!   roof <id> face surfaces: <class>=<n> ...
//!   roof <id> edge shapes: <shape>=<n> ...      (a shape is the class each
//!     `m_pFace` entry resolves to, or `none`, joined by ` + `)
//!   roof <id> component faces=<n> touching_open_edge=<n> surfaces: <class>=<n> ...
//!   roof <id> open edge <index> shape=<shape> face=<index|-> surf=<class>
//!     face_edges=<n> <the edge's fields, pointers followed one deep>
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re422_ruled_boundary -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::pointer;
use serde_json::Value;

const ROOFS: [u64; 4] = [1926040, 1975810, 1985275, 2112673];
/// How many open edges of each roof are listed with their fields.
const OPEN_LISTED: usize = 12;
/// How many components of each roof are listed, the largest first.
const COMPONENTS_LISTED: usize = 20;

fn class_of(g: &ObjectGraph, index: Option<usize>) -> &str {
    index
        .and_then(|i| g.objects.get(i))
        .map_or("none", |object| object.class_name.as_str())
}

fn clip(text: String, n: usize) -> String {
    text.chars().take(n).collect()
}

fn describe(g: &ObjectGraph, oi: usize, depth: usize) -> String {
    let Some(o) = g.objects.get(oi) else {
        return "?".into();
    };
    let mut parts = Vec::new();
    if let Some(map) = o.fields.as_object() {
        for (k, v) in map {
            parts.push(format!("{k}={}", value_text(g, oi, v, depth)));
        }
    }
    format!("{}#{oi}{{{}}}", o.class_name, parts.join(" "))
}

fn value_text(g: &ObjectGraph, oi: usize, v: &Value, depth: usize) -> String {
    match v {
        _ if rvt::research::native_parameters::pointer_parts(v).is_some() => {
            match pointer(g, oi, v) {
                Ok(Some(t)) if depth > 0 => describe(g, t, depth - 1),
                Ok(Some(t)) => format!("->{}#{t}", g.objects[t].class_name),
                Ok(None) => "null".into(),
                Err(_) => "->?".into(),
            }
        }
        Value::Array(a) => {
            let items: Vec<String> = a
                .iter()
                .take(6)
                .map(|x| value_text(g, oi, x, depth))
                .collect();
            let more = if a.len() > 6 {
                format!(",..{}", a.len())
            } else {
                String::new()
            };
            format!("[{}{more}]", items.join(","))
        }
        Value::Object(m) => {
            let items: Vec<String> = m
                .iter()
                .map(|(k, x)| format!("{k}:{}", value_text(g, oi, x, depth)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        other => other.to_string(),
    }
}

fn histogram(counts: &BTreeMap<&str, usize>) -> String {
    counts
        .iter()
        .map(|(class, n)| format!("{class}={n}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re422_ruled_boundary FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let options = Options {
        selected_ids: ROOFS.iter().copied().collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut records: BTreeMap<u64, Record> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |r| {
        records.insert(r.identity.element_id, r);
        Ok(())
    })?;
    for id in ROOFS {
        let Some(record) = records.get(&id) else {
            println!("roof {id} not read");
            continue;
        };
        let Some(g) = &record.graph else {
            println!(
                "roof {id} no graph: {}",
                record.diagnostic.clone().unwrap_or_default()
            );
            continue;
        };
        let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
        for object in &g.objects {
            *classes.entry(object.class_name.as_str()).or_default() += 1;
        }
        println!(
            "roof {id} objects={} {}",
            g.objects.len(),
            histogram(&classes)
        );
        // The surface class of each face.
        let mut surfaces: BTreeMap<&str, usize> = BTreeMap::new();
        let mut face_surface: BTreeMap<usize, &str> = BTreeMap::new();
        for (index, object) in g
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| object.class_name == "Face")
        {
            let surface = object
                .fields
                .get("m_pSurf")
                .and_then(|value| pointer(g, index, value).ok().flatten());
            let class = class_of(g, surface);
            *surfaces.entry(class).or_default() += 1;
            face_surface.insert(index, class);
        }
        println!("roof {id} face surfaces: {}", histogram(&surfaces));
        // The edges: their shapes, the faces they join and the faces they
        // leave open.
        let mut shapes: BTreeMap<String, usize> = BTreeMap::new();
        let mut adjacency: BTreeMap<usize, BTreeSet<usize>> = BTreeMap::new();
        let mut face_edges: BTreeMap<usize, usize> = BTreeMap::new();
        let mut open: Vec<(usize, Vec<Option<usize>>)> = Vec::new();
        for (index, object) in g
            .objects
            .iter()
            .enumerate()
            .filter(|(_, object)| object.class_name == "Edge")
        {
            let Some(entries) = object.fields.get("m_pFace").and_then(Value::as_array) else {
                *shapes.entry("no m_pFace".into()).or_default() += 1;
                continue;
            };
            let resolved: Vec<Option<usize>> = entries
                .iter()
                .map(|value| pointer(g, index, value).ok().flatten())
                .collect();
            let shape = resolved
                .iter()
                .map(|target| class_of(g, *target))
                .collect::<Vec<_>>()
                .join(" + ");
            *shapes.entry(shape).or_default() += 1;
            let faces: Vec<usize> = resolved
                .iter()
                .filter_map(|target| *target)
                .filter(|target| face_surface.contains_key(target))
                .collect();
            for face in &faces {
                *face_edges.entry(*face).or_default() += 1;
            }
            if entries.len() == 2 && faces.len() == 2 {
                adjacency.entry(faces[0]).or_default().insert(faces[1]);
                adjacency.entry(faces[1]).or_default().insert(faces[0]);
            } else {
                open.push((index, resolved));
            }
        }
        println!(
            "roof {id} edge shapes: {}",
            shapes
                .iter()
                .map(|(shape, n)| format!("[{shape}]={n}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        // The components the guard builds: faces joined through edges that
        // hold two faces.
        let open_faces: BTreeSet<usize> = open
            .iter()
            .flat_map(|(_, resolved)| resolved.iter().filter_map(|target| *target))
            .filter(|target| face_surface.contains_key(target))
            .collect();
        let mut seen: BTreeSet<usize> = BTreeSet::new();
        let mut components: Vec<BTreeSet<usize>> = Vec::new();
        for &start in face_surface.keys() {
            if !seen.insert(start) {
                continue;
            }
            let mut component = BTreeSet::from([start]);
            let mut queue = VecDeque::from([start]);
            while let Some(face) = queue.pop_front() {
                for &other in adjacency.get(&face).into_iter().flatten() {
                    if component.insert(other) {
                        queue.push_back(other);
                    }
                }
            }
            seen.extend(&component);
            components.push(component);
        }
        components.sort_by_key(|component| std::cmp::Reverse(component.len()));
        println!("roof {id} components {}", components.len());
        for component in components.iter().take(COMPONENTS_LISTED) {
            let mut histogram_of: BTreeMap<&str, usize> = BTreeMap::new();
            for face in component {
                *histogram_of.entry(face_surface[face]).or_default() += 1;
            }
            let touching = component
                .iter()
                .filter(|face| open_faces.contains(face))
                .count();
            println!(
                "roof {id} component faces={} touching_open_edge={touching} surfaces: {}",
                component.len(),
                histogram(&histogram_of)
            );
        }
        for (index, resolved) in open.iter().take(OPEN_LISTED) {
            let face = resolved
                .iter()
                .filter_map(|target| *target)
                .find(|target| face_surface.contains_key(target));
            let shape = resolved
                .iter()
                .map(|target| class_of(g, *target))
                .collect::<Vec<_>>()
                .join(" + ");
            println!(
                "roof {id} open edge {index} shape=[{shape}] face={} surf={} face_edges={} {}",
                face.map_or_else(|| "-".to_string(), |f| f.to_string()),
                face.map_or("-", |f| face_surface[&f]),
                face.map_or(0, |f| face_edges.get(&f).copied().unwrap_or(0)),
                clip(describe(g, *index, 0), 700)
            );
        }
        println!("roof {id} open edges {}", open.len());
    }
    eprintln!("{} roofs read", records.len());
    Ok(())
}
