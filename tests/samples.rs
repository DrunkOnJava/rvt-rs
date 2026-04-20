//! Integration tests against the 11-version RFA corpus. Corpus is the
//! public `rac_basic_sample_family` fixtures from phi-ag/rvt (LFS);
//! rvt-rs does not redistribute these files (see `SECURITY.md`).
//! Corpus path is resolved via `tests/common::samples_dir()` which
//! respects the `RVT_SAMPLES_DIR` env var used by CI.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::RevitFile;
use std::collections::HashSet;

fn all_years() -> Vec<u32> {
    ALL_YEARS.to_vec()
}

#[test]
fn opens_all_11_versions() {
    for year in all_years() {
        let p = sample_for_year(year);
        if !p.exists() {
            eprintln!("skipping {year}: sample not present (LFS not pulled?)");
            continue;
        }
        let mut rf = RevitFile::open(&p).unwrap_or_else(|_| panic!("{year}: open failed"));
        let s = rf
            .summarize_strict()
            .unwrap_or_else(|_| panic!("{year}: summarize failed"));
        assert_eq!(s.version, year, "version mismatch for {year}");
        assert_eq!(s.streams.len(), 13, "{year}: expected 13 streams");
        assert!(
            s.class_name_count > 1000,
            "{year}: class count {} unexpectedly low",
            s.class_name_count
        );
        let missing = rf.missing_required_streams();
        assert!(
            missing.is_empty(),
            "{year}: missing required streams: {missing:?}\n  \
             stream names present: {:?}",
            rf.stream_names()
        );
    }
}

#[test]
fn partition_matches_year() {
    for year in all_years() {
        let p = sample_for_year(year);
        if !p.exists() {
            continue;
        }
        let rf = RevitFile::open(&p).unwrap();
        let got = rf.partition_stream_name().unwrap();
        let expected_nn = rvt::streams::partition_for_year(year).unwrap();
        assert_eq!(
            got,
            format!("Partitions/{expected_nn}"),
            "{year}: partition stream name wrong"
        );
    }
}

#[test]
fn part_atom_parses_furniture_omniclass() {
    for year in all_years() {
        let p = sample_for_year(year);
        if !p.exists() {
            continue;
        }
        let mut rf = RevitFile::open(&p).unwrap();
        let pa = rf
            .part_atom()
            .unwrap_or_else(|_| panic!("{year}: part atom"));
        // These fixtures are a Furniture table across all years.
        assert!(
            pa.categories
                .iter()
                .any(|c| c.term.starts_with("23.40.20") || c.term == "Furniture"),
            "{year}: missing Furniture/OmniClass category",
        );
    }
}

#[test]
fn core_class_names_present() {
    let p = sample_for_year(2024);
    if !p.exists() {
        return;
    }
    let mut rf = RevitFile::open(&p).unwrap();
    let names = rf.class_names().unwrap();

    let expected_core: HashSet<&str> = [
        "ADocument",
        "APIAppInfo",
        "AProperty",
        "APropertyBoolean",
        "APropertyDouble1",
        "APropertyDouble3",
        "APropertyEnum",
        "APropertyFloat",
        "APropertyInteger",
        "A3PartyObject",
    ]
    .into_iter()
    .collect();

    for core in &expected_core {
        assert!(
            names.contains(*core),
            "expected class {core:?} missing from Formats/Latest decompressed inventory"
        );
    }
}

#[test]
fn preview_is_valid_png() {
    let p = sample_for_year(2024);
    if !p.exists() {
        return;
    }
    let mut rf = RevitFile::open(&p).unwrap();
    let png = rf.preview_png().unwrap();
    // PNG magic: 89 50 4E 47 0D 0A 1A 0A
    assert_eq!(
        &png[..8],
        &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
        "preview is not a PNG"
    );
}

#[test]
fn document_history_extracts_full_timeline() {
    // The 2026 sample has been migrated through every Revit version
    // since Revit 2018 preview. We should find >= 8 entries.
    let p = sample_for_year(2026);
    if !p.exists() {
        return;
    }
    let mut rf = RevitFile::open(&p).unwrap();
    let history = rvt::object_graph::DocumentHistory::from_revit_file(&mut rf).unwrap();
    assert!(
        history.entries.len() >= 8,
        "expected >= 8 history entries for the 2026 sample, got {}: {:?}",
        history.entries.len(),
        history.entries
    );
    // The entries should include markers for at least 2018 and 2026.
    assert!(history.entries.iter().any(|e| e.contains("2018")));
    assert!(history.entries.iter().any(|e| e.contains("2026")));
}

#[test]
fn schema_field_names_round_trip_byte_for_byte() {
    // Every field name rvt-schema emits should be findable byte-for-byte
    // in the raw decompressed Formats/Latest.
    let p = sample_for_year(2024);
    if !p.exists() {
        return;
    }
    let mut rf = RevitFile::open(&p).unwrap();
    let schema = rf.schema().unwrap();

    // Decompress Formats/Latest manually to grep field names.
    use rvt::compression;
    use rvt::streams::FORMATS_LATEST;
    let raw = rf.read_stream(FORMATS_LATEST).unwrap();
    let decompressed = compression::inflate_at(&raw, 0).unwrap();

    let classes_with_fields: Vec<_> = schema
        .classes
        .iter()
        .filter(|c| !c.fields.is_empty())
        .collect();
    assert!(
        !classes_with_fields.is_empty(),
        "expected at least some classes with fields"
    );

    // Check 10 fields (avoid exhaustively scanning to keep test fast)
    let sample_fields: Vec<_> = classes_with_fields
        .iter()
        .flat_map(|c| c.fields.iter())
        .take(10)
        .collect();
    for f in &sample_fields {
        assert!(
            decompressed
                .windows(f.name.len())
                .any(|w| w == f.name.as_bytes()),
            "field {:?} extracted by schema parser but not found in raw bytes",
            f.name
        );
    }
}

#[test]
fn rejects_non_cfb_input() {
    let bytes = b"not a cfb file at all".to_vec();
    assert!(matches!(
        RevitFile::open_bytes(bytes),
        Err(rvt::Error::NotACfbFile)
    ));
}

// L5B-58: walker element counts from corpus.
//
// Runs `read_adocument_lossy` against each available corpus fixture
// and asserts:
//
//   * 2024-2026: the walker returns a decoded ADocument with at
//     least one typed field (anything > 0 is evidence the walker
//     reached instance data rather than just locating the entry).
//   * 2016-2023: the walker returns `Decoded::partial` with a
//     diagnostic indicating it couldn't find / fully decode the
//     record — NOT a hard panic or error. This is the documented
//     "graceful degradation" behaviour from API-05..09.
//
// Taken together this pins that (a) the walker works on the fully
// supported releases and (b) the lossy path never crashes on
// partially supported releases, which is the contract the public
// API advertises. If a future commit breaks either half, this test
// catches it with a specific year-tagged failure message.
#[test]
fn read_adocument_lossy_succeeds_or_gracefully_degrades() {
    for year in all_years() {
        let p = sample_for_year(year);
        if !p.exists() {
            eprintln!("skipping {year}: sample not present (LFS not pulled?)");
            continue;
        }
        let mut rf = RevitFile::open(&p).unwrap_or_else(|_| panic!("{year}: RevitFile::open"));
        let decoded = rvt::walker::read_adocument_lossy(&mut rf)
            .unwrap_or_else(|e| panic!("{year}: read_adocument_lossy errored hard: {e}"));

        if (2024..=2026).contains(&year) {
            // Fully-supported releases: walker should find and
            // decode the ADocument with non-trivial progress.
            let inst = &decoded.value;
            assert!(
                !inst.fields.is_empty(),
                "{year}: expected walker to decode ≥ 1 field on fully-supported release, got 0",
            );
            let completeness = inst.completeness();
            assert!(
                completeness.typed > 0,
                "{year}: expected ≥ 1 typed field, got {typed}/{total}",
                typed = completeness.typed,
                total = completeness.total,
            );
            // Sanity: the walker's reported version matches the
            // file's declared release.
            assert_eq!(
                inst.version, year,
                "{year}: walker returned version {} for {year} fixture",
                inst.version,
            );
        } else {
            // 2016-2023: accept either "fully decoded" (if the
            // walker has been extended) OR "gracefully degraded
            // with diagnostics". What's forbidden is (a) a hard
            // panic/error (already caught above by unwrap_or_else)
            // and (b) silently returning a zero-field instance
            // without a diagnostic — that would be a regression of
            // API-09's completeness-marker work.
            let has_fields = !decoded.value.fields.is_empty();
            let has_diagnostics = !decoded.diagnostics.is_empty();
            assert!(
                has_fields || has_diagnostics,
                "{year}: walker returned empty instance AND no diagnostics — \
                 graceful-degradation contract broken",
            );
        }
    }
}
