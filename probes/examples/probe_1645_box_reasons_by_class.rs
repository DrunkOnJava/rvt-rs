//! #1645: why each element still drawn as its record's box was not drawn from the
//! mesh Revit saved for it, by IFC class.
//!
//! FACT under test: `exported.saved_mesh_refusals` counts the boxes by reason
//! over all classes, and the largest reasons are not the ones that cost the most
//! classes' bodies. This exports the model as `rvt-ifc` does and counts, for each
//! IFC class and each reason an element's `SavedMeshRefusal` property gives, the
//! elements that carry it, with the names of the first three, and then each
//! class's elements in all, so a reason's share of its class is the ratio.
//!
//! Output, tab separated:
//!   reason <class> <elements> <reason> <first three names>
//!   class <class> <elements>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1645_box_reasons_by_class -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, run_export};

const REFUSAL: &str = "SavedMeshRefusal";

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    let mut groups: BTreeMap<(String, String), (usize, Vec<String>)> = BTreeMap::new();
    let mut classes: BTreeMap<String, usize> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        *classes.entry(ifc_type.clone()).or_default() += 1;
        let Some(set) = property_set else {
            continue;
        };
        let mut reason = None;
        for property in &set.properties {
            if property.name == REFUSAL {
                if let PropertyValue::Text(text) = &property.value {
                    reason = Some(text.clone());
                }
            }
        }
        let Some(reason) = reason else {
            continue;
        };
        let entry = groups.entry((ifc_type.clone(), reason)).or_default();
        entry.0 += 1;
        if entry.1.len() < 3 {
            entry.1.push(name.clone());
        }
    }
    for ((class, reason), (count, names)) in &groups {
        println!("reason\t{class}\t{count}\t{reason}\t{}", names.join(" | "));
    }
    for (class, count) in &classes {
        println!("class\t{class}\t{count}");
    }
    Ok(())
}
