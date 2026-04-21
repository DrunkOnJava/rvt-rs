//! Integration tests for `elem_table::parse_records` against the
//! 11-release family corpus AND the project-file corpus. The project
//! corpus path is resolved via `RVT_PROJECT_CORPUS_DIR` env var
//! (defaults to `/private/tmp/rvt-corpus-probe/magnetar/Revit`) and
//! tests skip gracefully if the path doesn't exist — we do not
//! redistribute Autodesk-owned files.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::{RevitFile, elem_table};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

#[test]
fn family_files_use_implicit_12b_layout() {
    for year in ALL_YEARS {
        let p = sample_for_year(year);
        if !p.exists() {
            eprintln!("skipping {year}: family sample not present");
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
}

#[test]
fn project_2023_file_parses_all_declared_records() {
    let p = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !p.exists() {
        eprintln!("skipping: project 2023 corpus not present at {}", p.display());
        return;
    }
    let mut rf = RevitFile::open(&p).expect("open project 2023");
    let header = elem_table::parse_header(&mut rf).expect("header project 2023");
    let records = elem_table::parse_records(&mut rf).expect("records project 2023");

    // Header declares 2615 records; we parse 2614 cleanly (last may be a trailer).
    assert!(
        records.len() >= 2000,
        "expected 2000+ records, got {}",
        records.len()
    );
    assert!(
        records.len() <= header.record_count as usize,
        "parsed {} > record_count {}",
        records.len(),
        header.record_count
    );
    // First few ids should be 1, 2, 3, ...
    assert_eq!(records[0].id_primary, 1, "first id_primary");
    assert_eq!(records[1].id_primary, 2, "second id_primary");
    assert_eq!(records[2].id_primary, 3, "third id_primary");
    // id_primary == id_secondary on observed rows
    assert_eq!(
        records[0].id_primary, records[0].id_secondary,
        "id_primary/id_secondary mismatch"
    );
}

#[test]
fn project_2024_file_parses_all_declared_records() {
    let p = project_dir().join("2024_Core_Interior.rvt");
    if !p.exists() {
        eprintln!("skipping: project 2024 corpus not present at {}", p.display());
        return;
    }
    let mut rf = RevitFile::open(&p).expect("open project 2024");
    let header = elem_table::parse_header(&mut rf).expect("header project 2024");
    let records = elem_table::parse_records(&mut rf).expect("records project 2024");
    // Header declares 26,425 records; the corrected parser returns exactly
    // that count (previously returned 2 due to sentinel early-termination).
    assert_eq!(
        records.len() as u16,
        header.record_count,
        "parse_records should return exactly header.record_count on 2024 project files"
    );
    // First few records have small sequential ids.
    assert_eq!(records[0].id_primary, 1);
    assert_eq!(records[1].id_primary, 2);
    assert_eq!(records[2].id_primary, 3);
}
