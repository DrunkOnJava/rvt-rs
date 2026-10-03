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

/// Regression for #206 and #152 on committed bytes, no corpus required.
///
/// The records start at `0x06`, right after the `u16` tag and the `u32`
/// record count, and a 40-byte record closes with a zero `u32` and its
/// `u64` owner (`FF`×8 when unset). The record array is `record_count ×
/// stride` bytes and a 24-byte tail follows it to the end of the stream.
/// Framing from `0x1E` (24 bytes into each record) missed the first record
/// and read a last one, id 0, from the tail.
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
        layout.start, 0x06,
        "record 0 begins after the tag and count"
    );
    assert_eq!(layout.marker_offset, 28, "the owner closes the record");
    assert_eq!(
        layout.start + declared * layout.stride + 24,
        d.len(),
        "a 24-byte tail follows the record array"
    );

    let records = elem_table::parse_records_from_bytes(d, layout, declared);
    assert_eq!(records.len(), declared, "every declared record is walked");
    let offsets: Vec<usize> = records.iter().map(|r| r.offset).collect();
    assert_eq!(offsets, vec![0x06, 0x2e, 0x56, 0x7e, 0xa6, 0xce]);
    let ids: Vec<u32> = records.iter().map(|r| r.id_primary).collect();
    assert_eq!(ids, vec![0, 1, 2, 3, 133_205, 133_206]);
    let secondaries: Vec<u32> = records.iter().map(|r| r.id_secondary).collect();
    assert_eq!(secondaries, vec![0, 1, 2, 3, 133_205, 133_206]);
    // The record closes with a zero u32 field and the unset owner.
    assert_eq!(&records[0].raw[24..28], &[0, 0, 0, 0]);
    assert_eq!(&records[0].raw[28..36], &[0xFF; 8]);
    assert_eq!(records[0].owner_id, None);
}

#[test]
fn family_files_use_the_project_layouts() {
    // Families use the project records: 28 bytes through 2023, 40 bytes
    // from 2024, record 0 at 0x06. Ids start at 0, then 1 (2 is never
    // used), rise over the whole table, and equal their secondary copy.
    let mut missing: Vec<u32> = Vec::new();
    for year in ALL_YEARS {
        let p = sample_for_year(year);
        if !p.exists() {
            missing.push(year);
            continue;
        }
        let mut rf = RevitFile::open(&p).unwrap_or_else(|_| panic!("{year}: open"));
        let header = elem_table::parse_header(&mut rf)
            .unwrap_or_else(|e| panic!("{year}: parse_header: {e}"));
        let layout = elem_table::read_layout(&mut rf).unwrap_or_else(|e| panic!("{year}: {e}"));
        let records = elem_table::parse_records(&mut rf)
            .unwrap_or_else(|e| panic!("{year}: parse_records: {e}"));
        assert_eq!(layout.stride, if year >= 2024 { 40 } else { 28 }, "{year}");
        assert_eq!(layout.start, 0x06, "{year}");
        assert_eq!(records.len(), header.record_count as usize, "{year}");
        assert_eq!(
            [
                records[0].id_primary,
                records[1].id_primary,
                records[2].id_primary
            ],
            [0, 1, 3],
            "{year}"
        );
        assert!(
            records
                .windows(2)
                .all(|w| w[1].id_primary > w[0].id_primary),
            "{year}: ids rise"
        );
        assert!(
            records[1..].iter().all(|r| r.id_primary == r.id_secondary),
            "{year}: id pairs agree"
        );
        // The 17 at 0x1E / 0x22 closes the table's first record, at 0x06
        // (#152); the second record's owner field is unset.
        assert_eq!(records[0].owner_id, Some(17), "{year}: record 0's owner");
        assert_eq!(records[1].owner_id, None, "{year}: record 1's owner");
        assert_eq!(header.header_flag, 0x0011, "{year}");
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

    // 28-byte variant: the records start at 0x06 and close with their
    // owner, and a 19-byte tail follows the last of the declared 2615
    // (#152). The first record is AllProjectPhases, element 0 on this file.
    assert_eq!(records.len(), 2615, "project 2023 walks all 2615");
    assert_eq!(
        records.len(),
        header.record_count as usize,
        "parsed {} vs record_count {}",
        records.len(),
        header.record_count
    );
    // The ids are 0, 1, 2, 3, ...
    assert_eq!(records[0].id_primary, 0, "first id_primary");
    assert_eq!(records[1].id_primary, 1, "second id_primary");
    assert_eq!(records[2].id_primary, 2, "third id_primary");
    assert_eq!(records[3].id_primary, 3, "fourth id_primary");
    assert_eq!(records[0].offset, 0x06, "record 0 starts after the count");
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
    // and ISIZE verified) is exactly 0x06 + 26425*40 + 24 bytes, so the
    // parser must return every one of them — #206 was a four-byte origin
    // shift that lost the last record, and #152 the 24-byte frame offset
    // that missed the first.
    assert_eq!(
        records.len() as u16,
        header.record_count,
        "parse_records should return exactly header.record_count on 2024 project files"
    );
    let last = records.last().expect("at least one record");
    assert_eq!(
        last.offset + last.raw.len() + 24,
        header.decompressed_bytes,
        "a 24-byte tail follows the record array"
    );
    // The first records have small sequential ids; the first is
    // AllProjectPhases, element 0 on this file.
    assert_eq!(records[0].offset, 0x06, "record 0 begins after the count");
    assert_eq!(records[0].id_primary, 0);
    assert_eq!(records[1].id_primary, 1);
    assert_eq!(records[2].id_primary, 2);
    assert_eq!(records[3].id_primary, 3);
    // On observed 2024 projects, id_secondary matches id_primary on the
    // initial element-index records (bound at record +20 from 0x06, which
    // was +36 in the old 0x1E framing — regression-guarded here against a
    // real-corpus bug we caught).
    assert_eq!(records[1].id_secondary, 1);
    assert_eq!(records[2].id_secondary, 2);
    assert_eq!(records[3].id_secondary, 3);
}

/// RE-31: each record closes with an owner ElementId (`u32` at `+24` on
/// 2023, `u64` at `+28` on 2024, from the record's start at `0x06`, #152).
/// Every value set on these two files is an ElementId the same table
/// declares, and no record names itself.
#[test]
fn owner_ids_are_declared_element_ids() {
    for (name, set, parsed) in [
        ("Revit_IFC5_Einhoven.rvt", 339, 2615),
        ("2024_Core_Interior.rvt", 22_368, 26_425),
    ] {
        let Some(p) = project_file(name) else {
            return;
        };
        let mut rf = RevitFile::open(&p).expect("open project");
        let records = elem_table::parse_records(&mut rf).expect("records");
        assert_eq!(records.len(), parsed, "{name}");
        let declared: std::collections::BTreeSet<u32> =
            records.iter().map(|r| r.id_primary).collect();
        let owners: Vec<u32> = records.iter().filter_map(|r| r.owner_id).collect();
        assert_eq!(owners.len(), set, "{name}: records naming an owner");
        assert!(
            owners.iter().all(|id| declared.contains(id)),
            "{name}: every owner is a declared ElementId"
        );
        assert!(
            records.iter().all(|r| r.owner_id != Some(r.id_primary)),
            "{name}: no record names itself"
        );
    }
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
