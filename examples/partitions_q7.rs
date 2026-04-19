//! Q7: test whether the 44-byte Partitions/NN header encodes per-chunk
//! byte offsets. Compare trailer_u32 values to gzip-magic-scan positions.

use rvt::{RevitFile, partitions};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let sample_dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "samples/_phiag/examples/Autodesk".into());

    for year in [2016, 2018, 2020, 2022, 2024, 2026] {
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
                None => break,
            };
            let raw = rf.read_stream(&stream)?;
            let header = match partitions::parse_header(&raw) {
                Some(h) => h,
                None => break,
            };
            let chunks = partitions::find_chunks(&raw);
            let scan_offsets: Vec<usize> = chunks.iter().map(|c| c.raw_offset).collect();

            println!("═══ Revit {year} ({stream}) ═══");
            println!("  declared_count+1: {}", header.declared_count_plus_one);
            println!("  reserved_zero:    {}", header.reserved_zero);
            print!("  size_block:       ");
            for b in header.size_block {
                print!("{b:02x} ");
            }
            println!();
            println!("  trailer_u32[0..4]: {:?}", header.trailer_u32);
            println!("  chunks found by gzip-magic scan: {}", scan_offsets.len());
            print!("  gzip offsets: ");
            for o in &scan_offsets {
                print!("{o} ");
            }
            println!();

            // Test: do any trailer_u32 values match any chunk offset?
            let matches: Vec<(usize, u32)> = (0..4)
                .filter_map(|i| {
                    let v = header.trailer_u32[i];
                    if scan_offsets.contains(&(v as usize)) {
                        Some((i, v))
                    } else {
                        None
                    }
                })
                .collect();
            println!("  trailer_u32 ↔ chunk-offset matches: {matches:?}");

            // Test: is trailer_u32[0] the total chunk-area size?
            let chunk_area_size = raw.len() - partitions::HEADER_SIZE;
            println!(
                "  chunk_area_size = raw_len - header (44) = {}",
                chunk_area_size
            );
            // Size_block maybe contains first-chunk-length + second-chunk-length
            // The declared u32 field at byte 0x08 of the full header (size_block[0..4]):
            let sb_u32_a = u32::from_le_bytes(header.size_block[0..4].try_into().unwrap());
            let sb_u32_b = u32::from_le_bytes(header.size_block[4..8].try_into().unwrap());
            let sb_u32_c = u32::from_le_bytes(header.size_block[8..12].try_into().unwrap());
            println!("  size_block as 3 × u32: [{sb_u32_a}, {sb_u32_b}, {sb_u32_c}]");

            // How do the chunks' sizes compare to trailer_u32 values?
            let mut chunk_sizes: Vec<usize> = Vec::new();
            for i in 0..chunks.len() {
                let next = if i + 1 < chunks.len() {
                    chunks[i + 1].raw_offset
                } else {
                    raw.len()
                };
                chunk_sizes.push(next - chunks[i].raw_offset);
            }
            println!("  chunk sizes (bytes): {chunk_sizes:?}");
            println!();
            break;
        }
    }
    Ok(())
}
