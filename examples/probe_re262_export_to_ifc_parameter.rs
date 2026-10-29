//! #792, #879: does Revit's "Export to IFC" parameter explain the single
//! elements rvt-rs writes that Revit's export leaves out?
//!
//! RE-262, a measured negative. HYPOTHESIS: Revit's exporter first reads an element's
//! `IFC_EXPORT_ELEMENT` (BuiltInParameter -1019012, "Export to IFC": By Type,
//! Yes or No) and its type's `IFC_EXPORT_ELEMENT_TYPE` (-1019013), and leaves
//! out what says No, before any category mapping (revit-ifc
//! `ElementFilteringUtil`, CLEANROOM.md source 5). rvt-rs reads neither. The
//! elements it writes that Revit's export leaves out: RE1 Plumbing's water
//! closet tank 442378, the BIM4LCA office permit model's column 1927742, and
//! BIMcollab's three 2015 air terminals.
//!
//! The probe reads every declared element's saved parameters
//! (`native_element_parameters`, RE-207, Revit 2023 to 2027) and prints each
//! element carrying either parameter, with its value and level (its own or
//! its type's), then the count of elements by value. The hypothesis holds on
//! a file when the elements Revit's export leaves out carry a No that no
//! exported element carries.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re262_export_to_ifc_parameter -- FILE.rvt [ELEMENTID ...]
//!
//! Extra ElementIds print every built-in parameter those elements carry, to
//! see what else could tell them apart.
//!
//! The reader keeps only FamilyParams slots whose definition it recovers, so
//! the probe then lists every slot of the declared elements and of every
//! `FamilySymbol` that holds either parameter, resolved or not.

use rvt::RevitFile;
use rvt::research::native_document::{self, Options};
use rvt::research::native_element_parameters::{ParameterKind, read};
use std::collections::{BTreeMap, BTreeSet};

const EXPORT_TO_IFC: i64 = -1019012;
const EXPORT_TYPE_TO_IFC: i64 = -1019013;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().ok_or_else(|| {
        anyhow::anyhow!("usage: probe_re262_export_to_ifc_parameter FILE.rvt [ELEMENTID ...]")
    })?;
    let watch: BTreeSet<u64> = args.filter_map(|a| a.parse().ok()).collect();
    let mut rf = RevitFile::open(&path)?;
    let ids: BTreeSet<u64> = rvt::elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let parameters = read(&mut rf, &ids)?;
    println!(
        "{path}: {} declared elements, {} with saved parameters",
        ids.len(),
        parameters.elements.len()
    );
    let mut by_value: BTreeMap<(i64, String, String), usize> = BTreeMap::new();
    for (id, list) in &parameters.elements {
        for parameter in list {
            if parameter.parameter_id == EXPORT_TO_IFC
                || parameter.parameter_id == EXPORT_TYPE_TO_IFC
            {
                let level = format!("{:?}", parameter.level);
                println!(
                    "  {id} {} level {level} value {} from {} {}",
                    parameter.parameter_id,
                    parameter.value,
                    parameter.source_class,
                    parameter.source_element_id
                );
                *by_value
                    .entry((parameter.parameter_id, level, parameter.value.to_string()))
                    .or_default() += 1;
            }
            if watch.contains(id) && parameter.kind == ParameterKind::BuiltIn {
                println!(
                    "  watched {id}: {} = {} ({:?}, {})",
                    parameter.parameter_id,
                    parameter.value,
                    parameter.level,
                    parameter.source_class
                );
            }
        }
    }
    for id in &watch {
        if !parameters.elements.contains_key(id) {
            println!("  watched {id}: no saved parameters read");
        }
    }
    println!("elements by parameter, level and value:");
    for ((parameter, level, value), count) in &by_value {
        println!("  {parameter} {level} {value}: {count}");
    }
    let options = Options {
        selected_ids: ids.clone(),
        selected_classes: BTreeSet::from(["FamilySymbol".to_string()]),
        ..Default::default()
    };
    let mut slots: BTreeMap<(i64, bool, String), usize> = BTreeMap::new();
    native_document::extract(&mut rf, &options, |record| {
        let (Some(graph), Some(metadata)) = (&record.graph, &record.saved_metadata) else {
            return Ok(());
        };
        let class = &graph.objects[0].class_name;
        for slot in &metadata.family_parameter_slots {
            if slot.parameter_id != EXPORT_TO_IFC && slot.parameter_id != EXPORT_TYPE_TO_IFC {
                continue;
            }
            let resolved = metadata.resolved_family_parameters.iter().any(|value| {
                value.source_object == slot.source_object && value.parameter_id == slot.parameter_id
            });
            let watched = watch.contains(&record.identity.element_id);
            if watched || !resolved {
                println!(
                    "  slot {} {class} {} integer {} resolved {resolved}",
                    record.identity.element_id, slot.parameter_id, slot.integer_slot
                );
            }
            *slots
                .entry((slot.parameter_id, resolved, slot.integer_slot.to_string()))
                .or_default() += 1;
        }
        Ok(())
    })?;
    println!("FamilyParams slots by parameter, resolved and integer value:");
    for ((parameter, resolved, value), count) in &slots {
        println!("  {parameter} resolved {resolved} {value}: {count}");
    }
    Ok(())
}
