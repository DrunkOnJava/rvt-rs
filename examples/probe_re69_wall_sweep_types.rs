//! RE-69: a wall sweep names its type among the objects its record names.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a wall
//! sweep's type has no type-definition record of the sweep category
//! (`OST_Cornices`), so RE-44's type join finds none. The record names the
//! type all the same, among its profile (a family type with a name entry,
//! RE-38), its material and the walls it runs along. The type is the one
//! named id that has a type object (`01 00 00 00 · u64 id`, tag `db 0f`) and
//! no name entry, and its element-data name is Revit's type name: "Base_Simple",
//! "Crown_Flat", "Coping_Rainscreen Wall". Revit names those 249 sweeps
//! `Wall Sweep:<type>:<ElementId>`. The other 9 sweeps name no such object;
//! they are sweeps of a wall type's own structure, and Revit names them by
//! their ElementId alone.
//!
//! The probe prints one row per wall sweep record: its id, and the ids it
//! names that have a type object and no name entry, with their element-data
//! names.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re69_wall_sweep_types -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let (Some(marker), Some(header)) = (
        per::bbox_marker(version),
        rvt::partition_names::element_data_header(version),
    ) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let entries = rf.element_names();
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut sweeps: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        for record in per::find_category_records_assigned(
            &stream,
            inflated.bytes(),
            per::OST_CORNICES,
            &declared,
            &marker,
            ids,
        ) {
            let named: BTreeSet<u32> = record
                .references
                .iter()
                .filter_map(|&slot| u32::try_from(slot).ok())
                .filter(|id| *id != record.element_id && !entries.entries.contains_key(id))
                .collect();
            sweeps.entry(record.element_id).or_default().extend(named);
        }
    }
    let wanted: BTreeSet<u32> = sweeps.values().flatten().copied().collect();
    let mut objects = BTreeSet::new();
    let mut names = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        objects.extend(rvt::partition_names::find_type_object_ids(
            inflated.bytes(),
            &wanted,
        ));
        names.extend(rvt::partition_names::find_element_data_names(
            inflated.bytes(),
            &header,
            &wanted,
        ));
    }
    eprintln!(
        "Revit {version}: {} wall sweep records, {} named ids with a type object",
        sweeps.len(),
        objects.len()
    );
    for (id, named) in &sweeps {
        let types: Vec<String> = named
            .iter()
            .filter(|n| objects.contains(n))
            .map(|n| format!("{n} {:?}", names.get(n).map_or("-", String::as_str)))
            .collect();
        println!("{id}\t{}", types.join(" | "));
    }
    Ok(())
}
