//! #1899: why the two furred ceilings of the Revit 2023 save of Autodesk's
//! `rac_advanced` (247617 and 247677) get no type.
//!
//! FACT under test: on the 2022 and 2024 saves the export names them
//! `Compound Ceiling:Furred Ceiling:<id>` and writes their
//! `Pset_CoveringCommon`; the 2023 save names them `Ceiling-<id>` and writes
//! none. Their native records and saved parameters are the same on both saves
//! (`probe_1899_covering_parameters`), so what differs is the type, and on
//! 2023 a system-family type is found by its element-data name, one the
//! element's record names (`system_type_names_2023`, RE-111). RE-180 section 5
//! records that a list of one ElementId, read as a name, is a one-unit name:
//! it gave ceiling type 247676 the stray name `퉼` on the 2019 to 2022 saves,
//! next to its own name, and RE-180 drops every one-character element-data
//! name on 2019 to 2022 and keeps 2023's reading, because dropping them all
//! on 2023 would give five `rac_basic` walls a type their 2024 copies do not
//! give. The stray name is not ASCII where that wall type's `p` is, and
//! `partition_names::lone_non_ascii` is the rule RE-180 already applies to
//! name entries on every 32-bit release, 2023 included.
//!
//! Hypothesis, registered before the run: on the 2023 save the type 247676 has
//! two names across the partitions, the stray one and `Furred Ceiling`, so it
//! has none, and the ceilings find no type. Dropping a lone non-ASCII
//! element-data name before the names are merged across partitions gives the
//! ceilings 247676, and changes no other element of the four 2023 samples (the
//! `p` of `rac_basic` is ASCII and stays); on 2015 to 2022 the filter is
//! already applied (every one-character name is dropped), so nothing changes.
//!
//! The probe repeats `system_type_names_2023`'s choice for every exported wall,
//! floor, roof and ceiling record, once with the names as the reader merges
//! them (`before`) and once without a lone non-ASCII name (`after`), and
//! prints what each change would be.
//!
//! Output:
//!
//!   file <path> release <release>
//!   filter_already_applied <true|false>
//!   lone_names <n>
//!   lone id <id> names <name,...> in_references_of <n records>
//!   summary system_type_elements <n> unchanged <n> gained <n> changed <n> lost <n>
//!   element <id> category <category> before <type id:name|none> after <type id:name|none>
//!     ref <id> status <named|conflict|unnamed> name <name|-> lone_non_ascii <bool> named_by <categories>
//!
//! An element line is printed for every ceiling and for every element whose
//! type would change; its ref lines are the ids its record names that have no
//! record and no name entry of their own (the candidates).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1899_ceiling_type_names -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{
    OST_CEILINGS, OST_FLOORS, OST_ROOFS, OST_WALLS, PartitionElementRecord,
};
use rvt::research::partition_element_records_2023::scan_records;
use rvt::research::partition_names::{
    find_element_data_names_32, lone_non_ascii, name_entries_2023,
};
use std::collections::{BTreeMap, BTreeSet};

/// Walls, floors, roofs and ceilings: the categories whose types are
/// system-family types (RE-111).
const SYSTEM_TYPE_CATEGORIES: [i64; 4] = [OST_WALLS, OST_FLOORS, OST_ROOFS, OST_CEILINGS];

/// A name per ElementId; `None` where the partitions give two.
type Names = BTreeMap<u32, Option<String>>;

/// One element's type as `before` and `after` the filter.
struct Row<'a> {
    record: &'a PartitionElementRecord,
    before: Option<(u32, String)>,
    after: Option<(u32, String)>,
}

/// The merge `system_type_names_2023` makes across partitions: a second,
/// different name leaves the id with none.
fn merge(names: &mut Names, id: u32, name: &str) {
    match names.get(&id) {
        None => {
            names.insert(id, Some(name.to_string()));
        }
        Some(held) if held.as_deref() != Some(name) => {
            names.insert(id, None);
        }
        _ => {}
    }
}

/// `system_type_names_2023`'s choice for one record: of the named ids its
/// record names, the one whose naming records' categories are a strict subset
/// of each other's.
fn pick(
    names: &Names,
    named_by: &BTreeMap<u32, BTreeSet<i64>>,
    record: &PartitionElementRecord,
) -> Option<(u32, String)> {
    let candidates: BTreeSet<u32> = record
        .references
        .iter()
        .filter_map(|&reference| u32::try_from(reference).ok())
        .filter(|id| matches!(names.get(id), Some(Some(_))))
        .collect();
    let mut picked = candidates.iter().copied().filter(|id| {
        candidates.iter().all(|other| {
            other == id
                || (named_by[id].is_subset(&named_by[other]) && named_by[id] != named_by[other])
        })
    });
    let (Some(type_id), None) = (picked.next(), picked.next()) else {
        return None;
    };
    match names.get(&type_id) {
        Some(Some(name)) => Some((type_id, name.clone())),
        _ => None,
    }
}

fn show(pick: &Option<(u32, String)>) -> String {
    match pick {
        Some((id, name)) => format!("{id}:{name:?}"),
        None => "none".to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    let records = scan_records(&mut rf, version);
    if records.is_empty() {
        println!("not read: no 32-bit element records on release {version}");
        return Ok(());
    }
    // Every one-character name is already dropped on 2008 to 2022
    // (`find_element_data_names_32`), so the filter changes nothing there.
    println!("filter_already_applied {}", version != 2023);
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let entries = name_entries_2023(&mut rf, &declared);
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    // The categories of the records naming each id that has neither a record
    // nor a name entry of its own.
    let mut named_by: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    for record in &records {
        for id in record
            .references
            .iter()
            .filter_map(|&reference| u32::try_from(reference).ok())
            .filter(|id| !recorded.contains(id) && !entries.contains_key(id))
        {
            named_by
                .entry(id)
                .or_default()
                .insert(record.builtin_category);
        }
    }
    let wanted: BTreeSet<u32> = named_by.keys().copied().collect();
    let mut as_read = Names::new();
    let mut without = Names::new();
    let mut lone: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in find_element_data_names_32(inflated.bytes(), version, &wanted) {
            merge(&mut as_read, id, &name);
            if lone_non_ascii(&name) {
                lone.entry(id).or_default().insert(name);
            } else {
                merge(&mut without, id, &name);
            }
        }
    }
    println!("lone_names {}", lone.len());
    for (id, names) in &lone {
        let naming = records
            .iter()
            .filter(|record| {
                record
                    .references
                    .iter()
                    .any(|&reference| u32::try_from(reference) == Ok(*id))
            })
            .count();
        let shown: Vec<String> = names
            .iter()
            .map(|name| format!("{name:?}U+{:04X}", name.chars().next().map_or(0, u32::from)))
            .collect();
        println!(
            "lone id {id} names {} in_references_of {naming} records",
            shown.join(",")
        );
    }
    // As the reader does, an element's last record wins.
    let mut rows: BTreeMap<u32, Row> = BTreeMap::new();
    for record in records.iter().filter(|record| {
        record.is_exported_instance() && SYSTEM_TYPE_CATEGORIES.contains(&record.builtin_category)
    }) {
        rows.insert(
            record.element_id,
            Row {
                record,
                before: pick(&as_read, &named_by, record),
                after: pick(&without, &named_by, record),
            },
        );
    }
    let (mut unchanged, mut gained, mut changed, mut lost) = (0, 0, 0, 0);
    let mut lines: Vec<String> = Vec::new();
    for (id, row) in &rows {
        match (&row.before, &row.after) {
            (before, after) if before == after => unchanged += 1,
            (None, Some(_)) => gained += 1,
            (Some(_), None) => lost += 1,
            _ => changed += 1,
        }
        if row.before == row.after && row.record.builtin_category != OST_CEILINGS {
            continue;
        }
        lines.push(format!(
            "element {id} category {} before {} after {}",
            row.record.builtin_category,
            show(&row.before),
            show(&row.after)
        ));
        for reference in row
            .record
            .references
            .iter()
            .filter_map(|&reference| u32::try_from(reference).ok())
            .filter(|reference| named_by.contains_key(reference))
        {
            let (status, name) = match as_read.get(&reference) {
                Some(Some(name)) => ("named", name.clone()),
                Some(None) => ("conflict", "-".to_string()),
                None => ("unnamed", "-".to_string()),
            };
            lines.push(format!(
                "  ref {reference} status {status} name {name:?} lone_non_ascii {} named_by {:?}",
                lone.contains_key(&reference),
                named_by[&reference]
            ));
        }
    }
    println!(
        "summary system_type_elements {} unchanged {unchanged} gained {gained} changed {changed} lost {lost}",
        rows.len()
    );
    for line in lines {
        println!("{line}");
    }
    Ok(())
}
