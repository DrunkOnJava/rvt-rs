//! RE-67: model text names a text type, whose object carries a font.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, the 7
//! Generic Models Revit's own IFC export names `Model Text:<type>:<id>` are
//! not family instances. Each one's reference list names exactly one type
//! object (`01 00 00 00 · u64 id`, tag `db 0f`) that frames its own name
//! with tag 0x0149 and then a font with tag 0x07eb: "10\" Trebuchet MS" for
//! six of them and "18\" Trebuchet MS" for one, both in Trebuchet MS. That
//! name is the type half of Revit's name for all 7.
//!
//! The probe prints each Generic Models record with no RE-38 name: its id,
//! and the text types its list names with their names.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re67_model_text_types -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
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
    let entries = rf.element_names();
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut lists: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        for record in per::find_category_records_assigned(
            &stream,
            inflated.bytes(),
            per::OST_GENERIC_MODEL,
            &declared,
            &marker,
            ids,
        ) {
            let references: BTreeSet<u32> = record
                .references
                .iter()
                .filter_map(|&slot| u32::try_from(slot).ok())
                .filter(|id| *id != record.element_id)
                .collect();
            // A family instance names a type with a name entry (RE-38).
            if references.iter().any(|id| entries.entries.contains_key(id)) {
                continue;
            }
            lists
                .entry(record.element_id)
                .or_default()
                .extend(references);
        }
    }
    let wanted: BTreeSet<u32> = lists.values().flatten().copied().collect();
    let mut names = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        names.extend(rvt::partition_names::find_text_type_names(
            inflated.bytes(),
            &wanted,
        ));
    }
    eprintln!(
        "Revit {version}: {} Generic Models records without a family, {} text types among what they name",
        lists.len(),
        names.len()
    );
    for (id, references) in &lists {
        let text: Vec<String> = references
            .iter()
            .filter_map(|r| names.get(r).map(|n| format!("{r} {n:?}")))
            .collect();
        println!("{id}\t{}", text.join(" | "));
    }
    Ok(())
}
