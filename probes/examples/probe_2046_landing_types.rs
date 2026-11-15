//! #2046: which stair landings of a Revit 2024 to 2027 save get an
//! `IfcSlabType`, and what their native records name.
//!
//! FACT to check: a landing is written with a type object only when the
//! decode gives it a type id, a family name and a type name. On Revit 2024 the
//! family and name come from RE-65's byte anchors; on 2025 to 2027
//! `native_stair_parts` names a stair, its flights and its stringers from
//! native records and leaves a landing out. The probe exports the file and
//! counts, per release, the landings (`IfcSlab` `LANDING`) the export writes,
//! how many have each of the three, how many are typed by an `IfcSlabType`
//! (`step_writer::type_objects`), and, from the landings' native records, how
//! many name their stair (`m_stairsId`) and their type (`m_typeId`), and how
//! many of their landing types are monolithic (`m_hasMonolithicSupport`).
//!
//! Counts and a family word per line, and, for a save to be held to its project's other saves, one line per named landing with its ElementId and type (samples only).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2046_landing_types -- FILE.rvt

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::export_content::{FAMILY_NAME_PROPERTY, TYPE_NAME_PROPERTY};
use rvt::ifc::step_writer::type_objects;
use rvt::research::native_document::{Options, extract_graphs};
use std::collections::{BTreeMap, BTreeSet};

/// Starts every count line.
const TAG: &str = "landing types:";

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let typed: BTreeSet<usize> = type_objects(&model)
        .all()
        .flat_map(|object| object.elements.iter().copied())
        .collect();

    let mut landings: Vec<(usize, u32)> = Vec::new();
    let (mut with_type_id, mut with_names, mut named_as_part) = (0usize, 0usize, 0usize);
    let mut families: BTreeMap<String, usize> = BTreeMap::new();
    let mut named: Vec<(u32, String)> = Vec::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            predefined_type,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if !ifc_type.eq_ignore_ascii_case("IFCSLAB")
            || predefined_type.as_deref() != Some("LANDING")
        {
            continue;
        }
        let Some(id) = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok()) else {
            continue;
        };
        landings.push((index, id));
        with_type_id += usize::from(model.element_type_ids.contains_key(&id));
        named_as_part += usize::from(name.contains(" Landing "));
        let text = |key: &str| {
            property_set.as_ref().and_then(|set| {
                set.properties
                    .iter()
                    .find_map(|property| match &property.value {
                        PropertyValue::Text(text) if property.name == key => Some(text.clone()),
                        _ => None,
                    })
            })
        };
        if let (Some(family), Some(type_name)) =
            (text(FAMILY_NAME_PROPERTY), text(TYPE_NAME_PROPERTY))
        {
            with_names += 1;
            *families.entry(family.replace(' ', "_")).or_default() += 1;
            named.push((id, format!("{family}:{type_name}").replace(' ', "_")));
        }
    }
    // One line per named landing, for a save to be held to the same project's
    // other saves by ElementId: samples only, the held-out scrub drops it.
    for (id, name) in &named {
        println!("{TAG} release {version} landing {id} type {name}");
    }
    println!(
        "{TAG} release {version} landings {} with an element type id {with_type_id} with a family and a type name {with_names} named as a stair part {named_as_part} typed by a type object {}",
        landings.len(),
        landings
            .iter()
            .filter(|(index, _)| typed.contains(index))
            .count()
    );
    for (family, count) in &families {
        println!("{TAG} release {version} landings with family {family} {count}");
    }

    // What the landings' native records name.
    let wanted: BTreeSet<u64> = landings.iter().map(|(_, id)| u64::from(*id)).collect();
    if wanted.is_empty() || !rvt::research::native_document::NATIVE_RELEASES.contains(&version) {
        return Ok(());
    }
    let mut records = 0usize;
    let (mut stair, mut type_id, mut monolithic_types) = (0usize, 0usize, BTreeSet::new());
    let mut type_ids: BTreeSet<u64> = BTreeSet::new();
    let options = Options {
        selected_ids: wanted,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        else {
            return Ok(());
        };
        if record.class_name.as_deref() != Some("StairsLanding") {
            return Ok(());
        }
        records += 1;
        stair += usize::from(root.fields.get("m_stairsId").is_some());
        if let Some(id) = root
            .fields
            .get("m_typeId")
            .and_then(|value| value.get("m_id"))
            .and_then(|value| value.get("m_id64"))
            .and_then(|value| value.as_u64())
        {
            type_id += 1;
            type_ids.insert(id);
        }
        Ok(())
    })?;
    if !type_ids.is_empty() {
        let options = Options {
            selected_ids: type_ids.clone(),
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            if record.class_name.as_deref() != Some("StairsLandingType") {
                return Ok(());
            }
            let Some(root) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
            else {
                return Ok(());
            };
            if let Some(flag) = root
                .fields
                .get("m_hasMonolithicSupport")
                .and_then(|value| value.as_bool())
            {
                monolithic_types.insert((record.identity.element_id, flag));
            }
            Ok(())
        })?;
    }
    println!(
        "{TAG} release {version} native landing records {records} naming a stair {stair} naming a type {type_id} distinct landing types {} monolithic {} not monolithic {}",
        type_ids.len(),
        monolithic_types.iter().filter(|(_, flag)| *flag).count(),
        monolithic_types.iter().filter(|(_, flag)| !*flag).count()
    );
    Ok(())
}
