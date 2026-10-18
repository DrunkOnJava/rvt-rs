//! RE-178: a column that a joined slab cuts is exported as its cut body.
//!
//! Revit's IFC export cuts a column's body where a floor or roof joined to it
//! cuts it, and the column's geometry node holds that cut extent (the box at
//! id-48). BIM4LCA / Nordic LCA `ARK_NordicLCA_Office_Concrete_As-Built.rvt`
//! (Revit 2024; CC BY-SA 4.0,
//! <https://cris.vtt.fi/en/publications/bim-files-to-download/>) has 14 columns
//! whose top runs through a 450 mm slab: Revit's own IFC4 export stops their
//! extrusions at the slab's underside (bottoms and tops below, in feet, read
//! from its extrusion geometry), 1.476 ft below the columns' own top. Before
//! RE-178 rvt-rs drew them to their own top. Their `BoundingBoxHeight` stays
//! the column's own extent: only the body is cut.
//!
//! Runs against `RVT_NORDIC_LCA_DIR`, a folder holding that `.rvt`. Skips when
//! absent. An optional check on Autodesk's `rstbasicsampleproject.rvt`
//! (`RVT_REVIT2024_SAMPLES_DIR`) covers a column cut at its bottom: 1173925
//! starts 350 mm inside the slab below, and Revit's body starts at the slab's
//! top, 0.0 ft. Its placement stays at its own base, -1.148 ft.

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue, SolidShape};
use std::path::{Path, PathBuf};

/// (ElementId, bottom, top) in feet, from Revit's IFC geometry.
const REVIT_CUT_COLUMNS: [(u32, f64, f64); 14] = [
    (1343741, 169.61942257217848, 180.61023622047242),
    (3461611, 221.78477690288713, 234.48162729658793),
    (3461612, 221.78477690288713, 234.48162729658793),
    (3461615, 221.78477690288713, 234.48162729658793),
    (3461616, 221.78477690288713, 234.48162729658793),
    (3461619, 221.78477690288713, 234.48162729658793),
    (3461620, 221.78477690288713, 234.48162729658793),
    (3461623, 221.78477690288713, 234.48162729658793),
    (3461624, 221.78477690288713, 234.48162729658793),
    (3461627, 221.78477690288713, 234.48162729658793),
    (3461628, 221.78477690288713, 234.48162729658793),
    (3461630, 221.78477690288713, 234.48162729658793),
    (3461631, 221.78477690288713, 234.48162729658793),
    (3461633, 221.78477690288713, 234.48162729658793),
];

/// ElementId -> (body bottom, body top, `BoundingBoxHeight`) of every exported
/// `IfcColumn`, feet.
fn column_bodies(path: &Path) -> Vec<(u32, f64, f64, Option<f64>)> {
    let mut rf = RevitFile::open(path).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    result
        .model
        .entities
        .iter()
        .filter_map(|entity| {
            let IfcEntity::BuildingElement {
                ifc_type,
                type_guid,
                location_feet,
                extrusion,
                solid_shape,
                property_set,
                ..
            } = entity
            else {
                return None;
            };
            if !ifc_type.eq_ignore_ascii_case("IfcColumn") {
                return None;
            }
            let id = type_guid.as_deref()?.parse().ok()?;
            let base = location_feet.as_ref()?[2];
            // A body that starts above the column's base is placed there.
            let (bottom, height) = match solid_shape {
                Some(SolidShape::PlacedExtrusion {
                    origin_feet,
                    depth_feet,
                    ..
                }) => (base + origin_feet[2], *depth_feet),
                _ => (base, extrusion.as_ref()?.height_feet),
            };
            let record_height = property_set.as_ref().and_then(|set| {
                set.properties.iter().find_map(|p| match p.value {
                    PropertyValue::LengthFeet(value) if p.name == "BoundingBoxHeight" => {
                        Some(value)
                    }
                    _ => None,
                })
            });
            Some((id, bottom, bottom + height, record_height))
        })
        .collect()
}

fn find(ours: &[(u32, f64, f64, Option<f64>)], id: u32) -> (f64, f64, Option<f64>) {
    let (_, b, t, h) = ours
        .iter()
        .find(|(i, ..)| *i == id)
        .unwrap_or_else(|| panic!("column {id} not exported"));
    (*b, *t, *h)
}

#[test]
fn nordic_office_concrete_columns_stop_at_the_slab() {
    let Some(dir) = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_NORDIC_LCA_DIR is not set");
        return;
    };
    let rvt = dir.join("ARK_NordicLCA_Office_Concrete_As-Built.rvt");
    if !rvt.exists() {
        eprintln!("skipping: model absent");
        return;
    }
    let ours = column_bodies(&rvt);
    for (id, bottom, top) in REVIT_CUT_COLUMNS {
        let (b, t, own_height) = find(&ours, id);
        assert!(
            (b - bottom).abs() < 0.01,
            "{id}: body bottom {b}, Revit {bottom}"
        );
        assert!((t - top).abs() < 0.01, "{id}: body top {t}, Revit {top}");
        // The column's own extent runs 450 mm (1.476 ft) higher.
        let own_height = own_height.expect("BoundingBoxHeight");
        assert!(
            (own_height - (top - bottom) - 1.476).abs() < 0.01,
            "{id}: BoundingBoxHeight {own_height} is not the uncut extent"
        );
    }
}

#[test]
fn rst_basic_column_starts_at_the_slab_top() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let rvt = dir.join("rstbasicsampleproject.rvt");
    if !rvt.exists() {
        eprintln!("skipping: model absent");
        return;
    }
    let ours = column_bodies(&rvt);
    let (b, t, _) = find(&ours, 1173925);
    assert!(b.abs() < 0.01, "body bottom {b}, Revit 0.0");
    assert!(
        (t - 9.84251968503937).abs() < 0.01,
        "body top {t}, Revit 9.843"
    );
}
