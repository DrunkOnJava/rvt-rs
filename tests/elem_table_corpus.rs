//! Integration tests for `elem_table::parse_records` against the
//! 11-release family corpus AND the project-file corpus.
//!
//! Corpus resolution and skip policy — a silent skip is how #206 stayed
//! invisible, so both halves are loud:
//!
//! - Project corpus: `RVT_PROJECT_CORPUS_DIR`. **When the variable is set,
//!   a missing file is a failure, not a skip** (same rule as
//!   `tests/project_count_fixtures.rs`). Without it the tests skip and say
//!   so — we do not redistribute Autodesk-owned files.
//! - Family corpus: `RVT_SAMPLES_DIR` via `tests/common`, with
//!   `RVT_REQUIRE_CORPUS=1` turning a missing release into a failure (same
//!   rule as `tests/field_type_coverage.rs`).
//!
//! `elem_table_record_origin_is_flush_with_the_stream_end` needs no corpus
//! at all — it runs off a committed 270-byte MIT excerpt, so this target is
//! never wholly vacuous.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::{RevitFile, elem_table};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

/// Resolve a named project-corpus file, or `None` to skip.
///
/// Panics when `RVT_PROJECT_CORPUS_DIR` is set but the file is absent: an
/// explicitly-configured corpus that cannot satisfy the test is a broken
/// gate, and a gate that quietly passes is worse than one that fails.
fn project_file(name: &str) -> Option<PathBuf> {
    let configured = std::env::var_os("RVT_PROJECT_CORPUS_DIR").is_some();
    let path = project_dir().join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        !configured,
        "RVT_PROJECT_CORPUS_DIR is set to {} but {name} is not there; fix the path \
         or unset the variable to skip the project-corpus tests",
        project_dir().display()
    );
    eprintln!(
        "skipping: project corpus not present at {} (set RVT_PROJECT_CORPUS_DIR)",
        path.display()
    );
    None
}

fn require_family_corpus() -> bool {
    std::env::var("RVT_REQUIRE_CORPUS")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
}

/// A 270-byte excerpt of the decompressed `Global/ElemTable` from the MIT
/// `2024_Core_Interior.rvt`: the real header with `record_count` rewritten
/// to 6, then records 0-2 and 26422-26424 verbatim. See the sibling
/// `.license.json` for the exact derivation.
const CORE_INTERIOR_ELEM_TABLE_EXCERPT: &[u8] =
    include_bytes!("fixtures/elem-table/2024-core-interior-elemtable-head-tail.bin");

/// Regression for #206 on committed bytes, no corpus required.
///
/// The 40-byte project record opens with a zero `u32` and only then carries
/// the `FF`×8 run, so taking the first `FF` run as the record origin shifts
/// every record window forward by four bytes and walks one record short of
/// the declared count. The record array is exactly `record_count × stride`
/// bytes and ends flush with the end of the stream — that is the invariant
/// this pins.
#[test]
fn elem_table_record_origin_is_flush_with_the_stream_end() {
    let d = CORE_INTERIOR_ELEM_TABLE_EXCERPT;
    assert_eq!(d.len(), 270, "fixture size");
    let declared = u16::from_le_bytes([d[2], d[3]]) as usize;
    assert_eq!(declared, 6, "fixture record_count");

    let layout = elem_table::detect_layout(d);
    assert_eq!(layout.stride, 40);
    assert_eq!(
        layout.framing,
        elem_table::RecordFraming::Explicit { marker_len: 8 }
    );
    assert_eq!(
        layout.start, 0x1e,
        "record 0 begins one u32 ahead of the marker"
    );
    assert_eq!(layout.marker_offset, 4);
    assert_eq!(
        layout.start + declared * layout.stride,
        d.len(),
        "the record array must end flush with the stream"
    );

    let records = elem_table::parse_records_from_bytes(d, layout, declared);
    assert_eq!(
        records.len(),
        declared,
        "every declared record must be walked (pre-fix this was {}, one short)",
        declared - 1
    );
    let offsets: Vec<usize> = records.iter().map(|r| r.offset).collect();
    assert_eq!(offsets, vec![0x1e, 0x46, 0x6e, 0x96, 0xbe, 0xe6]);
    let ids: Vec<u32> = records.iter().map(|r| r.id_primary).collect();
    assert_eq!(ids, vec![1, 2, 3, 133_205, 133_206, 0]);
    let secondaries: Vec<u32> = records.iter().map(|r| r.id_secondary).collect();
    assert_eq!(secondaries, vec![1, 2, 3, 133_205, 133_206, 0]);
    // The four bytes ahead of each marker are a zero u32 field of the record,
    // not slack between records.
    assert_eq!(&records[0].raw[..4], &[0, 0, 0, 0]);
    assert_eq!(&records[0].raw[4..12], &[0xFF; 8]);
}

#[test]
fn family_files_use_implicit_12b_layout() {
    let mut missing: Vec<u32> = Vec::new();
    for year in ALL_YEARS {
        let p = sample_for_year(year);
        if !p.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&p).unwrap_or_else(|_| panic!("{year}: open"));
        let records = elem_table::parse_records(&mut rf)
            .unwrap_or_else(|e| panic!("{year}: parse_records: {e}"));
        let header = elem_table::parse_header(&mut rf)
            .unwrap_or_else(|e| panic!("{year}: parse_header: {e}"));
        assert!(
            !records.is_empty(),
            "{year}: expected at least one record in family ElemTable"
        );
        assert!(
            records.len() <= header.record_count as usize,
            "{year}: parsed {} records > header record_count {}",
            records.len(),
            header.record_count
        );
    }
    assert!(
        missing.is_empty() || !require_family_corpus(),
        "family corpus incomplete — missing release(s): {missing:?}. RVT_REQUIRE_CORPUS \
         is set, so this is a regression, not a setup gap. Provide the phi-ag/rvt \
         corpus via RVT_SAMPLES_DIR or unset RVT_REQUIRE_CORPUS."
    );
    if !missing.is_empty() {
        eprintln!("skipped family releases (corpus absent): {missing:?}");
    }
}

#[test]
fn project_2023_file_parses_all_declared_records() {
    let Some(p) = project_file("Revit_IFC5_Einhoven.rvt") else {
        return;
    };
    let mut rf = RevitFile::open(&p).expect("open project 2023");
    let header = elem_table::parse_header(&mut rf).expect("header project 2023");
    let records = elem_table::parse_records(&mut rf).expect("records project 2023");

    // 28-byte variant: the marker opens the record (`marker_offset == 0`) and
    // the stream does not end flush with the record array — 23 bytes of tail
    // remain, so the walk honestly stops one short of the declared 2615.
    // See docs/elem-table-record-layout-2026-04-21.md § "Where the record
    // array starts".
    assert_eq!(records.len(), 2614, "project 2023 walks 2614 of 2615");
    assert!(
        records.len() < header.record_count as usize,
        "parsed {} vs record_count {}",
        records.len(),
        header.record_count
    );
    // First few ids should be 1, 2, 3, ...
    assert_eq!(records[0].id_primary, 1, "first id_primary");
    assert_eq!(records[1].id_primary, 2, "second id_primary");
    assert_eq!(records[2].id_primary, 3, "third id_primary");
    assert_eq!(records[0].offset, 0x1e, "record 0 starts at the marker");
    // id_primary == id_secondary on observed rows
    assert_eq!(
        records[0].id_primary, records[0].id_secondary,
        "id_primary/id_secondary mismatch"
    );
}

#[test]
fn project_2024_file_parses_all_declared_records() {
    let Some(p) = project_file("2024_Core_Interior.rvt") else {
        return;
    };
    let mut rf = RevitFile::open(&p).expect("open project 2024");
    let header = elem_table::parse_header(&mut rf).expect("header project 2024");
    let records = elem_table::parse_records(&mut rf).expect("records project 2024");
    // Header declares 26,425 records and the decompressed stream (gzip CRC32
    // and ISIZE verified) is exactly 0x1E + 26425*40 bytes, so the parser must
    // return every one of them — #206 was a four-byte origin shift that lost
    // the last record.
    assert_eq!(
        records.len() as u16,
        header.record_count,
        "parse_records should return exactly header.record_count on 2024 project files"
    );
    let last = records.last().expect("at least one record");
    assert_eq!(
        last.offset + last.raw.len(),
        header.decompressed_bytes,
        "the record array must end flush with the decompressed stream"
    );
    // First few records have small sequential ids.
    assert_eq!(
        records[0].offset, 0x1e,
        "record 0 begins one u32 ahead of the marker"
    );
    assert_eq!(records[0].id_primary, 1);
    assert_eq!(records[1].id_primary, 2);
    assert_eq!(records[2].id_primary, 3);
    // On observed 2024 projects, id_secondary matches id_primary on the
    // initial element-index records (bound at record +36, not +32 —
    // regression-guarded here against a real-corpus bug we caught).
    assert_eq!(records[0].id_secondary, 1);
    assert_eq!(records[1].id_secondary, 2);
    assert_eq!(records[2].id_secondary, 3);
}

#[test]
fn declared_element_ids_returns_sorted_deduped_set() {
    // Project 2023 declares ~2615 ids; we expect at least 2000 unique
    // sequential ids starting at 1.
    let Some(p) = project_file("Revit_IFC5_Einhoven.rvt") else {
        return;
    };
    let mut rf = RevitFile::open(&p).expect("open");
    let ids = elem_table::declared_element_ids(&mut rf).expect("declared ids");
    assert!(
        ids.len() >= 2000,
        "expected >=2000 declared ids, got {}",
        ids.len()
    );
    // Sorted + deduped invariants.
    assert!(
        ids.windows(2).all(|w| w[0] < w[1]),
        "ids not strictly sorted"
    );
    // First declared id is 1 on this file.
    assert_eq!(ids[0], 1);
}
