//! RE-128 (probe): the flag word at `+0x46` of each element record (#223).
//!
//! Revit's own schema calls it `m_abFlags4Bytes` (Discussion #112). This
//! probe prints it for every record of the categories rvt-rs exports, one
//! JSON object per record: `id`, `category`, `class` (the `+0x4a` class
//! tag), `flags` (`+0x46`), `placement` (`+0x42`) and whether the record
//! has a container. `tools/re/record_flags_vs_ifc.py` joins them to
//! Revit's export by Tag and tabulates each bit against what it writes.
//!
//! As a check on the offsets, the probe re-reads `+0x4a` from the same
//! bytes and counts the records where it differs from the decoded class.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re128_record_flags -- MODEL.rvt > records.jsonl

use rvt::RevitFile;
use rvt::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let mut categories = vec![
        per::OST_WALLS,
        per::OST_DOORS,
        per::OST_WINDOWS,
        per::OST_FLOORS,
        per::OST_COLUMNS,
        per::OST_ROOMS,
    ];
    categories.extend(
        per::PRODUCT_RECORD_CATEGORIES
            .iter()
            .map(|(category, _)| *category),
    );
    categories.sort_unstable();
    categories.dedup();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let (mut read, mut mismatched) = (0usize, 0usize);
    for record in &records {
        let Ok(inflated) = rf.inflated_partition(&record.stream) else {
            continue;
        };
        let bytes = inflated.bytes();
        let word = |at: usize| {
            bytes
                .get(record.offset + at..record.offset + at + 4)
                .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
        };
        let (Some(flags), Some(class)) = (word(0x46), word(0x4a)) else {
            continue;
        };
        read += 1;
        if class & 0xffff != u32::from(record.class_tag) {
            mismatched += 1;
        }
        println!(
            "{{\"id\":{},\"category\":{},\"class\":{},\"flags\":{},\"placement\":{},\"contained\":{}}}",
            record.element_id,
            record.builtin_category,
            record.class_tag,
            flags,
            record.placement_kind,
            record.container != per::CONTAINER_NONE
        );
    }
    eprintln!(
        "Revit {version}: {} records, {read} read, +0x4a differs from the decoded class on {mismatched}",
        records.len()
    );
    Ok(())
}
