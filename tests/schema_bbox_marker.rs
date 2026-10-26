//! #421, item 1: the element-record marker comes from the file's own schema,
//! and on every file measured it equals the release's constant.
//!
//! The marker is `[tag of Outline][0xFF x 4][tag of ElementParents]` (RE-80).
//! On each 2024 to 2026 family of the phi-ag corpus (`RVT_SAMPLES_DIR`) and
//! each project under `RVT_PROJECT_CORPUS_DIR`:
//!
//! - `schema_bbox_marker` equals `bbox_marker` for the file's release;
//! - `file_bbox_marker` returns it;
//! - `record_prologue_constant` of it is the schema's `ElementHeader` tag,
//!   the `u16` every record chain header carries (RE-35).
//!
//! On 2016 to 2023, which have no constant, the gap from `ElementParents`
//! to `ElementHeader` is checked instead: 9 on 2016 to 2018, 12 from 2019,
//! so `record_prologue_constant` holds only from 2019.
//!
//! Skips what is absent.

mod common;

use common::sample_for_year;
use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use std::path::{Path, PathBuf};

/// The schema's marker against the release's constant on `path`, or `None`
/// for a release with no constant.
fn check(path: &Path) -> Option<u32> {
    let mut rf = RevitFile::open(path).expect("open");
    let release = rf.basic_file_info().expect("BasicFileInfo").version;
    let measured = per::bbox_marker(release)?;
    let (schema, header_tag) =
        rvt::research::partition_element_records_2023::record_marker(&mut rf)
            .unwrap_or_else(|| panic!("{}: schema gives no marker", path.display()));
    assert_eq!(schema, measured, "{}: schema marker", path.display());
    assert_eq!(
        per::file_bbox_marker(&mut rf, release),
        Some(schema),
        "{}: file_bbox_marker",
        path.display()
    );
    assert_eq!(
        per::record_prologue_constant(&schema),
        header_tag,
        "{}: ElementHeader tag",
        path.display()
    );
    Some(release)
}

#[test]
fn schema_marker_equals_constant_on_families() {
    let mut checked = Vec::new();
    for year in [2024, 2025, 2026] {
        let path = sample_for_year(year);
        if !path.exists() {
            eprintln!("skip: {} absent", path.display());
            continue;
        }
        assert_eq!(check(&path), Some(year), "{}", path.display());
        checked.push(year);
    }
    eprintln!("families checked: {checked:?}");
}

#[test]
fn schema_marker_equals_constant_on_projects() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skip: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("read RVT_PROJECT_CORPUS_DIR") {
        let path = entry.expect("dir entry").path();
        if path.extension().is_some_and(|e| e == "rvt") && check(&path).is_some() {
            checked += 1;
        }
    }
    eprintln!("projects checked: {checked}");
}

#[test]
fn header_tag_gap_is_9_before_2019_and_12_after() {
    let mut checked = Vec::new();
    for year in 2016..=2023 {
        let path = sample_for_year(year);
        if !path.exists() {
            eprintln!("skip: {} absent", path.display());
            continue;
        }
        let mut rf = RevitFile::open(&path).expect("open");
        let (marker, header_tag) =
            rvt::research::partition_element_records_2023::record_marker(&mut rf)
                .unwrap_or_else(|| panic!("{}: schema gives no marker", path.display()));
        let parents = u16::from_le_bytes([marker[6], marker[7]]);
        let gap = if year < 2019 { 9 } else { 12 };
        assert_eq!(parents - header_tag, gap, "{}", path.display());
        checked.push(year);
    }
    eprintln!("families checked: {checked:?}");
}
