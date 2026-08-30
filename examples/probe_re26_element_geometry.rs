//! Research probe (#215 / #228, RE-26): dump every `OST_Columns` and
//! `OST_Walls` partition element record — instances *and* type-symbol
//! envelopes — with both counted reference lists in full, so the
//! instance → type join and the wall location curve can be scored
//! against a reference IFC export offline.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::BTreeSet;

const CATEGORIES: &[(&str, i64)] = &[
    ("OST_Columns", per::OST_COLUMNS),
    ("OST_Walls", per::OST_WALLS),
    ("OST_Doors", per::OST_DOORS),
    ("OST_Windows", per::OST_WINDOWS),
    ("OST_Floors", per::OST_FLOORS),
    ("OST_BuildingPad", per::OST_BUILDING_PAD),
];

/// Bytes of record body dumped past the second reference list.
const TRAILER_BYTES: usize = 4096;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);

    let elem_records = rvt::elem_table::parse_records(&mut rf)?;
    let declared: BTreeSet<u32> = elem_records.iter().map(|r| r.id_primary).collect();
    let mut elem_rows: std::collections::BTreeMap<String, Vec<String>> =
        std::collections::BTreeMap::new();
    for record in &elem_records {
        elem_rows
            .entry(record.id_primary.to_string())
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
        let concat: Vec<u8> = compression::inflate_all_chunks_for_stream(stream, &raw)
            .into_iter()
            .flatten()
            .collect();
        for (name, category) in CATEGORIES {
            for record in per::find_category_records(stream, &concat, *category, &declared) {
                let at = record.offset + per::REFERENCE_LIST_OFFSET;
                let first = per::decode_reference_list(&concat, at);
                let second_at = first
                    .as_ref()
                    .map(|list| at + 4 + list.len() * 8)
                    .unwrap_or(at);
                let second = first
                    .as_ref()
                    .and_then(|_| per::decode_reference_list(&concat, second_at));
                let trailer_at = second
                    .as_ref()
                    .map(|list| second_at + 4 + list.len() * 8)
                    .unwrap_or(second_at);
                let trailer_end = (trailer_at + TRAILER_BYTES).min(concat.len());
                let trailer = if record.is_exported_instance() {
                    concat.get(trailer_at..trailer_end).unwrap_or(&[])
                } else {
                    &[]
                };
                rows.push(serde_json::json!({
                    "category": name,
                    "stream": stream,
                    "offset": record.offset,
                    "element_id": record.element_id,
                    "flags": record.flags,
                    "container": record.container_element_id(),
                    "placement_kind": record.placement_kind,
                    "is_exported_instance": record.is_exported_instance(),
                    "is_type_symbol": record.is_type_symbol(),
                    "bbox": record.bbox_feet,
                    "prologue_hex": hex(&concat[record.offset..record.offset + per::BBOX_OFFSET]),
                    "refs1": first,
                    "refs2": second,
                    "trailer_at": trailer_at,
                    "trailer_hex": hex(trailer),
                }));
            }
        }
    }

    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "declared_ids": declared.len(),
        "elem_table": elem_rows,
        "records": rows,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
