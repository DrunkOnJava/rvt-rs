//! RE-09 — probe partition-chunk 16-byte header structure.
//!
//! Hypothesis from RE-01 synthesis: each partition chunk begins with
//! `[u32 counter][u32 ?][u32 size?][u32 ?]`. Verify this by dumping
//! the 16-byte prefix of every chunk across Einhoven 2023's Partitions/0
//! and 2024 Core Interior's Partitions/46 + 48.
//!
//! Output columns:
//!   chunk_idx   u32[0]   u32[1]   u32[2]   u32[3]   body_len
//!
//! Interpretations to test:
//!   - u32[0] monotonically increases across chunks in the same partition
//!   - u32[2] roughly equals body_len (or body_len minus 16)
//!   - u32[1] and u32[3] pattern across chunks

use rvt::{RevitFile, compression};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());

    for (file, streams) in [
        (
            "Revit_IFC5_Einhoven.rvt",
            &["Partitions/0", "Partitions/5"][..],
        ),
        (
            "2024_Core_Interior.rvt",
            &["Partitions/46", "Partitions/48"][..],
        ),
    ] {
        let path = format!("{project_dir}/{file}");
        let Ok(mut rf) = RevitFile::open(&path) else {
            continue;
        };

        for stream in streams {
            let raw = rf.read_stream(stream).unwrap();
            let chunks = compression::inflate_all_chunks(&raw);

            println!("\n=== {file} :: {stream} — {} chunks ===", chunks.len());
            println!(
                "{:>4}  {:>10}  {:>10}  {:>10}  {:>10}  {:>10}  {:>10}",
                "idx", "u32[0]", "u32[1]", "u32[2]", "u32[3]", "size", "u32[2]-size"
            );
            println!("{}", "-".repeat(90));

            for (i, chunk) in chunks.iter().enumerate() {
                if chunk.len() < 16 {
                    println!("{i:>4}  <too short: {} B>", chunk.len());
                    continue;
                }
                let u0 = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                let u1 = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                let u2 = u32::from_le_bytes([chunk[8], chunk[9], chunk[10], chunk[11]]);
                let u3 = u32::from_le_bytes([chunk[12], chunk[13], chunk[14], chunk[15]]);
                let size = chunk.len();
                let size_u32 = size as i64;
                let u2_minus_size = (u2 as i64) - size_u32;
                println!(
                    "{i:>4}  {u0:>10}  {u1:>10}  {u2:>10}  {u3:>10}  {size:>10}  {u2_minus_size:>10}"
                );
            }

            // Summary statistics
            let u0s: Vec<u32> = chunks
                .iter()
                .filter(|c| c.len() >= 4)
                .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect();
            let u0_monotonic = u0s.windows(2).all(|w| w[0] <= w[1]);
            let u0_unique: std::collections::BTreeSet<u32> = u0s.iter().copied().collect();
            println!(
                "  u0 monotonic: {u0_monotonic}, unique u0 values: {}/{}",
                u0_unique.len(),
                u0s.len()
            );
        }
    }
}
