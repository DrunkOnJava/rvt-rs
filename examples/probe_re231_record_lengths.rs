//! RE-231 (#35, W7): which number of an element's native record, or of its
//! type's, is a length property Revit's export writes and rvt-rs does not.
//!
//! FACT under test: `probe_re231_stored_lengths` found the BIM4LCA office's
//! railing `Height` stored under its type as a length entry for 28 of its 41 railings and none for
//! the other 13, and no stored length for its beams' `Span` or its pipes'
//! `InvertElevation`. Hypothesis: each is a member of the element's own
//! native record or of its type's (a railing type's height, a beam's cut
//! length, a pipe's invert).
//!
//! Output, one line per railing, beam, pipe, stair and flight the export
//! writes, in the
//! format `tools/re/stored_lengths_vs_psets.py` reads:
//!   len <tag> <IFC class> <type|-> <owner>:<object>.<member>=<number> ...
//! `owner` `i` for the element's own record and `t` for its type's; the
//! numbers are every finite number at the top level of the first 200 objects
//! of the record, an object repeated in the record numbered `#2`, `#3`, ...
//!
//! Usage:
//!   cargo run --profile ci --example probe_re231_record_lengths -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::Value;

const CLASSES: [&str; 5] = [
    "IFCRAILING",
    "IFCBEAM",
    "IFCPIPESEGMENT",
    "IFCSTAIR",
    "IFCSTAIRFLIGHT",
];
const OBJECTS: usize = 200;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u32, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if !CLASSES.contains(&upper.as_str()) {
                continue;
            }
            if let Ok(id) = tag.parse() {
                classes.entry(id).or_insert(upper);
            }
        }
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut types: BTreeMap<u32, u32> = BTreeMap::new();
    for element in mvp.columns.iter().chain(&mvp.walls).chain(&mvp.products) {
        let Some(id) = element.id.filter(|id| classes.contains_key(id)) else {
            continue;
        };
        if let Some(type_id) = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        }) {
            types.entry(id).or_insert(type_id);
        }
    }
    let wanted: BTreeSet<u64> = classes
        .keys()
        .chain(types.values())
        .map(|id| u64::from(*id))
        .collect();
    let mut numbers: BTreeMap<u64, Vec<(String, f64)>> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: wanted,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
            let mut found = Vec::new();
            for object in graph.objects.iter().take(OBJECTS) {
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
                    if let (Value::Number(_), Some(number)) =
                        (value, value.as_f64().filter(|v| v.is_finite()))
                    {
                        found.push((format!("{label}.{name}"), number));
                    }
                }
            }
            numbers.insert(record.identity.element_id, found);
            Ok(())
        },
    )?;
    let listed = |owner: u32, prefix: &str| -> Vec<String> {
        numbers
            .get(&u64::from(owner))
            .into_iter()
            .flatten()
            .map(|(name, value)| format!("{prefix}:{name}={value}"))
            .collect()
    };
    for (id, class) in &classes {
        let mut entries = listed(*id, "i");
        let type_id = types.get(id);
        if let Some(type_id) = type_id {
            entries.extend(listed(*type_id, "t"));
        }
        let type_id = type_id.map_or_else(|| "-".to_string(), u32::to_string);
        println!("len {id} {class} {type_id} {}", entries.join(" "));
    }
    eprintln!(
        "{} elements, {} with a type, {} records read",
        classes.len(),
        types.len(),
        numbers.len()
    );
    Ok(())
}
