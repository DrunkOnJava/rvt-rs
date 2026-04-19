//! Phase 4c probe — dump 128 bytes at each tagged-class definition in
//! Formats/Latest AND at the first occurrence of that tag in Global/Latest,
//! side-by-side, so the class-record framing + instance-record framing
//! become visible together.
//!
//! Static-only: no execution, no network, no disk writes beyond stdout.
#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use rvt::{
    RevitFile, compression,
    streams::{FORMATS_LATEST, GLOBAL_LATEST},
};

fn looks_like_class_name(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes[0].is_ascii_uppercase()
        && bytes[1..]
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn dump_hex(label: &str, bytes: &[u8], offset: usize, len: usize) {
    println!("{label}  @ 0x{offset:x}");
    let end = (offset + len).min(bytes.len());
    for row_start in (offset..end).step_by(16) {
        let row_end = (row_start + 16).min(end);
        print!("  0x{row_start:06x}  ");
        for i in row_start..row_end {
            print!("{:02x} ", bytes[i]);
        }
        for _ in row_end..row_start + 16 {
            print!("   ");
        }
        print!(" |");
        for i in row_start..row_end {
            let b = bytes[i];
            print!(
                "{}",
                if (0x20..0x7f).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            );
        }
        println!("|");
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: record_framing <file.rfa>");
    let mut rf = RevitFile::open(&path)?;

    // Parse Formats/Latest → list of (name, tag, offset_of_name_header)
    let formats_raw = rf.read_stream(FORMATS_LATEST)?;
    let formats = compression::inflate_at(&formats_raw, 0)?;
    let scan_limit = (64 * 1024).min(formats.len());
    let f = &formats[..scan_limit];

    let mut tagged: Vec<(String, u16, usize)> = Vec::new();
    let mut i = 0;
    while i + 2 < f.len() {
        let len = u16::from_le_bytes([f[i], f[i + 1]]) as usize;
        if !(3..=60).contains(&len) || i + 2 + len + 2 > f.len() {
            i += 1;
            continue;
        }
        let name = &f[i + 2..i + 2 + len];
        if !looks_like_class_name(name) {
            i += 1;
            continue;
        }
        let after = i + 2 + len;
        let u16v = u16::from_le_bytes([f[after], f[after + 1]]);
        if u16v & 0x8000 != 0 {
            let n = std::str::from_utf8(name).unwrap().to_string();
            tagged.push((n, u16v & 0x7fff, i));
        }
        i += 2 + len;
    }
    tagged.sort_by_key(|t| t.1);
    tagged.dedup_by(|a, b| a.0 == b.0);
    println!("Parsed {} tagged classes from Formats/Latest", tagged.len());
    println!();

    // Decompress Global/Latest once
    let global_raw = rf.read_stream(GLOBAL_LATEST)?;
    let global = compression::inflate_at(&global_raw, 8)?;
    println!("Global/Latest: {} bytes decompressed", global.len());
    println!();

    // Inspect 5 tagged classes — top of distribution + one simple (ADocWarnings)
    let targets = [
        "AbsCurveGStep",
        "HostObjAttr",
        "AbsDbViewPressureLossReport",
        "ADocWarnings",
        "ATFProvenanceBaseCell",
    ];
    for target_name in targets {
        let (name, tag, off) = match tagged.iter().find(|t| t.0 == target_name) {
            Some(t) => t.clone(),
            None => {
                println!("(skip) {target_name} not tagged in this file");
                continue;
            }
        };
        let name_len = name.len();
        println!("═══════════════════════════════════════════════════════════════════════");
        println!("  Class '{name}'  tag=0x{tag:04x} ({tag})");
        println!("═══════════════════════════════════════════════════════════════════════");

        // Schema-side: class definition
        dump_hex("\n  [A] Schema record (Formats/Latest)", &formats, off, 128);

        // Also annotate: where name starts, where tag lives, what's immediately after tag
        let name_start = off + 2;
        let name_end = name_start + name_len;
        let tag_at = name_end;
        let post_tag = tag_at + 2;
        println!(
            "\n      name @ 0x{name_start:x}..0x{name_end:x} (len {name_len}), tag @ 0x{tag_at:x}, first post-tag byte @ 0x{post_tag:x}"
        );
        if post_tag + 8 <= formats.len() {
            let u16_a = u16::from_le_bytes([formats[post_tag], formats[post_tag + 1]]);
            let u16_b = u16::from_le_bytes([formats[post_tag + 2], formats[post_tag + 3]]);
            let u32_a = u32::from_le_bytes([
                formats[post_tag],
                formats[post_tag + 1],
                formats[post_tag + 2],
                formats[post_tag + 3],
            ]);
            let u32_b = u32::from_le_bytes([
                formats[post_tag + 4],
                formats[post_tag + 5],
                formats[post_tag + 6],
                formats[post_tag + 7],
            ]);
            println!(
                "      post-tag: u16={u16_a} ({u16_a:#x}), u16+2={u16_b} ({u16_b:#x}), u32={u32_a} ({u32_a:#x}), u32+4={u32_b} ({u32_b:#x})"
            );
        }

        // Instance-side: first occurrence of this tag in Global/Latest
        let mut found = None;
        for idx in 0..global.len().saturating_sub(2) {
            let v = u16::from_le_bytes([global[idx], global[idx + 1]]);
            if v == tag {
                // Heuristic filter: tag bytes preceded by something that looks
                // like a record header. For now, take the first match and dump.
                found = Some(idx);
                break;
            }
        }
        if let Some(idx) = found {
            let start = idx.saturating_sub(16);
            dump_hex(
                "\n  [B] First instance? (Global/Latest)",
                &global,
                start,
                128,
            );
            println!(
                "\n      (showing 16 bytes pre-context; tag @ 0x{idx:x} = offset {idx} in decompressed Global/Latest)"
            );
        } else {
            println!("\n  (no tag=0x{tag:04x} occurrence found in Global/Latest)");
        }
        println!();
    }
    Ok(())
}
