//! Q6.4 (refined): for each directory-table record body, treat the
//! first u16 as a class tag and look it up in the schema. If records
//! resolve to named classes, the directory is a per-instance class
//! map — telling us the type of each top-level object in Global/Latest.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, formats, object_graph, streams};
use std::collections::HashMap;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: directory_class_lookup <file>");
    let mut rf = RevitFile::open(&path)?;

    let history = object_graph::DocumentHistory::from_revit_file(&mut rf)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;

    // Re-locate post-history boundary.
    let probe = [b'R', 0, b'e', 0, b'v', 0, b'i', 0, b't', 0, b' ', 0];
    let mut last_revit_tail = history.string_section_offset;
    let mut scan = history.string_section_offset;
    loop {
        if scan + probe.len() >= d.len() {
            break;
        }
        let slice_end = (scan + 512).min(d.len());
        match d[scan..slice_end]
            .windows(probe.len())
            .position(|w| w == probe)
        {
            Some(p) => {
                let pos = scan + p;
                let mut end = pos;
                while end + 2 <= d.len() {
                    let c = u16::from_le_bytes([d[end], d[end + 1]]);
                    if c < 0x20 && c != b' ' as u16 {
                        break;
                    }
                    end += 2;
                }
                last_revit_tail = end;
                scan = end + 1;
            }
            None => break,
        }
    }
    let mut entry = last_revit_tail;
    let cap = (entry + 64).min(d.len());
    while entry < cap && (d[entry] == 0 || d[entry] == b'/' || d[entry] == b' ') {
        entry += 1;
    }

    // Build a schema lookup table.
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let tag_to_class: HashMap<u16, &str> = schema
        .classes
        .iter()
        .filter_map(|c| c.tag.map(|t| (t, c.name.as_str())))
        .collect();
    println!("Schema has {} classes with tags", tag_to_class.len());

    // Walk sequential IDs, collecting body-first-u16 values.
    let mut cursor = entry;
    let mut id_offsets: Vec<(u32, usize)> = Vec::new();
    let mut expect: u32 = 1;
    let search_limit = d.len();
    while cursor + 4 <= search_limit && expect < 500 {
        let marker = [
            (expect & 0xff) as u8,
            ((expect >> 8) & 0xff) as u8,
            ((expect >> 16) & 0xff) as u8,
            ((expect >> 24) & 0xff) as u8,
        ];
        let window_end = (cursor + 128).min(search_limit);
        if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
            id_offsets.push((expect, cursor + p));
            cursor = cursor + p + 4;
            expect += 1;
        } else {
            break;
        }
    }
    println!("Found {} sequential records", id_offsets.len());

    let mut hits = 0;
    let mut misses = 0;
    let mut adoc_candidates: Vec<(u32, u16, String)> = Vec::new();
    let mut results: Vec<String> = Vec::new();
    for (idx, (id, off)) in id_offsets.iter().enumerate() {
        let body_start = off + 4;
        let body_end = if idx + 1 < id_offsets.len() {
            id_offsets[idx + 1].1
        } else {
            (body_start + 16).min(d.len())
        };
        if body_start + 2 > body_end {
            continue;
        }
        let u16v = u16::from_le_bytes([d[body_start], d[body_start + 1]]);
        let class = tag_to_class
            .get(&u16v)
            .copied()
            .or_else(|| tag_to_class.get(&(u16v & 0x7fff)).copied());
        match class {
            Some(name) => {
                hits += 1;
                results.push(format!(
                    "  id={id:3} @0x{off:05x} tag=0x{u16v:04x} → {name}"
                ));
                if name.contains("Document") || name.contains("ADocument") {
                    adoc_candidates.push((*id, u16v, name.to_string()));
                }
            }
            None => {
                misses += 1;
                if misses < 6 {
                    results.push(format!(
                        "  id={id:3} @0x{off:05x} tag=0x{u16v:04x} → <no class for this tag>"
                    ));
                }
            }
        }
    }
    println!(
        "Tag resolution: {hits} hits, {misses} misses (of {} total)",
        id_offsets.len()
    );
    println!();
    for line in results.iter().take(50) {
        println!("{line}");
    }
    if results.len() > 50 {
        println!("  ... ({} more records)", results.len() - 50);
    }

    println!();
    println!("ADocument candidates:");
    for (id, tag, name) in &adoc_candidates {
        println!("  id={id} tag=0x{tag:04x} class={name}");
    }
    if adoc_candidates.is_empty() {
        println!("  (none found — ADocument may have null tag, so wouldn't appear here)");
    }
    Ok(())
}
