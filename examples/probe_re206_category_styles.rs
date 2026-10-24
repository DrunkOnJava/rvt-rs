//! RE-206: a category's material is the one its graphics styles name.
//!
//! FACT: each category's `GStyleElem` objects (`m_categoryId`) hold a
//! `GStyle` whose `m_materialElemId` is the category's material. Where the
//! object-styles entry RE-91 reads is unset (Snowdon Towers' Walls entry is
//! 0) or not read (every Revit 2025 file), a wall layer whose material is −1
//! ("By Category") takes it: "Default Wall" on Snowdon Towers' 16 such walls
//! and on every wall of the R2025 models, as Revit's export writes.
//!
//! The probe prints the Walls, Floors, Roofs and Ceilings categories'
//! materials as the object-styles entry and as the graphics styles give
//! them.
//!
//! Usage:
//!   cargo run --release --example probe_re206_category_styles -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{OST_CEILINGS, OST_FLOORS, OST_ROOFS, OST_WALLS};
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re206_category_styles MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let declared = rvt::elem_table::declared_ids(&records);
    let names =
        rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?;
    for (label, category) in [
        ("Walls", OST_WALLS),
        ("Floors", OST_FLOORS),
        ("Roofs", OST_ROOFS),
        ("Ceilings", OST_CEILINGS),
    ] {
        let effective =
            rvt::research::partition_materials::scan_category_material(&mut rf, version, category)?;
        let styles =
            rvt::research::partition_materials::native_category_material(&mut rf, category)?;
        println!(
            "{}",
            json!({
                "category": label,
                "effective": effective.and_then(|m| names.get(&m)),
                "graphics_styles": styles.and_then(|m| names.get(&m)),
            })
        );
    }
    Ok(())
}
