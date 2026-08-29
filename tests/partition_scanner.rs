//! Integration tests for the generic partition scanner (M3-03 / M3-04).
//!
//! Tier-1 synthetic fixtures exercise the version guard and empty-stream
//! path. Optional magnetar corpus tests skip without `RVT_PROJECT_CORPUS_DIR`.

use rvt::arc_wall_record::ArcWallRecord;
use rvt::partition_scanner::{
    PartitionScanStatus, ScanOptions, arcwall_standard_offsets, declared_but_unlocated_ids,
    element_id_partition_index, iter_partition_candidates, link_elem_table_to_partitions,
    linkage_coverage, scan_partitions, scanner_status, supports_revit_version,
};
use rvt::{RevitFile, compression, elem_table};
use std::path::{Path, PathBuf};

fn tier1_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/tier1")
}

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/rvt-corpus-probe/magnetar/Revit"))
}

#[test]
fn tier1_structural_2023_scanner_supported_empty_partitions() {
    let path = tier1_dir().join("structural-2023/structural-2023.rvt");
    assert!(path.exists(), "missing tier1 fixture {}", path.display());

    let mut rf = RevitFile::open(&path).expect("open structural-2023");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    assert_eq!(version, 2023);
    assert!(supports_revit_version(version));

    let scan = scan_partitions(&mut rf, version, &ScanOptions::default()).expect("scan");
    assert_eq!(
        scan.status,
        PartitionScanStatus::Supported {
            revit_version: 2023
        }
    );
    // Synthetic fixtures ship empty Partitions/NN streams — no false positives.
    assert!(
        scan.candidates.is_empty(),
        "tier1 structural-2023 must not invent partition candidates, got {}",
        scan.candidates.len()
    );
}

#[test]
fn tier1_architectural_2024_scanner_supported() {
    let path = tier1_dir().join("architectural-2024/architectural-2024.rvt");
    assert!(path.exists(), "missing tier1 fixture {}", path.display());

    let mut rf = RevitFile::open(&path).expect("open architectural-2024");
    let scan = iter_partition_candidates(&mut rf).expect("scan");
    assert_eq!(
        scan.status,
        PartitionScanStatus::Supported {
            revit_version: 2024
        }
    );
    assert!(scan.candidates.is_empty());
}

#[test]
fn tier1_mep_2024_elem_table_link_is_empty_without_ids() {
    let path = tier1_dir().join("mep-2024/mep-2024.rvt");
    assert!(path.exists(), "missing tier1 fixture {}", path.display());

    let mut rf = RevitFile::open(&path).expect("open mep-2024");
    let scan = iter_partition_candidates(&mut rf).expect("scan");
    let partition_index = element_id_partition_index(&scan.candidates);
    assert!(partition_index.is_empty());

    let declared = elem_table::declared_element_ids(&mut rf).unwrap_or_default();
    let missing = declared_but_unlocated_ids(&declared, &partition_index);
    assert_eq!(missing.len(), declared.len());
    assert_eq!(linkage_coverage(&declared, &partition_index), 0.0);
}

#[test]
fn einhoven_generic_scanner_matches_arcwall_find_all() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    assert_eq!(version, 2023);

    let raw = rf.read_stream("Partitions/5").expect("Partitions/5");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();
    let direct = ArcWallRecord::find_all(&concat);

    let scan = scan_partitions(&mut rf, version, &ScanOptions::arcwall_2023_only())
        .expect("arcwall-only scan");
    assert!(scan.status.is_supported());
    let from_part5: Vec<_> = scan
        .candidates
        .iter()
        .filter(|c| c.stream == "Partitions/5")
        .cloned()
        .collect();
    let scanner_offsets = arcwall_standard_offsets(&from_part5);
    assert_eq!(
        scanner_offsets, direct,
        "generic scanner ArcWall path must match ArcWallRecord::find_all"
    );
    assert!(
        scanner_offsets.len() >= 10,
        "expected ≥10 ArcWall candidates on Einhoven Partitions/5"
    );
}

#[test]
fn einhoven_elem_table_linkage_coverage_for_arcwall_ids() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let scan = scan_partitions(&mut rf, version, &ScanOptions::arcwall_2023_only()).expect("scan");
    let partition_index = element_id_partition_index(&scan.candidates);
    assert!(
        partition_index.len() >= 20,
        "expected ≥20 ArcWall ElementIds from scanner, got {}",
        partition_index.len()
    );

    let records = elem_table::parse_records(&mut rf).expect("ElemTable");
    let elem_index = elem_table::index_by_element_id(&records);
    let linked = link_elem_table_to_partitions(&elem_index, &partition_index);
    assert_eq!(
        linked.len(),
        partition_index.len(),
        "every recovered ArcWall ElementId should hit ElemTable"
    );

    // Of recovered ArcWall ElementIds, ≥80% must land in ElemTable.
    // On Einhoven RE-15 observed 23/24 standard walls validate (~96%).
    let recovered_count = partition_index.len();
    let linked_count = linked.len();
    let coverage = linked_count as f64 / recovered_count as f64;
    assert!(
        coverage >= 0.80,
        "wall ElementId → ElemTable coverage {coverage:.3} ({linked_count}/{recovered_count}) < 0.80"
    );
    assert!(linked_count >= 20);

    for link in &linked {
        assert_eq!(link.partition_ref.partition, "Partitions/5");
        assert!(elem_index.contains_key(&link.element_id));
    }
}

#[test]
fn core_interior_2024_runs_scanner_without_2023_arcwall_false_positives() {
    let path = project_dir().join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} not present", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    assert_eq!(version, 2024);
    assert_eq!(
        scanner_status(version),
        PartitionScanStatus::Supported {
            revit_version: 2024
        }
    );

    // ArcWall-2023 allowlist on a 2024 file: tag 0x0191 may still appear
    // as noise, but the 0.85 envelope gate should keep standard ArcWall
    // offsets empty (decode_standard requires 2023 variant 0x07fa).
    let scan = scan_partitions(&mut rf, version, &ScanOptions::arcwall_2023_only()).expect("scan");
    let high = arcwall_standard_offsets(&scan.candidates);
    assert!(
        high.is_empty(),
        "2024 file must not emit 2023 ArcWall-standard candidates via generic scanner"
    );
}

#[test]
fn fixture_path_helper_exists() {
    // Keep the tier1 root discoverable for future lanes.
    assert!(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("corpus/tier1")
            .is_dir()
    );
}
