//! W6: what a hollow structural section's type holds that Revit's export
//! draws its column from, on the BIM4LCA office's 79 boxed
//! `Rectangular and Square Hollow Sections-Column` columns.
//!
//! FACT under test: a hollow section type stores its Width and Height under
//! RE-103's parameters and its wall thickness (and corner radii) under
//! other structural-section BuiltInParameters near them, in the
//! `f64 · ff×8 · i64` entries of its value block.
//!
//! Output:
//!   elem <id>                              up to 3 columns per hollow-section type
//!   type <type id> <family>:<type name> columns <n>
//!   param <type id> <parameter> <value, feet>
//!   native <parameter> <object index> <record class>/<object class> <scalar members>
//!                                          the element of every positive parameter id
//!                                          every `f64 · ff×8 · i64` entry in its blocks
//!
//! `tools/re/body_forms_list.py` prints Revit's body of each `elem`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_hss_sections -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_schema_mvp::{
    FAMILY_NAME_FIELD, TYPE_ID_FIELD, TYPE_NAME_FIELD, recover_partition_schema_mvp,
};
use rvt::research::partition_type_parameters::partition_length_entries;
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};

fn text(element: &DecodedElement, wanted: &str) -> Option<String> {
    element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(text) if name == wanted => Some(text.clone()),
        _ => None,
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut by_type: BTreeMap<u32, (String, Vec<u32>)> = BTreeMap::new();
    for element in mvp.columns.iter().chain(mvp.products.iter()) {
        let family = text(element, FAMILY_NAME_FIELD).unwrap_or_default();
        let name = text(element, TYPE_NAME_FIELD).unwrap_or_default();
        if !family.contains("Hollow") {
            continue;
        }
        let type_id = element
            .fields
            .iter()
            .find_map(|(field, value)| match value {
                InstanceField::ElementId { id, .. } if field == TYPE_ID_FIELD => Some(*id),
                _ => None,
            });
        let (Some(id), Some(type_id)) = (element.id, type_id) else {
            continue;
        };
        by_type
            .entry(type_id)
            .or_insert_with(|| (format!("{family}:{name}"), Vec::new()))
            .1
            .push(id);
    }
    for (type_id, (name, columns)) in &by_type {
        println!(
            "type {type_id} {} columns {}",
            name.replace(' ', "_"),
            columns.len()
        );
        for id in columns.iter().take(3) {
            println!("elem {id}");
        }
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let types: BTreeSet<u32> = by_type.keys().copied().collect();
    let mut found: BTreeMap<(u32, i64), BTreeSet<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in partition_length_entries(inflated.bytes(), &declared) {
            if !types.contains(&owner) {
                continue;
            }
            for (parameter, value) in entries {
                found
                    .entry((owner, parameter))
                    .or_default()
                    .insert(format!("{value:.6}"));
            }
        }
    }
    let mut elements: BTreeSet<u64> = BTreeSet::new();
    for ((owner, parameter), values) in found {
        let values: Vec<String> = values.into_iter().collect();
        println!("param {owner} {parameter} {}", values.join(" "));
        if let Ok(id) = u64::try_from(parameter) {
            if id > 1000 && id < u64::from(u32::MAX) {
                elements.insert(id);
            }
        }
    }
    let options = Options {
        selected_ids: elements,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate().take(6) {
            let members: Vec<String> = object
                .fields
                .as_object()
                .into_iter()
                .flat_map(|fields| fields.iter())
                .filter(|(_, v)| v.is_number() || v.is_boolean() || v.is_string())
                .map(|(name, v)| format!("{name}={v}"))
                .collect();
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members.join(" ")
            );
        }
        Ok(())
    })?;
    Ok(())
}
