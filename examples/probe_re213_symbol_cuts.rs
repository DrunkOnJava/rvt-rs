//! RE-213 (#810, #844): what a family symbol's own record holds of the cut it
//! makes in its host.
//!
//! FACT under test: a door's or window's `OverallWidth` and `OverallHeight`
//! in Revit's export are the extents of the opening it cuts (#810, run
//! 39187209511), which rvt-rs does not read. A `FamilySymbol` record names
//! `m_pCutLoops`, `m_cutDirs`, `m_cutIndices`, `m_cutPlaneHeights` and
//! `m_closurePlanes` (RE-212's field list). This probe prints them, and every
//! object of the record's graph they lead to, for each symbol whose cut
//! loops are not empty; `tools/re/symbol_cuts_vs_ifc.py` measures them
//! against the sizes Revit's export gives the symbol's doors and windows.
//!
//! Output, one JSON value per line:
//!   cut <symbol> <family> <the five fields as one JSON object>
//!   object <symbol> <class name> <class tag> <start> <token> <fields>
//!     the first 400 objects of that symbol's graph after its root
//!
//! Usage:
//!   cargo run --profile ci --example probe_re213_symbol_cuts -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const FIELDS: [&str; 5] = [
    "m_pCutLoops",
    "m_cutDirs",
    "m_cutIndices",
    "m_cutPlaneHeights",
    "m_closurePlanes",
];

fn element_id(value: &Value) -> Option<i64> {
    value.get("m_id")?.get("m_id64")?.as_i64()
}

fn empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::Array(items) => items.is_empty(),
        Value::Object(map) => map.is_empty() || map.values().all(empty),
        _ => false,
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: ["FamilySymbol".to_string()].into(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let loops = root.fields.get("m_pCutLoops").unwrap_or(&Value::Null);
        if empty(loops) {
            return Ok(());
        }
        let id = record.identity.element_id;
        let family = root.fields.get("m_familyId").and_then(element_id);
        let cut: serde_json::Map<String, Value> = FIELDS
            .iter()
            .filter_map(|name| Some((name.to_string(), root.fields.get(*name)?.clone())))
            .collect();
        println!(
            "cut {id} {} {}",
            family.map_or("-".to_string(), |f| f.to_string()),
            serde_json::to_string(&cut)?
        );
        for object in graph.objects.iter().skip(1).take(400) {
            println!(
                "object {id} {} {} {} {} {}",
                object.class_name,
                object.class_tag,
                object.start,
                object.token,
                serde_json::to_string(&object.fields)?
            );
        }
        Ok(())
    })?;
    Ok(())
}
