//! Find the invariant byte region in Global/PartitionTable that is
//! identical across all 11 Revit releases, and try to decode it as a
//! UUIDv1 (or other structured identifier).
//!
//! Memory note: project_rvt_rs_2026_04_19.md mentions a 30-byte
//! invariant block "byte-for-byte identical 2016→2026", likely a format
//! GUID. This probe locates that block and renders candidates.

use rvt::{compression, streams::GLOBAL_PARTITION_TABLE, RevitFile};
use std::path::PathBuf;

fn decompress(path: &PathBuf) -> anyhow::Result<Vec<u8>> {
    let mut rf = RevitFile::open(path)?;
    let bytes = rf.read_stream(GLOBAL_PARTITION_TABLE)?;
    // Global/PartitionTable: 8-byte custom header + gzip, same as
    // Global/Latest.
    let decomp = compression::inflate_at(&bytes, 8)?;
    Ok(decomp)
}

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".to_string());

    let mut per_release: Vec<(String, Vec<u8>)> = Vec::new();
    for v in 2016..=2026 {
        for filename in [
            format!("racbasicsamplefamily-{v}.rfa"),
            format!("rac_basic_sample_family-{v}.rfa"),
        ] {
            let path = PathBuf::from(&sample_dir).join(&filename);
            if path.exists() {
                let bytes = decompress(&path)?;
                per_release.push((v.to_string(), bytes));
                break;
            }
        }
    }

    println!("Loaded {} releases of Global/PartitionTable", per_release.len());
    for (v, bytes) in &per_release {
        println!("  {v}: {} bytes decompressed", bytes.len());
    }

    // Find the common byte range by intersecting prefixes.
    let min_len = per_release.iter().map(|(_, b)| b.len()).min().unwrap_or(0);
    println!("\nShortest stream: {min_len} bytes");

    // Walk from offset 0 and find the longest run of fully-invariant bytes
    // across all streams. Print every run ≥ 8 bytes long with offset + hex.
    let mut i = 0;
    let mut invariant_runs: Vec<(usize, usize, Vec<u8>)> = Vec::new();  // (start, length, bytes)
    while i < min_len {
        let first = per_release[0].1[i];
        let is_invariant = per_release.iter().all(|(_, b)| b[i] == first);
        if is_invariant {
            let start = i;
            let mut run = Vec::new();
            while i < min_len
                && per_release
                    .iter()
                    .all(|(_, b)| b[i] == per_release[0].1[i])
            {
                run.push(per_release[0].1[i]);
                i += 1;
            }
            if run.len() >= 8 {
                invariant_runs.push((start, run.len(), run));
            }
        } else {
            i += 1;
        }
    }

    println!("\nInvariant runs ≥ 8 bytes (common across all 11 releases):");
    for (start, length, bytes) in &invariant_runs {
        print!("  offset 0x{start:04x} ({start:5}) len={length:3}: ");
        for b in bytes.iter().take(32) {
            print!("{b:02x} ");
        }
        if bytes.len() > 32 {
            print!("... ({} more bytes)", bytes.len() - 32);
        }
        println!();

        // Try to decode the first 16 bytes as a UUID if length allows
        if *length >= 16 {
            let mut uuid_bytes = [0u8; 16];
            uuid_bytes.copy_from_slice(&bytes[..16]);
            // UUIDv1 layout: 4-2-2-2-6 bytes in big-endian. But Windows
            // typically writes GUIDs little-endian for the first three
            // fields. Try both.
            let be = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                uuid_bytes[0], uuid_bytes[1], uuid_bytes[2], uuid_bytes[3],
                uuid_bytes[4], uuid_bytes[5], uuid_bytes[6], uuid_bytes[7],
                uuid_bytes[8], uuid_bytes[9], uuid_bytes[10], uuid_bytes[11],
                uuid_bytes[12], uuid_bytes[13], uuid_bytes[14], uuid_bytes[15],
            );
            let le = format!(
                "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
                uuid_bytes[3], uuid_bytes[2], uuid_bytes[1], uuid_bytes[0],
                uuid_bytes[5], uuid_bytes[4], uuid_bytes[7], uuid_bytes[6],
                uuid_bytes[8], uuid_bytes[9], uuid_bytes[10], uuid_bytes[11],
                uuid_bytes[12], uuid_bytes[13], uuid_bytes[14], uuid_bytes[15],
            );
            println!("    as UUID (BE): {be}");
            println!("    as GUID (LE): {le}");
        }
        // Try to decode as ASCII if printable
        let printable = bytes.iter().all(|&b| b.is_ascii_graphic() || b == b' ');
        if printable && bytes.len() >= 4 {
            println!("    as ASCII: \"{}\"", std::str::from_utf8(bytes).unwrap_or("?"));
        }
    }

    Ok(())
}
