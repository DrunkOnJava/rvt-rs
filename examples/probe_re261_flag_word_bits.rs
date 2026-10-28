//! #223, RE-261: the element record's flag word at `+0x46` beside the
//! record's class at `+0x4a`, for every first-prologue element record of a
//! file, and a raw count of room records the category scan does not decode.
//!
//! FACTS under test (puzzbobb's table on #223, 2026-10-26):
//! - bit 7 (`0x80`) is a `WallSweep` whose type is unset; RE-128 measured it
//!   on every type record instead.
//! - bit 12 (`0x1000`) on a wall is "an end is joined" and on a room is
//!   "placed"; `tools/re/flag_word_vs_ifc.py` checks both against Revit's
//!   export (`IfcRelConnectsPathElements`, `IfcSpace`).
//! - `scan_category_records_multi` misses room records with an empty
//!   computed block.
//!
//! Output:
//!   rec <id> <BuiltInCategory> <class> <+0x46 as hex> <+0x42 placement kind> <container>
//!   room-frame <id> <class> <marker at +0x50: yes|no> <decoded: yes|no> <+0x46 as hex>
//!   bit7 <class> <records with 0x80> <records>
//!
//! Usage:
//!   cargo run --profile ci --example probe_re261_flag_word_bits -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_element_records as per;

const FLAG_OFFSET: usize = 0x46;
const CLASS_OFFSET: usize = 0x4a;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(at..at + 2)?.try_into().ok()?))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let schema = rf.schema_classes()?;
    let class_name = |tag: u16| {
        schema
            .by_tag(tag)
            .map_or_else(|| format!("tag{tag}"), |c| c.name.clone())
    };
    let Some(marker) = per::file_bbox_marker(&mut rf, version) else {
        eprintln!("no bbox marker for Revit {version}");
        return Ok(());
    };
    let room_tag = schema
        .classes
        .iter()
        .find(|c| c.name == "RoomElem")
        .map(|c| c.tag);

    let mut seen = BTreeSet::new();
    let mut bit7: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut decoded_rooms = BTreeSet::new();
    let mut room_frames = Vec::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(record) = per::decode_at_with_marker(&stream, buf, offset, &declared, &marker)
            else {
                continue;
            };
            if !seen.insert(record.element_id) {
                continue;
            }
            let (Some(flags), Some(tag)) = (
                u32_at(buf, offset + FLAG_OFFSET),
                u16_at(buf, offset + CLASS_OFFSET),
            ) else {
                continue;
            };
            let class = class_name(tag);
            let entry = bit7.entry(class.clone()).or_default();
            entry.1 += 1;
            if flags & 0x80 != 0 {
                entry.0 += 1;
            }
            if record.builtin_category == per::OST_ROOMS {
                decoded_rooms.insert(record.element_id);
            }
            println!(
                "rec {} {} {} {:#010x} {:#010x} {:#x}",
                record.element_id,
                record.builtin_category,
                class,
                flags,
                record.placement_kind,
                record.container
            );
        }
        let needle = (per::OST_ROOMS as u64).to_le_bytes();
        for hit in memchr::memmem::find_iter(buf, &needle) {
            let Some(offset) = hit.checked_sub(per::CATEGORY_OFFSET) else {
                continue;
            };
            let (Some(raw_id), Some(flags), Some(tag)) = (
                u64_at(buf, offset),
                u32_at(buf, offset + FLAG_OFFSET),
                u16_at(buf, offset + CLASS_OFFSET),
            ) else {
                continue;
            };
            if raw_id > u64::from(u32::MAX) || !declared.contains(&(raw_id as u32)) {
                continue;
            }
            if Some(tag) != room_tag {
                continue;
            }
            let has_marker = buf
                .get(offset + per::BBOX_MARKER_OFFSET..offset + per::BBOX_MARKER_OFFSET + 8)
                == Some(&marker[..]);
            room_frames.push((raw_id as u32, class_name(tag), has_marker, flags));
        }
    }
    room_frames.sort();
    room_frames.dedup_by_key(|f| f.0);
    for (id, class, has_marker, flags) in &room_frames {
        println!(
            "room-frame {id} {class} {} {} {flags:#010x}",
            if *has_marker { "yes" } else { "no" },
            if decoded_rooms.contains(id) {
                "yes"
            } else {
                "no"
            }
        );
    }
    let scanned = per::scan_category_records_multi(&mut rf, version, &[per::OST_ROOMS], &declared)?;
    let scanned_ids: BTreeSet<u32> = scanned.iter().map(|r| r.element_id).collect();
    let missed = room_frames
        .iter()
        .filter(|f| !scanned_ids.contains(&f.0))
        .count();
    for (class, (set, total)) in &bit7 {
        println!("bit7 {class} {set} {total}");
    }
    eprintln!(
        "{} records; RoomElem frames {}, scan_category_records_multi rooms {}, frames it misses {missed}",
        seen.len(),
        room_frames.len(),
        scanned_ids.len()
    );
    Ok(())
}
