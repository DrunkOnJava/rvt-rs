//! RE-302: the saved `GEllipse` curves that profile a ruled surface or a
//! surface of revolution (#1247, RE-224).
//!
//! FACT under test: `native_parametric_surface::Curve::read` takes a
//! profile only as a `GLine` or a `GArc`, so a `RuledSurf` or `SurfRev`
//! whose profile is a `GEllipse` fails, and RE-224 keeps the element's box:
//! 51 BIM4LCA office elements after RE-304 (Measure 41572297689). This
//! prints what a `GEllipse` holds, and which classes point to it, so the
//! ellipse can be evaluated as an arc is.
//!
//! Output: one `ellipse` line per object for the first 8 per file, with its
//! owner ElementId, the classes pointing to it and its fields, then `census`
//! lines: the ellipse and profile counts, the field sets, and the parent
//! classes.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re302_ellipse_profiles -- model.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re302_ellipse_profiles FILE.rvt");
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
    let mut field_sets: BTreeMap<String, usize> = BTreeMap::new();
    let mut parents: BTreeMap<String, usize> = BTreeMap::new();
    let mut profiles = 0usize;
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let mut in_edges: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for edge in &graph.edges {
            in_edges
                .entry(edge.target_object_index)
                .or_default()
                .push(edge.source_object_index);
        }
        for (index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "GEllipse" {
                continue;
            }
            total += 1;
            let keys: Vec<&str> = object
                .fields
                .as_object()
                .map(|map| map.keys().map(String::as_str).collect())
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
            let is_profile = parent_classes
                .iter()
                .any(|class| matches!(*class, "RuledSurf" | "SurfRev"));
            if is_profile {
                profiles += 1;
            }
            if is_profile && printed < 8 {
                printed += 1;
                let fields: Vec<String> = object
                    .fields
                    .as_object()
                    .map(|map| {
                        map.iter()
                            .map(|(key, value)| match value {
                                Value::Object(inner) if inner.contains_key("pointer_token") => {
                                    format!("{key}=->")
                                }
                                other => {
                                    let text = other.to_string();
                                    format!("{key}={}", text.chars().take(120).collect::<String>())
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                println!(
                    "ellipse owner={} parents={:?} {}",
                    record.identity.element_id,
                    parent_classes,
                    fields.join(" ")
                );
            }
        }
        Ok(())
    })?;
    println!("census ellipses={total} profiles={profiles}");
    for (set, n) in &field_sets {
        println!("census fields {n} {set}");
    }
    for (class, n) in &parents {
        println!("census parent {n} {class}");
    }
    Ok(())
}
