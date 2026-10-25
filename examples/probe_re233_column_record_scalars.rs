//! RE-233 (#35, W7): every scalar a column's native record holds, for finding
//! which tells the columns Revit's export gives `Pset_ColumnCommon.Slope`.
//!
//! FACT under test: revit-ifc's `SlopeCalculator` (CLEANROOM.md source 5)
//! gives a vertical column the slope of the largest upward face of the last
//! object of its geometry, and nothing when that object has none, as a
//! family instance drawn by its symbol's shared geometry has not. On the
//! BIM4LCA office Revit's export writes `Slope` 0 on 73 of the 279 columns,
//! 13 of 168 of one family, 53 of 79 of another. Hypothesis: one field of
//! the column's record (a cut, a join, own geometry) marks those 73.
//! `tools/re/record_scalars_vs_ifc.py` ranks every field against whether
//! Revit writes the property (`present:`).
//!
//! Output, one line per column the export writes:
//!   rec <tag> <record class> <object class>.<field>=<value> ...
//! for every number, boolean and short string of the first 400 objects of
//! the record, an ElementId as its sign.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re233_column_record_scalars -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const OBJECTS: usize = 400;

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::String(s) if s.len() <= 40 => Some(s.replace(' ', "_")),
        Value::Object(_) => value
            .pointer("/m_id/m_id64")
            .and_then(Value::as_i64)
            .map(|id| format!("@{}", id.signum())),
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
            } if ifc_type.eq_ignore_ascii_case("IFCCOLUMN") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut lines: BTreeMap<u64, String> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: ids.clone(),
            ..Options::default()
        },
        |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let mut seen_classes: BTreeMap<&str, usize> = BTreeMap::new();
            let mut parts = Vec::new();
            for object in graph.objects.iter().take(OBJECTS) {
                let n = seen_classes.entry(object.class_name.as_str()).or_default();
                *n += 1;
                let label = if *n == 1 {
                    object.class_name.clone()
                } else {
                    format!("{}#{n}", object.class_name)
                };
                let Some(fields) = object.fields.as_object() else {
                    continue;
                };
                for (name, value) in fields {
                    if let Some(text) = scalar(value) {
                        parts.push(format!("{label}.{name}={text}"));
                    }
                }
            }
            lines.insert(
                record.identity.element_id,
                format!("{class} {}", parts.join(" ")),
            );
            Ok(())
        },
    )?;
    for id in &ids {
        match lines.get(id) {
            Some(line) => println!("rec {id} {line}"),
            None => println!("rec {id} -"),
        }
    }
    eprintln!("{} columns, {} records", ids.len(), lines.len());
    Ok(())
}
