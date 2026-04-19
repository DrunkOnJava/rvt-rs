//! Probe the Partitions/NN raw stream header: the ~44 bytes before the first
//! gzip magic. Hypothesis (from memory notes): this prefix encodes a chunk
//! table (offset + size per concatenated gzip chunk).
#![allow(
    clippy::needless_range_loop,
    clippy::type_complexity,
    clippy::collapsible_if,
    clippy::collapsible_match
)]

use rvt::RevitFile;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".into());

    for year in [2016, 2020, 2024, 2026] {
        for filename in [
            format!("racbasicsamplefamily-{year}.rfa"),
            format!("rac_basic_sample_family-{year}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if !path.exists() {
                continue;
            }
            let mut rf = RevitFile::open(&path)?;
            let stream = match rf.partition_stream_name() {
                Some(n) => n,
                None => {
                    break;
                }
            };
            let raw = rf.read_stream(&stream)?;
            // Find first gzip magic
            let first_gzip = (0..raw.len().saturating_sub(3))
                .find(|&i| raw[i] == 0x1f && raw[i + 1] == 0x8b && raw[i + 2] == 0x08);
            let header_len = first_gzip.unwrap_or(raw.len());

            println!("═══ Revit {year} — {stream} ═══");
            println!(
                "  raw: {} bytes, header: {} bytes, first gzip at 0x{:x}",
                raw.len(),
                header_len,
                first_gzip.unwrap_or(0)
            );

            // Dump the header bytes
            let h = &raw[..header_len.min(96)];
            for row_start in (0..h.len()).step_by(16) {
                let row_end = (row_start + 16).min(h.len());
                print!("  0x{row_start:04x}  ");
                for i in row_start..row_end {
                    print!("{:02x} ", h[i]);
                }
                for _ in row_end..row_start + 16 {
                    print!("   ");
                }
                print!(" |");
                for i in row_start..row_end {
                    let b = h[i];
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

            // Find ALL gzip magics
            let mut gzips: Vec<usize> = Vec::new();
            for i in 0..raw.len().saturating_sub(3) {
                if raw[i] == 0x1f && raw[i + 1] == 0x8b && raw[i + 2] == 0x08 {
                    gzips.push(i);
                }
            }
            println!("  {} gzip chunks at offsets:", gzips.len());
            for (n, off) in gzips.iter().enumerate() {
                let gap = if n == 0 { 0 } else { off - gzips[n - 1] };
                println!("    #{n}  0x{off:08x} ({off})  gap={gap}");
            }

            // If there are multiple gzips, compute gaps (chunk sizes)
            if gzips.len() >= 2 {
                // Look at the header bytes and try to decode u32 pairs
                // matching the chunk offsets.
                println!("  header as u32 LE sequence:");
                let mut idx = 0;
                while idx + 4 <= header_len.min(96) {
                    let v =
                        u32::from_le_bytes([raw[idx], raw[idx + 1], raw[idx + 2], raw[idx + 3]]);
                    let maybe_offset = gzips.contains(&(v as usize));
                    let marker = if maybe_offset {
                        " ← matches a gzip offset!"
                    } else {
                        ""
                    };
                    println!("    u32@0x{idx:04x}  =  {v}  (0x{v:08x}){marker}");
                    idx += 4;
                }
            }
            println!();
            break;
        }
    }
    Ok(())
}
