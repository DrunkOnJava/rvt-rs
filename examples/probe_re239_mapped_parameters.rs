//! RE-239 (#35, W7): which of an element's own parameters hold the values
//! Revit's export of the BIM4LCA office writes in sets IFC4 does not give
//! the element.
//!
//! FACT under test: Revit's export writes `Pset_DoorWindowGlazingType`
//! (`IsLaminated`, `IsTempered`) on 604 walls and 151 curtain walls, and
//! `Pset_SpaceOccupancyRequirements.OccupancyType` on 234 spaces, neither of
//! which revit-ifc's own definitions (CLEANROOM.md source 5) puts there. An
//! export-time user-defined property set file maps a parameter to such a
//! set. Hypothesis: each value is one of the element's saved parameters
//! (RE-207), named by the file's own definitions, and the file only maps it.
//!
//! Output, one line per wall, curtain wall and room the export writes:
//!   par <tag> <IFC class> <parameter id>:<name|->=<value> ...
//! every Yes/No and text value of its saved parameters and its type's,
//! spaces as `_`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re239_mapped_parameters -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_element_parameters;
use serde_json::Value;

const CLASSES: [&str; 3] = ["IFCWALL", "IFCCURTAINWALL", "IFCSPACE"];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) {
                if let Ok(id) = tag.parse() {
                    classes.entry(id).or_insert(upper);
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let read = native_element_parameters::read(&mut rf, &ids)?;
    for (id, class) in &classes {
        let values: Vec<String> = read
            .elements
            .get(id)
            .into_iter()
            .flatten()
            .filter_map(|parameter| {
                let value = match &parameter.value {
                    Value::Bool(b) => b.to_string(),
                    Value::String(text) => text.replace(' ', "_"),
                    _ => return None,
                };
                Some(format!(
                    "{}:{}={value}",
                    parameter.parameter_id,
                    parameter.name.as_deref().unwrap_or("-").replace(' ', "_")
                ))
            })
            .collect();
        println!("par {id} {class} {}", values.join(" "));
    }
    eprintln!(
        "{} elements, {} with parameters",
        classes.len(),
        read.elements.len()
    );
    Ok(())
}
