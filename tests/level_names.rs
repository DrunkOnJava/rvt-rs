//! RE-51: Level names and elevations read from the Levels' name blocks on
//! Revit 2025 files, and the storeys they make.
//!
//! Every RE1 model (Revit 2025, MIT) has the two Levels Revit's own export
//! writes as storeys, with their names and elevations, and each storey
//! rvt-rs writes has Revit's GlobalId (RE-48). Corpus-gated through
//! `RVT_PROJECT_CORPUS_DIR`.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use rvt::partition_level_records::recover_partition_levels;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// Every GlobalId of `entity` in a STEP file.
fn global_ids_of(step: &str, entity: &str) -> BTreeSet<String> {
    let needle = format!("={entity}('");
    step.lines()
        .filter_map(|line| {
            let at = line.find(&needle)? + needle.len();
            line.get(at..at + 22).map(str::to_string)
        })
        .collect()
}

#[test]
fn re1_levels_and_storeys_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    // Revit's export writes Level 2 at 3,660 mm.
    let level_2 = 3660.0 / 304.8;
    let mut checked = 0;
    for model in ["Architecture", "Mechanical", "Plumbing", "Electrical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            continue;
        }
        let mut rf = RevitFile::open(&rvt).expect("open");
        let levels = recover_partition_levels(&mut rf, 2025).expect("levels");
        let read: Vec<(&str, f64)> = levels
            .iter()
            .map(|level| (level.name.as_str(), level.elevation_feet))
            .collect();
        assert_eq!(read.len(), 2, "RE1 {model}: {read:?}");
        assert_eq!(read[0], ("Level 1", 0.0), "RE1 {model}");
        assert_eq!(read[1].0, "Level 2", "RE1 {model}");
        assert!((read[1].1 - level_2).abs() < 1e-9, "RE1 {model}: {read:?}");

        let ours = write_step(
            &RvtDocExporter
                .export_with_diagnostics(&mut rf)
                .expect("export")
                .model,
        );
        let theirs = std::fs::read_to_string(&reference).expect("reference IFC");
        let revit = global_ids_of(&theirs, "IFCBUILDINGSTOREY");
        assert_eq!(revit.len(), 2, "RE1 {model}");
        assert_eq!(
            global_ids_of(&ours, "IFCBUILDINGSTOREY"),
            revit,
            "RE1 {model}"
        );
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no RE1 model under RVT_PROJECT_CORPUS_DIR");
    }
}
