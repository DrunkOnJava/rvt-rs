//! #1603: does a family type's own record on a Revit 2015 to 2023 save hold the
//! geometry-material map (`FamilySymbol.m_geomTag2MaterialId`, RE-203) that
//! names the windows', railings' and doors' materials, and does reading it
//! name materials the 32-bit scan (`type_material_names_32`, RE-113) misses?
//!
//! FACT 1 to check: `native_type_materials::geometry_maps` reads a map entry's
//! material as `/second/m_id/m_id64`, the 2024 shape of an ElementId; a save
//! before Revit 2024 holds `/second/m_id/m_id` (RE-280). The probe reads either
//! and counts each shape, so a release that holds the map but in the shape the
//! reader does not take shows as such.
//!
//! FACT 2 to check: on a 2015 to 2023 save the map names materials of the
//! file's material set for the types of its windows, railings and other
//! family instances, and the scan names none for some of them. For each class
//! of instance the probe counts the types its instances name, how many of them
//! the native read reached, how many name at least one material through the map
//! and through the scan, and how many only through the map or only through the
//! scan.
//!
//! The release is not changed: `Options::releases` is widened to the native
//! releases here, as a reader measured on 2015 to 2022 would name them.
//!
//! Counts only on a model fetched without a licence: the `type maps pre2024:`
//! lines carry numbers beside a release and a class name and nothing else
//! (`tools/ci/scrub-holdout-results.sh` keeps them); the `type map:` lines name
//! ElementIds, for pairing a save with the same project's later save, and the
//! scrub drops them.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_type_maps_pre2024 -- FILE.rvt

use rvt::RevitFile;
use rvt::research::native_document::{self, NATIVE_RELEASES, Options};
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};

/// Starts every count line, so the held-out scrub can keep them by it.
const TAG: &str = "type maps pre2024:";

/// What the native read found for one type.
#[derive(Default)]
struct Found {
    entries: usize,
    unset: bool,
    named: BTreeSet<u32>,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;

    // Each type the family instances name, with the classes that name it.
    let mut classes: BTreeMap<u64, BTreeSet<String>> = BTreeMap::new();
    for element in decoded
        .columns
        .iter()
        .chain(&decoded.doors)
        .chain(&decoded.windows)
        .chain(&decoded.products)
    {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(u64::from(*id)),
            _ => None,
        });
        if let Some(type_id) = type_id {
            classes
                .entry(type_id)
                .or_default()
                .insert(element.class.clone());
        }
    }
    let types: BTreeSet<u64> = classes.keys().copied().collect();

    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let (scanned, materials): (BTreeMap<u32, Vec<String>>, BTreeSet<u32>) =
        if rvt::research::partition_element_records_2023::is_32bit_release(version) {
            let (_, names) =
                rvt::research::partition_materials::scan_materials_32(&mut rf, version, &declared);
            (
                rvt::research::partition_type_materials::type_material_names_32(&mut rf, version),
                names.keys().copied().collect(),
            )
        } else {
            let names = rvt::research::partition_materials::scan_material_names(
                &mut rf, version, &declared,
            )?;
            (
                rvt::research::partition_type_materials::type_material_names(&mut rf, version),
                names.keys().copied().collect(),
            )
        };

    let options = Options {
        selected_ids: types.clone(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut read: BTreeSet<u64> = BTreeSet::new();
    let mut found: BTreeMap<u64, Found> = BTreeMap::new();
    let (mut wide, mut narrow) = (0usize, 0usize);
    let mut records = 0usize;
    let summary = native_document::extract_graphs(&mut rf, &options, |record| {
        records += 1;
        let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        else {
            return Ok(());
        };
        let Some(map) = root
            .fields
            .get("m_geomTag2MaterialId")
            .and_then(|value| value.as_array())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        read.insert(id);
        let mut here = Found {
            entries: map.len(),
            ..Found::default()
        };
        for entry in map {
            let wide_id = entry
                .pointer("/second/m_id/m_id64")
                .and_then(|value| value.as_i64());
            let narrow_id = entry
                .pointer("/second/m_id/m_id")
                .and_then(|value| value.as_i64());
            wide += usize::from(wide_id.is_some());
            narrow += usize::from(wide_id.is_none() && narrow_id.is_some());
            match wide_id.or(narrow_id) {
                Some(-1) => here.unset = true,
                Some(material) => {
                    if let Some(material) = u32::try_from(material)
                        .ok()
                        .filter(|material| materials.contains(material))
                    {
                        here.named.insert(material);
                    }
                }
                None => {}
            }
        }
        found.insert(id, here);
        Ok(())
    });
    if summary.is_err() {
        println!("{TAG} release {version} native read failed");
        return Ok(());
    }

    println!(
        "{TAG} release {version} types {} records {records} read {} entries in the 2024 shape {wide} in the 2015 to 2023 shape {narrow}",
        types.len(),
        read.len()
    );

    // Per class of instance: the types it names, by what reads them.
    let mut by_class: BTreeMap<&str, BTreeSet<u64>> = BTreeMap::new();
    for (type_id, names) in &classes {
        for class in names {
            by_class.entry(class.as_str()).or_default().insert(*type_id);
        }
    }
    for (class, ids) in &by_class {
        let scan_names = |type_id: &u64| {
            u32::try_from(*type_id)
                .ok()
                .and_then(|id| scanned.get(&id))
                .is_some_and(|list| !list.is_empty())
        };
        let map_names = |type_id: &u64| {
            found
                .get(type_id)
                .is_some_and(|found| !found.named.is_empty())
        };
        println!(
            "{TAG} class {class} types {} read {} named by the map {} named by the scan {} only by the map {} only by the scan {} unset only {}",
            ids.len(),
            ids.iter().filter(|id| read.contains(id)).count(),
            ids.iter().filter(|id| map_names(id)).count(),
            ids.iter().filter(|id| scan_names(id)).count(),
            ids.iter()
                .filter(|id| map_names(id) && !scan_names(id))
                .count(),
            ids.iter()
                .filter(|id| scan_names(id) && !map_names(id))
                .count(),
            ids.iter()
                .filter(|id| found
                    .get(id)
                    .is_some_and(|found| found.named.is_empty() && found.unset))
                .count(),
        );
    }

    // One line per type that holds a map, for the pairing script (ElementIds).
    for (type_id, here) in &found {
        println!(
            "type map: {type_id} classes {} entries {} unset {} named {}",
            classes
                .get(type_id)
                .map(|names| names.iter().cloned().collect::<Vec<_>>().join(","))
                .unwrap_or_default(),
            here.entries,
            here.unset,
            here.named
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        );
    }
    Ok(())
}
