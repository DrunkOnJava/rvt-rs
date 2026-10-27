//! RE-254 (#1016, W7): the curves each railing's native record holds, for
//! finding Revit's `Qto_RailingBaseQuantities.Length`.
//!
//! FACT under test: revit-ifc's `LengthCalculator` (CLEANROOM.md source 5)
//! takes a railing's `Length` from its `CURVE_ELEM_LENGTH` parameter, its
//! path's length. Hypothesis: the railing's record holds its path as `GLine`
//! and `GArc` objects whose lengths add up to it.
//! `tools/re/railing_length_vs_ifc.py` compares the sums with Revit's export.
//!
//! Output, one line per railing the export writes:
//!   rail <tag> <line length sum> <arc length sum> <curve classes ...>
//! lengths in feet, each `GLine` the span of its `m_endParams` and each
//! `GArc` its radius times the span of its `m_endParams` (radians); then
//! `classes <record class> <object class>=<count> ...` per railing, and for
//! the first three railings each field of each object of its record as
//! `field <tag> <class> <name> <value as JSON, cut at 400 characters>`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re254_railing_paths -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

fn pair(value: Option<&Value>) -> Option<(f64, f64)> {
    match value?.as_array()?.as_slice() {
        [a, b] => Some((a.as_f64()?, b.as_f64()?)),
        _ => None,
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCRAILING") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut rows: BTreeMap<u64, (f64, f64, Vec<String>)> = BTreeMap::new();
    let mut census: BTreeMap<u64, (String, BTreeMap<String, usize>)> = BTreeMap::new();
    let dumped: BTreeSet<u64> = ids.iter().copied().take(3).collect();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: ids.clone(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let (mut lines, mut arcs, mut classes) = (0.0, 0.0, Vec::new());
            let id = record.identity.element_id;
            let entry = census.entry(id).or_default();
            entry.0 = record.class_name.clone().unwrap_or_default();
            for object in &graph.objects {
                *entry.1.entry(object.class_name.clone()).or_default() += 1;
                if dumped.contains(&id) {
                    if let Some(fields) = object.fields.as_object() {
                        for (name, value) in fields {
                            let value = value.to_string();
                            let cut: String = value.chars().take(400).collect();
                            println!("field {id} {} {name} {cut}", object.class_name);
                        }
                    }
                }
            }
            for object in &graph.objects {
                let name = object.class_name.as_str();
                if !name.starts_with('G')
                    || !(name.contains("Line") || name.contains("Arc") || name.contains("Curve"))
                {
                    continue;
                }
                classes.push(name.to_string());
                let Some((a, b)) = pair(object.fields.get("m_endParams")) else {
                    continue;
                };
                match name {
                    "GLine" => lines += (b - a).abs(),
                    "GArc" => {
                        if let Some(radius) = object.fields.get("m_radius").and_then(Value::as_f64)
                        {
                            arcs += radius * (b - a).abs();
                        }
                    }
                    _ => {}
                }
            }
            rows.insert(record.identity.element_id, (lines, arcs, classes));
            Ok(())
        },
    )?;
    for id in &ids {
        match rows.get(id) {
            Some((lines, arcs, classes)) => {
                println!("rail {id} {lines} {arcs} {}", classes.join(" "))
            }
            None => println!("rail {id} - -"),
        }
    }
    for (id, (class, objects)) in &census {
        let counts: Vec<String> = objects.iter().map(|(c, n)| format!("{c}={n}")).collect();
        println!("classes {id} {class} {}", counts.join(" "));
    }
    eprintln!("{} railings, {} records", ids.len(), rows.len());
    Ok(())
}
