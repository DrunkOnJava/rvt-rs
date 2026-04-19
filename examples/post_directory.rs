//! Q6.4 probe: find where the sequential-ID directory table stops in
//! Global/Latest, then dump ~128 bytes immediately after. If ADocument
//! follows the directory, its first field (a Pointer) should be
//! recognisable in the post-directory bytes. If something else follows,
//! that structure will teach us what the directory is actually indexing.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, object_graph, streams};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: post_directory <file>");
    let mut rf = RevitFile::open(&path)?;

    let history = object_graph::DocumentHistory::from_revit_file(&mut rf)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;

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

    // Find highest sequential ID by scanning forward looking for the
    // next `[expect_id 00 00 00]` marker within a 128-byte window.
    let mut cursor = entry;
    let mut highest_id = 0u32;
    let mut last_record_start = entry;
    let mut expect: u32 = 1;
    let search_limit = d.len();
    while cursor + 4 <= search_limit && expect < 10_000 {
        let marker = [
            (expect & 0xff) as u8,
            ((expect >> 8) & 0xff) as u8,
            ((expect >> 16) & 0xff) as u8,
            ((expect >> 24) & 0xff) as u8,
        ];
        let window_end = (cursor + 128).min(search_limit);
        if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
            last_record_start = cursor + p;
            highest_id = expect;
            cursor = cursor + p + 4;
            expect += 1;
        } else {
            break;
        }
    }
    println!("entry offset: 0x{entry:x}");
    println!("highest sequential id: {highest_id}");
    println!("last record header offset: 0x{last_record_start:x}");

    // Empirical last-record length: find the NEXT non-trivial-looking
    // record. The record body for the last id is variable, so we need
    // to bound it. Easiest heuristic: dump bytes from
    // `last_record_start` forward and look at what's there.
    let dump_start = last_record_start;
    let dump_end = (dump_start + 256).min(d.len());
    println!(
        "\nBytes from last record ({} bytes, starting at 0x{dump_start:x}):",
        dump_end - dump_start
    );
    for row_start in (dump_start..dump_end).step_by(16) {
        let row_end = (row_start + 16).min(dump_end);
        print!("  0x{row_start:06x}  ");
        for i in row_start..row_end {
            print!("{:02x} ", d[i]);
        }
        for _ in row_end..row_start + 16 {
            print!("   ");
        }
        print!(" |");
        for i in row_start..row_end {
            let b = d[i];
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

    // Also: scan for occurrences of well-known wire patterns that mark
    // the start of a class instance. Candidates:
    //   - UTF-16LE string pattern (length u32 followed by printable ascii)
    //   - UUID/GUID shape (16 bytes that look randomy distributed)
    //   - Class tag (u16) values seen in schema
    println!("\n--- Nearby printable-ascii windows (likely strings) ---");
    let search_from = entry;
    let search_to = d.len();
    let mut found = 0;
    for i in search_from..search_to.saturating_sub(32) {
        let len_le_u16 = u16::from_le_bytes([d[i], d[i + 1]]) as usize;
        if (3..=80).contains(&len_le_u16) && i + 2 + len_le_u16 <= d.len() {
            let slice = &d[i + 2..i + 2 + len_le_u16];
            if slice.iter().all(|b| b.is_ascii_graphic() || *b == b' ')
                && slice.iter().filter(|b| b.is_ascii_alphabetic()).count() >= 3
            {
                println!(
                    "  0x{i:06x} u16-len={len_le_u16:3} ASCII: {:?}",
                    std::str::from_utf8(slice).unwrap_or("?")
                );
                found += 1;
                if found >= 10 {
                    break;
                }
            }
        }
    }
    Ok(())
}
