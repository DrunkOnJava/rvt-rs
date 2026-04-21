//! RE critical finding — partition streams have multiple gzip chunks.
//! The first chunk is header-only (~128KB padded with 0xFF); real
//! element data is in later chunks. inflate_at_auto reaches only the
//! first chunk, so probes using it have been seeing header data, not
//! element data.
//!
//! This probe uses inflate_all_chunks() to get every decompressed chunk
//! for Einhoven 2023's Partitions/0 (raw 464KB), prints the size +
//! head of each, and looks for class-name ASCII markers that would
//! indicate Wall/Floor/Door data.

use rvt::{RevitFile, compression};

fn main() {
    let project_dir = std::env::var("RVT_PROJECT_CORPUS_DIR")
        .unwrap_or_else(|_| "/private/tmp/rvt-corpus-probe/magnetar/Revit".into());
    let path = format!("{project_dir}/Revit_IFC5_Einhoven.rvt");
    let mut rf = RevitFile::open(&path).unwrap();

    for stream in ["Partitions/0", "Partitions/5"] {
        let raw = rf.read_stream(stream).unwrap();
        let chunks = compression::inflate_all_chunks(&raw);
        let total: usize = chunks.iter().map(|c| c.len()).sum();
        println!(
            "\n=== {stream}: {} B raw, {} chunks, {} B total decomp ===",
            raw.len(),
            chunks.len(),
            total
        );

        // Scan each chunk for class-name ASCII occurrences — this is a
        // quick "does this chunk contain element data" signal.
        let target_classes: &[&[u8]] = &[
            b"Wall", b"Floor", b"Door", b"Window", b"Stair", b"Column", b"Beam", b"Roof",
            b"Ceiling", b"HostObj",
        ];
        for (i, c) in chunks.iter().enumerate() {
            let mut hits = Vec::new();
            for target in target_classes {
                let n = c.windows(target.len()).filter(|w| w == target).count();
                if n > 0 {
                    hits.push((std::str::from_utf8(target).unwrap(), n));
                }
            }
            println!(
                "  chunk[{i:3}] {:>7} B head={:02x?} class_hits={:?}",
                c.len(),
                &c[..c.len().min(16)],
                hits
            );
        }
    }
}
