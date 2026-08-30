//! Research probe (#212 / RE-22): dump every `OST_Floors` and
//! `OST_BuildingPad` partition element record with a wide body slice,
//! and search the inflated partitions for the strings Revit's IFC
//! export-override parameter would have to carry, so the
//! `IfcSlab` vs `IfcShadingDevice` split can be tested offline.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

const CATEGORIES: &[(&str, i64)] = &[
    ("OST_Floors", -2_000_032),
    ("OST_BuildingPad", -2_001_263),
    ("OST_Roofs", -2_000_035),
    ("OST_Ceilings", -2_000_038),
    ("OST_StructuralFoundation", -2_001_300),
];

/// Strings an "IFC Export As" instance override would have to name.
const NEEDLES: &[&str] = &[
    "IfcExportAs",
    "IfcShadingDevice",
    "ShadingDevice",
    "IfcSlab",
    "IfcExportType",
];

const BODY_BYTES: usize = 1024;

/// Bytes either side of an export-override string hit to dump.
const CONTEXT_BYTES: usize = 3072;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn find_all(haystack: &[u8], needle: &[u8]) -> Vec<usize> {
    let mut out = Vec::new();
    if needle.is_empty() || haystack.len() < needle.len() {
        return out;
    }
    for start in 0..=(haystack.len() - needle.len()) {
        if &haystack[start..start + needle.len()] == needle {
            out.push(start);
        }
    }
    out
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
    let mut needle_hits: BTreeMap<String, Vec<serde_json::Value>> = BTreeMap::new();
    let mut shading_context: Vec<serde_json::Value> = Vec::new();
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();

        for needle in NEEDLES {
            let ascii: Vec<usize> = find_all(&concat, needle.as_bytes());
            let utf16: Vec<u8> = needle.encode_utf16().flat_map(u16::to_le_bytes).collect();
            let wide: Vec<usize> = find_all(&concat, &utf16);
            if !ascii.is_empty() || !wide.is_empty() {
                needle_hits
                    .entry((*needle).to_string())
                    .or_default()
                    .push(serde_json::json!({
                        "stream": stream,
                        "ascii_offsets": ascii,
                        "utf16_offsets": wide,
                    }));
            }
        }

        // Context around every "IfcShadingDevice" hit: the surrounding
        // window, plus every declared ElementId that appears as a u32
        // inside it, so the string can be tested for an element join.
        let wide: Vec<u8> = "IfcShadingDevice"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        for hit in find_all(&concat, &wide) {
            let lo = hit.saturating_sub(CONTEXT_BYTES);
            let hi = (hit + CONTEXT_BYTES).min(concat.len());
            let mut ids: Vec<(usize, u32)> = Vec::new();
            let mut cursor = lo;
            while cursor + 4 <= hi {
                let value = u32::from_le_bytes(concat[cursor..cursor + 4].try_into().unwrap());
                if value != 0 && declared.contains(&value) {
                    ids.push((cursor as isize as usize, value));
                }
                cursor += 1;
            }
            shading_context.push(serde_json::json!({
                "stream": stream,
                "offset": hit,
                "window_lo": lo,
                "declared_ids_in_window": ids
                    .iter()
                    .map(|(o, v)| serde_json::json!({"delta": *o as i64 - hit as i64, "id": v}))
                    .collect::<Vec<_>>(),
                "context_hex": hex(&concat[lo..hi]),
            }));
        }

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
                rows.push(serde_json::json!({
                    "category": name,
                    "builtin_category": record.builtin_category,
                    "stream": stream,
                    "offset": record.offset,
                    "element_id": record.element_id,
                    "flags": record.flags,
                    "container": record.container,
                    "placement_kind": record.placement_kind,
                    "exported_instance": record.is_exported_instance(),
                    "bbox": record.bbox_feet,
                    "prologue_hex": hex(prologue),
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
        "needle_hits": needle_hits,
        "shading_context": shading_context,
        "records": rows,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
