//! #746: a Level that is not a building storey may share a storey's
//! elevation without losing every Level.
//!
//! BIM4LCA / Nordic LCA `ARK_NordicLCA_Housing_Timber_BuildingPermit.rvt`
//! (Revit 2024; VTT, Granlund, Gravicon, Insinööritoimisto Kallinen; CC BY-SA
//! 4.0, <https://cris.vtt.fi/en/publications/bim-files-to-download/>) holds the
//! reference level `Bet 1-lattia` at `Level_01`'s 170.6037 ft. Revit's own IFC4
//! export of the file has eight storeys; the elevations below are its, in
//! metres.
//!
//! Runs against `RVT_NORDIC_LCA_DIR`, a folder holding the `.rvt`. Skips when
//! absent.

use rvt::RevitFile;
use std::path::PathBuf;

const REVIT_STOREYS_M: [(&str, f64); 8] = [
    ("Sea level", 0.0),
    ("Foundations", 47.3),
    ("Basement", 49.07),
    ("Level_01", 52.0),
    ("Level_02", 55.144),
    ("Level_03", 58.458),
    ("Level_04", 61.772),
    ("Roof", 64.677),
];

#[test]
fn nordic_housing_timber_keeps_its_storeys() {
    const NAME: &str = "ARK_NordicLCA_Housing_Timber_BuildingPermit.rvt";
    let Some(dir) = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_NORDIC_LCA_DIR is not set");
        return;
    };
    let rvt = dir.join(NAME);
    if !rvt.exists() {
        eprintln!("skipping: {NAME} absent");
        return;
    }
    let mut rf = RevitFile::open(&rvt).expect("open");
    let storeys: Vec<(String, f64)> =
        rvt::partition_level_records::recover_partition_levels(&mut rf, 2024)
            .expect("levels")
            .into_iter()
            .filter(|level| level.building_story != Some(false))
            .map(|level| (level.name, level.elevation_feet * 0.3048))
            .collect();
    assert_eq!(
        storeys.len(),
        REVIT_STOREYS_M.len(),
        "storeys recovered: {storeys:?}"
    );
    for (name, metres) in REVIT_STOREYS_M {
        assert!(
            storeys
                .iter()
                .any(|(n, m)| n == name && (m - metres).abs() < 1e-3),
            "{name} at {metres} m not recovered: {storeys:?}"
        );
    }
}
