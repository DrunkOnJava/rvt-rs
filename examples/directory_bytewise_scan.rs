//! Q6.4 (third-pass): byte-level scan for directory u16 body values
//! across every other decompressed stream in the file. Bypasses the
//! rough-record parsers (which are inconsistent across releases) and
//! asks the direct question: where in the file do these u16 values
//! actually live?
//!
//! Hypothesis: if the directory is indexing something, its u16 values
//! should appear as 16-bit little-endian quantities in the target
//! stream's decompressed bytes at a rate much higher than uniform-
//! random (2^-16 ≈ 0.0015%). Previously, Phase D showed class tags
//! from `Formats/Latest` occur in `Global/Latest` at 340× random rate,
//! confirming schema-as-type-dictionary — the same test here tells us
//! whether the directory values are ElementIds, class tags, or
//! something else entirely.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, object_graph, streams};
use std::collections::HashSet;

fn find_directory_u16s(d: &[u8], entry: usize) -> Vec<(u32, u16)> {
    let mut cursor = entry;
    let mut id_offsets: Vec<(u32, usize)> = Vec::new();
    let mut expect: u32 = 1;
    while cursor + 4 <= d.len() && expect < 500 {
        let marker = [
            (expect & 0xff) as u8,
            ((expect >> 8) & 0xff) as u8,
            ((expect >> 16) & 0xff) as u8,
            ((expect >> 24) & 0xff) as u8,
        ];
        let window_end = (cursor + 128).min(d.len());
        if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
            id_offsets.push((expect, cursor + p));
            cursor = cursor + p + 4;
            expect += 1;
        } else {
            break;
        }
    }
    let mut out = Vec::new();
    for (idx, (id, off)) in id_offsets.iter().enumerate() {
        if *id == 1 {
            continue;
        }
        let body_start = off + 4;
        let body_end = if idx + 1 < id_offsets.len() {
            id_offsets[idx + 1].1
        } else {
            (body_start + 16).min(d.len())
        };
        if body_end - body_start >= 2 {
            out.push((*id, u16::from_le_bytes([d[body_start], d[body_start + 1]])));
        }
    }
    out
}

fn count_u16_in(bytes: &[u8], needle: u16) -> usize {
    let a = (needle & 0xff) as u8;
    let b = (needle >> 8) as u8;
    bytes.windows(2).filter(|w| w[0] == a && w[1] == b).count()
}

fn expected_uniform_rate(len: usize) -> f64 {
    // For a u16 value scanned 2-byte-windowed over `len` bytes (sliding
    // window, not aligned), expected count under uniform-random is
    // (len - 1) / 2^16.
    (len.saturating_sub(1) as f64) / 65536.0
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: directory_bytewise_scan <file>");
    let mut rf = RevitFile::open(&path)?;

    // 1. Extract directory u16s from Global/Latest.
    let history = object_graph::DocumentHistory::from_revit_file(&mut rf)?;
    let raw_gl = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d_gl = compression::inflate_at(&raw_gl, 8)?;
    let probe = [b'R', 0, b'e', 0, b'v', 0, b'i', 0, b't', 0, b' ', 0];
    let mut last_tail = history.string_section_offset;
    let mut scan = history.string_section_offset;
    loop {
        if scan + probe.len() >= d_gl.len() {
            break;
        }
        let slice_end = (scan + 512).min(d_gl.len());
        match d_gl[scan..slice_end]
            .windows(probe.len())
            .position(|w| w == probe)
        {
            Some(p) => {
                let pos = scan + p;
                let mut end = pos;
                while end + 2 <= d_gl.len() {
                    let c = u16::from_le_bytes([d_gl[end], d_gl[end + 1]]);
                    if c < 0x20 && c != b' ' as u16 {
                        break;
                    }
                    end += 2;
                }
                last_tail = end;
                scan = end + 1;
            }
            None => break,
        }
    }
    let mut entry = last_tail;
    let cap = (entry + 64).min(d_gl.len());
    while entry < cap && (d_gl[entry] == 0 || d_gl[entry] == b'/' || d_gl[entry] == b' ') {
        entry += 1;
    }
    let dir = find_directory_u16s(&d_gl, entry);
    let unique: HashSet<u16> = dir.iter().map(|(_, v)| *v).collect();
    println!(
        "Directory: {} records with u16 bodies, {} unique values",
        dir.len(),
        unique.len()
    );

    // 2. Load every other decompressed stream and count u16 hits.
    let target_streams: Vec<&str> = vec![
        streams::GLOBAL_ELEM_TABLE,
        streams::GLOBAL_CONTENT_DOCUMENTS,
        streams::GLOBAL_DOC_INCREMENT_TABLE,
        streams::GLOBAL_HISTORY,
        streams::GLOBAL_PARTITION_TABLE,
        streams::FORMATS_LATEST,
    ];
    println!();
    println!(
        "{:32} {:>10} {:>10} {:>8} {:>10} {:>8}",
        "stream", "decomp_B", "dir_hits", "uniq/130", "expected", "ratio"
    );
    println!("{}", "-".repeat(80));
    for stream_name in &target_streams {
        let raw = match rf.read_stream(stream_name) {
            Ok(v) => v,
            Err(_) => {
                println!("{stream_name:32} <missing>");
                continue;
            }
        };
        let decompressed =
            compression::inflate_at(&raw, 8).or_else(|_| compression::inflate_at(&raw, 0));
        let Ok(d) = decompressed else {
            println!("{stream_name:32} <inflate failed>");
            continue;
        };

        let mut total_hits: usize = 0;
        let mut unique_hits: usize = 0;
        for v in &unique {
            let c = count_u16_in(&d, *v);
            total_hits += c;
            if c > 0 {
                unique_hits += 1;
            }
        }
        let expected = expected_uniform_rate(d.len()) * unique.len() as f64;
        let ratio = if expected > 0.0 {
            total_hits as f64 / expected
        } else {
            0.0
        };
        println!(
            "{:32} {:>10} {:>10} {:>5}/{} {:>10.2} {:>7.1}×",
            stream_name,
            d.len(),
            total_hits,
            unique_hits,
            unique.len(),
            expected,
            ratio
        );
    }

    // 3. Also check against Global/Latest itself (excluding the
    //    directory table) for self-references.
    let post_directory_start = {
        // Rough: find the end of the sequential ID run
        let mut cursor = entry;
        let mut expect: u32 = 1;
        let mut last_end = entry;
        while cursor + 4 <= d_gl.len() && expect < 500 {
            let marker = [
                (expect & 0xff) as u8,
                ((expect >> 8) & 0xff) as u8,
                ((expect >> 16) & 0xff) as u8,
                ((expect >> 24) & 0xff) as u8,
            ];
            let window_end = (cursor + 128).min(d_gl.len());
            if let Some(p) = d_gl[cursor..window_end]
                .windows(4)
                .position(|w| w == marker)
            {
                last_end = cursor + p + 4;
                cursor = last_end;
                expect += 1;
            } else {
                break;
            }
        }
        last_end
    };
    let post = &d_gl[post_directory_start..];
    let mut total_hits: usize = 0;
    let mut unique_hits: usize = 0;
    for v in &unique {
        let c = count_u16_in(post, *v);
        total_hits += c;
        if c > 0 {
            unique_hits += 1;
        }
    }
    let expected = expected_uniform_rate(post.len()) * unique.len() as f64;
    let ratio = if expected > 0.0 {
        total_hits as f64 / expected
    } else {
        0.0
    };
    println!(
        "{:32} {:>10} {:>10} {:>5}/{} {:>10.2} {:>7.1}×",
        "Global/Latest (post-directory)",
        post.len(),
        total_hits,
        unique_hits,
        unique.len(),
        expected,
        ratio
    );

    // 4. Summary
    let total_dir_usage = post.len();
    println!();
    println!("Interpretation guide:");
    println!("  ratio ≈ 1× → directory values are uniform-random (not references)");
    println!("  ratio ≈ 10× → plausibly referenced; worth investigating");
    println!("  ratio ≥ 100× → strong evidence these are references INTO that stream");
    println!(
        "  (for comparison: Phase D found class tags from Formats/Latest appear in\n   \
         Global/Latest at ~340× random — the moat-break finding.)"
    );
    let _ = total_dir_usage;
    Ok(())
}
