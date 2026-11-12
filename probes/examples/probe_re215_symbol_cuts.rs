//! RE-215 (#844, #810): the outline a door's or window's symbol cuts in its
//! host gives the `OverallWidth` and `OverallHeight` Revit's export writes.
//!
//! FACT: a `FamilySymbol` record's `m_pCutLoops` points at one `CurveLoop`
//! of `GLine`s in the symbol's XZ plane. A `FamilyInstance` names its type
//! in `m_masterSymbolId` and the symbol that draws it in its `InstanceInfo`
//! object's `m_symbolId` (revit-ifc's `ExporterIFCUtils.GetOriginalSymbol`,
//! CLEANROOM.md source 5). That symbol's loop's extent along X and Z is
//! Revit's `OverallWidth` and `OverallHeight` on every door and window with
//! one: Core Interior's 132 doors and 6 windows (whose type, 17335, is
//! 1010 mm square and whose symbol, 19275, cuts 1500 x 2700 mm), RE1
//! Architecture's 4 doors, and the door and 3 windows of each of the R2025
//! models 12 to 17. RE1's curtain-wall door 445975 has none.
//!
//! Verify with Measure (`-f r2025=true -f probe=probe_re215_symbol_cuts
//! -f probe_scorer=symbol_cuts_vs_ifc`): the scorer counts, per class, the
//! elements whose `InstanceInfo.m_symbolId` loop equals Revit's size.
//!
//! Output, one JSON value per line:
//!   cut <symbol> <family> <owner> <the five fields as one JSON object>
//!   symbolrefs <symbol> <its root's fields that hold an ElementId>
//!   object <symbol> <class name> <class tag> <start> <token> <fields>
//!     the first 400 objects of that symbol's graph after its root
//!   instance <id> <owner> <its root's fields that hold an ElementId>
//!   deep <id> <[object class, field, ElementId] for every object>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re215_symbol_cuts -- MODEL.rvt

use std::collections::BTreeSet;

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

/// The root's fields that hold an ElementId anywhere in their value.
fn id_fields(fields: &Value) -> serde_json::Map<String, Value> {
    fields
        .as_object()
        .into_iter()
        .flatten()
        .filter(|(_, value)| value.to_string().contains("m_id64"))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect()
}

/// Every `m_id64` in `value` other than -1.
fn ids_in(value: &Value, out: &mut BTreeSet<i64>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = map
                .get("m_id64")
                .and_then(Value::as_i64)
                .filter(|id| *id != -1)
            {
                out.insert(id);
            }
            map.values().for_each(|v| ids_in(v, out));
        }
        Value::Array(items) => items.iter().for_each(|v| ids_in(v, out)),
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut cut_symbols = BTreeSet::new();
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
        let owner = record.identity.owning_element_id;
        if let Ok(id) = i64::try_from(id) {
            cut_symbols.insert(id);
        }
        let family = root.fields.get("m_familyId").and_then(element_id);
        let cut: serde_json::Map<String, Value> = FIELDS
            .iter()
            .filter_map(|name| Some((name.to_string(), root.fields.get(*name)?.clone())))
            .collect();
        println!(
            "cut {id} {} {owner} {}",
            family.map_or("-".to_string(), |f| f.to_string()),
            serde_json::to_string(&cut)?
        );
        println!(
            "symbolrefs {id} {}",
            serde_json::to_string(&id_fields(&root.fields))?
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
    let options = Options {
        selected_classes: ["FamilyInstance".to_string()].into(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let master = root.fields.get("m_masterSymbolId").and_then(element_id);
        if !master.is_some_and(|m| cut_symbols.contains(&m)) {
            return Ok(());
        }
        let id = record.identity.element_id;
        let owner = record.identity.owning_element_id;
        println!(
            "instance {id} {owner} {}",
            serde_json::to_string(&id_fields(&root.fields))?
        );
        let mut deep = Vec::new();
        for object in &graph.objects {
            for (name, value) in object.fields.as_object().into_iter().flatten() {
                let mut ids = BTreeSet::new();
                ids_in(value, &mut ids);
                for found in ids {
                    deep.push(serde_json::json!([object.class_name, name, found]));
                }
            }
        }
        println!("deep {id} {}", serde_json::to_string(&deep)?);
        Ok(())
    })?;
    Ok(())
}
