//! Q6.3 probe: validate the 0.6-confidence hypothesis from Q6.2 that
//! the sequential-ID TLV block starting at the post-history boundary
//! is ADocument's instance, with each record keyed by 1-indexed
//! field-number.
//!
//! Strategy: walk forward from the post-history boundary scanning for
//! `[u32 id]` where id = 1, 2, 3, ... (sequential). For each record
//! found, compute the body length as
//! `next_id_offset minus this_id_offset minus 4`. Cross-reference
//! against ADocument.fields[id-1] and dump the body bytes alongside
//! the declared FieldType.
//!
//! Key output columns:
//!
//! ```text
//! ID  | field name                       | FieldType              | body_len | body hex              | u16 val | u32 val
//! ```
//!
//! If the hypothesis holds, the body-length column should correlate
//! with the FieldType column (pointers have one width, element-id-
//! refs another, containers yet another). If body-length is random,
//! the hypothesis is refuted and we're looking at something else.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, formats, object_graph, streams};
use streams::GLOBAL_LATEST;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: adocument_walk <file>");
    let mut rf = RevitFile::open(&path)?;

    let history = object_graph::DocumentHistory::from_revit_file(&mut rf)?;
    let raw = rf.read_stream(GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;

    // Re-find the post-history boundary using the same heuristic as
    // examples/adocument_entry.rs so results are comparable.
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
    println!("entry offset: 0x{entry:x}");

    // Fetch ADocument schema for cross-reference.
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| anyhow::anyhow!("ADocument not in schema"))?;
    println!("ADocument declared fields: {}", adoc.fields.len());

    // Scan ahead looking for sequential [u32 id] markers id=1..N within
    // a bounded window. We stop when we fail to find id=N for some N.
    let max_ids: u32 = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let _ = adoc.fields.len(); // schema retained only for annotation
    let search_limit = (entry + 16384).min(d.len());
    let mut id_offsets: Vec<(u32, usize)> = Vec::new();
    let mut expect_id: u32 = 1;
    let mut cursor = entry;
    while expect_id <= max_ids && cursor + 4 <= search_limit {
        let marker = [
            (expect_id & 0xff) as u8,
            ((expect_id >> 8) & 0xff) as u8,
            ((expect_id >> 16) & 0xff) as u8,
            ((expect_id >> 24) & 0xff) as u8,
        ];
        let window_end = (cursor + 64).min(search_limit);
        if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
            let pos = cursor + p;
            id_offsets.push((expect_id, pos));
            cursor = pos + 4;
            expect_id += 1;
        } else {
            println!(
                "  (stop) could not find id={expect_id} within {}B of cursor 0x{cursor:x}",
                window_end - cursor
            );
            break;
        }
    }
    println!("Found {} sequential IDs", id_offsets.len());
    println!();

    // For each record, compute body = bytes between this_id_offset+4
    // and next_id_offset (or end-of-window for the last one).
    println!(
        "{:<4} | {:<32} | {:<48} | {:>4} | body hex                                               | u16        | u32",
        "ID", "field name", "FieldType", "blen"
    );
    println!("{}", "-".repeat(160));
    for (idx, (id, off)) in id_offsets.iter().enumerate() {
        let body_start = off + 4;
        let body_end = if idx + 1 < id_offsets.len() {
            id_offsets[idx + 1].1
        } else {
            (body_start + 16).min(d.len())
        };
        let blen = body_end - body_start;
        let body = &d[body_start..body_end];
        let hex_row: Vec<String> = body.iter().take(16).map(|b| format!("{b:02x}")).collect();
        let hex_str = hex_row.join(" ");

        let fi = (*id as usize).wrapping_sub(1);
        let field_desc = if fi < adoc.fields.len() {
            let f = &adoc.fields[fi];
            let ft = f
                .field_type
                .as_ref()
                .map(|ft| format!("{ft:?}"))
                .unwrap_or_else(|| "None".into());
            // Shorten very long FieldType debugs (e.g. Container body bytes)
            let ft_short = if ft.len() > 46 {
                format!("{}...", &ft[..43])
            } else {
                ft
            };
            (f.name.clone(), ft_short)
        } else {
            (String::from("<beyond declared field count>"), String::new())
        };

        let u16_val = if body.len() >= 2 {
            format!("0x{:04x}", u16::from_le_bytes([body[0], body[1]]))
        } else {
            String::from("-")
        };
        let u32_val = if body.len() >= 4 {
            format!(
                "0x{:08x}",
                u32::from_le_bytes([body[0], body[1], body[2], body[3]])
            )
        } else {
            String::from("-")
        };

        println!(
            "{:<4} | {:<32} | {:<48} | {:>4} | {:<54} | {:<10} | {}",
            id, field_desc.0, field_desc.1, blen, hex_str, u16_val, u32_val
        );
    }

    // Summary: body-length distribution
    let mut blen_hist: std::collections::BTreeMap<usize, u32> = std::collections::BTreeMap::new();
    for (idx, (_, off)) in id_offsets.iter().enumerate() {
        let body_start = off + 4;
        let body_end = if idx + 1 < id_offsets.len() {
            id_offsets[idx + 1].1
        } else {
            break; // last record has no next marker, skip
        };
        *blen_hist.entry(body_end - body_start).or_insert(0) += 1;
    }
    println!();
    println!("Body-length distribution:");
    for (len, count) in &blen_hist {
        println!("  {len} bytes × {count}");
    }

    Ok(())
}
