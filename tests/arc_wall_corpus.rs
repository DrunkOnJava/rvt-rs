//! Integration test — decode real ArcWall records from Einhoven
//! Partitions/5. Ships task DEC-05 ("IfcWall count > 0 on real file")
//! and validates the RE-14.3 wire format against the same corpus
//! the probe was built against.
//!
//! Skips gracefully when `RVT_PROJECT_CORPUS_DIR` is unset or the
//! file isn't present — Autodesk sample files are not redistributed
//! by this crate.

use rvt::arc_wall_record::{ArcWallRecord, ARC_WALL_TAG, ARC_WALL_VARIANT_STANDARD};
use rvt::{RevitFile, compression};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
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
    let raw = rf
        .read_stream("Partitions/5")
        .expect("read Partitions/5");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();
    assert!(
        concat.len() > 1_000,
        "Einhoven Partitions/5 decompressed too small: {} B",
        concat.len()
    );

    // Scan for standard-variant ArcWall records.
    let offsets = ArcWallRecord::find_all(&concat);
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
    let raw = rf
        .read_stream("Partitions/0")
        .expect("read Partitions/0");
    let chunks = compression::inflate_all_chunks(&raw);
    let concat: Vec<u8> = chunks.into_iter().flatten().collect();

    let offsets = ArcWallRecord::find_all(&concat);
    assert_eq!(
        offsets.len(),
        0,
        "RE-14.2 observed zero ArcWall records on Einhoven Partitions/0 — \
         got {}",
        offsets.len()
    );
}
