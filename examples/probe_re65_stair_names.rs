//! RE-65: stair, landing and support types carry their system family, and a
//! stair's parts are numbered in ElementId order.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample:
//! - A stair type's object (`01 00 00 00 · u64 id`, tag `db 0f`) holds a
//!   `u32` construction at +0x129. It is 0 on the types of the 23 stairs
//!   Revit's own IFC export names `Assembled Stair:Stair:<id>`, and 1 on the
//!   types of the 3 it names `Cast-In-Place Stair:Stair:<id>`.
//! - The stair type lists its run, landing and support types in seven u64
//!   slots from +0xc9.
//! - After its parameter entries (`u32` count at +0x13f, each an `i64`
//!   BuiltInParameter and a string), the type holds its name.
//! - A landing type's flag at +0x95 is 0, with its name at +0x98, on the one
//!   type Revit names Non-Monolithic Landing.
//! - A support type's flag at +0x89 is 0 on the types Revit names Stringer
//!   and 1 on those it names Carriage, with the name at +0x92.
//! - Revit names each run, landing and support `<stair name> Run|Landing|
//!   Stringer N`. N is the part's rank by ElementId among every record of
//!   its category whose reference list names the stair, exported or not.
//!
//! The probe prints each stair's type, and then each part:
//! - its stair or stairs;
//! - its candidate types;
//! - its number.
//!
//! Join the rows to Revit's export by `Tag`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re65_stair_names -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_stairs as ps;
use rvt::partition_type_records as ptr;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let Some(marker) = per::bbox_marker(version) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let kinds = [
        ("Stair", per::OST_STAIRS),
        ("Run", per::OST_STAIRS_RUNS),
        ("Landing", per::OST_STAIRS_LANDINGS),
        ("Stringer", per::OST_STAIRS_STRINGER_CARRIAGE),
    ];
    let mut types: BTreeMap<&str, BTreeSet<u32>> = BTreeMap::new();
    for (word, category) in kinds {
        types.insert(
            word,
            ptr::type_definition_ids(&ptr::scan_type_records(
                &mut rf, version, category, &declared,
            )?),
        );
    }
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut records: BTreeMap<&str, Vec<per::PartitionElementRecord>> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        for (word, category) in kinds {
            for record in per::find_category_records_assigned(
                &stream,
                inflated.bytes(),
                category,
                &declared,
                &marker,
                ids,
            ) {
                if seen.insert(record.element_id) {
                    records.entry(word).or_default().push(record);
                }
            }
        }
    }
    let named = |references: &[u64], set: &BTreeSet<u32>| -> Vec<u32> {
        let mut out: Vec<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| set.contains(id))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    };
    let stairs: BTreeSet<u32> = records
        .get("Stair")
        .map(|list| list.iter().map(|record| record.element_id).collect())
        .unwrap_or_default();
    let stair_type_ids: BTreeSet<u32> = records
        .get("Stair")
        .into_iter()
        .flatten()
        .flat_map(|record| named(&record.references, &types["Stair"]))
        .collect();
    let stair_types =
        ps::scan_component_types(&mut rf, version, ps::ComponentKind::Stair, &stair_type_ids)?;
    eprintln!(
        "Revit {version}: {} stairs, {} stair types read",
        stairs.len(),
        stair_types.len()
    );
    for record in records.get("Stair").into_iter().flatten() {
        let type_ids = named(&record.references, &types["Stair"]);
        let described = match type_ids.as_slice() {
            [id] => match stair_types.get(id) {
                Some(found) => format!(
                    "type {id} {:?} {:?} components {:?}",
                    found.family.unwrap_or("-"),
                    found.name,
                    found.components
                ),
                None => format!("type {id} does not read"),
            },
            _ => format!("types {type_ids:?}"),
        };
        println!("stair\t{}\t{described}", record.element_id);
    }
    for word in ["Run", "Landing", "Stringer"] {
        let list = records.get(word).cloned().unwrap_or_default();
        let mut members: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        for record in &list {
            for stair in named(&record.references, &stairs) {
                members.entry(stair).or_default().insert(record.element_id);
            }
        }
        for record in &list {
            let of = named(&record.references, &stairs);
            let numbers: Vec<String> = of
                .iter()
                .map(|stair| {
                    let rank = members[stair]
                        .iter()
                        .position(|id| *id == record.element_id)
                        .map_or(0, |at| at + 1);
                    format!("{stair} {word} {rank}")
                })
                .collect();
            println!(
                "{word}\t{}\ttypes {:?}\t{}",
                record.element_id,
                named(&record.references, &types[word]),
                numbers.join(" | ")
            );
        }
    }
    Ok(())
}
