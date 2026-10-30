//! #1117: where a viewport's position on its sheet is stored.
//!
//! QUESTION under test: which members of a viewport's native records give
//! its centre on its sheet. No Revit IFC export carries sheets or
//! viewports, so no reference model can score the answer; the oracle is a
//! comparison with the centres the Revit API reports for the same file.
//! This probe lists the candidates for that comparison.
//!
//! The schema names four viewport classes (`Viewport` with `m_offset`,
//! `m_labelOrigin`, `m_scale`, `m_dbViewId`, `m_dbDrawingId` and
//! `m_oPlaceholderBoxOutline` among its members, `GCSViewport` derived from
//! it, `ViewportAttributes` and `ViewportInfo`).
//!
//! Output:
//!   record <json>                 every record of those classes, in full:
//!                                 identity, class, status and object graph
//!   ref <record> <class> <path> <id> <class> <members>
//!                                 each element id a record's root object
//!                                 holds, with the class and scalar members
//!                                 of the record it names (a sheet, a view)
//!   summary <class> <records>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1117_viewports -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const VIEWPORT_CLASSES: [&str; 4] = [
    "Viewport",
    "GCSViewport",
    "ViewportAttributes",
    "ViewportInfo",
];

fn find_ids(value: &Value, path: String, out: &mut Vec<(String, u64)>) {
    match value {
        Value::Object(members) => {
            if let Some(id) = members.get("m_id64").and_then(Value::as_u64) {
                out.push((path.clone(), id));
            }
            for (name, member) in members {
                find_ids(member, format!("{path}.{name}"), out);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                find_ids(item, format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn scalars(fields: &Value) -> String {
    let Some(members) = fields.as_object() else {
        return String::new();
    };
    members
        .iter()
        .filter(|(_, value)| value.is_number() || value.is_boolean() || value.is_string())
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: VIEWPORT_CLASSES
            .iter()
            .map(|class| class.to_string())
            .collect(),
        ..Options::default()
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut refs: Vec<(u64, String, String, u64)> = Vec::new();
    extract_graphs(&mut rf, &options, |record| {
        let class = record.class_name.clone().unwrap_or_default();
        *counts.entry(class.clone()).or_default() += 1;
        if let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        {
            let mut found = Vec::new();
            find_ids(&root.fields, String::new(), &mut found);
            for (member, id) in found {
                if id != record.identity.element_id {
                    refs.push((record.identity.element_id, class.clone(), member, id));
                }
            }
        }
        println!(
            "record {}",
            serde_json::to_string(&record).unwrap_or_default()
        );
        Ok(())
    })?;
    let wanted: BTreeSet<u64> = refs.iter().map(|(_, _, _, id)| *id).collect();
    let mut named: BTreeMap<u64, (String, String)> = BTreeMap::new();
    if !wanted.is_empty() {
        let options = Options {
            selected_ids: wanted,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            let class = record.class_name.clone().unwrap_or_default();
            let members = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
                .map(|root| scalars(&root.fields))
                .unwrap_or_default();
            named.insert(record.identity.element_id, (class, members));
            Ok(())
        })?;
    }
    for (record, class, member, id) in &refs {
        let (named_class, members) = named
            .get(id)
            .cloned()
            .unwrap_or_else(|| ("-".to_string(), String::new()));
        println!("ref {record} {class} {member} {id} {named_class} {members}");
    }
    for (class, records) in &counts {
        println!("summary {class} {records}");
    }
    Ok(())
}
