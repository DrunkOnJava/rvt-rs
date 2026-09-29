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
    // Where each MEP type's name is stored relative to its ElementId: the
    // types Revit exports for RE1's pipes and ducts, by Tag.
    let types: [(&str, u64); 5] = [
        ("221116-WTR", 191_045),
        ("233113-DUCT-Tees", 53_292),
        ("SANWST-PVC", 53_456),
        ("221316-SAN", 53_456),
        ("WTR-Copper B", 191_061),
    ];
    for (name, id) in types {
        let units: Vec<u8> = name.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let id_bytes = id.to_le_bytes();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, &units) {
                let from = at.saturating_sub(0x4000);
                let to = (at + 0x4000).min(buf.len());
                let near: Vec<i64> = memchr::memmem::find_iter(&buf[from..to], &id_bytes)
                    .map(|i| (from + i) as i64 - at as i64)
                    .collect();
                let before: Vec<String> = buf[at.saturating_sub(40)..at]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                println!(
                    "{{\"type_name\":{name:?},\"type\":{id},\"stream\":{stream:?},\"at\":{at},\"id_offsets\":{near:?},\"before\":{:?}}}",
                    before.join(" ")
                );
            }
        }
    }
    eprintln!("Revit {version}: {} records", records.len());
    Ok(())
}
