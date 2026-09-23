//! RE-43: some element frames hold the invalid ElementId at `+0x00`.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 samples, 181 (architectural)
//! and 8 (structural) placed-instance frames hold `u32::MAX` at `+0x00`,
//! the 32-bit form of Revit's invalid ElementId −1. Every one sits inside a
//! partition chain record (RE-35), which carries the element's id. On the
//! architectural sample they are 99 sketch lines, 35 lines, 31 railing path
//! extension lines, 7 room separation lines and stair run 1644693, which
//! Revit's own IFC4 export holds. No other file in the corpus has one.
//!
//! The probe prints, per category, the placed-instance frames that hold
//! `u32::MAX` and whether each sits inside a chain record, with the record
//! ids of the first few.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re43_invalid_id_frames -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, CATEGORY_OFFSET, CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE,
    PLACEMENT_KIND_OFFSET, bbox_marker, enclosing_record, partition_record_chain,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known record marker");
        return Ok(());
    };
    // category -> (inside a chain record, outside, first record ids)
    let mut by_category: BTreeMap<i64, (usize, usize, Vec<u64>)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = partition_record_chain(buf, &marker);
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            if u64_at(buf, offset) != Some(u64::from(u32::MAX)) {
                continue;
            }
            let placed = u64_at(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && u64_at(buf, offset + PLACEMENT_KIND_OFFSET)
                    .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE);
            let Some(category) = u64_at(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
                continue;
            };
            if !placed {
                continue;
            }
            let entry = by_category.entry(category).or_default();
            match enclosing_record(&chain, offset) {
                Some(span) => {
                    entry.0 += 1;
                    if entry.2.len() < 4 {
                        entry.2.push(span.element_id);
                    }
                }
                None => entry.1 += 1,
            }
        }
    }
    println!("release {version}: placed frames holding u32::MAX at +0x00");
    for (category, (inside, outside, ids)) in by_category {
        println!(
            "  {category:>9}  in a chain record {inside:>5}, outside {outside:>3}  e.g. {ids:?}"
        );
    }
    Ok(())
}
