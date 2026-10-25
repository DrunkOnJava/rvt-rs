//! RE-228 (#35, W7): every scalar a wall's native record holds, for finding
//! which one Revit's export reads `Pset_WallCommon.ExtendToStructure` from.
//!
//! FACT under test: revit-ifc's `ExtendToStructureCalculator` (CLEANROOM.md
//! source 5) gives a wall Revit's own `IsWallJoinedToTop`, which its public
//! source does not show. On the BIM4LCA office Revit's export writes it on
//! all 607 walls, true on 465 and false on 142. Hypothesis: one field of
//! the wall's `SWall` record (an attach-top flag, a top constraint) holds
//! it. `tools/re/record_scalars_vs_ifc.py` ranks every field by how well one
//! of its values tells Revit's true from false.
//!
//! Output, one line per wall the export writes:
//!   rec <tag> <record class> <object class>.<field>=<value> ...
//! for every number, boolean and short string of the first 400 objects of
//! the record, an ElementId as its number.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re228_wall_record_scalars -- MODEL.rvt

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
            } if ifc_type.eq_ignore_ascii_case("IFCWALL")
                || ifc_type.eq_ignore_ascii_case("IFCWALLSTANDARDCASE") =>
            {
                tag.parse().ok()
            }
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
    eprintln!("{} walls, {} records", ids.len(), lines.len());
    Ok(())
}
