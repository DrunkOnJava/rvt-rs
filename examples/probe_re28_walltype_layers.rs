//! Research probe (#88 / #34, RE-28): the Revit wall *type* record
//! and the `OST_Materials` record family on `2024_Core_Interior.rvt`.
//!
//! Two modes, both over the inflated `Partitions/*` streams:
//!
//! ```text
//! probe_re28_walltype_layers <file.rvt> ids <id>[,<id>...]
//! probe_re28_walltype_layers <file.rvt> category <builtin-category>
//! ```
//!
//! `ids` dumps every offset whose leading `u64` is a requested
//! ElementId and whose `+0x0c` word is the `0x0000059f` every Revit
//! 2024 record prologue carries.
//!
//! `ids` additionally runs the #88 layer sweep over a
//! [`LAYER_SWEEP_BYTES`]-wide span each side of every hit: it reports
//! every `f64` in `(0, 3]` feet and every run of consecutive `f64`
//! summing to a requested nominal wall thickness, so "no compound
//! layer lives near the wall type record" is a measurement rather
//! than an impression.
//!
//! `category` dumps every record of the given `BuiltInCategory` that
//! carries the bbox-less record marker
//! [`rvt::partition_level_records::RECORD_MARKER`] at `+0x50`, with
//! the counted `u64` slot list at `+0x56` and a trailing window.
//!
//! Both modes also emit every RE-24-shaped name block in the file, so
//! the record ids can be joined to display names offline.
//!
//! Not part of the shipped decode path.

use rvt::partition_element_records as per;
use rvt::partition_level_records as plr;
use rvt::{RevitFile, compression};
use std::collections::BTreeSet;

/// Bytes dumped after a record's prologue.
const WINDOW: usize = 2048;
/// Word every Revit 2024 record prologue carries at `+0x0c`.
const PROLOGUE_MAGIC: u32 = 0x0000_059f;
/// Offset of the counted `u64` slot list on a bbox-less record.
const SLOT_COUNT_OFFSET: usize = 0x56;
/// Largest slot count the probe will follow.
const SLOT_MAX: usize = 4096;
/// Bytes swept each side of a hit for compound-layer candidates.
const LAYER_SWEEP_BYTES: usize = 65_536;
/// Nominal wall thicknesses on `2024_Core_Interior.rvt`, in feet:
/// 6", 8" and 18" — the `IfcWallType` widths RE-26 matched on 360 of
/// 360 walls, and the sums any real layer list has to reproduce.
const NOMINAL_THICKNESS_FEET: &[f64] = &[0.5, 2.0 / 3.0, 1.5];
/// Longest layer run the sweep will try.
const LAYER_RUN_MAX: usize = 12;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn slots_at(buf: &[u8], offset: usize) -> Option<Vec<u64>> {
    let count = read_u32(buf, offset + SLOT_COUNT_OFFSET)? as usize;
    if count == 0 || count > SLOT_MAX {
        return None;
    }
    let start = offset + SLOT_COUNT_OFFSET + 4;
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        out.push(read_u64(buf, start + index * 8)?);
    }
    Some(out)
}

fn record_json(stream: &str, buf: &[u8], at: usize) -> serde_json::Value {
    let end = (at + WINDOW).min(buf.len());
    serde_json::json!({
        "stream": stream,
        "at": at,
        "element_id": read_u64(buf, at),
        "flags": read_u32(buf, at + 0x08),
        "category": read_u64(buf, at + per::CATEGORY_OFFSET).map(|v| v as i64),
        "container": read_u64(buf, at + per::CONTAINER_OFFSET),
        "placement_kind": read_u32(buf, at + per::PLACEMENT_KIND_OFFSET),
        "has_bbox_marker": buf
            .get(at + per::BBOX_MARKER_OFFSET..at + per::BBOX_MARKER_OFFSET + 8)
            .map(|s| s == per::BBOX_MARKER),
        "slots": slots_at(buf, at),
        "window_hex": hex(&buf[at..end]),
    })
}

/// The #88 compound-layer sweep over one byte span.
///
/// Reports every `f64` that could be a layer thickness, every `f64`
/// that equals a nominal wall width exactly, and every run of
/// consecutive `f64` whose sum equals one. `base` is the span's
/// offset relative to the hit, so the reported positions are signed
/// distances from the record.
fn layer_sweep(span: &[u8], base: i64) -> serde_json::Value {
    let read =
        |at: usize| -> f64 { f64::from_le_bytes(span[at..at + 8].try_into().expect("8 bytes")) };
    let plausible = |v: f64| v.is_finite() && v > 1e-5 && v <= 3.0;

    let mut candidates = 0usize;
    let mut exact: Vec<serde_json::Value> = Vec::new();
    let mut runs: Vec<serde_json::Value> = Vec::new();
    if span.len() < 8 {
        return serde_json::json!({
            "span_bytes": span.len(),
            "layer_candidate_f64": 0,
            "nominal_exact_f64": exact,
            "runs_summing_to_nominal": runs,
        });
    }
    for at in 0..span.len() - 8 {
        let value = read(at);
        if !plausible(value) {
            continue;
        }
        candidates += 1;
        if let Some(nominal) = NOMINAL_THICKNESS_FEET
            .iter()
            .find(|n| (value - **n).abs() < 1e-9)
        {
            exact.push(serde_json::json!({ "at": base + at as i64, "feet": nominal }));
        }
    }
    // Runs are only meaningful 8-byte aligned relative to the record.
    let mut at = (-base).rem_euclid(8) as usize;
    while at + 16 <= span.len() {
        let mut sum = 0.0f64;
        for count in 1..=LAYER_RUN_MAX {
            let end = at + count * 8;
            if end > span.len() {
                break;
            }
            let value = read(at + (count - 1) * 8);
            if !plausible(value) {
                break;
            }
            sum += value;
            if count >= 2
                && NOMINAL_THICKNESS_FEET
                    .iter()
                    .any(|n| (sum - *n).abs() < 1e-9)
            {
                runs.push(serde_json::json!({
                    "at": base + at as i64,
                    "layers": count,
                    "sum_feet": sum,
                }));
            }
        }
        at += 8;
    }
    serde_json::json!({
        "span_bytes": span.len(),
        "layer_candidate_f64": candidates,
        "nominal_exact_f64": exact,
        "runs_summing_to_nominal": runs,
    })
}

/// Every RE-24-shaped `(owner ElementId, name)` block in `buf`, with
/// the elevation requirement dropped so the shape can be measured on
/// categories other than `OST_Levels`.
fn find_owner_named_blocks(buf: &[u8], declared: &BTreeSet<u32>) -> Vec<(u32, String, usize)> {
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < buf.len() {
        if buf[index] != 0xff {
            index += 1;
            continue;
        }
        let run_start = index;
        while index < buf.len() && buf[index] == 0xff {
            index += 1;
        }
        if index - run_start != plr::OWNER_SENTINEL_RUN_LEN {
            continue;
        }
        let Some(value) = run_start.checked_add(plr::OWNER_OFFSET_BEFORE_NAME - 8) else {
            continue;
        };
        let Some(owner_at) = run_start.checked_sub(8) else {
            continue;
        };
        let Some(pad) = buf.get(index..value - plr::LENGTH_PREFIX_OFFSET_BEFORE_NAME) else {
            continue;
        };
        if !pad.iter().all(|byte| *byte == 0) {
            continue;
        }
        let Some(owner) = read_u64(buf, owner_at) else {
            continue;
        };
        if owner == 0 || owner > u64::from(u32::MAX) || !declared.contains(&(owner as u32)) {
            continue;
        }
        let Some(chars) = read_u32(buf, value - plr::LENGTH_PREFIX_OFFSET_BEFORE_NAME) else {
            continue;
        };
        let chars = chars as usize;
        if chars == 0 || chars > plr::MAX_NAME_CHARS {
            continue;
        }
        let Some(raw) = buf.get(value..value + chars * 2) else {
            continue;
        };
        let units: Vec<u16> = raw
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(name) = String::from_utf16(&units) else {
            continue;
        };
        if name.chars().any(|c| c.is_control()) {
            continue;
        }
        out.push((owner as u32, name, value));
    }
    out
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt> …");
    let mode = std::env::args().nth(2).unwrap_or_else(|| "ids".into());
    let arg = std::env::args().nth(3).unwrap_or_default();

    let ids: BTreeSet<u32> = if mode == "ids" {
        arg.split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse().expect("element id"))
            .collect()
    } else {
        BTreeSet::new()
    };
    let category: i64 = if mode == "category" {
        arg.trim().parse().expect("builtin category")
    } else {
        0
    };

    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .iter()
        .map(|r| r.id_primary)
        .collect();

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();

    let mut records: Vec<serde_json::Value> = Vec::new();
    let mut names: Vec<serde_json::Value> = Vec::new();

    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks_for_stream(stream, &raw)
            .into_iter()
            .flatten()
            .collect();

        if mode == "ids" {
            for id in &ids {
                let needle = u64::from(*id).to_le_bytes();
                let mut cursor = 0usize;
                while let Some(found) = memchr::memmem::find(&concat[cursor..], &needle) {
                    let at = cursor + found;
                    cursor = at + 1;
                    if read_u32(&concat, at + 0x0c) == Some(PROLOGUE_MAGIC) {
                        let mut row = record_json(stream, &concat, at);
                        let from = at.saturating_sub(LAYER_SWEEP_BYTES);
                        let to = (at + LAYER_SWEEP_BYTES).min(concat.len());
                        row["layer_sweep"] =
                            layer_sweep(&concat[from..to], from as i64 - at as i64);
                        records.push(row);
                    }
                }
            }
        } else if mode == "string" {
            let needle: Vec<u8> = arg.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
            let mut cursor = 0usize;
            while let Some(found) = memchr::memmem::find(&concat[cursor..], &needle) {
                let at = cursor + found;
                cursor = at + 1;
                let from = at.saturating_sub(512);
                let to = (at + 512).min(concat.len());
                records.push(serde_json::json!({
                    "stream": stream,
                    "at": at,
                    "window_from": from,
                    "window_hex": hex(&concat[from..to]),
                }));
            }
        } else {
            let needle = (category as u64).to_le_bytes();
            let mut cursor = 0usize;
            while let Some(found) = memchr::memmem::find(&concat[cursor..], &needle) {
                let hit = cursor + found;
                cursor = hit + 1;
                if hit < per::CATEGORY_OFFSET {
                    continue;
                }
                let at = hit - per::CATEGORY_OFFSET;
                if read_u32(&concat, at + 0x0c) != Some(PROLOGUE_MAGIC) {
                    continue;
                }
                let Some(raw_id) = read_u64(&concat, at) else {
                    continue;
                };
                if raw_id == 0 || raw_id > u64::from(u32::MAX) {
                    continue;
                }
                if !declared.contains(&(raw_id as u32)) {
                    continue;
                }
                let marker =
                    concat.get(at + plr::RECORD_MARKER_OFFSET..at + plr::RECORD_MARKER_OFFSET + 6);
                if marker != Some(&plr::RECORD_MARKER[..]) {
                    continue;
                }
                records.push(record_json(stream, &concat, at));
            }
        }

        for (element_id, name, at) in find_owner_named_blocks(&concat, &declared) {
            names.push(serde_json::json!({
                "stream": stream,
                "element_id": element_id,
                "name": name,
                "name_at": at,
            }));
        }
    }

    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "mode": mode,
        "requested_ids": ids,
        "category": category,
        "records": records,
        "name_blocks": names,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
