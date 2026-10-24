//! RE-213: Revit 2025 frames a wall type's face angles (RE-86) with twelve
//! zero bytes after its `u32 2`, and its tapered walls are drawn tapered.
//!
//! On Autodesk's Snowdon Towers architectural sample saved by Revit 2025,
//! 58 elements store the frame. The three "Solar Wall" types and 703553
//! store [0, 10 degrees, 0], the other 54 [0, 0, 0], as on 2024. The 21
//! walls of those types whose orientation word is 2 are each exported with
//! `BodySource` `partition_wall_tapered_section`; on main none was, and each
//! kept its record box.
//!
//! The sample cannot be redistributed, so this runs only when
//! `RVT_PROJECT_CORPUS_DIR` holds the 2025 `Snowdon Towers Sample
//! Architectural.rvt`. It skips without it, and on a copy saved by another
//! release.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use rvt::research::partition_compound_structure as pcs;
use std::path::PathBuf;

const SOLAR_WALL_TYPES: [u32; 4] = [703553, 1345241, 1686275, 1686415];

#[test]
fn snowdon_2025_tapered_walls_are_drawn_tapered() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let path = dir.join("Snowdon Towers Sample Architectural.rvt");
    if !path.exists() {
        eprintln!(
            "skipping: Snowdon Towers Sample Architectural.rvt is not in RVT_PROJECT_CORPUS_DIR"
        );
        return;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("basic file info").version;
    if version != 2025 {
        eprintln!("skipping: this copy was saved by Revit {version}, not 2025");
        return;
    }
    let declared = rvt::elem_table::declared_ids(
        &rvt::elem_table::parse_records(&mut rf).expect("element table"),
    );
    let angles = pcs::scan_wall_type_face_angles(&mut rf, version, &declared).expect("face angles");
    assert_eq!(angles.len(), 58, "elements storing the face angles");
    let tapered: Vec<u32> = angles
        .iter()
        .filter(|(_, a)| a[1] != 0.0)
        .map(|(id, _)| *id)
        .collect();
    assert_eq!(tapered, SOLAR_WALL_TYPES, "types storing a non-zero angle");
    for id in SOLAR_WALL_TYPES {
        let [first, exterior, third] = angles[&id];
        assert_eq!((first, third), (0.0, 0.0), "{id}: first and third angle");
        assert!(
            (exterior - 10f64.to_radians()).abs() < 1e-12,
            "{id}: exterior face angle {exterior}"
        );
    }
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let step = write_step(&result.model);
    let drawn = step
        .lines()
        .filter(|line| line.contains("'partition_wall_tapered_section'"))
        .count();
    assert_eq!(drawn, 21, "walls drawn as their tapered cross-section");
}
