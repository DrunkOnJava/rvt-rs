//! RE-115: Revit 2023's object styles give each category its material, as
//! 2024's do (RE-91), in a 2023 layout.
//!
//! FACT: a 2023 category's object-styles entry is `i32 BuiltInCategory ·
//! ff ff ff ff · u32 1|2 · u32 1 · ff ff ff ff 3f 01 · 8 bytes · i32
//! -3000010 · u32 material`, held twice. On two 2023 projects the Walls,
//! Floors and Roofs entries hold the material Revit's own IFC4 export
//! writes for the layers of those categories that take their category's
//! material.
//!
//! The probe prints the material, and its name (RE-113), of the Walls,
//! Floors, Roofs and Ceilings entries.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re115_category_materials_2023 -- MODEL.rvt

use rvt::RevitFile;

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let table = rvt::elem_table::parse_records(&mut rf).expect("Global/ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let (_, names) = rvt::partition_materials::scan_materials_2023(&mut rf, &declared);
    for (label, category) in [
        ("Walls", rvt::partition_element_records::OST_WALLS),
        ("Floors", rvt::partition_element_records::OST_FLOORS),
        ("Roofs", rvt::partition_element_records::OST_ROOFS),
        ("Ceilings", rvt::partition_element_records::OST_CEILINGS),
    ] {
        let material = rvt::partition_materials::scan_category_material(&mut rf, version, category)
            .expect("object styles");
        let name = material.and_then(|id| names.get(&id));
        println!("{label}\t{material:?}\t{name:?}");
    }
}
