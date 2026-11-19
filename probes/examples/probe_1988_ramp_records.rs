//! #1988 (RE-390 follow-up): what a ramp's own record holds that tells whether
//! Revit's export makes the ramp a container of ramp flights, and so takes the
//! railings that name it as parts.
//!
//! FACT under test: on the BIM4LCA concrete housing Revit's export writes the
//! ramp 1347699 as an `IfcRamp` with an `IfcRampFlight` and its two railings
//! (`IfcRelAggregates`), where rvt-rs writes the `IfcRamp` alone and the
//! railings as plain elements of their storey; on the BIM4LCA office Revit's
//! export writes four ramps that railings name (1439414, 1440087, 1465824,
//! 1465840) as `IfcRamp`s with no part, and a fifth (1440596) with a flight.
//! The records of the housing ramp and the four office ramps hold the same
//! classes of object, so a difference between the ramps that Revit makes into
//! a container and the ones it does not is in a value.
//!
//! Output, for every record of class `Ramp` or `Level` (a level's elevation and
//! name tell which levels a railing's `m_assocLevelId` and its host's lie between):
//!   <ramp|level> <id> record <class> objects <class=count ...>
//!   <ramp|level> <id> field <object class> <name> <value as JSON, cut at 400 characters>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1988_ramp_records -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut selected_classes = BTreeSet::new();
    selected_classes.insert("Ramp".to_string());
    selected_classes.insert("Level".to_string());
    extract_graphs(
        &mut rf,
        &Options {
            selected_classes,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                *classes.entry(object.class_name.as_str()).or_default() += 1;
            }
            let census: Vec<String> = classes
                .iter()
                .map(|(name, count)| format!("{name}={count}"))
                .collect();
            let class = record.class_name.clone().unwrap_or_default();
            let kind = class.to_lowercase();
            println!("{kind} {id} record {class} objects {}", census.join(" "));
            for object in &graph.objects {
                if let Some(fields) = object.fields.as_object() {
                    for (name, value) in fields {
                        let cut: String = value.to_string().chars().take(400).collect();
                        println!("{kind} {id} field {} {name} {cut}", object.class_name);
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}
