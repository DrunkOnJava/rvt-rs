//! RE-44: a system-family type names itself in its serialised data.
//!
//! FACT: a wall, floor, ceiling, roof or railing type has no name entry
//! (RE-38), but its serialised element data opens with
//! `ff ff ff ff · u16 · 01 00 00 00` and the type's `u64` ElementId, and the
//! first framed string after it (`ff ff ff ff · u16 tag · u32 n · n UTF-16
//! units`, tag never `0xffff`) is the type's name. The `u16` is `0x02d3` on
//! Revit 2024 and `0x02ef` on Revit 2025. Joined to elements through the
//! RE-28 type records, the name equals the type half of Revit's own
//! `ObjectType` for every wall, floor, ceiling, roof and railing on Core
//! Interior, Snowdon Towers Architectural and RE1 Architecture.
//!
//! The probe prints, per category, every type-definition record, the name
//! its data gives, and the types with no data block (RE1's one railing
//! type). Check a name against the export's `IfcWallType` / `IfcSlabType`
//! `Name`, or against the `ObjectType` of an element that names that type.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re44_system_type_names -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    OST_BUILDING_PAD, OST_CEILINGS, OST_FLOORS, OST_ROOFS, OST_STAIRS_RAILING, OST_WALLS,
};
use rvt::partition_names::{element_data_header, find_element_data_names};
use rvt::partition_type_records::{scan_type_records, type_definition_ids};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(header) = element_data_header(version) else {
        println!("release {version}: no measured element-data header");
        return Ok(());
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories = [
        ("Walls", OST_WALLS),
        ("Floors", OST_FLOORS),
        ("BuildingPad", OST_BUILDING_PAD),
        ("Ceilings", OST_CEILINGS),
        ("Roofs", OST_ROOFS),
        ("StairsRailing", OST_STAIRS_RAILING),
    ];
    let mut types: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    for (label, category) in categories {
        let records = scan_type_records(&mut rf, version, category, &declared)?;
        types.insert(label, type_definition_ids(&records));
    }
    let wanted: BTreeSet<u32> = types.values().flatten().copied().collect();
    let mut names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in find_element_data_names(inflated.bytes(), &header, &wanted) {
            names.entry(id).or_default().insert(name);
        }
    }
    println!("release {version}: system-family type records and their data names");
    for (label, ids) in types {
        let named = ids.iter().filter(|id| names.contains_key(id)).count();
        println!("  {label}: {} type records, {named} named", ids.len());
        for id in ids {
            match names.get(&id) {
                Some(found) if found.len() == 1 => {
                    println!("    {id:>9}  {:?}", found.first().expect("one name"));
                }
                Some(found) => println!("    {id:>9}  streams disagree: {found:?}"),
                None => println!("    {id:>9}  no element data"),
            }
        }
    }
    Ok(())
}
