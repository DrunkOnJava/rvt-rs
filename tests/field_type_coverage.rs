//! CI regression gate for the Q5.2 100%-classification result.
//!
//! Opens every Revit sample in the 11-version reference corpus, parses
//! its `Formats/Latest` schema, and asserts that zero fields decode to
//! `FieldType::Unknown`. If any decoder arm ever regresses, or if a
//! newly-added corpus file contains a byte pattern we haven't mapped,
//! this test fails with a concrete per-file breakdown.
//!
//! Corpus source is Autodesk-owned `rac_basic_sample_family` data that
//! rvt-rs does not redistribute (see `SECURITY.md`). Test therefore
//! requires the caller to provide the samples via either:
//!   - Default path `../../samples/` relative to the crate manifest
//!     (layout used by the local `rvt-recon-*` workspace), or
//!   - `RVT_SAMPLES_DIR` env var pointing to a directory that contains
//!     the 11 `.rfa` files (used by CI when it checks out phi-ag/rvt).
//!
//! If the corpus is missing, the test fails — it does NOT silently
//! skip. This guarantees the regression gate cannot pass vacuously.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::{RevitFile, compression, formats, streams};

#[test]
fn field_type_coverage_is_100_percent_across_corpus() {
    let mut missing: Vec<u32> = Vec::new();
    let mut per_year: Vec<(u32, usize, usize)> = Vec::new();

    for year in ALL_YEARS {
        let path = sample_for_year(year);
        if !path.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&path).unwrap_or_else(|e| {
            panic!("{year}: RevitFile::open failed at {}: {e}", path.display())
        });
        let raw = rf
            .read_stream(streams::FORMATS_LATEST)
            .unwrap_or_else(|e| panic!("{year}: read Formats/Latest: {e}"));
        let decompressed = compression::inflate_at(&raw, 0)
            .unwrap_or_else(|e| panic!("{year}: inflate Formats/Latest: {e}"));
        let schema = formats::parse_schema(&decompressed)
            .unwrap_or_else(|e| panic!("{year}: parse_schema: {e}"));

        let total: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
        let unknown: usize = schema
            .classes
            .iter()
            .flat_map(|c| c.fields.iter())
            .filter(|f| matches!(f.field_type, Some(formats::FieldType::Unknown { .. })))
            .count();
        per_year.push((year, total, unknown));
    }

    if !missing.is_empty() {
        panic!(
            "corpus incomplete — missing release(s): {:?}.\n  \
             Samples dir: {}\n  \
             Either set RVT_SAMPLES_DIR to a directory containing the 11 .rfa files, \
             or place the phi-ag/rvt sample corpus at '../../samples/' relative to the crate manifest. \
             rvt-rs intentionally does not redistribute these files (see SECURITY.md).",
            missing,
            samples_dir().display()
        );
    }

    let mut regressed: Vec<(u32, usize, usize)> = Vec::new();
    for (year, total, unknown) in &per_year {
        println!("  {year}: {total} schema fields, {unknown} Unknown");
        if *unknown > 0 {
            regressed.push((*year, *total, *unknown));
        }
    }
    assert!(
        regressed.is_empty(),
        "Q5.2 100%-classification regressed — \
         the following release(s) contain FieldType::Unknown fields: {:?}. \
         Run `cargo run --release --example unknown_bytes_deep -- <file>` to see the byte patterns.",
        regressed
    );

    // Sanity gate: a vacuously-passing corpus (zero files scanned or zero
    // fields found) would also report zero unknowns. Require a plausible
    // lower bound on total fields across the whole corpus.
    let corpus_total: usize = per_year.iter().map(|(_, t, _)| t).sum();
    assert!(
        corpus_total >= 10_000,
        "corpus total ({corpus_total} fields across {} releases) is suspiciously low; \
         the regression gate must see >= 10,000 fields to guarantee non-vacuous coverage",
        per_year.len()
    );
}
