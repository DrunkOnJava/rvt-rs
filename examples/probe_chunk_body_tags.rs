//! RE-11 first pass — look inside partition chunk bodies for u16 LE
//! values that match schema class tags. Goal: identify whether chunks
//! begin with (or contain near-start) a class-tag header that lets us
//! classify them as Wall / Floor / Door / etc.
//!
//! Approach:
//!   1. Parse Formats/Latest to get the full schema → class_tag map.
//!   2. For each partition chunk on Einhoven Partitions/0, scan the
//!      first 256 bytes for u16 LE values matching any known class
//!      tag, and record which ones hit.
//!   3. Also report the top-N most-frequent u16 values in chunk-head
//!      bytes, in case the wire tag encoding is shifted or offset-
//!      encoded.

use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).unwrap();

    // Parse schema → get full class → tag map.
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    let formats_d = compression::inflate_at(&formats_raw, 0).unwrap();
    let schema = formats::parse_schema(&formats_d).unwrap();
    let tag_to_name: BTreeMap<u16, String> = schema
        .classes
        .iter()
        .filter_map(|c| c.tag.map(|t| (t, c.name.clone())))
        .collect();
    println!(
        "Schema: {} classes total, {} with tags",
        schema.classes.len(),
        tag_to_name.len()
    );

    // Tags we specifically care about.
    let interesting: &[&str] = &[
        "Wall",
        "WallType",
        "Floor",
        "FloorType",
        "Door",
        "Window",
        "Stair",
        "Column",
        "Beam",
        "Roof",
        "Ceiling",
        "HostObj",
        "HostObjAttr",
        "Level",
        "Grid",
        "FamilyInstance",
    ];
    let interesting_tags: Vec<(u16, &str)> = schema
        .classes
        .iter()
        .filter_map(|c| {
            c.tag
                .and_then(|t| interesting.iter().find(|n| **n == c.name).map(|n| (t, *n)))
        })
        .collect();
    println!("\nTags of interest:");
    for (t, n) in &interesting_tags {
        println!("  0x{t:04x} = {n}");
    }

    // Now examine partition chunks.
    for stream in ["Partitions/0", "Partitions/5"] {
        let raw = rf.read_stream(stream).unwrap();
        let chunks = compression::inflate_all_chunks(&raw);
        println!(
            "\n=== {stream}: {} chunks — tag hits in first 256 B of body (skipping 16-B header) ===",
            chunks.len()
        );

        for (i, chunk) in chunks.iter().enumerate() {
            if chunk.len() < 48 {
                continue;
            }
            let body = &chunk[16..chunk.len().min(272)]; // 16-B hdr, then first 256 B of body

            // Find all u16 LE positions in body that match an interesting tag.
            let mut hits = Vec::new();
            for off in 0..body.len().saturating_sub(1) {
                let v = u16::from_le_bytes([body[off], body[off + 1]]);
                if let Some((_, name)) = interesting_tags.iter().find(|(t, _)| *t == v) {
                    hits.push((off, v, *name));
                }
            }
            // Also check all schema tags (not just interesting subset) for first 32 B.
            let mut any_tag_hits = 0usize;
            for off in 0..body.len().min(32).saturating_sub(1) {
                let v = u16::from_le_bytes([body[off], body[off + 1]]);
                if tag_to_name.contains_key(&v) {
                    any_tag_hits += 1;
                }
            }
            if !hits.is_empty() || any_tag_hits > 0 {
                println!(
                    "  chunk[{i:2}] size={:>6} B: {} tag-hits in first 32B | {} specific-class hits: {:?}",
                    chunk.len(),
                    any_tag_hits,
                    hits.len(),
                    hits.iter()
                        .take(5)
                        .map(|(off, _, n)| format!("+{off}={n}"))
                        .collect::<Vec<_>>()
                );
            }
        }
    }
}
