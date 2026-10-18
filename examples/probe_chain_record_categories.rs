//! The category each partition chain record carries, whether or not it has
//! the bounding-box frame the element-record scan needs (#763).
//!
//! An element-record frame (RE-21) carries its `BuiltInCategory` at `+0x12`
//! and the release's bounding-box marker at `+0x50`; the category scan finds
//! records by that marker. A first-prologue frame starts its chain record
//! (RE-35), so every chain record whose `+0x12` holds an id in the
//! `BuiltInCategory` band is counted here by category, with how many carry
//! the marker at `+0x50` and how many do not.
//!
//! On the BIM4LCA office, Revit's export has 30 spaces (gross floor area,
//! heated net area, offices) whose GlobalIds name declared elements with no
//! element record in any category the scan reads, rooms and areas included
//! (`probe_global_id_categories`). Their ElementIds are listed below; the
//! probe dumps each one's chain records: stream, offset, size, the value at
//! `+0x12`, the marker slot and the first 0x60 bytes.
//!
//! Usage:
//!   cargo run --profile ci --example probe_chain_record_categories -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    builtin_category_name, file_bbox_marker, partition_record_chain,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// The 30 office spaces' ElementIds (as-built file; #763).
const WANTED: [u32; 30] = [
    1488004, 1488121, 1488166, 1488221, 2669650, 2670181, 2670211, 2670242, 2670863, 2670893,
    2670990, 2691046, 2695148, 2696623, 2696626, 2698022, 2698025, 2698028, 2698031, 2698369,
    2698373, 2698376, 2698379, 2797827, 2800550, 2800592, 2800633, 2800674, 2801814, 3201003,
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = file_bbox_marker(&mut rf, version) else {
        println!("release {version}: no record marker");
        return Ok(());
    };
    let mut by_category: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    let mut chain_records = 0usize;
    let mut dumps: Vec<String> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in partition_record_chain(buf, &marker) {
            chain_records += 1;
            let category = buf
                .get(span.start + CATEGORY_OFFSET..span.start + CATEGORY_OFFSET + 8)
                .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")));
            let marked = buf
                .get(span.start + BBOX_MARKER_OFFSET..span.start + BBOX_MARKER_OFFSET + 8)
                == Some(&marker[..]);
            if let Some(category) =
                category.filter(|c| (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(c))
            {
                let slot = by_category.entry(category).or_default();
                if marked {
                    slot.0 += 1;
                } else {
                    slot.1 += 1;
                }
            }
            if u32::try_from(span.element_id).is_ok_and(|id| WANTED.contains(&id)) {
                let head = &buf[span.start..(span.start + 0x60).min(buf.len())];
                dumps.push(format!(
                    "  {} {stream} {:#x} size {} at+0x12 {} marker {} | {}",
                    span.element_id,
                    span.start,
                    span.end - span.start,
                    category.map_or("-".to_string(), |c| format!(
                        "{c} ({})",
                        builtin_category_name(c).unwrap_or("?")
                    )),
                    if marked { "yes" } else { "no" },
                    hex(head)
                ));
            }
        }
    }
    println!("release {version}: {chain_records} chain records");
    println!("chain records by the category at +0x12: with the marker at +0x50 / without");
    for (category, (marked, unmarked)) in &by_category {
        println!(
            "  {category} {}: {marked} / {unmarked}",
            builtin_category_name(*category).unwrap_or("?")
        );
    }
    println!(
        "records of the {} wanted ids: {}",
        WANTED.len(),
        dumps.len()
    );
    for line in &dumps {
        println!("{line}");
    }
    Ok(())
}
