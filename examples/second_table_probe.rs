//! Q6.4 (fourth-pass): what's the *second* table in `Global/Latest`?
//!
//! `post_directory.rs` showed the first directory ends around 0x7e7
//! (2024) and a new sequential-id structure begins around 0x881. This
//! probe: scan the whole stream for all contiguous sequential-id runs
//! (not just the first one starting from the post-history boundary),
//! so we can see how MANY tables there are, what their lengths are,
//! and whether they share the same record encoding.

#![allow(clippy::needless_range_loop)]

use rvt::{RevitFile, compression, streams};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: second_table_probe <file>");
    let mut rf = RevitFile::open(&path)?;
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;

    // Scan byte-by-byte looking for `[u32 1][body][u32 2][body][u32 3]...`
    // starting sequences. A "table" is any position i where d[i..i+4] =
    // [1, 0, 0, 0] and the next-expected ID marker appears within 64B.
    let mut i = 0;
    let mut tables: Vec<(usize, u32)> = Vec::new(); // (start, records)
    while i + 4 < d.len() {
        if d[i..i + 4] == [1, 0, 0, 0] {
            // Try to extend into a sequential run.
            let mut cursor = i + 4;
            let mut expect: u32 = 2;
            let mut last_run_end = i + 4;
            while cursor + 4 <= d.len() {
                let marker = [
                    (expect & 0xff) as u8,
                    ((expect >> 8) & 0xff) as u8,
                    ((expect >> 16) & 0xff) as u8,
                    ((expect >> 24) & 0xff) as u8,
                ];
                let window_end = (cursor + 64).min(d.len());
                if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
                    last_run_end = cursor + p + 4;
                    cursor = last_run_end;
                    expect += 1;
                } else {
                    break;
                }
            }
            let records = expect - 1; // last accepted id
            if records >= 5 {
                // only report runs of >= 5 sequential ids
                tables.push((i, records));
                i = last_run_end;
                continue;
            }
        }
        i += 1;
    }

    println!("Global/Latest decompressed: {} bytes", d.len());
    println!("Sequential-id tables found (>=5 records):");
    println!();
    println!("  start      records   approx bytes");
    println!("  ---------  -------   ------------");
    for (idx, (start, records)) in tables.iter().enumerate() {
        let end = if idx + 1 < tables.len() {
            tables[idx + 1].0
        } else {
            d.len()
        };
        println!("  0x{:06x}   {:5}     {:8}", start, records, end - start);
    }
    println!();
    println!("{} distinct tables total", tables.len());

    // Dump 48 bytes at the start of each table so we can eyeball whether
    // record structure is the same.
    println!();
    println!("--- First 48 bytes of each table ---");
    for (start, _) in tables.iter().take(10) {
        print!("  0x{start:06x}  ");
        for k in 0..48.min(d.len() - start) {
            print!("{:02x} ", d[start + k]);
            if (k + 1) % 16 == 0 {
                println!();
                print!("            ");
            }
        }
        println!();
    }
    Ok(())
}
