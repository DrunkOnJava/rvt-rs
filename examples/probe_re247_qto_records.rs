//! RE-247 (#1016, W7): every scalar of a column's or curtain panel's native
//! record, and the object classes of its host's, for finding which tells the
//! ones Revit's export gives no base quantity drawn from its body.
//!
//! FACT under test: Revit's export gives a column `Qto_ColumnBaseQuantities.Length`
//! wherever it draws the column as an extrusion, and a curtain panel
//! `Qto_PlateBaseQuantities.Width` on all but three of the BIM4LCA office's
//! panels. On the office at permit it draws 18 columns as a mesh. Hypothesis:
//! a field of the element's record or a class of its host's (a cut, a join,
//! an edited profile) marks them. `tools/re/record_scalars_vs_ifc.py` ranks
//! every field against `body:not-extruded` and whether Revit writes each
//! quantity (`present:`).
//!
//! Output, one line per column and curtain panel the export writes:
//!   rec <tag> <record class> <object class>.<field>=<value> ... H.<class>=true ... by.<class>.<entity>=<n> ...
//! for every number, boolean and short string of the first 400 objects of
//! the record, an ElementId as its sign, each object class of the record
//! its `m_hostId` names, and for each pair step (`CutPairGStep`,
//! `CleanPairGStep`, ...) the number of them whose `m_otherId` names an
//! element rvt-rs exports as that IFC entity (else its record class).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re247_qto_records -- MODEL.rvt

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
    let entity_of: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } => tag
                .parse()
                .ok()
                .map(|id| (id, ifc_type.to_ascii_uppercase())),
            _ => None,
        })
        .collect();
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCCOLUMN")
                || ifc_type.eq_ignore_ascii_case("IFCPLATE") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect();
    let mut lines: BTreeMap<u64, String> = BTreeMap::new();
    let mut hosts: BTreeMap<u64, u64> = BTreeMap::new();
    let mut others: BTreeMap<u64, Vec<(String, u64)>> = BTreeMap::new();
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
            let host = graph.objects.iter().find_map(|object| {
                object
                    .fields
                    .get("m_hostId")
                    .and_then(|value| value.pointer("/m_id/m_id64"))
                    .and_then(Value::as_i64)
                    .and_then(|id| u64::try_from(id).ok())
            });
            if let Some(host) = host {
                hosts.insert(record.identity.element_id, host);
            }
            for object in &graph.objects {
                if !object.class_name.ends_with("PairGStep") {
                    continue;
                }
                let other = object
                    .fields
                    .get("m_otherId")
                    .and_then(|value| value.pointer("/m_id/m_id64"))
                    .and_then(Value::as_i64)
                    .and_then(|id| u64::try_from(id).ok());
                if let Some(other) = other {
                    others
                        .entry(record.identity.element_id)
                        .or_default()
                        .push((object.class_name.clone(), other));
                }
            }
            lines.insert(
                record.identity.element_id,
                format!("{class} {}", parts.join(" ")),
            );
            Ok(())
        },
    )?;
    let mut host_classes: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    let mut record_class: BTreeMap<u64, String> = BTreeMap::new();
    let mut second: BTreeSet<u64> = hosts.values().copied().collect();
    second.extend(others.values().flatten().map(|(_, other)| *other));
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: second,
            ..Options::default()
        },
        |record| {
            if let Some(class) = record.class_name.as_deref() {
                record_class.insert(record.identity.element_id, class.to_string());
            }
            if let Some(graph) = record.graph.as_ref() {
                host_classes
                    .entry(record.identity.element_id)
                    .or_default()
                    .extend(graph.objects.iter().map(|object| object.class_name.clone()));
            }
            Ok(())
        },
    )?;
    for id in &ids {
        match lines.get(id) {
            Some(line) => {
                let host = hosts
                    .get(id)
                    .and_then(|host| host_classes.get(host))
                    .map(|classes| {
                        classes
                            .iter()
                            .map(|class| format!(" H.{class}=true"))
                            .collect::<String>()
                    })
                    .unwrap_or_default();
                let mut by: BTreeMap<String, usize> = BTreeMap::new();
                for (step, other) in others.get(id).into_iter().flatten() {
                    let kind = entity_of
                        .get(other)
                        .or_else(|| record_class.get(other))
                        .map_or("-", String::as_str);
                    *by.entry(format!("by.{step}.{kind}")).or_default() += 1;
                }
                let by: String = by.iter().map(|(name, n)| format!(" {name}={n}")).collect();
                println!("rec {id} {line}{host}{by}");
            }
            None => println!("rec {id} -"),
        }
    }
    eprintln!("{} columns and panels, {} records", ids.len(), lines.len());
    Ok(())
}
