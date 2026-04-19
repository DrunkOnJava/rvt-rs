//! Probe the decompressed Global/ElemTable stream across a few releases.
//! Goal: learn the record layout (length-prefixed? fixed-size? indexed by
//! ElementId?) enough to scaffold a parser module.

use rvt::{compression, streams::GLOBAL_ELEM_TABLE, RevitFile};
use std::path::PathBuf;

fn dump_hex_at(bytes: &[u8], offset: usize, len: usize, label: &str) {
    println!("  {label}");
    let end = (offset + len).min(bytes.len());
    for row_start in (offset..end).step_by(16) {
        let row_end = (row_start + 16).min(end);
        print!("    0x{row_start:06x}  ");
        for i in row_start..row_end { print!("{:02x} ", bytes[i]); }
        for _ in row_end..row_start + 16 { print!("   "); }
        print!(" |");
        for i in row_start..row_end {
            let b = bytes[i];
            print!("{}", if (0x20..0x7f).contains(&b) { b as char } else { '.' });
        }
        println!("|");
    }
}

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
            if !path.exists() { continue; }
            println!("\n═══ Revit {year} — {filename} ═══");
            let mut rf = RevitFile::open(&path)?;
            let raw = rf.read_stream(GLOBAL_ELEM_TABLE)?;
            println!("  raw stream: {} bytes", raw.len());
            // Global/ElemTable uses the same 8-byte custom header + gzip as Global/Latest.
            let d = match compression::inflate_at(&raw, 8) {
                Ok(v) => v,
                Err(e) => {
                    // Try gzip at offset 0 as a fallback
                    match compression::inflate_at(&raw, 0) {
                        Ok(v) => v,
                        Err(_) => {
                            println!("  (decompress failed: {e})");
                            break;
                        }
                    }
                }
            };
            println!("  decompressed: {} bytes", d.len());
            if d.len() >= 64 {
                dump_hex_at(&d, 0, 64, "[head]");
            }
            if d.len() >= 96 {
                let tail = d.len().saturating_sub(64);
                dump_hex_at(&d, tail, 64, "[tail]");
            }
            // Look for repeating 4-byte or 8-byte patterns
            if d.len() >= 32 {
                // Most common u32 LE value in first 256 bytes
                let mut freq: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
                for i in (0..d.len().min(512)).step_by(4) {
                    if i + 4 <= d.len() {
                        let v = u32::from_le_bytes([d[i], d[i+1], d[i+2], d[i+3]]);
                        *freq.entry(v).or_insert(0) += 1;
                    }
                }
                let mut sorted: Vec<_> = freq.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));
                print!("  top u32 LE values (first 512 bytes):");
                for (v, c) in sorted.iter().take(5) {
                    print!(" 0x{:08x}×{}", v, c);
                }
                println!();
            }
            break;
        }
    }
    Ok(())
}
