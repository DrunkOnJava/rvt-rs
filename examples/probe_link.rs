//! Null-hypothesis check for the Phase D tag-linking experiment.
//!
//! If Revit's Global/Latest stream encoded classes by ASCII name (like JSON or
//! XML), well-known class names like `ADocument`, `Symbol`, `HostObj`,
//! `Category` would appear as literal strings inside the decompressed bytes.
//!
//! This probe demonstrates that they *don't* — class names live only in
//! `Formats/Latest`, confirming Revit uses integer class tags for data-side
//! references. Run it before `link_schema.rs` to understand why tag-based
//! linkage is the correct approach.

use rvt::{
    RevitFile, compression,
    streams::{FORMATS_LATEST, GLOBAL_LATEST},
};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_link <file.rvt|.rfa>");
    let mut rf = RevitFile::open(&path)?;

    let raw = rf.read_stream(GLOBAL_LATEST)?;
    // Global/Latest: 8-byte custom header + truncated-gzip body.
    let decomp = compression::inflate_at(&raw, 8)?;
    println!(
        "Global/Latest: {} bytes raw → {} bytes decompressed ({:.1}x)",
        raw.len(),
        decomp.len(),
        decomp.len() as f64 / raw.len() as f64
    );

    let probes = [
        "ADocument",
        "DBView",
        "Symbol",
        "HostObj",
        "Category",
        "ElementId",
        "ModelIdentity",
        "PathType",
    ];
    for p in probes {
        let bytes = p.as_bytes();
        let count = decomp.windows(bytes.len()).filter(|w| *w == bytes).count();
        println!("  ASCII '{p}' (len {}): {count} occurrences", bytes.len());
    }

    // For comparison: the same probes against Formats/Latest where class
    // names DO exist.
    let schema_raw = rf.read_stream(FORMATS_LATEST)?;
    // Formats/Latest: gzip header at offset 0 (no custom wrapper).
    let schema_decomp = compression::inflate_at(&schema_raw, 0)?;
    println!(
        "\nFormats/Latest: {} bytes decompressed",
        schema_decomp.len()
    );
    for p in ["ADocument", "DBView", "Symbol", "HostObj", "Category"] {
        let bytes = p.as_bytes();
        let count = schema_decomp
            .windows(bytes.len())
            .filter(|w| *w == bytes)
            .count();
        println!("  ASCII '{p}' in Formats/Latest: {count} occurrences");
    }
    Ok(())
}
