//! Integration test — decode real ArcWall records from Einhoven
//! Partitions/5. Ships task DEC-05 ("IfcWall count > 0 on real file")
//! and validates the RE-14.3 wire format against the same corpus
//! the probe was built against.
//!
//! Skips gracefully when `RVT_PROJECT_CORPUS_DIR` is unset or the
//! file isn't present — Autodesk sample files are not redistributed
//! by this crate.

use rvt::arc_wall_record::{
    ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD, ArcWallRecord, ArcWallScanStatus,
};
use rvt::partition_arc_walls::{
    element_id_partition_index, iter_partition_arc_walls, storeys_from_arc_wall_base_elevations,
};
use rvt::{RevitFile, compression, elem_table};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/rvt-corpus-probe/magnetar/Revit"))
}

#[test]
fn einhoven_partitions_5_yields_decodable_arcwalls() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!(
            "skipping arc_wall corpus test: {} not present",
            path.display()
        );
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let version = rf.basic_file_info().expect("read BasicFileInfo").version;
    let raw = rf.read_stream("Partitions/5").expect("read Partitions/5");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();
    assert!(
        concat.len() > 1_000,
        "Einhoven Partitions/5 decompressed too small: {} B",
        concat.len()
    );

    // Scan for standard-variant ArcWall records.
    let scan = ArcWallRecord::scan_standard_for_revit_version(version, &concat);
    assert_eq!(
        scan.status,
        ArcWallScanStatus::Supported {
            revit_version: 2023
        }
    );
    let offsets = scan.offsets;
    assert!(
        offsets.len() >= 10,
        "expected ≥10 standard ArcWall records on Einhoven Partitions/5, \
         found only {}. RE-14.3 observed 26 standard walls + 2 compound + \
         4 metadata/index records = 32 total",
        offsets.len()
    );

    // Decode each and sanity-check.
    let mut decoded = 0usize;
    let mut coords_match_count = 0usize;
    for &off in &offsets {
        let rec = ArcWallRecord::decode_standard(&concat, off)
            .unwrap_or_else(|e| panic!("offset {off} must decode: {e}"));
        assert_eq!(rec.tag, ARC_WALL_TAG);
        assert_eq!(rec.variant, ARC_WALL_VARIANT_STANDARD);
        for c in &rec.coords {
            assert!(c.is_finite(), "coord must be finite at offset {off}: {c}");
        }
        if rec.coords_match() {
            coords_match_count += 1;
        }
        decoded += 1;
    }
    assert_eq!(
        decoded,
        offsets.len(),
        "every find_all offset should decode cleanly"
    );
    assert!(
        coords_match_count > 0,
        "expected ≥1 record with coords matching coords_dup — RE-14.3 observed \
         ~80% of records have this property"
    );
    eprintln!(
        "[arc_wall_corpus] Einhoven Partitions/5: {} ArcWall records decoded, \
         {} with matching coords/coords_dup",
        decoded, coords_match_count
    );
}

#[test]
fn einhoven_partitions_0_has_no_arcwalls() {
    // RE-14.2 observed ArcWall only in Partitions/5 on Einhoven,
    // zero in Partitions/0. This test pins that finding — breakage
    // would indicate either (a) our scanner false-positives or (b)
    // the corpus file changed.
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!(
            "skipping arc_wall Partitions/0 test: {} not present",
            path.display()
        );
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let raw = rf.read_stream("Partitions/0").expect("read Partitions/0");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    let version = rf.basic_file_info().expect("read BasicFileInfo").version;
    let offsets = ArcWallRecord::scan_standard_for_revit_version(version, &concat).offsets;
    assert_eq!(
        offsets.len(),
        0,
        "RE-14.2 observed zero ArcWall records on Einhoven Partitions/0 — \
         got {}",
        offsets.len()
    );
}

#[test]
fn core_interior_2024_suppresses_2023_arcwall_pattern() {
    let path = project_dir().join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!(
            "skipping 2024 arc_wall guard test: {} not present",
            path.display()
        );
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open 2024 Core Interior");
    let version = rf.basic_file_info().expect("read BasicFileInfo").version;
    assert_eq!(version, 2024, "fixture should identify as Revit 2024");

    let raw = match rf.read_stream("Partitions/46") {
        Ok(raw) => raw,
        Err(e) => {
            eprintln!("skipping 2024 arc_wall guard test: cannot read Partitions/46: {e}");
            return;
        }
    };
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    let scan = ArcWallRecord::scan_standard_for_revit_version(version, &concat);
    assert_eq!(
        scan.status,
        ArcWallScanStatus::UnsupportedVersion {
            revit_version: 2024
        }
    );
    assert!(
        scan.offsets.is_empty(),
        "2024 Core Interior must not run the 2023 ArcWall decoder"
    );
}

#[test]
fn einhoven_shared_partition_api_recovers_trailer_ids_and_storeys() {
    let path = project_dir().join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!(
            "skipping shared partition ArcWall API test: {} not present",
            path.display()
        );
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let scan = iter_partition_arc_walls(&mut rf).expect("scan partitions");
    assert!(
        scan.walls.len() >= 10,
        "shared API should see the same ArcWall population as the IFC path"
    );

    let with_ids = scan
        .walls
        .iter()
        .filter(|w| w.element_id().is_some())
        .count();
    assert!(
        with_ids >= 20,
        "RE-15: expected ≥20 validated trailer ElementIds, got {with_ids}"
    );
    assert!(
        scan.walls.iter().all(|w| w.thickness_feet().is_none()),
        "RE-15: thickness is not in the ArcWall trailer"
    );
    assert!(
        scan.walls
            .iter()
            .filter(|w| w.height_feet().is_some())
            .count()
            >= 20,
        "RE-15: height comes from core Z delta"
    );

    let storeys = storeys_from_arc_wall_base_elevations(&scan.walls);
    assert!(
        storeys.len() >= 2,
        "distinct base elevations should yield real storeys, got {}",
        storeys.len()
    );

    let level_names = {
        let records =
            rvt::object_graph::string_records_from_partitions(&mut rf).expect("partition strings");
        rvt::partition_name_candidates::building_storey_name_candidates(
            records.iter().map(|r| r.value.as_str()),
        )
    };
    assert!(
        level_names.iter().any(|n| n == "Level 1"),
        "RE-15/#86: Einhoven partition strings should include Level 1, got {level_names:?}"
    );
    let recovery =
        rvt::partition_arc_walls::recover_storeys_from_arc_walls(&scan.walls, &level_names);
    assert!(
        recovery.named_from_partition >= 1,
        "at least one storey should receive a partition Level name"
    );
    assert!(
        recovery
            .storeys
            .iter()
            .any(|s| s.name == "Level 1" || s.name == "Roof"),
        "expected Level 1 and/or Roof on named storeys: {:?}",
        recovery.storeys
    );

    let partition_index = element_id_partition_index(&scan.walls);
    let elem_records = elem_table::parse_records(&mut rf).expect("ElemTable");
    let elem_index = elem_table::index_by_element_id(&elem_records);
    let linked = elem_table::link_arcwall_element_ids(&elem_index, &partition_index);
    assert!(
        linked.len() >= 20,
        "RE-15: ElementId → ElemTable ∩ partition ref join should cover ≥20 walls, got {}",
        linked.len()
    );
    for link in &linked {
        assert_eq!(link.partition_ref.partition, "Partitions/5");
        assert!(elem_index.contains_key(&link.element_id));
    }
}
