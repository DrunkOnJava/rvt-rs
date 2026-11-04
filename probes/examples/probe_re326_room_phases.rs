//! RE-326 (probe): where a room's phase is stored, and whether it is the
//! export phase (#1194).
//!
//! Revit's exporter leaves out a room whose own phase parameter
//! (`ROOM_PHASE`) is set and is not the export phase, the last of the
//! project's phases (revit-ifc, `ElementFilteringUtil.IsRoomInInvalidPhase`;
//! RE-184 §5). rvt-rs applies the phase rule to no room, because no model
//! here had shown where a room's phase is stored.
//!
//! FACT under test: a room's phase is an ElementId value in its own native
//! record (`RoomElem`), either one of its saved parameters (a built-in
//! parameter with ElementId storage whose value is a `ProjectPhase`) or a
//! field of its data, and Revit's export holds exactly the rooms whose phase
//! is the last one.
//!
//! The probe reads, for every `RoomElem` record of the file:
//!
//! - the Element base's `m_createdPhaseId` and `m_demolishedPhaseId`;
//! - every ElementId in its object graph (any object, any field) whose value
//!   is one of the project's phases, by object class and field path;
//! - every ElementId-storage saved parameter, with its built-in id and value;
//! - the string value of its `IfcGUID` parameter (-1019000), where it holds one;
//! - what rvt-rs writes it as: `room` (a room it writes), `area` (an Area it
//!   writes) or `other` (a record it does not write: an unplaced or not
//!   enclosed room, an MEP space), with the number and name it writes;
//! - Revit's GlobalId for the element (RE-48).
//!
//! Output, one JSON object per line:
//!   {"kind":"file", ...}   the file, its release, the phase order and names
//!   {"kind":"room"|"area"|"other", ...}   one per RoomElem record
//!
//! `tools/re/room_phases_vs_ifc.py PROBE.txt REVIT.ifc` pairs each line with
//! Revit's `IfcSpace` by GlobalId and tabulates, for each candidate phase
//! (the created phase, and each phase-valued parameter), the rooms in the
//! export phase and not, and which of them Revit's export holds.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re326_room_phases -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::research::native_document;
use rvt::research::partition_schema_mvp::{ROOM_NUMBER_FIELD, recover_partition_schema_mvp};
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The IfcGUID built-in parameter.
const IFC_GUID_PARAMETER: i64 = -1_019_000;

/// An ElementId field's value, as either shape the schema saves it:
/// `{"m_id":{"m_id64":n}}` from Revit 2024, `{"m_id":{"m_id":n}}` before.
fn ident(value: &Value) -> Option<i64> {
    let identifier = value.get("m_id")?;
    identifier
        .get("m_id64")
        .or_else(|| identifier.get("m_id"))
        .and_then(Value::as_i64)
}

/// Every ElementId-shaped value under `value`, with its path.
fn identifiers(value: &Value, path: &str, out: &mut Vec<(String, i64)>) {
    match value {
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(id) = ident(value) {
                    out.push((path.to_string(), id));
                    return;
                }
            }
            for (key, child) in map {
                identifiers(child, &format!("{path}/{key}"), out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                identifiers(child, &format!("{path}/{index}"), out);
            }
        }
        _ => {}
    }
}

fn text(element: &DecodedElement, wanted: &str) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text) if name == wanted => Some(text.clone()),
        _ => None,
    })
}

/// Number and name of each element rvt-rs writes, by ElementId.
fn written_by_id(elements: &[DecodedElement]) -> BTreeMap<u32, (String, String)> {
    elements
        .iter()
        .filter_map(|element| {
            Some((
                element.id?,
                (
                    text(element, ROOM_NUMBER_FIELD).unwrap_or_default(),
                    text(element, "m_name").unwrap_or_default(),
                ),
            ))
        })
        .collect()
}

fn probe(path: &str) -> anyhow::Result<()> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;

    // Pass 1: the phases, and every RoomElem record's base.
    let phase_options = native_document::Options {
        graph_classes: Some(BTreeSet::from([
            "AllProjectPhases".to_string(),
            "ProjectPhase".to_string(),
        ])),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    let mut order: Option<Vec<i64>> = None;
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    let mut bases: BTreeMap<u64, (i64, i64)> = BTreeMap::new();
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    native_document::extract_graphs(&mut rf, &phase_options, |record| {
        let class = record.class_name.clone().unwrap_or_default();
        if let Some(base) = &record.base {
            let lower = class.to_ascii_lowercase();
            if lower.contains("room") || lower.contains("area") || lower.contains("space") {
                *census.entry(class.clone()).or_default() += 1;
            }
            if class == "RoomElem" {
                bases.insert(
                    record.identity.element_id,
                    (base.created_phase_id, base.demolished_phase_id),
                );
            }
            return Ok(());
        }
        let Some(object) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
            return Ok(());
        };
        match class.as_str() {
            "AllProjectPhases" if ident(&object.fields["m_famId"]) == Some(-1) => {
                let ids: Vec<i64> = object.fields["m_phaseIds"]
                    .as_array()
                    .map(|a| a.iter().filter_map(ident).collect())
                    .unwrap_or_default();
                order = Some(ids);
            }
            "ProjectPhase" => {
                if let Some(name) = object.fields["m_name"].as_str() {
                    names.insert(record.identity.element_id as i64, name.to_string());
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let order = order.unwrap_or_default();
    let phase_ids: BTreeSet<i64> = order.iter().chain(names.keys()).copied().collect();
    let export_phase = order.last().copied();

    // What rvt-rs writes.
    let (rooms, areas, written_error) =
        match recover_partition_schema_mvp(&mut rf, revit, WalkerLimits::default()) {
            Ok(mvp) => (written_by_id(&mvp.rooms), written_by_id(&mvp.areas), None),
            Err(error) => (BTreeMap::new(), BTreeMap::new(), Some(error.to_string())),
        };
    let global_ids = rvt::research::revit_global_ids::revit_global_ids(&mut rf).unwrap_or_default();

    let phase_list: Vec<Value> = order.iter().map(|id| json!([id, names.get(id)])).collect();
    println!(
        "{}",
        json!({
            "kind": "file",
            "file": file,
            "revit": revit,
            "phase_order": phase_list,
            "export_phase": export_phase,
            "spatial_census": census,
            "room_elem_records": bases.len(),
            "written_rooms": rooms.len(),
            "written_areas": areas.len(),
            "written_error": written_error,
        })
    );
    if bases.is_empty() {
        return Ok(());
    }

    // Pass 2: the graphs and saved parameters of those records.
    let options = native_document::Options {
        selected_ids: bases.keys().copied().collect(),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    native_document::extract_graphs(&mut rf, &options, |record| {
        let id = record.identity.element_id;
        seen.insert(id);
        let element = u32::try_from(id).ok();
        let (kind, written) = match element {
            Some(e) if rooms.contains_key(&e) => ("room", rooms.get(&e)),
            Some(e) if areas.contains_key(&e) => ("area", areas.get(&e)),
            _ => ("other", None),
        };
        let (created, demolished) = bases.get(&id).copied().unwrap_or((-2, -2));
        let mut graph_refs = Vec::new();
        let mut root_refs = Vec::new();
        let mut parameters = Vec::new();
        let mut ifc_guid = None;
        if let Some(graph) = &record.graph {
            for (index, object) in graph.objects.iter().enumerate() {
                let mut refs = Vec::new();
                identifiers(
                    &object.fields,
                    &format!("#{index}:{}", object.class_name),
                    &mut refs,
                );
                for (path, value) in refs {
                    if index == 0 {
                        root_refs.push(json!([path.clone(), value]));
                    }
                    if phase_ids.contains(&value) {
                        graph_refs.push(json!([path, value]));
                    }
                }
            }
        }
        if let Some(metadata) = &record.saved_metadata {
            for set in &metadata.parameter_sets {
                for parameter in &set.parameters {
                    if parameter.storage_type == "ElementId" {
                        parameters.push(json!([
                            parameter.serialized_parameter_id,
                            parameter.raw_value.as_i64(),
                            set.root_pointer_field
                        ]));
                    }
                    if parameter.serialized_parameter_id == IFC_GUID_PARAMETER {
                        ifc_guid = parameter.raw_value.as_str().map(str::to_string);
                    }
                }
            }
            for parameter in &metadata.raw_field_parameters {
                if parameter.storage_type == "ElementId" {
                    parameters.push(json!([
                        parameter.parameter_id,
                        parameter.raw_value.as_i64(),
                        parameter.source_field
                    ]));
                }
            }
        }
        println!(
            "{}",
            json!({
                "kind": kind,
                "file": file,
                "id": id,
                "number": written.map(|w| w.0.clone()),
                "name": written.map(|w| w.1.clone()),
                "global_id": element.and_then(|e| global_ids.get(&e)),
                "ifc_guid_parameter": ifc_guid,
                "graph_read": record.graph.is_some(),
                "created": created,
                "demolished": demolished,
                "phase_refs": graph_refs,
                "root_refs": root_refs,
                "element_id_parameters": parameters,
            })
        );
        Ok(())
    })?;
    for (id, (created, demolished)) in &bases {
        if !seen.contains(id) {
            println!(
                "{}",
                json!({"kind": "unread", "file": file, "id": id, "created": created, "demolished": demolished})
            );
        }
    }
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re326_room_phases MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in &paths {
        if let Err(err) = probe(path) {
            println!(
                "{{\"kind\":\"error\",\"file\":{path:?},\"error\":{:?}}}",
                err.to_string()
            );
        }
    }
}
