//! #1713: do the walls and spaces of the BIM4LCA permit files that rvt-rs
//! leaves without a predefined type hold two copies of the parameter that
//! disagree?
//!
//! FACT to check: `scan_export_parameters` drops a key an element sets to two
//! different values, in one partition or across partitions. On the three
//! permit files (housing concrete, office, timber office) Revit's export
//! writes `PARTITIONING`, `SOLIDWALL` and the like on 40 walls, and
//! `USERDEFINED` on 42 family-instance spaces of the timber office, where
//! rvt-rs reads no value; the as-built files read all of them.
//!
//! The probe lists, for the instance and the type "IFC Predefined Type"
//! (`IFC_EXPORT_PREDEFINEDTYPE`, `IFC_EXPORT_PREDEFINEDTYPE_TYPE`), every
//! entry that a data object of a declared element holds: its stream, offset
//! and value. It then counts the owners with entries in one place, in several
//! places that agree, and in several places that disagree, and prints the
//! disagreeing owners with every entry, so that the Tags a scorer reports
//! unset can be looked up.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1713_export_parameter_copies -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_ifc_export_overrides::{
    IFC_EXPORT_PREDEFINEDTYPE, IFC_EXPORT_PREDEFINEDTYPE_TYPE,
};
use rvt::research::partition_room_parameters::DATA_OBJECT_HEADER;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The UTF-16 string at `at` (`u32` length, then the units), when every unit
/// is printable ASCII.
fn utf16_value(buf: &[u8], at: usize) -> Option<String> {
    let units = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=64).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    bytes
        .chunks_exact(2)
        .map(|pair| {
            u8::try_from(u16::from_le_bytes([pair[0], pair[1]]))
                .ok()
                .filter(|byte| byte.is_ascii_graphic() || *byte == b' ')
                .map(char::from)
        })
        .collect()
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    println!("release {version}");
    // (owner, key) -> every entry as (stream, offset, value).
    let mut entries: BTreeMap<(u32, i64), Vec<(String, usize, String)>> = BTreeMap::new();
    let mut undeclared = 0usize;
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let Ok(objects) = rf.partition_data_objects(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for key in [IFC_EXPORT_PREDEFINEDTYPE, IFC_EXPORT_PREDEFINEDTYPE_TYPE] {
            for hit in memchr::memmem::find_iter(buf, &key.to_le_bytes()) {
                let Some(value) = utf16_value(buf, hit + 8) else {
                    continue;
                };
                let Some(owner) = objects
                    .partition_point(|(p, _)| *p <= hit)
                    .checked_sub(1)
                    .map(|index| objects[index])
                    .filter(|(p, object)| hit >= p + DATA_OBJECT_HEADER && hit + 8 <= object.end)
                    .map(|(_, object)| object.element_id)
                else {
                    continue;
                };
                if !declared.contains(&owner) {
                    undeclared += 1;
                    continue;
                }
                entries
                    .entry((owner, key))
                    .or_default()
                    .push((stream.clone(), hit, value));
            }
        }
    }
    println!(
        "entries in a declared element's data object: {}",
        entries.values().map(Vec::len).sum::<usize>()
    );
    println!("entries in an undeclared element's data object: {undeclared}");
    for (key, label) in [
        (IFC_EXPORT_PREDEFINEDTYPE, "instance"),
        (IFC_EXPORT_PREDEFINEDTYPE_TYPE, "type"),
    ] {
        let mine: Vec<(&u32, &Vec<(String, usize, String)>)> = entries
            .iter()
            .filter(|((_, k), _)| *k == key)
            .map(|((owner, _), list)| (owner, list))
            .collect();
        let one = mine.iter().filter(|(_, list)| list.len() == 1).count();
        let agree = mine
            .iter()
            .filter(|(_, list)| {
                list.len() > 1 && list.iter().map(|e| &e.2).collect::<BTreeSet<_>>().len() == 1
            })
            .count();
        let disagree: Vec<_> = mine
            .iter()
            .filter(|(_, list)| list.iter().map(|e| &e.2).collect::<BTreeSet<_>>().len() > 1)
            .collect();
        println!(
            "{label} predefined type: {} owners; one entry {one}; several entries that agree {agree}; several that disagree {}",
            mine.len(),
            disagree.len()
        );
        let mut values: BTreeMap<String, usize> = BTreeMap::new();
        for (_, list) in &mine {
            for e in list.iter() {
                *values.entry(e.2.clone()).or_default() += 1;
            }
        }
        println!("  values over all entries: {values:?}");
        for (owner, list) in disagree {
            let parts: Vec<String> = list
                .iter()
                .map(|(stream, at, value)| format!("{stream} {at:#x} {value}"))
                .collect();
            println!("  disagree {label} owner {owner}: {}", parts.join(" | "));
        }
    }
    Ok(())
}
