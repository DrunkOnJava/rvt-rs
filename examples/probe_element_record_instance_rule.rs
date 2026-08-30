//! Research probe (#211 / #216): dump every partition element record for a
//! set of `BuiltInCategory` ids together with its full 88-byte prologue, a
//! slice of the record body, and the file's `Global/ElemTable` row for the
//! same ElementId, so an instance-selection rule can be derived offline.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

const CATEGORIES: &[(&str, i64)] = &[
    ("OST_Walls", -2_000_011),
    ("OST_Doors", -2_000_023),
    ("OST_Windows", -2_000_014),
    ("OST_Columns", -2_000_100),
    ("OST_Floors", -2_000_032),
];

const BODY_BYTES: usize = 256;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);

    let elem_records = rvt::elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<u32> = elem_records.iter().map(|r| r.id_primary).collect();
    let mut elem_raw: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for record in &elem_records {
        elem_raw
            .entry(record.id_primary)
            .or_default()
            .push(hex(&record.raw));
    }

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut rows: Vec<serde_json::Value> = Vec::new();
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        for (name, category) in CATEGORIES {
            for record in per::find_category_records(stream, &concat, *category, &declared) {
                let start = record.offset;
                let prologue = &concat[start..(start + per::BBOX_OFFSET).min(concat.len())];
                let body_start = start + per::RECORD_MIN_LEN;
                let body_end = (body_start + BODY_BYTES).min(concat.len());
                let body = if body_start < concat.len() {
                    &concat[body_start..body_end]
                } else {
                    &[][..]
                };
                let pre_start = start.saturating_sub(32);
                rows.push(serde_json::json!({
                    "category": name,
                    "builtin_category": record.builtin_category,
                    "stream": stream,
                    "offset": record.offset,
                    "element_id": record.element_id,
                    "flags": record.flags,
                    "bbox": record.bbox_feet,
                    "family_local": record.is_family_local(),
                    "prologue_hex": hex(prologue),
                    "pre_hex": hex(&concat[pre_start..start]),
                    "body_hex": hex(body),
                    "elem_table_raw": elem_raw.get(&record.element_id).cloned().unwrap_or_default(),
                }));
            }
        }
    }

    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "declared_ids": declared.len(),
        "records": rows,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
