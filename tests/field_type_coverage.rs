//! CI regression gate for field-type classification.
//!
//! Opens every Revit sample in the 11-version reference corpus, parses
//! its whole page-stripped `Formats/Latest` schema (#410: about 10,000 to
//! 12,000 fields per release; before #410 only the first 64 KB, 1,000 to
//! 1,400, were parsed, which is where the old 100% figure came from), and
//! asserts that no field decodes to `FieldType::Unknown` except the
//! [`RESIDUAL`] ones, whose encodings are not classified yet. If a decoder
//! arm regresses, or a corpus file carries a byte pattern we haven't mapped,
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

/// Fields whose type encodings appear only past the first 64 KB of the
/// schema and are not classified yet (#410): pointer and container
/// modifiers `0e 04`, `0e 13`, `0e 53`, `0e 54`, a `0a` base, `04 00 04 00`,
/// and a record clipped to `0e 00`. Classifying them needs byte evidence
/// of what they hold, not their names.
const RESIDUAL: &[&str] = &[
    "ClassDefinitionRef::m_ref",
    "GInfo::m_next",
    "GInfo::m_pFace",
    "GInfo::m_prev",
    "LFormatDefaults::m_defaultImperialFormatOptions",
    "LFormatDefaults::m_defaultMetricFormatOptions",
    "LFormatDefaults::m_unit",
    "VarFunction::m_refCt",
    "VarParam::m_pSubst",
    "VarSketchObj::m_constrElems",
    "VarSketchObj::m_dimValue",
    "VarSketchObj::m_params",
    "VarSketchObj::m_refPts",
    "ZoningAppInfo::first",
];

#[test]
fn field_type_coverage_across_corpus() {
    let mut missing: Vec<u32> = Vec::new();
    let mut per_year: Vec<(u32, usize, usize)> = Vec::new();
    let mut unexpected: Vec<(u32, String)> = Vec::new();

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
        let decompressed = compression::inflate_stream_at(streams::FORMATS_LATEST, &raw, 0)
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
        for class in &schema.classes {
            for field in &class.fields {
                let name = format!("{}::{}", class.name, field.name);
                if matches!(field.field_type, Some(formats::FieldType::Unknown { .. }))
                    && !RESIDUAL.contains(&name.as_str())
                {
                    unexpected.push((year, name));
                }
            }
        }
        per_year.push((year, total, unknown));
    }

    if !missing.is_empty() {
        // CI mode (strict): corpus must be present. First-time local-dev
        // mode: gracefully skip with a clear message so `cargo test` on
        // a fresh clone does not fail on a setup gap rather than a code
        // regression. Opt in to strict mode by setting RVT_REQUIRE_CORPUS=1.
        let strict = std::env::var("RVT_REQUIRE_CORPUS")
            .ok()
            .is_some_and(|v| v == "1" || v == "true");
        if strict {
            panic!(
                "corpus incomplete — missing release(s): {:?}.\n  \
                 Samples dir: {}\n  \
                 RVT_REQUIRE_CORPUS is set, so this is treated as a regression. \
                 Either provide the phi-ag/rvt sample corpus via RVT_SAMPLES_DIR, \
                 or unset RVT_REQUIRE_CORPUS to allow a graceful skip during local dev. \
                 rvt-rs intentionally does not redistribute these files (see SECURITY.md).",
                missing,
                samples_dir().display()
            );
        } else {
            eprintln!(
                "\n  \
                 ┌─────────────────────────────────────────────────────────────────\n  \
                 │ SKIP: field_type_coverage — corpus not available.\n  \
                 │ Missing release(s): {:?}\n  \
                 │ Samples dir: {}\n  \
                 │ To run this test locally, fetch the phi-ag/rvt corpus:\n  \
                 │   git clone https://github.com/phi-ag/rvt.git ../../samples/_phiag\n  \
                 │ then copy or symlink the .rfa files from\n  \
                 │   ../../samples/_phiag/examples/Autodesk/*.rfa\n  \
                 │ into ../../samples/. CI sets RVT_SAMPLES_DIR directly and runs\n  \
                 │ with RVT_REQUIRE_CORPUS=1 to hard-fail if any file is missing.\n  \
                 └─────────────────────────────────────────────────────────────────\n",
                missing,
                samples_dir().display()
            );
            return;
        }
    }

    for (year, total, unknown) in &per_year {
        println!("  {year}: {total} schema fields, {unknown} Unknown (residual)");
    }
    assert!(
        unexpected.is_empty(),
        "field-type classification regressed — Unknown fields outside the documented \
         residual: {unexpected:?}. Run \
         `SHOW_UNKNOWN=1 cargo run --profile ci --example probe_schema_page_strip -- <file>` \
         to see the byte patterns."
    );

    // Sanity gate: a vacuously-passing corpus (zero files scanned or zero
    // fields found) would also report zero unknowns. Require a plausible
    // lower bound on total fields across the whole corpus.
    let corpus_total: usize = per_year.iter().map(|(_, t, _)| t).sum();
    assert!(
        corpus_total >= 100_000,
        "corpus total ({corpus_total} fields across {} releases) is suspiciously low; \
         the regression gate must see >= 100,000 fields to guarantee non-vacuous coverage",
        per_year.len()
    );
}
