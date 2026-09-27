//! RE-82: a family type's geometry-material map (#355).
//!
//! FACT: a family type's parameter value block (RE-77) holds a counted map
//! `u32 n · n × (u32 key · u64 material)` whose every value is a material
//! ElementId: the materials the type's geometry is drawn in. A material the
//! type only names in a parameter (an alternative cladding or finish) is not
//! in it. Revit's IFC export gives each instance of the type exactly these
//! materials, as an `IfcMaterialConstituentSet`.
//!
//! Prints each type's map materials by name:
//!
//! ```text
//! cargo run --profile ci --example probe_re82_type_material_maps -- model.rvt
//! ```
//!
//! and `tools/re/element_materials_vs_ifc.py` compares an rvt-rs export's
//! element materials with Revit's own export of the same model.

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re82_type_material_maps FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let maps = rvt::partition_type_materials::type_material_names(&mut rf, version);
    for (owner, names) in &maps {
        println!("{owner}\t{}", names.join("; "));
    }
    eprintln!(
        "Revit {version}: {} types with a geometry-material map",
        maps.len()
    );
    Ok(())
}
