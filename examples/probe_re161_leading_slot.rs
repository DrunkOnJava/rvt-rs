//! RE-161 (probe): what the first slot of an element record's reference list
//! is (B39, #228).
//!
//! RE-155 found the first slot is 3 on 79% of Core Interior's records and on
//! most RE1 MEP records, and never on RE1 Architecture's, whose first slot is
//! a Level. This probe prints the file's worksharing state and the first
//! slot's values, and looks for a parameter that holds the same value: for
//! every record whose frame decodes, every BuiltInParameter id (-1,000,000
//! to -2,000,000) in the element's own verified data objects (RE-153) whose
//! next `u32` or `u64` equals the first slot. It prints the ids that do so
//! on the most records, with how many records carry each id at all.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re161_leading_slot -- MODEL.rvt ...

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

/// Parameter ids printed per model.
const SHOWN: usize = 12;

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn i64_at(b: &[u8], at: usize) -> Option<i64> {
    Some(i64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut s) = (1u32, 0u32);
    for chunk in data.chunks(5552) {
        for &x in chunk {
            a += u32::from(x);
            s += a;
        }
        a %= 65521;
        s %= 65521;
    }
    (s << 16) | a
}

/// Every verified data object in `b` (RE-153), by ElementId: `(start, end)`.
fn objects(b: &[u8]) -> BTreeMap<u32, Vec<(usize, usize)>> {
    let mut out: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
    let mut p = 0;
    while p + 20 <= b.len() {
        if u32_at(b, p + 4) == Some(0) {
            if let (Some(id), Some(sum), Some(size), Some(class)) = (
                u32_at(b, p),
                u32_at(b, p + 8),
                u32_at(b, p + 12),
                u32_at(b, p + 16),
            ) {
                let size = size as usize;
                let end = p + 20 + size;
                if size >= 4 && end <= b.len() && u32_at(b, end - 4) == Some(size as u32) {
                    let mut data = class.to_le_bytes().to_vec();
                    data.extend_from_slice(&b[p + 20..end]);
                    data.truncate(size);
                    if adler32(&data) == sum {
                        out.entry(id).or_default().push((p, end));
                    }
                }
            }
        }
        p += 1;
    }
    out
}

fn probe(path: &str) -> anyhow::Result<Vec<String>> {
    let mut rf = RevitFile::open(path)?;
    let info = rf.basic_file_info()?;
    let revit = info.version;
    let worksharing = info.worksharing().map(str::to_string);
    let Some(marker) = per::file_bbox_marker(&mut rf, revit) else {
        return Ok(vec![format!(
            "{{\"file\":{path:?},\"skipped\":\"no bbox marker\"}}"
        )]);
    };
    let mut first_values: BTreeMap<u64, usize> = BTreeMap::new();
    // parameter id -> (records whose value equals the first slot, records
    // carrying the id)
    let mut agree: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    let mut records = 0usize;
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let objs = objects(buf);
        for span in per::partition_record_chain(buf, &marker) {
            if !seen.insert(span.element_id) {
                continue;
            }
            let Ok(own) = u32::try_from(span.element_id) else {
                continue;
            };
            let Some(frame) = per::decode_frame_as(&stream, buf, span.start, own, &marker) else {
                continue;
            };
            let Some(&first) = frame.references.first() else {
                continue;
            };
            records += 1;
            *first_values.entry(first).or_default() += 1;
            let mut carried: BTreeMap<i64, bool> = BTreeMap::new();
            for (start, end) in objs.get(&own).into_iter().flatten() {
                for at in start + 20..end.saturating_sub(16) {
                    let Some(id) = i64_at(buf, at) else { continue };
                    if !(-2_000_000..=-1_000_000).contains(&id) {
                        continue;
                    }
                    let equal = u32_at(buf, at + 8).map(u64::from) == Some(first)
                        || i64_at(buf, at + 8).map(|v| v as u64) == Some(first);
                    let slot = carried.entry(id).or_default();
                    *slot |= equal;
                }
            }
            for (id, equal) in carried {
                let slot = agree.entry(id).or_default();
                slot.1 += 1;
                slot.0 += usize::from(equal);
            }
        }
    }
    let mut firsts: Vec<(&u64, &usize)> = first_values.iter().collect();
    firsts.sort_by_key(|(_, n)| std::cmp::Reverse(**n));
    let firsts: Vec<String> = firsts
        .iter()
        .take(8)
        .map(|(v, n)| format!("\"{v}\":{n}"))
        .collect();
    let mut best: Vec<(&i64, &(usize, usize))> = agree.iter().filter(|(_, (a, _))| *a > 0).collect();
    best.sort_by_key(|(_, (a, _))| std::cmp::Reverse(*a));
    let best: Vec<String> = best
        .iter()
        .take(SHOWN)
        .map(|(id, (a, n))| format!("\"{id}\":\"{a}/{n}\""))
        .collect();
    Ok(vec![format!(
        "{{\"file\":{path:?},\"revit\":{revit},\"worksharing\":{worksharing:?},\"records\":{records},\
         \"first_slot_values\":{{{}}},\"parameters_equal_to_first_slot\":{{{}}}}}",
        firsts.join(","),
        best.join(",")
    )])
}

fn main() {
    // Measure passes flags such as `--records` after the paths.
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        match probe(path) {
            Ok(lines) => lines.iter().for_each(|l| println!("{l}")),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
