//! RE-241 (#35, W7): every scalar a curtain mullion's or panel's native
//! record holds, its type's and its host's, for finding which tells the
//! mullions Revit's export gives `Pset_MemberCommon.IsExternal` true.
//!
//! FACT under test: on the BIM4LCA office Revit's export writes
//! `IsExternal` on all 10,003 mullions, true on 498, and on 260 panels'
//! `Pset_PlateCommon`, true on 30. Neither the curtain wall holding a
//! mullion (RE-193 §6, RE-226 §2) nor its type decides, and the mullions
//! hold no saved Yes/No parameter (RE-239 §5). Hypothesis: a field of the
//! mullion's own record, its type's (`m_masterSymbolId`) or its host's
//! (`m_hostId`) does. `tools/re/record_scalars_vs_ifc.py` ranks every field
//! against the property.
//!
//! Output, one line per member and plate the export writes:
//!   rec <tag> <record class> <object class>.<field>=<value> ...
//! for every number, boolean and short string of the first 400 objects of
//! the element's record, then those of the first 60 objects of its type's
//! record and the first 20 of its host's, prefixed `T.` and `H.`; an
//! ElementId as its sign.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re241_mullion_record_scalars -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use serde_json::Value;

const OBJECTS: usize = 400;
const TYPE_OBJECTS: usize = 60;
const HOST_OBJECTS: usize = 20;

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

fn scalars(graph: &ObjectGraph, prefix: &str, limit: usize) -> Vec<String> {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut parts = Vec::new();
    for object in graph.objects.iter().take(limit) {
        let n = seen.entry(object.class_name.as_str()).or_default();
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
                parts.push(format!("{prefix}{label}.{name}={text}"));
            }
        }
    }
    parts
}

fn linked(graph: &ObjectGraph, field: &str) -> Option<u64> {
    graph.objects.iter().find_map(|object| {
        object
            .fields
            .get(field)?
            .pointer("/m_id/m_id64")?
            .as_i64()
            .and_then(|id| u64::try_from(id).ok())
    })
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
            } if ifc_type.eq_ignore_ascii_case("IFCMEMBER")
                || ifc_type.eq_ignore_ascii_case("IFCPLATE") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect();
    let mut lines: BTreeMap<u64, (String, Vec<String>)> = BTreeMap::new();
    let mut links: BTreeMap<u64, [Option<u64>; 2]> = BTreeMap::new();
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
            let id = record.identity.element_id;
            links.insert(
                id,
                [linked(graph, "m_masterSymbolId"), linked(graph, "m_hostId")],
            );
            lines.insert(id, (class.to_string(), scalars(graph, "", OBJECTS)));
            Ok(())
        },
    )?;
    let others: BTreeSet<u64> = links.values().flatten().flatten().copied().collect();
    let mut other_scalars: BTreeMap<u64, (Vec<String>, Vec<String>)> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: others,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            other_scalars.insert(
                record.identity.element_id,
                (
                    scalars(graph, "T.", TYPE_OBJECTS),
                    scalars(graph, "H.", HOST_OBJECTS),
                ),
            );
            Ok(())
        },
    )?;
    for id in &ids {
        let Some((class, parts)) = lines.get(id) else {
            println!("rec {id} -");
            continue;
        };
        let [kind, host] = links.get(id).copied().unwrap_or([None, None]);
        let mut all = parts.clone();
        if let Some((typed, _)) = kind.and_then(|k| other_scalars.get(&k)) {
            all.extend(typed.iter().cloned());
        }
        if let Some((_, hosted)) = host.and_then(|h| other_scalars.get(&h)) {
            all.extend(hosted.iter().cloned());
        }
        println!("rec {id} {class} {}", all.join(" "));
    }
    eprintln!("{} members and plates, {} records", ids.len(), lines.len());
    Ok(())
}
