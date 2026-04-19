//! Round-trip test for write_with_patches: patch a small stream
//! (BasicFileInfo), write to /tmp, re-open, verify the decompressed
//! bytes match.

use rvt::writer::{write_with_patches, StreamPatch, StreamFraming};
use rvt::{compression, streams, RevitFile};
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let src = PathBuf::from("../../samples/_phiag/examples/Autodesk/racbasicsamplefamily-2024.rfa");
    let dst = std::env::temp_dir().join("rvt-rs-writepatch.rfa");

    // Read the Formats/Latest stream from source, decompress, modify, re-embed.
    let mut rf_src = RevitFile::open(&src)?;
    let src_formats_raw = rf_src.read_stream(streams::FORMATS_LATEST)?;
    let src_formats_decomp = compression::inflate_at(&src_formats_raw, 0)?;
    println!("src Formats/Latest: {} raw bytes, {} decompressed",
        src_formats_raw.len(), src_formats_decomp.len());

    // Build a patch: replace the first 16 bytes with a known marker so
    // we can verify it survives the round-trip.
    let mut patched = src_formats_decomp.clone();
    let marker = b"rvt-rs-PATCH!!!!";
    for (i, b) in marker.iter().enumerate() {
        patched[i] = *b;
    }

    let patch = StreamPatch {
        stream_name: "Formats/Latest".to_string(),
        new_decompressed: patched.clone(),
        framing: StreamFraming::RawGzipFromZero,
    };
    write_with_patches(&src, &dst, &[patch])?;

    // Re-open dst, decompress Formats/Latest, verify the marker is there.
    let mut rf_dst = RevitFile::open(&dst)?;
    let dst_formats_raw = rf_dst.read_stream(streams::FORMATS_LATEST)?;
    let dst_formats_decomp = compression::inflate_at(&dst_formats_raw, 0)?;

    println!("dst Formats/Latest: {} raw bytes, {} decompressed",
        dst_formats_raw.len(), dst_formats_decomp.len());

    let first_16 = &dst_formats_decomp[..16];
    let first_16_str = std::str::from_utf8(first_16).unwrap_or("<non-utf8>");
    println!("dst first 16 bytes: {first_16_str:?}");

    assert_eq!(first_16, marker, "marker did not survive round-trip!");
    assert_eq!(
        dst_formats_decomp.len(), patched.len(),
        "decompressed length should match patched source"
    );
    assert_eq!(
        &dst_formats_decomp[16..],
        &patched[16..],
        "rest of the decompressed stream should be identical"
    );

    // Verify the UNPATCHED streams still round-trip byte-for-byte.
    let src_contents = rf_src.read_stream(streams::CONTENTS)?;
    let dst_contents = rf_dst.read_stream(streams::CONTENTS)?;
    assert_eq!(src_contents, dst_contents, "Contents stream must be identical");

    println!("\n✓ round-trip PATCH verified:");
    println!("  - Formats/Latest was patched with marker bytes, survived re-compression");
    println!("  - Contents stream (unpatched) is byte-for-byte identical");
    println!("  - dst file at: {}", dst.display());
    std::fs::remove_file(&dst)?;
    Ok(())
}
