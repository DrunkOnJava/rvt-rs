//! RE-352 (#1185, W7): where Revit's export of the BIM4LCA office takes a
//! door type's `FireRating` and `ThermalTransmittance` from.
//!
//! FACT under test: Revit's export writes `Pset_DoorCommon.FireRating` on six
//! door type objects (`EI60`, `EI30` and `-`; no door type holds the text as
//! an entry of RE-77's built-in form, RE-289) and `ThermalTransmittance`
//! (3.7021) on one, while the doors of those types carry their own values
//! (`EI60` on instances of a type whose own is `EI30`). The saved parameters
//! of each door and of its type (RE-207) are listed to find which parameter
//! holds each type value, and how it is named.
//!
//! Output, one line per parameter, text, number, integer and Yes/No values
//! only, spaces in names and values as `_`:
//!   door <tag> type <type id>
//!   ipar <tag> <parameter id>:<name|->:<kind>:<storage>=<value>   the door's own
//!   tpar <type id> <parameter id>:<name|->:<kind>:<storage>=<value>   its type's, once per type
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re352_door_type_parameters -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_element_parameters::{self, ElementParameter, ParameterLevel};
use serde_json::Value;

fn render(parameter: &ElementParameter) -> Option<String> {
    let value = match &parameter.value {
        Value::String(text) => text.replace(' ', "_"),
        Value::Bool(flag) => flag.to_string(),
        Value::Number(number) => number.to_string(),
        _ => return None,
    };
    Some(format!(
        "{}:{}:{:?}:{}={value}",
        parameter.parameter_id,
        parameter.name.as_deref().unwrap_or("-").replace(' ', "_"),
        parameter.kind,
        parameter.storage_type,
    ))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut doors: BTreeSet<u64> = BTreeSet::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if ifc_type.eq_ignore_ascii_case("IFCDOOR") {
                if let Ok(id) = tag.parse::<u64>() {
                    doors.insert(id);
                }
            }
        }
    }
    let read = native_element_parameters::read(&mut rf, &doors)?;
    let mut types: BTreeMap<i64, BTreeSet<String>> = BTreeMap::new();
    for (id, parameters) in &read.elements {
        let own_type = parameters
            .iter()
            .find(|parameter| parameter.level == ParameterLevel::Type)
            .map(|parameter| parameter.source_element_id);
        println!(
            "door {id} type {}",
            own_type.map_or_else(|| "-".to_string(), |t| t.to_string())
        );
        for parameter in parameters {
            let Some(line) = render(parameter) else {
                continue;
            };
            match parameter.level {
                ParameterLevel::Instance => println!("ipar {id} {line}"),
                ParameterLevel::Type => {
                    types
                        .entry(parameter.source_element_id)
                        .or_default()
                        .insert(line);
                }
            }
        }
    }
    for (type_id, lines) in &types {
        for line in lines {
            println!("tpar {type_id} {line}");
        }
    }
    Ok(())
}
