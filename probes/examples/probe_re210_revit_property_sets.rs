//! RE-210: Revit's own property sets, one per parameter group, from an
//! element's named saved parameters (RE-207).
//!
//! FACT: on the BIM4LCA as-built office (Revit 2024), the Yes/No parameter
//! `FireExit` of the group Identity Data, which `rvt-ifc
//! --revit-property-sets` writes in the set `Identity Data`, equals Revit's
//! `Pset_DoorCommon.FireExit` and `Pset_WindowCommon.FireExit` on all 498
//! doors and windows Revit writes it for, as a Boolean on both sides.
//! The sets' names, which parameters each holds and the type objects' sets
//! follow revit-ifc's `Release_25.x.x` branch (cited in
//! `src/ifc/revit_property_sets.rs`), and are measured against Revit 2025's
//! exports of Autodesk's Snowdon Towers HVAC, Site and Architectural with
//! "Export Revit property sets" (local only, not committed; RE-210 §5).
//! Score such an export with `tools/re/revit_property_sets_vs_ifc.py`.
//!
//! The probe exports the model as `rvt-ifc --revit-property-sets` does and
//! prints one JSON line: what the writer reports, the properties written
//! per set name on elements and on type objects, and the named parameters
//! left out, by reason.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re210_revit_property_sets -- MODEL.rvt

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::revit_property_sets::{attach, group_label, property};
use rvt::research::native_element_parameters::read;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re210_revit_property_sets MODEL.rvt"))?;
    let mut file = RevitFile::open(&path)?;
    let mut model = RvtDocExporter.export_with_diagnostics(&mut file)?.model;
    let before: BTreeSet<(usize, String, String)> = element_properties(&model.entities);
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let report = attach(&mut file, &mut model)?;

    let mut on_elements: BTreeMap<String, usize> = BTreeMap::new();
    for (_, set, _) in element_properties(&model.entities).difference(&before) {
        *on_elements.entry(set.clone()).or_default() += 1;
    }
    let mut on_types: BTreeMap<String, usize> = BTreeMap::new();
    for set in model.type_property_sets.values().flatten() {
        *on_types.entry(set.name.clone()).or_default() += set.properties.len();
    }

    // Why each named parameter is left out, over every element's and type's.
    let saved = read(&mut file, &ids)?;
    let mut left_out: BTreeMap<String, usize> = BTreeMap::new();
    for parameter in saved.elements.values().flatten() {
        if parameter.name.is_none() {
            continue;
        }
        let group = parameter.group_type_id.as_deref();
        let reason = if group_label(group).is_none() {
            format!("group {}", group.unwrap_or(""))
        } else if property(parameter).is_some() {
            continue;
        } else if parameter.storage_type == "String" && parameter.value == "" {
            "empty text, which Revit writes none for".to_string()
        } else if parameter.storage_type == "ElementId" && parameter.value == -1 {
            "no element (-1), which Revit writes none for".to_string()
        } else if parameter.storage_type == "ElementId" && parameter.target_name.is_none() {
            "element not named by rvt-rs".to_string()
        } else {
            let spec = parameter.spec_type_id.as_deref().unwrap_or("");
            let spec = spec.rsplit_once('-').map_or(spec, |(s, _)| s);
            format!("{} {spec}", parameter.storage_type)
        };
        *left_out.entry(reason).or_default() += 1;
    }

    println!(
        "{}",
        json!({
            "elements": report.elements,
            "types": report.types,
            "untyped_elements": report.untyped_elements,
            "properties": report.properties,
            "left_out": report.left_out,
            "on_elements": on_elements,
            "on_types": on_types,
            "left_out_by_reason": left_out,
        })
    );
    Ok(())
}

/// (element, set, property) of every element's set.
fn element_properties(entities: &[IfcEntity]) -> BTreeSet<(usize, String, String)> {
    entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::ElementPropertySet { element, set } => Some((element, set)),
            _ => None,
        })
        .flat_map(|(element, set)| {
            set.properties
                .iter()
                .map(move |p| (*element, set.name.clone(), p.name.clone()))
        })
        .collect()
}
