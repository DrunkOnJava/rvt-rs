//! RE-15 — ArcWall trailer (177 B post-core) field discovery.
//!
//! Per RE-14.3, standard ArcWall records on Einhoven Partitions/5
//! have a 115 B fixed core (+0x00..+0x73) followed by either 177 B
//! (singleton stride 292) or 453 B (paired stride 568) of trailer.
//! The core is fully decoded by `arc_wall_record::ArcWallRecord`.
//! The trailer is unexplored.
//!
//! This probe extracts the 177 B trailer for every standard ArcWall
//! record on Einhoven and runs three analyses:
//!
//!   1. **Per-column byte histogram** — find fixed constants vs
//!      variable fields.
//!   2. **f64 sweep** — at every 8-byte-aligned offset in the trailer,
//!      interpret as f64 LE, flag any value in a "plausible wall
//!      parameter" range (height 1-40 ft, thickness 0.1-3 ft).
//!   3. **u32 sweep** — at every 4-byte offset, collect u32 LE values
//!      whose repeat-count across records suggests shared references
//!      (level id, wall-type id, etc.).
//!
//! Run:
//!
//!     RVT_PROJECT_CORPUS_DIR=/private/tmp/rvt-corpus-probe/magnetar/Revit \
//!         cargo run --release --example probe_arcwall_trailer
//!
//! Output: raw trailer hex per record, then three summary tables.

use rvt::{RevitFile, compression, streams};
use std::collections::BTreeMap;

// Trailer geometry — must stay in sync with arc_wall_record.rs.
const CORE_END: usize = 0x73; // first byte past fixed core
const SINGLE_STRIDE: usize = 292;
const SINGLE_TRAILER_LEN: usize = SINGLE_STRIDE - CORE_END; // 177

// ArcWall tag on Revit 2023 (Einhoven).
const ARC_WALL_TAG: u16 = 0x0191;
const ARC_WALL_VARIANT_STANDARD: u16 = 0x07fa;

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let file = "Revit_IFC5_Einhoven.rvt";
    let partition = "Partitions/5";

    let path = format!("{project_dir}/{file}");
    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let _ = streams::FORMATS_LATEST;
    let raw = rf.read_stream(partition).expect("read partition");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    // Find all standard ArcWall records (record-prefix filter + variant).
    let mut standard_offsets: Vec<usize> = Vec::new();
    for i in 0..concat.len().saturating_sub(SINGLE_STRIDE) {
        let v = u16::from_le_bytes([concat[i], concat[i + 1]]);
        if v != ARC_WALL_TAG {
            continue;
        }
        if concat[i + 2] != 0 || concat[i + 3] != 0 {
            continue;
        }
        let variant = u16::from_le_bytes([concat[i + 0x10], concat[i + 0x11]]);
        if variant != ARC_WALL_VARIANT_STANDARD {
            continue;
        }
        standard_offsets.push(i);
    }

    println!("=== {file} {partition} — ArcWall trailer probe ===");
    println!(
        "  Buffer: {} B, {} standard records (variant 0x{ARC_WALL_VARIANT_STANDARD:04x})",
        concat.len(),
        standard_offsets.len()
    );
    println!(
        "  Trailer length: {SINGLE_TRAILER_LEN} B (offsets +0x{CORE_END:02x}..+0x{SINGLE_STRIDE:02x} of each record)"
    );
    println!();

    // Collect trailer slices.
    let mut trailers: Vec<Vec<u8>> = Vec::new();
    for &off in &standard_offsets {
        if off + SINGLE_STRIDE > concat.len() {
            continue;
        }
        trailers.push(concat[off + CORE_END..off + SINGLE_STRIDE].to_vec());
    }
    println!("  {} trailers captured", trailers.len());
    println!();

    // -----------------------------------------------------------------
    // 1. Raw hex dump of first 6 trailers, 16 bytes per line, offsets
    //    shown relative to start-of-record (so +0x73 is line 1, col 0).
    // -----------------------------------------------------------------
    println!("--- Raw hex dump (first 6 trailers) ---");
    for (i, t) in trailers.iter().take(6).enumerate() {
        println!("\n  Trailer #{i} (record @ {}):", standard_offsets[i]);
        for (row_i, chunk) in t.chunks(16).enumerate() {
            let addr = CORE_END + row_i * 16;
            let hex: String = chunk
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            let ascii: String = chunk
                .iter()
                .map(|&b| {
                    if (0x20..0x7f).contains(&b) {
                        b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            println!("    +0x{addr:03x}: {hex:<48}  {ascii}");
        }
    }

    // -----------------------------------------------------------------
    // 2. Per-column byte histogram.
    // -----------------------------------------------------------------
    println!("\n--- Per-column byte histogram (variability profile) ---");
    println!("  col_off   unique_vals  mode(count)                   interpretation");
    let mut fixed_cols = 0usize;
    let mut var_cols = 0usize;
    for col in 0..SINGLE_TRAILER_LEN {
        let mut hist: BTreeMap<u8, usize> = BTreeMap::new();
        for t in &trailers {
            *hist.entry(t[col]).or_default() += 1;
        }
        let unique = hist.len();
        let (mode, mode_count) = hist
            .iter()
            .max_by_key(|&(_, c)| *c)
            .map(|(&b, &c)| (b, c))
            .unwrap_or((0, 0));
        if unique == 1 {
            fixed_cols += 1;
            println!(
                "  +0x{:03x}    {:>3}          0x{mode:02x} ({mode_count})      FIXED",
                CORE_END + col,
                unique,
            );
        } else if unique <= 4 {
            println!(
                "  +0x{:03x}    {:>3}          0x{mode:02x} ({mode_count})      low-card",
                CORE_END + col,
                unique,
            );
            var_cols += 1;
        } else {
            var_cols += 1;
        }
    }
    println!(
        "  SUMMARY  {fixed_cols} fixed + {var_cols} variable columns (of {SINGLE_TRAILER_LEN})"
    );

    // -----------------------------------------------------------------
    // 3. f64 sweep — at every 8-byte-aligned offset in the trailer,
    //    check if the value lies in a plausible wall parameter range.
    // -----------------------------------------------------------------
    println!("\n--- f64 sweep (height/thickness candidates) ---");
    println!("  Plausible wall parameter = finite, non-zero, in [-100, +100]");
    for f64_off in (0..SINGLE_TRAILER_LEN.saturating_sub(8)).step_by(8) {
        let abs_off = CORE_END + f64_off;
        // Collect values from all trailers at this offset.
        let mut vals: Vec<f64> = Vec::new();
        for t in &trailers {
            let v = f64::from_le_bytes([
                t[f64_off],
                t[f64_off + 1],
                t[f64_off + 2],
                t[f64_off + 3],
                t[f64_off + 4],
                t[f64_off + 5],
                t[f64_off + 6],
                t[f64_off + 7],
            ]);
            vals.push(v);
        }
        let plausible_count = vals
            .iter()
            .filter(|&&v| v.is_finite() && v != 0.0 && v.abs() < 100.0)
            .count();
        // Require a majority of records to have plausible values here
        // before flagging the slot as a likely f64 field.
        if plausible_count * 2 >= vals.len() && plausible_count >= 10 {
            let min = vals.iter().copied().fold(f64::INFINITY, f64::min);
            let max = vals.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let mean: f64 = vals.iter().sum::<f64>() / vals.len() as f64;
            let unique: std::collections::BTreeSet<u64> =
                vals.iter().map(|v| v.to_bits()).collect();
            println!(
                "  +0x{abs_off:03x} (+{f64_off:3}): {} plausible / {}, unique={}, min={:.3}, max={:.3}, mean={:.3}",
                plausible_count,
                vals.len(),
                unique.len(),
                min,
                max,
                mean,
            );
            // Show first 6 values for eyeball.
            let sample: Vec<String> = vals.iter().take(6).map(|v| format!("{v:.3}")).collect();
            println!("           sample: [{}]", sample.join(", "));
        }
    }

    // -----------------------------------------------------------------
    // 4. u32 sweep — find slots with high inter-record repetition
    //    (= shared references like wall-type handles, level handles).
    // -----------------------------------------------------------------
    println!("\n--- u32 sweep (shared reference candidates) ---");
    println!("  A slot is reported if its modal value appears in ≥ 40% of records.");
    for u32_off in 0..SINGLE_TRAILER_LEN.saturating_sub(4) {
        let abs_off = CORE_END + u32_off;
        let mut hist: BTreeMap<u32, usize> = BTreeMap::new();
        for t in &trailers {
            let v =
                u32::from_le_bytes([t[u32_off], t[u32_off + 1], t[u32_off + 2], t[u32_off + 3]]);
            *hist.entry(v).or_default() += 1;
        }
        let total = trailers.len();
        let (mode, count) = hist.iter().max_by_key(|(_, c)| **c).unwrap();
        if *count * 100 / total >= 40 && *count < total {
            // Not all-same (that'd be a fixed byte pattern already reported),
            // but a strong mode — candidate handle/reference.
            let non_mode_count = total - count;
            println!(
                "  +0x{abs_off:03x} (+{u32_off:3}): mode=0x{mode:08x} ({count}/{total} = {}%), other={non_mode_count}",
                count * 100 / total,
            );
        }
    }

    println!("\n=== done ===");
}
