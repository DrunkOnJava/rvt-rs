//! `Global/ElemTable` read as the table it is (#152): a `u16` tag, a `u32`
//! record count at `0x02`, and that many records from `0x06`, 28 bytes each
//! through Revit 2023 and 40 from 2024, each closing with its owner
//! (STE1200's frame, measured by puzzbobb on 30 files from 2008 to 2027).
//!
//! What every file of the family corpus (2016 to 2026), the two MIT projects
//! (Einhoven 2023, Core Interior 2024) and, when they are in the project
//! corpus directory, Autodesk's Snowdon Towers samples (2024, not
//! redistributable) must give, from that description alone:
//!
//! 1. `parse_records` returns the count the table states, no more and no
//!    fewer.
//! 2. The records' ElementIds rise strictly: the table is in ascending id
//!    order, so no record is read from the bytes after the last one. The
//!    ElementId is the record's second id (`id_secondary`); the first agrees
//!    with it except on Autodesk's Snowdon Towers samples, where it does not
//!    rise.
//! 3. Every owner a record names is an ElementId of the same table, no
//!    record owns itself and no owner chain loops.
//! 4. On a family file the first record's owner is element 17, the value
//!    `parse_header` reports as `header_flag`.
//!
//! Corpus resolution follows `tests/elem_table_corpus.rs`: a configured but
//! incomplete corpus fails, an absent one skips and says so.

mod common;

use common::{ALL_YEARS, sample_for_year};
use rvt::{RevitFile, compression, elem_table, streams::GLOBAL_ELEM_TABLE};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn require_family_corpus() -> bool {
    std::env::var("RVT_REQUIRE_CORPUS")
        .ok()
        .is_some_and(|v| v == "1" || v == "true")
}

fn project_file(name: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)?;
    let path = dir.join(name);
    assert!(
        path.exists(),
        "RVT_PROJECT_CORPUS_DIR is set to {} but {name} is not there",
        dir.display()
    );
    Some(path)
}

/// The record count the table states, the `u32` at `0x02`.
fn stated_count(path: &Path) -> usize {
    let mut rf = RevitFile::open(path).expect("open");
    let raw = rf.read_stream(GLOBAL_ELEM_TABLE).expect("ElemTable stream");
    let d = compression::inflate_stream_at(GLOBAL_ELEM_TABLE, &raw, 8).expect("inflate");
    u32::from_le_bytes(d[2..6].try_into().expect("four bytes")) as usize
}

/// Checks 1 to 3 on one file; returns the records for file-specific checks.
fn check_table(path: &Path, label: &str) -> Vec<elem_table::ElemRecord> {
    let count = stated_count(path);
    let mut rf = RevitFile::open(path).expect("open");
    let records = elem_table::parse_records(&mut rf).expect("parse_records");

    assert_eq!(
        records.len(),
        count,
        "{label}: parse_records returns the {count} records the table states"
    );
    if let Some(at) = records
        .windows(2)
        .position(|w| w[1].id_secondary <= w[0].id_secondary)
    {
        panic!(
            "{label}: ElementIds rise strictly, but record {} has id {} after {} (of {})",
            at + 1,
            records[at + 1].id_secondary,
            records[at].id_secondary,
            records.len()
        );
    }

    let ids: BTreeSet<u32> = records.iter().map(|r| r.id_secondary).collect();
    let owners: BTreeMap<u32, u32> = records
        .iter()
        .filter_map(|r| r.owner_id.map(|owner| (r.id_secondary, owner)))
        .collect();
    let undeclared: Vec<_> = owners.values().filter(|o| !ids.contains(o)).collect();
    assert!(
        undeclared.is_empty(),
        "{label}: every owner is an id of the table; not declared: {:?}",
        &undeclared[..undeclared.len().min(10)]
    );
    let self_owned: Vec<_> = owners.iter().filter(|(id, owner)| id == owner).collect();
    assert!(
        self_owned.is_empty(),
        "{label}: no record owns itself: {self_owned:?}"
    );
    for &start in owners.keys() {
        let mut at = start;
        for _ in 0..=owners.len() {
            match owners.get(&at) {
                Some(&next) => {
                    assert_ne!(next, start, "{label}: owner chain from {start} loops");
                    at = next;
                }
                None => break,
            }
        }
    }
    records
}

#[test]
fn family_tables_hold_every_stated_record() {
    let mut missing = Vec::new();
    for year in ALL_YEARS {
        let path = sample_for_year(year);
        if !path.exists() {
            missing.push(year);
            continue;
        }
        let label = format!("{year} family");
        let records = check_table(&path, &label);
        assert_eq!(
            records.first().and_then(|r| r.owner_id),
            Some(17),
            "{label}: the first record's owner is element 17"
        );
    }
    assert!(
        missing.is_empty() || !require_family_corpus(),
        "family corpus incomplete, missing {missing:?}, and RVT_REQUIRE_CORPUS is set"
    );
    if !missing.is_empty() {
        eprintln!("skipped family releases (corpus absent): {missing:?}");
    }
}

#[test]
fn project_tables_hold_every_stated_record() {
    let mut read = 0;
    for name in ["Revit_IFC5_Einhoven.rvt", "2024_Core_Interior.rvt"] {
        if let Some(path) = project_file(name) {
            check_table(&path, name);
            read += 1;
        }
    }
    if read == 0 {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
    }
}

/// Autodesk's Snowdon Towers samples cannot be redistributed, so they are
/// read only when someone has put them in the project corpus directory.
/// Their first and second ids differ on 27 (Structural) and 84
/// (Architectural) records, and the first does not rise there: a layout
/// test on the first id took the 12-byte fallback and the export lost every
/// element.
#[test]
fn snowdon_tables_rise_by_element_id() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for (name, differing) in [
        ("Snowdon Towers Sample Structural.rvt", 27),
        ("Snowdon Towers Sample Architectural.rvt", 84),
    ] {
        let path = dir.join(name);
        if !path.exists() {
            eprintln!("skipping {name}: not in RVT_PROJECT_CORPUS_DIR");
            continue;
        }
        let records = check_table(&path, name);
        assert!(
            records
                .iter()
                .all(|r| r.raw.len() == elem_table::RECORD_LEN_40),
            "{name}: read as the 40-byte records of a 2024 project"
        );
        assert_eq!(
            records
                .iter()
                .filter(|r| r.id_primary != r.id_secondary)
                .count(),
            differing,
            "{name}: records whose two ids differ"
        );
    }
}
