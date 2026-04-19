//! Q6.4 (fifth-pass): resolve Table B's record structure. Per
//! `second_table_probe.rs`, Table A (the "directory" we've been
//! studying) is tiny and contains only ~1342 bytes of 131 compact
//! records; Table B starts at ~0x8a1 in the 2024 sample and is
//! several hundred KB. Table B is the much more likely location of
//! ADocument's actual instance data.
//!
//! This probe:
//! 1. Finds Table B's start.
//! 2. Walks forward collecting all sequential `[u32 id]` markers with
//!    a large look-ahead window (up to 8 KB), so records can be
//!    arbitrarily large.
//! 3. Reports per-record byte length distribution + dumps the first
//!    few records in hex for structural inspection.
//!
//! If Table B has 141+ records with stable per-record-layout
//! signatures, it IS the object-data table. If it's more irregular,
//! we need another interpretation.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, streams};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: table_b_structure <file>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;

    // Find Table A's end (first sequential run starting post-history).
    // Since second_table_probe showed the tables, we'll search for the
    // SECOND `01 00 00 00` after the first one, with a gap.
    let marker1 = [0x01u8, 0x00, 0x00, 0x00];
    let first = d.windows(4).position(|w| w == marker1).unwrap();
    // Advance past the first table: it's ~1342 bytes. Start next search
    // 2 KB in.
    let second_search_start = first + 2048;
    let second = second_search_start
        + d[second_search_start..]
            .windows(4)
            .position(|w| w == marker1)
            .unwrap();
    println!(
        "Table A start: 0x{first:06x}   Table B start: 0x{second:06x}   delta: {} bytes",
        second - first
    );
    println!();

    // Walk Table B with generous look-ahead (up to 8192 bytes between
    // sequential markers).
    let mut cursor = second;
    let mut id_offsets: Vec<(u32, usize)> = Vec::new();
    let mut expect: u32 = 1;
    while cursor + 4 <= d.len() && expect < 5000 {
        let marker = [
            (expect & 0xff) as u8,
            ((expect >> 8) & 0xff) as u8,
            ((expect >> 16) & 0xff) as u8,
            ((expect >> 24) & 0xff) as u8,
        ];
        let window_end = (cursor + 8192).min(d.len());
        if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
            id_offsets.push((expect, cursor + p));
            cursor = cursor + p + 4;
            expect += 1;
        } else {
            break;
        }
    }
    println!("Table B sequential records found: {}", id_offsets.len());
    if !id_offsets.is_empty() {
        let start_off = id_offsets.first().unwrap().1;
        let end_off = id_offsets.last().unwrap().1;
        println!(
            "  first record at 0x{:06x}, last at 0x{:06x}, span {} bytes",
            start_off,
            end_off,
            end_off - start_off
        );
    }
    println!();

    // Record-length distribution.
    let mut lengths: Vec<usize> = Vec::new();
    for i in 0..id_offsets.len().saturating_sub(1) {
        let a = id_offsets[i].1;
        let b = id_offsets[i + 1].1;
        lengths.push(b - a);
    }
    if !lengths.is_empty() {
        lengths.sort_unstable();
        let sum: usize = lengths.iter().sum();
        let avg = sum as f64 / lengths.len() as f64;
        let min = *lengths.first().unwrap();
        let max = *lengths.last().unwrap();
        let median = lengths[lengths.len() / 2];
        println!(
            "Record-length stats: min={min}, median={median}, avg={avg:.1}, max={max}, total={}",
            sum
        );
        println!("  Length histogram (top 10 most common):");
        let mut hist: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
        for l in &lengths {
            *hist.entry(*l).or_insert(0) += 1;
        }
        let mut top: Vec<_> = hist.iter().collect();
        top.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
        for (len, count) in top.iter().take(10) {
            println!("    {len:5} bytes × {count}");
        }
    }
    println!();

    // Dump the first 3 records with generous context.
    println!("--- First 3 Table B records (hex dump of first 64 bytes each) ---");
    for i in 0..id_offsets.len().min(3) {
        let (id, off) = id_offsets[i];
        let end = id_offsets
            .get(i + 1)
            .map(|(_, o)| *o)
            .unwrap_or((off + 64).min(d.len()));
        let record_len = end - off;
        println!("\n  record id={id}  offset=0x{off:06x}  length={record_len}");
        let dump_len = record_len.min(64);
        for row_start in (0..dump_len).step_by(16) {
            let row_end = (row_start + 16).min(dump_len);
            print!("    0x{:06x}  ", off + row_start);
            for k in row_start..row_end {
                print!("{:02x} ", d[off + k]);
            }
            for _ in row_end..row_start + 16 {
                print!("   ");
            }
            print!(" |");
            for k in row_start..row_end {
                let b = d[off + k];
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
    Ok(())
}
