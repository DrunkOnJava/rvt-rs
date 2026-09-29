//! RE-130 (probe): the reference list at `+0x88` of each element record
//! (#228), for attributing its slots against Revit's export.
//!
//! Prints one JSON object per record of the categories rvt-rs exports,
//! `{id, category, refs}`, and one per Level name entry, `{level, name}`.
//! `tools/re/reference_slots_vs_ifc.py` finds the slot holding the
//! element's storey (Revit names each storey after its Level) and its type
//! (the type object's Tag) in Revit's export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re130_reference_slots -- MODEL.rvt > refs.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;

/// Revit's `OST_Levels`.
const OST_LEVELS: i64 = -2_000_240;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names = rf.element_names();
    for entry in names.entries.values() {
        if entry.builtin_category == OST_LEVELS {
            println!(
                "{{\"level\":{},\"name\":{:?}}}",
                entry.element_id, entry.name
            );
        }
    }
    let mut categories = vec![
        per::OST_WALLS,
        per::OST_DOORS,
        per::OST_WINDOWS,
        per::OST_FLOORS,
        per::OST_COLUMNS,
    ];
    categories.extend(
        per::PRODUCT_RECORD_CATEGORIES
            .iter()
            .map(|(category, _)| *category),
    );
    categories.sort_unstable();
    categories.dedup();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for record in &records {
        let Ok(inflated) = rf.inflated_partition(&record.stream) else {
            continue;
        };
        let refs = per::decode_reference_list(
            inflated.bytes(),
            record.offset + per::REFERENCE_LIST_OFFSET,
        )
        .unwrap_or_default();
        let refs: Vec<String> = refs.iter().map(u64::to_string).collect();
        println!(
            "{{\"id\":{},\"category\":{},\"refs\":[{}]}}",
            record.element_id,
            record.builtin_category,
            refs.join(",")
        );
    }
    eprintln!("Revit {version}: {} records", records.len());
    Ok(())
}
