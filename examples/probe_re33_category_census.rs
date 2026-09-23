//! Census of partition element records by `BuiltInCategory` (RE-33).
//!
//! Anchors on the release's bbox marker at record `+0x50`, decodes every
//! frame whose `+0x00` ElementId is declared in `Global/ElemTable`, and
//! prints, per category, how many frames decode and how many pass the
//! RE-21 instance rule (no container reference, placed). With `--ids` it
//! also prints the exported-instance ElementIds per category as JSON, so
//! they can be compared with the `Tag` sets of Revit's own IFC export of
//! the same file.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re33_category_census -- FILE.rvt [--ids]

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{bbox_marker, decode_at_with_marker};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const BBOX_MARKER_OFFSET: usize = 0x50;

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = PathBuf::from(args.next().expect("usage: FILE.rvt [--ids]"));
    let print_ids = args.any(|a| a == "--ids");

    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known bbox marker");
        return Ok(());
    };
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

    let mut frames: BTreeMap<i64, usize> = BTreeMap::new();
    let mut instances: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut at = 0;
        while let Some(hit) = buf[at..].windows(8).position(|w| w == marker) {
            let pos = at + hit;
            at = pos + 1;
            let Some(offset) = pos.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(record) = decode_at_with_marker(&stream, buf, offset, &declared, &marker)
            else {
                continue;
            };
            *frames.entry(record.builtin_category).or_default() += 1;
            if record.is_exported_instance() {
                instances
                    .entry(record.builtin_category)
                    .or_default()
                    .insert(record.element_id);
            }
        }
    }

    // Second-prologue frames (RE-30): the marker and category are in
    // place but `+0x00` holds no ElementId. Test whether the RE-21
    // instance fields (container at +0x32, placement at +0x42) read the
    // same way on them.
    let mut unattributed: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut at = 0;
        while let Some(hit) = buf[at..].windows(8).position(|w| w == marker) {
            let pos = at + hit;
            at = pos + 1;
            let Some(offset) = pos.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let word = |o: usize| {
                buf.get(offset + o..offset + o + 8)
                    .map(|b| u64::from_le_bytes(b.try_into().expect("8")))
            };
            let (Some(id), Some(category), Some(container), Some(placement)) =
                (word(0), word(0x12), word(0x32), word(0x42))
            else {
                continue;
            };
            if id != 0 && id <= u64::from(u32::MAX) {
                continue;
            }
            let entry = unattributed.entry(category as i64).or_default();
            entry.0 += 1;
            if container == u64::MAX && (placement & 0xffff_ffff) as u32 == 0xffff_ef7f {
                entry.1 += 1;
            }
        }
    }

    println!("release {version}: {} declared ElementIds", declared.len());
    println!("{:>12} {:>7} {:>9}", "category", "frames", "instances");
    for (category, n) in &frames {
        let k = instances.get(category).map_or(0, BTreeSet::len);
        println!("{category:>12} {n:>7} {k:>9}");
    }
    println!("unattributed frames: category total instance-like");
    for (category, (total, placed)) in &unattributed {
        if *category >= -2_100_000 && *category <= -1_990_000 {
            println!("{category:>12} {total:>7} {placed:>9}");
        }
    }
    if print_ids {
        let json: BTreeMap<String, Vec<u32>> = instances
            .iter()
            .map(|(c, ids)| (c.to_string(), ids.iter().copied().collect()))
            .collect();
        println!("{}", serde_json::to_string(&json).expect("json"));
    }
    Ok(())
}
