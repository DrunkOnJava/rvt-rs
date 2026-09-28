//! RE-113: a Revit 2023 family type's geometry-material map, and the
//! materials it names.
//!
//! FACT: a 2023 family type's parameter value block is `[owner u32][28 x
//! 0xff][3 x 0x00]`, and its first counted map `u32 n · n × (u32 key · u32
//! material)` names the materials its geometry uses, as 2024's `u64` form
//! does (RE-82). A 2023 material is an object `01 00 00 00 · u32 id` whose
//! class tag `0x09fb` sits 0x27 bytes past the id, behind `00 00 00 · ff ff
//! ff ff`; it names itself in its element data or, when it sits inside a
//! family's data, in a `-1001203` parameter entry.
//!
//! The probe prints every material with its name, then every type's map;
//! compare the maps with the `IfcMaterialConstituentSet` Revit's own export
//! gives the type's instances.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re113_type_materials_2023 -- MODEL.rvt

use rvt::RevitFile;

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let table = rvt::elem_table::parse_records(&mut rf).expect("Global/ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let (materials, names) = rvt::partition_materials::scan_materials_2023(&mut rf, &declared);
    println!("{} materials, {} named", materials.len(), names.len());
    for id in &materials {
        println!("material {id}\t{:?}", names.get(id));
    }
    for (owner, map) in rvt::partition_type_materials::type_material_names_2023(&mut rf) {
        println!("type {owner}\t{map:?}");
    }
}
