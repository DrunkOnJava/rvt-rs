//! RE-111: on Revit 2023 a system-family type (a wall, floor or roof type)
//! names itself in its element data, and the element's record names it.
//!
//! FACT: a 2023 element's serialised data opens with `ff ff ff ff`, the
//! schema tag of `CellList` (`0x02c0` on 2023), `01 00 00 00` and a `u32`
//! ElementId, and a system-family type's name is the first framed string
//! after that, as on 2024 (#322), except that another element's data header
//! is not a name. Of the ids an exported wall's, floor's or roof's record
//! names that have no record of their own, no RE-109 name entry and such a
//! name, its type is the one whose naming records are of a strict subset of
//! the categories naming each of the others.
//!
//! The probe prints, for every exported 2023 wall, floor and roof, those
//! ids with their names and the categories of the records naming them;
//! compare them with the `IfcRelDefinesByType` Revit's own export gives the
//! element's `Tag`. On the two 2023 projects with a Revit export, Revit's
//! type is always among them, and always the most specific.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re111_system_types_2023 -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records_2023::{REVIT_2023, scan_records};
use std::collections::{BTreeMap, BTreeSet};

/// OST_Walls, OST_Floors, OST_Roofs.
const SYSTEM_CATEGORIES: [i64; 3] = [-2_000_011, -2_000_032, -2_000_035];

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let records = scan_records(&mut rf, REVIT_2023);
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let table = rvt::elem_table::parse_records(&mut rf).expect("Global/ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let entries = rvt::partition_names::name_entries_2023(&mut rf, &declared);
    let wanted: BTreeSet<u32> = declared
        .iter()
        .copied()
        .filter(|id| !recorded.contains(id) && !entries.contains_key(id))
        .collect();
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        names.extend(rvt::partition_names::find_element_data_names_2023(
            inflated.bytes(),
            &wanted,
        ));
    }
    // The categories of the records naming each id.
    let mut named_by: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    for record in &records {
        for &reference in &record.references {
            if let Ok(id) = u32::try_from(reference) {
                named_by
                    .entry(id)
                    .or_default()
                    .insert(record.builtin_category);
            }
        }
    }
    let mut seen = BTreeSet::new();
    for record in records.iter().filter(|record| {
        record.is_exported_instance() && SYSTEM_CATEGORIES.contains(&record.builtin_category)
    }) {
        if !seen.insert(record.element_id) {
            continue;
        }
        let candidates: Vec<(u32, &String, Vec<i64>)> = record
            .references
            .iter()
            .filter_map(|&reference| {
                let id = u32::try_from(reference).ok()?;
                names.get(&id).map(|name| {
                    let categories = named_by.get(&id).into_iter().flatten().copied().collect();
                    (id, name, categories)
                })
            })
            .collect();
        println!(
            "{}\t{}\t{:?}",
            record.element_id, record.builtin_category, candidates
        );
    }
}
