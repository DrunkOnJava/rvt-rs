//! RE-251 (#1016, W7): every number a wall's or room's native record and its
//! saved parameters hold, for finding whether Revit keeps its walls' and
//! rooms' computed Length, Height, Area and Volume.
//!
//! FACT under test: revit-ifc's `LengthCalculator` (CLEANROOM.md source 5)
//! takes a wall's `Qto_WallBaseQuantities.Length` from its Length parameter,
//! which is not its location line's length on 186 of the BIM4LCA office's 607
//! walls. Hypothesis: the record or its saved parameters hold that Length, or
//! the wall's Area and Volume, as numbers. `tools/re/wall_numbers_vs_qtos.py`
//! looks for each of Revit's wall quantities among them.
//!
//! Output, one line per wall and room (space) the export writes:
//!   rec <tag> <record class> N=<name> <object class>.<field>=<number> ... P.<parameter id>=<number> ...
//! a room's name being the space's IFC name, spaces as `_`;
//! every number of every object of the wall's record, those of its saved
//! parameter sets (`MP.`), raw-field (`MR.`) and resolved family (`MF.`)
//! parameters, and the RE-207 reader's numeric parameters (`P.`).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re251_wall_numbers -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
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
    let names: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCWALL")
                || ifc_type.eq_ignore_ascii_case("IFCWALLSTANDARDCASE")
                || ifc_type.eq_ignore_ascii_case("IFCSPACE") =>
            {
                tag.parse().ok().map(|id| (id, name.replace(' ', "_")))
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
        let mut all = vec![format!("N={}", names.get(id).map_or("-", String::as_str))];
        all.extend(parts.iter().cloned());
        for parameter in saved.elements.get(id).into_iter().flatten() {
            if let Some(n) = number(&parameter.value) {
                all.push(format!("P.{}={n}", parameter.parameter_id));
            }
        }
        println!("rec {id} {class} {}", all.join(" "));
    }
    eprintln!("{} walls and rooms, {} records", ids.len(), lines.len());
    Ok(())
}
