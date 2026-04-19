//! Q6.4 (second-pass probe): does the Global/Latest post-history
//! directory-table's u16 body values correspond to ElementIds in
//! Global/ElemTable?
//!
//! Motivation: the first record of the directory (id=1) has a 12-byte
//! body that matches ElemTable's 3×u32 record shape exactly
//! (`00 00 00 00 01 00 00 00 dc 00 00 00`). If the rest of the
//! directory is compact references into ElemTable, the u16 body values
//! should resolve against ElemTable's record IDs with high hit-rate.
//!
//! Output: per-record hit/miss against both (a) ElemTable record IDs
//! (u16 first field of each `[u32][u32][u32]` triple) and (b) the same
//! IDs with high-bit masking. Also prints ElemTable size + record count
//! for comparison with directory size.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, elem_table, object_graph, streams};
use std::collections::HashSet;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: directory_vs_elemtable <file>");
    let mut rf = RevitFile::open(&path)?;

    // 1. Enumerate the directory-table records (reusing the scan logic
    //    from examples/adocument_walk.rs).
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
    println!(
        "Directory at 0x{entry:x}: {} sequential records",
        id_offsets.len()
    );

    // Collect the u16 body values of each record (skipping id=1 which is
    // the 12-byte-body header-like record).
    let mut dir_u16s: Vec<(u32, u16)> = Vec::new();
    for (idx, (id, off)) in id_offsets.iter().enumerate() {
        if *id == 1 {
            continue; // header-ish record, handle separately
        }
        let body_start = off + 4;
        let body_end = if idx + 1 < id_offsets.len() {
            id_offsets[idx + 1].1
        } else {
            (body_start + 16).min(d.len())
        };
        if body_end - body_start < 2 {
            continue;
        }
        let v = u16::from_le_bytes([d[body_start], d[body_start + 1]]);
        dir_u16s.push((*id, v));
    }
    println!(
        "Directory u16 body values to cross-check: {}",
        dir_u16s.len()
    );

    // 2. Pull ElemTable and enumerate its records.
    let header = elem_table::parse_header(&mut rf)?;
    println!(
        "\nGlobal/ElemTable header: element_count={} record_count={} decomp_bytes={}",
        header.element_count, header.record_count, header.decompressed_bytes
    );

    let records = elem_table::parse_records_rough(&mut rf, 20_000)?;
    println!("ElemTable records parsed: {}", records.len());

    // Collect the first u32 of each ElemTable record (as both u32 and
    // truncated u16 — the directory values are u16, ElemTable is u32).
    let mut et_first_u32: HashSet<u32> = HashSet::new();
    let mut et_first_u16: HashSet<u16> = HashSet::new();
    let mut et_any_u16: HashSet<u16> = HashSet::new();
    for r in &records {
        et_first_u32.insert(r.presumptive_u32_triple[0]);
        et_first_u16.insert(r.presumptive_u32_triple[0] as u16);
        for v in r.presumptive_u32_triple {
            et_any_u16.insert(v as u16);
            et_any_u16.insert((v >> 16) as u16);
        }
    }
    println!(
        "ElemTable: {} distinct first-u32, {} distinct first-u16, {} distinct u16-chunks (any field)",
        et_first_u32.len(),
        et_first_u16.len(),
        et_any_u16.len()
    );

    // 3. Cross-reference.
    let mut hit_first_u16 = 0;
    let mut hit_any_u16 = 0;
    let mut misses: Vec<(u32, u16)> = Vec::new();
    for (id, v) in &dir_u16s {
        let h1 = et_first_u16.contains(v);
        let h2 = et_any_u16.contains(v);
        if h1 {
            hit_first_u16 += 1;
        }
        if h2 {
            hit_any_u16 += 1;
        }
        if !h2 {
            misses.push((*id, *v));
        }
    }
    let n = dir_u16s.len() as f64;
    println!("\n--- Cross-reference results ---");
    println!(
        "  Directory u16 ∈ ElemTable first-u16 as u16: {} / {} ({:.1}%)",
        hit_first_u16,
        dir_u16s.len(),
        100.0 * hit_first_u16 as f64 / n
    );
    println!(
        "  Directory u16 ∈ any u16-chunk of any ElemTable record: {} / {} ({:.1}%)",
        hit_any_u16,
        dir_u16s.len(),
        100.0 * hit_any_u16 as f64 / n
    );
    if !misses.is_empty() && misses.len() <= 20 {
        println!("\n  Misses (not in ElemTable any-u16):");
        for (id, v) in misses.iter().take(20) {
            println!("    id={id:3}  u16=0x{v:04x}");
        }
    } else if !misses.is_empty() {
        println!("\n  Misses: {} total — showing first 10", misses.len());
        for (id, v) in misses.iter().take(10) {
            println!("    id={id:3}  u16=0x{v:04x}");
        }
    }

    // 4. Also check the special id=1 record's 12-byte body: does it
    //    match ANY ElemTable record exactly?
    if let Some((_, off1)) = id_offsets.first() {
        let body = &d[*off1 + 4..*off1 + 16];
        let u32a = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let u32b = u32::from_le_bytes([body[4], body[5], body[6], body[7]]);
        let u32c = u32::from_le_bytes([body[8], body[9], body[10], body[11]]);
        println!("\nid=1 12-byte body as u32 triple: [0x{u32a:08x}, 0x{u32b:08x}, 0x{u32c:08x}]");
        let exact_match = records
            .iter()
            .any(|r| r.presumptive_u32_triple == [u32a, u32b, u32c]);
        println!(
            "  Exact match as an ElemTable record: {}",
            if exact_match { "YES" } else { "no" }
        );
        // Also: does (u32a, u32b) appear as the first two fields of any ET record?
        let pair_match = records
            .iter()
            .filter(|r| r.presumptive_u32_triple[0] == u32a && r.presumptive_u32_triple[1] == u32b)
            .count();
        if pair_match > 0 {
            println!("  First-two u32s match {pair_match} ElemTable record(s)");
        }
    }

    Ok(())
}
