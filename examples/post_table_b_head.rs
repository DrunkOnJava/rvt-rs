//! Q6.5 candidate: dump the first 256 bytes of the post-Table-B
//! region and annotate any u16 values that resolve to schema class
//! tags. If ADocument's instance data starts right after Table B,
//! we expect to see a recognisable class-tag pattern in the first
//! handful of bytes — most likely a "parent document" class like
//! ADocument (though ADocument itself has `tag: null`, so we'll be
//! looking for structural markers that open the root record).

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, formats, streams};
use std::collections::HashMap;

fn find_table_b_end(d: &[u8]) -> usize {
    let mut last_end = 0usize;
    let mut i = 0;
    while i + 4 < d.len() {
        if d[i..i + 4] == [1, 0, 0, 0] {
            let mut cursor = i + 4;
            let mut expect: u32 = 2;
            let mut end = i + 4;
            while cursor + 4 <= d.len() {
                let marker = [
                    (expect & 0xff) as u8,
                    ((expect >> 8) & 0xff) as u8,
                    ((expect >> 16) & 0xff) as u8,
                    ((expect >> 24) & 0xff) as u8,
                ];
                let window_end = (cursor + 64).min(d.len());
                if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
                    end = cursor + p + 4;
                    cursor = end;
                    expect += 1;
                } else {
                    break;
                }
            }
            let records = expect - 1;
            if records >= 5 {
                last_end = end + 32;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    last_end
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: post_table_b_head <file>");
    let mut rf = RevitFile::open(&path)?;
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let tag_to_class: HashMap<u16, &str> = schema
        .classes
        .iter()
        .filter_map(|c| c.tag.map(|t| (t, c.name.as_str())))
        .collect();

    let raw_gl = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw_gl, 8)?;
    let cutoff = find_table_b_end(&d);
    println!("Post-Table-B region starts at 0x{cutoff:06x}");
    println!();

    // Hex dump + class-tag annotation.
    let dump_len = 384.min(d.len() - cutoff);
    for row_start in (0..dump_len).step_by(16) {
        let row_end = (row_start + 16).min(dump_len);
        print!("  0x{:06x}  ", cutoff + row_start);
        for k in row_start..row_end {
            print!("{:02x} ", d[cutoff + k]);
        }
        for _ in row_end..row_start + 16 {
            print!("   ");
        }
        print!(" |");
        for k in row_start..row_end {
            let b = d[cutoff + k];
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

    // Find tagged-class hits in this window and list them.
    println!();
    println!("--- Class-tag hits in the first {dump_len} bytes ---");
    for i in 0..dump_len.saturating_sub(1) {
        let v = u16::from_le_bytes([d[cutoff + i], d[cutoff + i + 1]]);
        if let Some(&name) = tag_to_class.get(&v) {
            println!(
                "  @offset +0x{i:04x} (abs 0x{:06x}): u16=0x{v:04x} → class `{name}`",
                cutoff + i
            );
        }
        // Also check u16 with 0x8000 flag set (top-level tagged)
        let v_flagged = v | 0x8000;
        if v_flagged != v {
            if let Some(&name) = tag_to_class.get(&(v_flagged & 0x7fff)) {
                let _ = name;
            }
        }
    }
    Ok(())
}
