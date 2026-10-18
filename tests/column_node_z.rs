//! RE-173: a column's z-range is Revit's, not the record box's.
//!
//! The record box of a column can start at z = 0.0 while Revit draws the column
//! higher. BIM4LCA / Nordic LCA `ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt`
//! (Revit 2024; CC BY-SA 4.0, <https://cris.vtt.fi/en/publications/bim-files-to-download/>)
//! has 7 columns, all of which main places on z = 0.0. Revit's own IFC4 export
//! gives the bottoms and tops below, in feet (read from its extrusion
//! geometry). On main the bottoms come out as 0.0; with the geometry node they
//! equal Revit's.
//!
//! Runs against `RVT_NORDIC_LCA_DIR`, a folder holding that `.rvt`. Skips when
//! absent. An optional check on Autodesk's `rmeadvancedsampleproject.rvt`
//! (`RVT_REVIT2024_SAMPLES_DIR`) covers 131 columns that main puts on
//! z = 0.0 and Revit on their levels (0.309, 12.467 and 23.950 ft), measured
//! against Revit's Base Level and Base Offset.

use rvt::RevitFile;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{InstanceField, WalkerLimits};
use std::path::PathBuf;

/// (ElementId, bottom, top) in feet, from Revit's IFC geometry.
const REVIT_COLUMNS: [(u32, f64, f64); 7] = [
    (1406153, 170.60367454068242, 177.6082677165345),
    (1521545, 170.83333333333331, 177.6574803149597),
    (1573131, 170.83333333333331, 177.65748031496014),
    (1635009, 170.60367454068242, 177.65748031496014),
    (1650929, 170.60367454068242, 178.14960629921208),
    (1799080, 160.99081364829397, 169.61942257217848),
    (2123310, 170.60367454068242, 177.6082677165345),
];

fn column_ranges(path: &std::path::Path) -> Vec<(u32, f64, f64)> {
    let mut rf = RevitFile::open(path).expect("open");
    let out = recover_partition_schema_mvp(&mut rf, 2024, WalkerLimits::default()).expect("walk");
    out.columns
        .iter()
        .chain(out.products.iter())
        .filter(|e| matches!(e.class.as_str(), "Column" | "StructuralColumn"))
        .filter_map(|e| {
            let float = |wanted: &str| {
                e.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::Float { value, .. } if name == wanted => Some(*value),
                    _ => None,
                })
            };
            let bottom = float("m_locationZ")?;
            Some((e.id?, bottom, bottom + float("m_bboxHeight")?))
        })
        .collect()
}

#[test]
fn nordic_housing_concrete_column_z_is_revits() {
    let Some(dir) = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_NORDIC_LCA_DIR is not set");
        return;
    };
    let rvt = dir.join("ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt");
    if !rvt.exists() {
        eprintln!("skipping: model absent");
        return;
    }
    let ours = column_ranges(&rvt);
    for (id, bottom, top) in REVIT_COLUMNS {
        let (_, b, t) = ours
            .iter()
            .find(|(i, _, _)| *i == id)
            .unwrap_or_else(|| panic!("column {id} not decoded"));
        assert!(
            (b - bottom).abs() < 0.01,
            "{id}: bottom {b}, Revit {bottom}"
        );
        assert!((t - top).abs() < 0.01, "{id}: top {t}, Revit {top}");
    }
}

#[test]
fn rme_advanced_columns_start_where_revit_does() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let rvt = dir.join("rmeadvancedsampleproject.rvt");
    if !rvt.exists() {
        eprintln!("skipping: model absent");
        return;
    }
    let ours = column_ranges(&rvt);
    let raised = ours.iter().filter(|(_, b, _)| *b > 0.1).count();
    assert!(
        raised >= 131,
        "{raised} columns start above z = 0, expected 131"
    );
}
