//! RE-14.1 — record-envelope probe for HostObjAttr.
//!
//! Tag 0x006b is the cleanest "this is actually a record, not
//! byte-coincidence" signal we have per RE-11. Byte pattern `0x6b 0x00`
//! is unlikely to appear in numeric data (0x6b = ASCII 'k' = 107).
//! 5,600 occurrences on Einhoven Partitions/0 gives a strong sample.
//!
//! For each occurrence, we collect:
//!   - byte offset in the concat buffer
//!   - 64 bytes following the tag
//!   - distance to next 0x006b occurrence (forward delta)
//!
//! Analysis:
//!   1. Distance-to-next-same-tag histogram. Sharp peak = fixed-size
//!      record. Bimodal with large second mode = length-prefixed
//!      variable-size record. Heavy-tail = probably noise, not a
//!      record boundary.
//!   2. Column-wise byte histograms at +0..+32 relative to the tag.
//!      Columns with low entropy (one byte value dominates) are
//!      fixed fields. Columns with monotonic-increasing values are
//!      likely offsets, lengths, or IDs.
//!   3. Hex-dump the first 10 occurrences for manual inspection.

use rvt::{RevitFile, compression, streams};
use std::collections::BTreeMap;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let target_tag: u16 = 0x006b;
    let ctx: usize = 64;

    for (file, partition) in [
        ("Revit_IFC5_Einhoven.rvt", "Partitions/0"),
        ("Revit_IFC5_Einhoven.rvt", "Partitions/5"),
        ("2024_Core_Interior.rvt", "Partitions/46"),
    ] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };
        let Ok(raw) = rf.read_stream(partition) else {
            continue;
        };
        // Also dump what the FORMATS_LATEST stream is available for
        // reference (we already know tag 0x006b = HostObjAttr in 2023
        // and 2024, so no re-lookup needed here).
        let _ = streams::FORMATS_LATEST;
        let chunks = compression::inflate_all_chunks(&raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        if concat.len() < ctx + 2 {
            continue;
        }

        // Find every occurrence.
        let mut occurrences: Vec<usize> = Vec::new();
        for i in 0..concat.len().saturating_sub(1) {
            let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
            if v == target_tag {
                occurrences.push(i);
            }
        }

        println!("\n=== {file} {partition} — tag 0x{target_tag:04x} (HostObjAttr) ===");
        println!(
            "  Buffer: {} bytes, {} u16 positions",
            concat.len(),
            concat.len() - 1
        );
        println!("  Occurrences: {}", occurrences.len());
        if occurrences.len() < 10 {
            continue;
        }

        // Distance-to-next-same-tag histogram.
        let mut deltas: Vec<usize> = Vec::with_capacity(occurrences.len() - 1);
        for w in occurrences.windows(2) {
            deltas.push(w[1] - w[0]);
        }
        let mut delta_hist: BTreeMap<usize, usize> = BTreeMap::new();
        for &d in &deltas {
            *delta_hist.entry(d).or_insert(0) += 1;
        }
        let mut top: Vec<(usize, usize)> = delta_hist.iter().map(|(&k, &v)| (k, v)).collect();
        top.sort_by_key(|(_, v)| std::cmp::Reverse(*v));

        println!("\n  Top-15 distance-to-next-0x{target_tag:04x}:");
        println!("    {:>8}  {:>6}  {:>6}", "delta", "count", "pct");
        for (d, c) in top.iter().take(15) {
            let pct = 100.0 * (*c as f64) / (deltas.len() as f64);
            println!("    {d:>8}  {c:>6}  {pct:>5.1}%");
        }

        // Summary stats on deltas.
        let min = *deltas.iter().min().unwrap();
        let max = *deltas.iter().max().unwrap();
        let mean = deltas.iter().sum::<usize>() as f64 / deltas.len() as f64;
        let median = {
            let mut d = deltas.clone();
            d.sort_unstable();
            d[d.len() / 2]
        };
        // Compute mode weight = fraction of deltas equal to top value.
        let top_delta = top[0].0;
        let top_delta_count = top[0].1;
        let mode_fraction = 100.0 * (top_delta_count as f64) / (deltas.len() as f64);
        println!(
            "\n  Delta stats: min={min}, max={max}, mean={mean:.1}, median={median}, \
             mode={top_delta} ({mode_fraction:.1}%)"
        );

        // Column-wise byte-value distributions at +0..+16 after the tag.
        // For each column c, build a histogram of values. Report entropy
        // (low entropy = fixed field).
        println!("\n  Column-wise byte histogram (first 16 B after tag at +2..+18):");
        println!(
            "    {:>4}  {:>10}  {:>10}  {:>6}",
            "col", "top1_val", "top2_val", "uniq"
        );
        for c in 2..18 {
            // skip the 2 tag bytes, examine +2..
            let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
            for &off in &occurrences {
                if off + c < concat.len() {
                    *hist.entry(concat[off + c]).or_insert(0) += 1;
                }
            }
            let mut sorted: Vec<(u8, usize)> = hist.into_iter().collect();
            sorted.sort_by_key(|(_, v)| std::cmp::Reverse(*v));
            let top1 = sorted[0];
            let top2 = if sorted.len() > 1 {
                sorted[1]
            } else {
                (0u8, 0usize)
            };
            let total: usize = sorted.iter().map(|(_, v)| *v).sum();
            let top1_pct = 100.0 * (top1.1 as f64) / (total as f64);
            let top2_pct = 100.0 * (top2.1 as f64) / (total as f64);
            println!(
                "    {:>4}  0x{:02x} ({:>4.1}%)  0x{:02x} ({:>4.1}%)  {:>6}",
                c - 2,
                top1.0,
                top1_pct,
                top2.0,
                top2_pct,
                sorted.len(),
            );
        }

        // Hex-dump first 5 occurrences for manual inspection.
        println!("\n  Hex of first 5 occurrences (32 B including tag):");
        for &off in occurrences.iter().take(5) {
            let end = (off + 32).min(concat.len());
            let bytes = &concat[off..end];
            let hex: String = bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            println!("    @{off:>8}: {hex}");
        }
    }
}
