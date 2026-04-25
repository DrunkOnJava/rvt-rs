//! RE-14.3 — ArcWall record layout RE on Einhoven Partitions/5.
//!
//! Per D17 of `RE-14.2-synthesis.md`, ArcWall (tag 0x0191) has 32
//! occurrences in Einhoven Partitions/5, all passing the record-prefix
//! filter. This is the cleanest wall-like signal in the corpus —
//! small sample, 100% real, no text-artifact noise.
//!
//! This probe:
//!   1. Finds all 32 occurrences of tag 0x0191 with buf[+2..+4] == 0x0000.
//!   2. For each, hex-dumps 128 B starting from the tag byte.
//!   3. Computes forward-distance-to-next-real-ArcWall.
//!   4. Per-column byte-value histograms at +2..+64 to find fixed
//!      fields vs variable fields vs length prefixes.
//!   5. Scans each record for embedded u32 values that might be IDs
//!      (references to ElemTable entries, owner IDs, parent refs).
//!
//! Expected outcome: enough evidence of the record envelope to write
//! a concrete ArcWall decoder and wire it into the IFC exporter.

use rvt::{RevitFile, compression, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let target_tag: u16 = 0x0191; // ArcWall on Einhoven 2023
    let file = "Revit_IFC5_Einhoven.rvt";
    let partition = "Partitions/5";
    let dump_len = 128;

    let path = format!("{project_dir}/{file}");
    let mut rf = RevitFile::open(&path).unwrap();
    let _ = streams::FORMATS_LATEST;
    let raw = rf.read_stream(partition).unwrap();
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    // Find occurrences of 0x0191 passing the record-prefix filter.
    let mut occurrences: Vec<usize> = Vec::new();
    for i in 0..concat.len().saturating_sub(3) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v != target_tag {
            continue;
        }
        if concat[i + 2] == 0x00 && concat[i + 3] == 0x00 {
            occurrences.push(i);
        }
    }

    println!("=== {file} {partition} — ArcWall (0x{target_tag:04x}) records ===");
    println!(
        "  Buffer: {} bytes, {} clean ArcWall occurrences",
        concat.len(),
        occurrences.len(),
    );

    // Distance-to-next histogram.
    println!("\n  Distance-to-next-ArcWall:");
    let mut deltas: Vec<usize> = Vec::new();
    for w in occurrences.windows(2) {
        deltas.push(w[1] - w[0]);
    }
    if !deltas.is_empty() {
        let min = *deltas.iter().min().unwrap();
        let max = *deltas.iter().max().unwrap();
        let mean = deltas.iter().sum::<usize>() as f64 / deltas.len() as f64;
        let mut sorted = deltas.clone();
        sorted.sort_unstable();
        let median = sorted[sorted.len() / 2];
        println!(
            "    count={}, min={}, max={}, mean={:.0}, median={}",
            deltas.len(),
            min,
            max,
            mean,
            median,
        );
        // Show all deltas as a sequence.
        let delta_str = deltas
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("    deltas: [{}]", delta_str);
    }

    // Per-column byte histograms at +2..+64.
    println!("\n  Column-wise byte histogram (+2..+32 after tag start):");
    println!(
        "    {:>4}  {:>10}  {:>10}  {:>10}  {:>6}",
        "col", "top1", "top2", "top3", "uniq"
    );
    for c in 2..32 {
        let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
        for &off in &occurrences {
            if off + c < concat.len() {
                *hist.entry(concat[off + c]).or_insert(0) += 1;
            }
        }
        let mut sorted: Vec<(u8, usize)> = hist.into_iter().collect();
        sorted.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
        let pick = |i: usize| -> String {
            sorted
                .get(i)
                .map(|(v, c)| format!("0x{v:02x} ({})", c))
                .unwrap_or_else(|| "—".to_string())
        };
        println!(
            "    {:>4}  {:>10}  {:>10}  {:>10}  {:>6}",
            c - 2,
            pick(0),
            pick(1),
            pick(2),
            sorted.len()
        );
    }

    // Hex-dump all 32 occurrences.
    println!("\n  Hex-dump of all records (128 B each from tag start):");
    for (i, &off) in occurrences.iter().enumerate() {
        let end = (off + dump_len).min(concat.len());
        let bytes = &concat[off..end];
        println!(
            "\n  #{i:>2} @ offset {off:>6} (delta_next={}):",
            if i + 1 < occurrences.len() {
                (occurrences[i + 1] - off).to_string()
            } else {
                "—".to_string()
            }
        );
        // 16 bytes per line, with ASCII sidebar.
        for (row, chunk) in bytes.chunks(16).enumerate() {
            let hex_part: String = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii_part: String = chunk
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            println!("    +{:03x}: {:<48}  {}", row * 16, hex_part, ascii_part);
        }
    }

    // Look for any u32 values that repeat across records — these are
    // likely structural constants (class ID, schema version, etc).
    println!("\n  u32 constants appearing in >=4 records at any position within first 128 B:");
    let mut u32_votes: BTreeMap<u32, usize> = BTreeMap::new();
    for &off in &occurrences {
        let slice_end = (off + dump_len).min(concat.len());
        let slice = &concat[off..slice_end];
        if slice.len() < 4 {
            continue;
        }
        // Unique u32 values in this record (avoid double-counting within same record).
        let mut seen: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
        for w in 0..slice.len().saturating_sub(3) {
            let v = u32::from_le_bytes([slice[w], slice[w + 1], slice[w + 2], slice[w + 3]]);
            if v == 0 || v == u32::MAX {
                continue;
            }
            seen.insert(v);
        }
        for v in seen {
            *u32_votes.entry(v).or_insert(0) += 1;
        }
    }
    let mut top_u32: Vec<(u32, usize)> = u32_votes.into_iter().filter(|(_, v)| *v >= 4).collect();
    top_u32.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
    println!(
        "    {:>12}  {:>12}  {:>10}",
        "u32 (hex)", "decimal", "records"
    );
    for (v, count) in top_u32.iter().take(20) {
        println!("    0x{v:08x}  {v:>12}  {count:>10}");
    }
}
