//! RE-226 (#35, W7): what host a curtain mullion's or panel's native record
//! names, for the `IsExternal` RE-193 §6 could not decide.
//!
//! FACT under test: revit-ifc's `CategoryUtil.IsElementExternal`
//! (CLEANROOM.md source 5) gives an element with no `IsExternal` parameter
//! and no type Function its host's value when it is a family instance with
//! a `Host`, and false otherwise. RE-193 took the curtain wall that holds a
//! mullion, and on the BIM4LCA office mullions of one type on external
//! curtain walls were not external 8,460 times and external 397 times.
//! Hypothesis: those 397 are the members whose own record names a host
//! (`m_hostId`), and the others name none.
//!
//! Output, for every `IfcMember` and `IfcPlate` the export writes:
//!   member <tag> <IFC class> <record class> host=<id|-> hostclass=<class|-> <member=value ...>
//! where the members are the record root's fields whose names hold `host`,
//! `grid`, `wall`, `curtain`, `panel` or `mullion` (any case), scalars as
//! they are and element references as `@<id>`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re226_mullion_hosts -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const WORDS: [&str; 6] = ["host", "grid", "wall", "curtain", "panel", "mullion"];

fn element_id(value: &Value) -> Option<i64> {
    value.pointer("/m_id/m_id64")?.as_i64()
}

fn described(fields: &Value) -> String {
    fields
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .filter(|(name, _)| {
            let lower = name.to_ascii_lowercase();
            WORDS.iter().any(|w| lower.contains(w))
        })
        .filter_map(|(name, value)| match value {
            Value::Number(_) | Value::Bool(_) => Some(format!("{name}={value}")),
            Value::String(text) => Some(format!("{name}={}", text.replace(' ', "_"))),
            Value::Array(items) => Some(format!("{name}=[{}]", items.len())),
            _ => element_id(value).map(|id| format!("{name}=@{id}")),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if upper == "IFCMEMBER" || upper == "IFCPLATE" {
                if let Ok(id) = tag.parse() {
                    classes.insert(id, upper);
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let mut rows: BTreeMap<u64, (String, Option<i64>, String)> = BTreeMap::new();
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
            let Some(root) = graph.objects.first() else {
                return Ok(());
            };
            let host = root.fields.get("m_hostId").and_then(element_id);
            rows.insert(
                record.identity.element_id,
                (class.to_string(), host, described(&root.fields)),
            );
            Ok(())
        },
    )?;
    let hosts: BTreeSet<u64> = rows
        .values()
        .filter_map(|(_, host, _)| host.and_then(|h| u64::try_from(h).ok()))
        .collect();
    let mut host_classes: BTreeMap<u64, String> = BTreeMap::new();
    if !hosts.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: hosts,
                ..Options::default()
            },
            |record| {
                if let Some(class) = record.class_name.as_deref() {
                    host_classes.insert(record.identity.element_id, class.to_string());
                }
                Ok(())
            },
        )?;
    }
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for (tag, ifc_class) in &classes {
        let Some((class, host, fields)) = rows.get(tag) else {
            println!("member {tag} {ifc_class} - host=- hostclass=-");
            continue;
        };
        let host_class = host
            .and_then(|h| u64::try_from(h).ok())
            .and_then(|h| host_classes.get(&h))
            .map_or("-", String::as_str);
        let host_text = host.map_or("-".to_string(), |h| h.to_string());
        *counts
            .entry((class.clone(), host_class.to_string()))
            .or_default() += 1;
        println!(
            "member {tag} {ifc_class} {class} host={host_text} hostclass={host_class} {fields}"
        );
    }
    eprintln!(
        "{} members and plates, {} records; by record class and host class: {:?}",
        classes.len(),
        rows.len(),
        counts
    );
    Ok(())
}
