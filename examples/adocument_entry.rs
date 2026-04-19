//! Q6.2: find the ADocument singleton's entry point in Global/Latest.
//!
//! Strategy: the document-upgrade-history UTF-16LE block sits at the top.
//! After the final history entry + its terminator, the binary payload
//! begins. If the first record there corresponds to ADocument's 13
//! declared fields, we've found the entry point.
#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use rvt::{RevitFile, compression, formats, object_graph, streams};
use streams::GLOBAL_LATEST;

fn dump_hex_at(bytes: &[u8], offset: usize, len: usize) {
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
        .expect("usage: adocument_entry <file>");
    let mut rf = RevitFile::open(&path)?;

    // Find where the history-string section ends.
    let history = object_graph::DocumentHistory::from_revit_file(&mut rf)?;
    println!(
        "Document upgrade history: {} entries",
        history.entries.len()
    );
    println!(
        "  string section begins at 0x{:x}",
        history.string_section_offset
    );

    let raw = rf.read_stream(GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    println!("  Global/Latest decompressed: {} bytes", d.len());

    // Dump hex around the end of the history section — the binary
    // payload should start right after the last UTF-16LE entry.
    // History entries end with "/" or " " padding followed by 0x00 0x00.
    // We'll scan for two consecutive 0x00 bytes followed by non-zero
    // data, past the last history entry's ASCII.
    // Bound to the FIRST contiguous "Revit "-prefixed block. Stop as
    // soon as we don't find another "Revit " within the next 512 bytes.
    let search_start = history.string_section_offset;
    let probe = [b'R', 0, b'e', 0, b'v', 0, b'i', 0, b't', 0, b' ', 0];
    let mut last_revit_tail = search_start;
    let mut scan = search_start;
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
    let mut p = last_revit_tail;
    let cap = (p + 64).min(d.len());
    while p < cap && (d[p] == 0 || d[p] == b'/' || d[p] == b' ') {
        p += 1;
    }
    println!("  history block ends at 0x{last_revit_tail:x}");
    println!("  first post-history byte at 0x{:x} (0x{:02x})", p, d[p]);

    println!("\nHex dump 64 bytes after history section:");
    dump_hex_at(&d, p, 128);

    // Now look for ADocument's declared schema and see if we can match
    // its expected layout against the bytes at `p`.
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema.classes.iter().find(|c| c.name == "ADocument");
    if let Some(adoc) = adoc {
        println!(
            "\nADocument schema: @0x{:x}, {} fields declared",
            adoc.offset,
            adoc.fields.len()
        );
        for f in adoc.fields.iter().take(13) {
            let ft = f
                .field_type
                .as_ref()
                .map(|ft| format!("{ft:?}"))
                .unwrap_or("None".into());
            println!("  {} :: {}", f.name, ft);
        }

        // Try to sum expected byte-size using FieldType
        let mut expected_size: usize = 0;
        let mut unknown_count = 0;
        for f in &adoc.fields {
            if let Some(ft) = &f.field_type {
                let sz = match ft {
                    formats::FieldType::Primitive { size, .. } => *size as usize,
                    formats::FieldType::ElementId => 8, // guess: 8 bytes for ElementId
                    formats::FieldType::Pointer { .. } => 4, // guess: 4 bytes for a pointer
                    formats::FieldType::Guid => 16,
                    formats::FieldType::String => 0, // variable
                    formats::FieldType::Vector { .. } => 0, // variable
                    formats::FieldType::Container { .. } => 0, // variable
                    formats::FieldType::Unknown { .. } => {
                        unknown_count += 1;
                        0
                    }
                };
                expected_size += sz;
            }
        }
        println!(
            "\nADocument expected fixed-size byte footprint (ignoring variable-size fields and {unknown_count} Unknowns): {expected_size} bytes"
        );
    }

    Ok(())
}
