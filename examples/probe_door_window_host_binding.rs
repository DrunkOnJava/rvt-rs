//! Research probe (#222): decode the ElementId reference list that follows
//! every partition element record and report it for doors, windows and walls.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4")))
}

fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    buf.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8")))
}

/// Decode `(count, entries)` of the reference list at `at`.
fn ref_list(buf: &[u8], at: usize, max: usize) -> Option<(u32, Vec<u64>)> {
    let count = read_u32(buf, at)?;
    if count == 0 || count as usize > max {
        return None;
    }
    let mut out = Vec::with_capacity(count as usize);
    for index in 0..count as usize {
        out.push(read_u64(buf, at + 4 + index * 8)?);
    }
    Some((count, out))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;

    let elem_records = rvt::elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<u32> = elem_records.iter().map(|r| r.id_primary).collect();

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut inflated: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        inflated.insert(stream.clone(), chunks.into_iter().flatten().collect());
    }

    const CATEGORIES: &[(&str, i64)] = &[
        ("wall", per::OST_WALLS),
        ("door", per::OST_DOORS),
        ("window", per::OST_WINDOWS),
        ("column", per::OST_COLUMNS),
        ("floor", per::OST_FLOORS),
    ];

    let mut rows: Vec<serde_json::Value> = Vec::new();
    for (stream, buf) in &inflated {
        for (kind, category) in CATEGORIES {
            for record in per::find_category_records(stream, buf, *category, &declared) {
                let at = record.offset + per::RECORD_MIN_LEN;
                let (count, entries) = ref_list(buf, at, 4096).unwrap_or((0, Vec::new()));
                rows.push(serde_json::json!({
                    "kind": kind,
                    "stream": stream,
                    "offset": record.offset,
                    "element_id": record.element_id,
                    "flags": record.flags,
                    "instance": record.is_exported_instance(),
                    "container": record.container,
                    "placement_kind": record.placement_kind,
                    "bbox": record.bbox_feet,
                    "count": count,
                    "entries": entries,
                }));
            }
        }
    }

    println!(
        "{}",
        serde_json::to_string(&serde_json::json!({
            "file": path,
            "declared_ids": declared.len(),
            "records": rows,
        }))?
    );
    Ok(())
}
