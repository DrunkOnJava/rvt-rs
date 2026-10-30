//! W7: the inputs of the office properties rvt-rs does not write yet.
//!
//! FACT (to be shown by the output joined with Revit's export): Revit writes
//! `Pset_ColumnCommon.Slope` on 73 of the BIM4LCA office's columns,
//! `Pset_BeamCommon.Roll` on its 8 beams and
//! `Pset_BuildingStoreyCommon.Reference` and `.EntranceLevel` on its storeys.
//! The probe prints, one JSON line each, what the file holds on those
//! elements: the parameter definitions it declares, every Level's native
//! object classes and scalar fields and saved parameters, and the structural
//! columns', structural framing's and pipes' native object class counts,
//! scalar fields of each object, decoded fields and saved parameters.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_w7_office_gaps -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_element_parameters;
use rvt::research::partition_level_records::recover_partition_levels;
use rvt::research::partition_room_parameters::find_parameter_definitions;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

const MAX_DEPTH: usize = 5;
const MAX_OBJECTS: usize = 120;

/// The scalars and short scalar arrays of a JSON value, by dotted path.
fn flatten(prefix: &str, value: &Value, depth: usize, out: &mut Map<String, Value>) {
    match value {
        Value::Object(map) => {
            if depth == 0 {
                return;
            }
            for (key, inner) in map {
                let name = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(&name, inner, depth - 1, out);
            }
        }
        Value::Array(items) => {
            let flat = items
                .iter()
                .all(|item| !item.is_object() && !item.is_array());
            if items.len() <= 8 && flat {
                out.insert(prefix.to_string(), value.clone());
            }
        }
        _ => {
            out.insert(prefix.to_string(), value.clone());
        }
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;

    let definitions = find_parameter_definitions(&mut rf, version);
    println!(
        "{}",
        json!({"kind": "definitions", "version": version, "definitions": definitions})
    );

    let levels = recover_partition_levels(&mut rf, version).unwrap_or_default();
    let elements =
        recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?.into_elements();
    let wanted_classes = ["StructuralColumn", "StructuralFraming", "Pipe"];

    let mut ids: BTreeSet<u64> = BTreeSet::new();
    let mut class_of: BTreeMap<u64, String> = BTreeMap::new();
    for level in &levels {
        ids.insert(u64::from(level.element_id));
        class_of.insert(u64::from(level.element_id), "Level".to_string());
    }
    for element in &elements {
        if !wanted_classes.contains(&element.class.as_str()) {
            continue;
        }
        if let Some(id) = element.id {
            ids.insert(u64::from(id));
            class_of.insert(u64::from(id), element.class.clone());
        }
    }

    for level in &levels {
        println!(
            "{}",
            json!({
                "kind": "level",
                "id": level.element_id,
                "name": level.name,
                "elevation_feet": level.elevation_feet,
                "building_story": level.building_story,
            })
        );
    }
    for element in &elements {
        if !wanted_classes.contains(&element.class.as_str()) {
            continue;
        }
        let fields: Vec<String> = element
            .fields
            .iter()
            .map(|(name, value)| format!("{name}={value:?}"))
            .collect();
        println!(
            "{}",
            json!({"kind": "decoded", "id": element.id, "class": element.class, "fields": fields})
        );
    }

    let parameters = native_element_parameters::read(&mut rf, &ids)?;
    for (id, list) in &parameters.elements {
        let entries: Vec<Value> = list
            .iter()
            .filter_map(|parameter| serde_json::to_value(parameter).ok())
            .collect();
        println!(
            "{}",
            json!({"kind": "parameters", "id": id, "class": class_of.get(id), "parameters": entries})
        );
    }

    let options = Options {
        selected_ids: ids,
        ..Options::default()
    };
    native_document::extract_graphs(&mut rf, &options, |record| {
        let id = record.identity.element_id;
        let Some(graph) = record.graph.as_ref() else {
            println!(
                "{}",
                json!({"kind": "native", "id": id, "class": class_of.get(&id), "graph": false})
            );
            return Ok(());
        };
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for object in &graph.objects {
            *counts.entry(object.class_name.as_str()).or_default() += 1;
        }
        let mut objects = Vec::new();
        for object in graph.objects.iter().take(MAX_OBJECTS) {
            let mut flat = Map::new();
            flatten("", &object.fields, MAX_DEPTH, &mut flat);
            objects.push(json!({"c": object.class_name, "f": flat}));
        }
        println!(
            "{}",
            json!({
                "kind": "native",
                "id": id,
                "class": class_of.get(&id),
                "root": record.class_name,
                "counts": counts,
                "objects": objects,
            })
        );
        Ok(())
    })?;
    Ok(())
}
