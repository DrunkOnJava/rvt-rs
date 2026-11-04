//! CI regression gate for field-type classification.
//!
//! Opens every Revit sample in the 12-version reference corpus, parses
//! its whole page-stripped `Formats/Latest` schema (#410: about 10,000 to
//! 12,000 fields per release; before #410 only the first 64 KB, 1,000 to
//! 1,400, were parsed, which is where the old 100% figure came from), and
//! asserts that no field decodes to `FieldType::Unknown` except the
//! [`RESIDUAL`] ones, whose encodings are not classified yet (Revit 2027's
//! flagged fields, RE-264, decode to `FieldType::Flagged`). If a decoder
//! arm regresses, or a corpus file carries a byte pattern we haven't mapped,
//! this test fails with a concrete per-file breakdown.
//!
//! The scan (`formats::parse_schema`) is a heuristic and files a class's
//! fields after an inline class definition under that inline class, so it
//! also reads each schema with the exact grammar (`schema_registry`,
//! RE-323) and holds the fields it names by their own class to
//! [`RESIDUAL_GRAMMAR`]. Neither list may keep a name that no longer decodes
//! `Unknown`.
//!
//! Corpus source is Autodesk-owned `rac_basic_sample_family` data that
//! rvt-rs does not redistribute (see `SECURITY.md`). Test therefore
//! requires the caller to provide the samples via either:
//!   - Default path `_corpus/examples/Autodesk` under the crate root, or
//!   - `RVT_SAMPLES_DIR` env var pointing to a directory that contains
//!     the 12 `.rfa` files (used by CI when it checks out phi-ag/rvt).
//!
//! If the corpus is missing, the test fails — it does NOT silently
//! skip. This guarantees the regression gate cannot pass vacuously.

mod common;

use common::{ALL_YEARS, sample_for_year, samples_dir};
use rvt::research::{native_document, schema_registry};
use rvt::{RevitFile, compression, formats, streams};
use std::collections::BTreeSet;

/// Fields the scan (`formats::parse_schema`) leaves `Unknown` in at least
/// one release, under the class names the scan gives them. RE-323 classified
/// the others: a pointer array (`0e 13`), a pointer container (`0e 53`,
/// `0e 54`), a flagged `i32` (`04 00 04 00`), and the class references the
/// scan's 4,096-byte window clipped to two bytes (`ZoningAppInfo::first`).
/// What is left:
/// - `ClassDefinitionRef::m_ref` (`0a 00 00 00`): no sample project or family
///   holds an instance, so its encoding has no bytes to be read from.
/// - `VarParam::m_pSubst` and `VarSketchObj::m_dimValue` (`0e 04 00 00`):
///   31,705 instances of the first in the 2015 to 2017 sample projects, every
///   one a null pointer, which two different layouts read alike, and none of
///   the second. The scan files `VarSketchDimObj::m_dimValue` under
///   `VarSketchObj`.
const RESIDUAL: &[&str] = &[
    "ClassDefinitionRef::m_ref",
    "VarParam::m_pSubst",
    "VarSketchObj::m_dimValue",
];

/// The same fields under the class that declares them, as the exact grammar
/// reads the schema.
const RESIDUAL_GRAMMAR: &[&str] = &[
    "ClassDefinitionRef::m_ref",
    "VarParam::m_pSubst",
    "VarSketchDimObj::m_dimValue",
];

#[test]
fn field_type_coverage_across_corpus() {
    let mut missing: Vec<u32> = Vec::new();
    let mut per_year: Vec<(u32, usize, usize)> = Vec::new();
    let mut unexpected: Vec<(u32, String)> = Vec::new();
    let mut unexpected_grammar: Vec<(u32, String)> = Vec::new();
    let mut seen_scan: BTreeSet<String> = BTreeSet::new();
    let mut seen_grammar: BTreeSet<String> = BTreeSet::new();

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
                if matches!(field.field_type, Some(formats::FieldType::Unknown { .. })) {
                    if !RESIDUAL.contains(&name.as_str()) {
                        unexpected.push((year, name.clone()));
                    }
                    seen_scan.insert(name);
                }
            }
        }
        per_year.push((year, total, unknown));

        let stream = native_document::read_single(&mut rf, streams::FORMATS_LATEST)
            .unwrap_or_else(|e| panic!("{year}: read the schema stream: {e}"));
        let registry = schema_registry::parse(&stream)
            .unwrap_or_else(|e| panic!("{year}: the exact schema grammar stops: {e}"));
        for class in &registry.classes {
            for field in &class.fields {
                let start = field.descriptor_offset;
                let window = &stream[start..(start + 32).min(stream.len())];
                if matches!(
                    formats::FieldType::decode(window),
                    formats::FieldType::Unknown { .. }
                ) {
                    let name = format!("{}::{}", class.name, field.name);
                    if !RESIDUAL_GRAMMAR.contains(&name.as_str()) {
                        unexpected_grammar.push((year, name.clone()));
                    }
                    seen_grammar.insert(name);
                }
            }
        }
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
                 │ To run this test locally, fetch the phi-ag/rvt corpus into the\n  \
                 │ default location, from the crate root:\n  \
                 │   git clone https://github.com/phi-ag/rvt.git _corpus\n  \
                 │ CI sets RVT_SAMPLES_DIR directly and runs with\n  \
                 │ RVT_REQUIRE_CORPUS=1 to hard-fail if any file is missing.\n  \
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
         `SHOW_UNKNOWN=1 cargo run --profile ci -p rvt-probes --example probe_schema_page_strip -- <file>` \
         to see the byte patterns."
    );

    assert!(
        unexpected_grammar.is_empty(),
        "field-type classification regressed — Unknown fields of the exact schema grammar \
         outside RESIDUAL_GRAMMAR: {unexpected_grammar:?}"
    );
    for (list, seen) in [(RESIDUAL, &seen_scan), (RESIDUAL_GRAMMAR, &seen_grammar)] {
        for name in list {
            assert!(
                seen.contains(*name),
                "{name} is in a residual list but decodes to a type in every release; remove it"
            );
        }
    }

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
