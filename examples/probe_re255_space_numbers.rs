//! RE-255 (#1016, W7): every number a room's native record and its saved
//! parameters hold, for finding whether Revit keeps its rooms' computed
//! Height, Area and Volume, which its export writes as an `IfcSpace`'s
//! `Qto_SpaceBaseQuantities`.
//!
//! FACT under test: revit-ifc's `NetVolumeCalculator` (CLEANROOM.md source 5)
//! takes a space's `NetVolume` from a parameter before its geometry.
//! Hypothesis: the room's record or saved parameters hold that volume, its
//! height or its areas, as numbers. `tools/re/space_numbers_vs_qtos.py` looks
//! for each of Revit's space quantities among them.
//!
//! Output, one line per room the export writes: `rec <tag> <record class>
//! <object class>.<field>=<number> ... P.<parameter id>=<number> ...`, as
//! RE-251's probe prints a wall's, and `roomname <tag> <name>`, the `Name`
//! rvt-rs gives its `IfcSpace`, which keys a space in the quantity census.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re255_space_numbers -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, ProfileDef, PropertyValue};
use rvt::ifc::export_content::{ROOM_NAME_PROPERTY, ROOM_NUMBER_PROPERTY};
use rvt::ifc::from_decoded::polygon_area_sqft;
use rvt::research::native_document::{Options, extract};
use rvt::research::native_element_parameters;
use serde_json::Value;

fn number(value: &Value) -> Option<String> {
    value
        .as_f64()
        .filter(|n| n.is_finite())
        .map(|n| n.to_string())
}

/// Every number in `value`, its path below `label` joined with `.`; arrays of
/// up to 4 numbers as `label[i]`.
fn numbers(label: &str, value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Number(_) => {
            if let Some(n) = number(value) {
                out.push(format!("{label}={n}"));
            }
        }
        Value::Object(map) => {
            for (name, inner) in map {
                numbers(&format!("{label}.{name}"), inner, out);
            }
        }
        Value::Array(items) if items.len() <= 4 => {
            for (i, inner) in items.iter().enumerate() {
                numbers(&format!("{label}[{i}]"), inner, out);
            }
        }
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    // rvt-rs's own body of each space: its profile's area and that times
    // the extrusion's depth, square and cubic feet.
    let bodies: BTreeMap<u64, (f64, f64)> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                extrusion: Some(body),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCSPACE") => {
                let area = match &body.profile_override {
                    None => body.width_feet * body.depth_feet,
                    Some(ProfileDef::Rectangle {
                        width_feet,
                        depth_feet,
                    }) => width_feet * depth_feet,
                    Some(ProfileDef::ArbitraryClosed { points }) => polygon_area_sqft(points),
                    Some(ProfileDef::ArbitraryWithVoids { points, voids }) => {
                        polygon_area_sqft(points)
                            - voids.iter().map(|v| polygon_area_sqft(v)).sum::<f64>()
                    }
                    Some(_) => return None,
                };
                Some((tag.parse().ok()?, (area, body.height_feet)))
            }
            _ => None,
        })
        .collect();
    let names: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                property_set,
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCSPACE") => {
                // The `Name` the writer gives the space: its room number,
                // else `<room name>:<ElementId>`, else its own name.
                let text = |key: &str| {
                    property_set
                        .as_ref()?
                        .properties
                        .iter()
                        .find_map(|p| match &p.value {
                            PropertyValue::Text(text)
                                if p.name == key && !text.trim().is_empty() =>
                            {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                };
                let written = text(ROOM_NUMBER_PROPERTY)
                    .or_else(|| text(ROOM_NAME_PROPERTY).map(|room| format!("{room}:{tag}")))
                    .unwrap_or_else(|| name.clone());
                Some((tag.parse().ok()?, written))
            }
            _ => None,
        })
        .collect();
    let ids: BTreeSet<u64> = names.keys().copied().collect();
    let mut lines: BTreeMap<u64, (String, Vec<String>)> = BTreeMap::new();
    extract(
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
            let mut parts = Vec::new();
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                let n = seen.entry(object.class_name.as_str()).or_default();
                *n += 1;
                let label = if *n == 1 {
                    object.class_name.clone()
                } else {
                    format!("{}#{n}", object.class_name)
                };
                numbers(&label, &object.fields, &mut parts);
            }
            if let Some(saved) = record.saved_metadata.as_ref() {
                for set in &saved.parameter_sets {
                    for parameter in &set.parameters {
                        numbers(
                            &format!("MP.{}", parameter.serialized_parameter_id),
                            &parameter.raw_value,
                            &mut parts,
                        );
                    }
                }
                for parameter in &saved.raw_field_parameters {
                    numbers(
                        &format!("MR.{}", parameter.parameter_id),
                        &parameter.raw_value,
                        &mut parts,
                    );
                }
                for parameter in &saved.resolved_family_parameters {
                    numbers(
                        &format!("MF.{}", parameter.parameter_id),
                        &parameter.raw_value,
                        &mut parts,
                    );
                }
            }
            lines.insert(record.identity.element_id, (class.to_string(), parts));
            Ok(())
        },
    )?;
    let saved = native_element_parameters::read(&mut rf, &ids)?;
    for id in &ids {
        let Some((class, parts)) = lines.get(id) else {
            println!("rec {id} -");
            continue;
        };
        let mut all = parts.clone();
        for parameter in saved.elements.get(id).into_iter().flatten() {
            if let Some(n) = number(&parameter.value) {
                all.push(format!("P.{}={n}", parameter.parameter_id));
            }
        }
        if let Some((area, height)) = bodies.get(id) {
            all.push(format!("B.area={area}"));
            all.push(format!("B.height={height}"));
            all.push(format!("B.volume={}", area * height));
            let room_height = parts
                .iter()
                .find_map(|part| part.strip_prefix("RoomElem.m_height=")?.parse::<f64>().ok());
            if let Some(room_height) = room_height {
                all.push(format!("B.area_x_room_height={}", area * room_height));
            }
        }
        println!("rec {id} {class} {}", all.join(" "));
    }
    for (id, name) in &names {
        println!("roomname {id} {name}");
    }
    eprintln!("{} rooms, {} records", ids.len(), lines.len());
    Ok(())
}
