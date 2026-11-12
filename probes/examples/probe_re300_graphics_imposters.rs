//! RE-300: what a saved graphics `GImposter` holds (#1247, RE-224).
//!
//! FACT: a `GImposter` is a light source. It holds a transform (`m_trf`) and
//! a rendering `Asset` of scalar properties, and no surface. Its parent is
//! always a `GFilter` of its own (`m_flags` 557060) that also holds the shape
//! and lines that draw the light: a `Geometry`, and often a `GPolyMesh`
//! whose normal binding is 2. Revit's export draws none of that filter:
//! with it left out, the 39 BIM4LCA office and 2 RE1 Electrical bodies it
//! kept as boxes are exact to 0.001 ft. With only the `GImposter` left out,
//! the office's exit signs were 12 times Revit's volume.
//!
//! `native_graphics_traversal` stopped at the class, and
//! `saved_meshes::attach` then kept the element's box (RE-224): 1,813 of the
//! 3,020 such elements on the BIM4LCA office, RE1 and Autodesk's 2024
//! samples, every one of Snowdon Towers electrical's 511 light fixtures.
//!
//! Output: one `imposter` line per object for the first 8 per file, with
//! its owner ElementId, scalar fields, parents and pointer targets, and a
//! `siblings` line with what else its parent points to (a `GPolyMesh`'s
//! normal binding and counts, a condition's fields), then `census` lines
//! over all of them: field sets, parents, flags, the classes found under
//! them within 8 pointer steps, and their parents' sets of children.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re300_graphics_imposters -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};

fn scalars(fields: &Fv) -> String {
    let Some(map) = fields.as_object() else {
        return String::new();
    };
    map.iter()
        .map(|(key, value)| {
            let text = match value {
                Fv::Object(inner) if inner.contains_key("pointer_token") => "->".to_string(),
                Fv::Array(items) => format!("[{}]", items.len()),
                Fv::Object(_) => {
                    let text = value.to_string();
                    text.chars().take(80).collect()
                }
                other => other.to_string(),
            };
            format!("{key}={text}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A `GPolyMesh`'s normal binding and its points, normals and facets.
fn polymesh(
    graph: &rvt::research::native_parameters::ObjectGraph,
    out_edges: &BTreeMap<usize, Vec<(usize, usize)>>,
    index: usize,
) -> String {
    let Some(topology) = out_edges.get(&index).and_then(|edges| {
        edges
            .iter()
            .map(|(_, target)| &graph.objects[*target])
            .find(|object| object.class_name.starts_with("FacetedTopology"))
    }) else {
        return "no topology".into();
    };
    let f = &topology.fields;
    let len = |key: &str| f[key].as_array().map_or(0, <[Fv]>::len);
    format!(
        "flag={} points={} normals={} facets={}",
        f["m_normalsFlag"],
        len("m_pointsArr"),
        len("m_normalsArr"),
        len("m_facetsArr")
    )
}

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re300_graphics_imposters FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let options = Options {
        selected_ids: rvt::elem_table::declared_ids(&records)
            .into_iter()
            .map(u64::from)
            .collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut printed = 0;
    let mut total = 0usize;
    let mut owners = BTreeSet::new();
    let mut field_sets: BTreeMap<String, usize> = BTreeMap::new();
    let mut parents: BTreeMap<String, usize> = BTreeMap::new();
    let mut flags: BTreeMap<String, usize> = BTreeMap::new();
    let mut under: BTreeMap<String, usize> = BTreeMap::new();
    let mut sibling_sets: BTreeMap<String, usize> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let mut out_edges: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        let mut in_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for edge in &graph.edges {
            out_edges
                .entry(edge.source_object_index)
                .or_default()
                .push((edge.pointer_offset, edge.target_object_index));
            in_edges
                .entry(edge.target_object_index)
                .or_default()
                .push(edge.source_object_index);
        }
        for (index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "GImposter" {
                continue;
            }
            total += 1;
            owners.insert(record.identity.element_id);
            let keys: Vec<&str> = object
                .fields
                .as_object()
                .map(|map| map.keys().collect())
                .unwrap_or_default();
            *field_sets.entry(keys.join(",")).or_default() += 1;
            let parent_classes: Vec<&str> = in_edges
                .get(&index)
                .map(|sources| {
                    sources
                        .iter()
                        .map(|&s| graph.objects[s].class_name.as_str())
                        .collect()
                })
                .unwrap_or_default();
            for class in &parent_classes {
                *parents.entry((*class).to_string()).or_default() += 1;
            }
            *flags
                .entry(object.fields["m_GInfo"]["m_flags"].to_string())
                .or_default() += 1;
            let mut seen = BTreeSet::from([index]);
            let mut queue = VecDeque::from([(index, 0)]);
            let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
            while let Some((at, depth)) = queue.pop_front() {
                if depth == 8 {
                    continue;
                }
                for &(_, target) in out_edges.get(&at).map(Vec::as_slice).unwrap_or(&[]) {
                    if seen.insert(target) {
                        *classes
                            .entry(graph.objects[target].class_name.as_str())
                            .or_default() += 1;
                        queue.push_back((target, depth + 1));
                    }
                }
            }
            for (class, n) in &classes {
                *under.entry((*class).to_string()).or_default() += n;
            }
            if let Some(&parent) = in_edges.get(&index).and_then(|p| p.first()) {
                let siblings: Vec<String> = out_edges
                    .get(&parent)
                    .map(|edges| {
                        edges
                            .iter()
                            .map(|(offset, target)| {
                                let object = &graph.objects[*target];
                                let detail = match object.class_name.as_str() {
                                    "GPolyMesh" => polymesh(graph, &out_edges, *target),
                                    name if name.starts_with("GCondition") => {
                                        scalars(&object.fields)
                                    }
                                    _ => String::new(),
                                };
                                format!("+{offset}:{}{{{detail}}}", object.class_name)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                *sibling_sets
                    .entry(
                        siblings
                            .iter()
                            .map(|s| s.split(['{', ':']).nth(1).unwrap_or("").to_string())
                            .collect::<Vec<_>>()
                            .join(","),
                    )
                    .or_default() += 1;
                if printed < 8 {
                    println!(
                        "siblings owner={} filter={} grandparents={:?} {}",
                        record.identity.element_id,
                        scalars(&graph.objects[parent].fields),
                        in_edges
                            .get(&parent)
                            .map(|g| g
                                .iter()
                                .map(|&i| graph.objects[i].class_name.clone())
                                .collect::<Vec<_>>())
                            .unwrap_or_default(),
                        siblings.join(" ")
                    );
                }
            }
            if printed < 8 {
                printed += 1;
                let targets: Vec<String> = out_edges
                    .get(&index)
                    .map(|edges| {
                        edges
                            .iter()
                            .map(|(offset, target)| {
                                format!("+{offset}:{}", graph.objects[*target].class_name)
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                println!(
                    "imposter owner={} record={} parents={:?} targets={:?} under={:?} fields: {}",
                    record.identity.element_id,
                    record.class_name.as_deref().unwrap_or("?"),
                    parent_classes,
                    targets,
                    classes,
                    scalars(&object.fields)
                );
            }
        }
        Ok(())
    })?;
    println!("census imposters={total} owners={}", owners.len());
    for (set, n) in &field_sets {
        println!("census fields {n} {set}");
    }
    for (class, n) in &parents {
        println!("census parent {n} {class}");
    }
    for (value, n) in &flags {
        println!("census flags {n} {value}");
    }
    for (class, n) in &under {
        println!("census under {n} {class}");
    }
    for (set, n) in &sibling_sets {
        println!("census siblings {n} {set}");
    }
    Ok(())
}
