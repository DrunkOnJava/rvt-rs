//! RE-58: material names, and material layer sets in the IFC export.
//!
//! FACT: a material's object (`rvt::partition_materials`) holds its name,
//! and `partition_materials::name_at` reads it from the first of:
//! - its own name field after the class tag: `u32 n` and `n` UTF-16 units;
//! - where that field holds parameter entries first, the name ending at the
//!   first `ff ff ff ff` followed by `17 0c` (Revit 2024) or `6b 0c`
//!   (Revit 2025);
//! - a framed string or a name parameter entry further on.
//!
//! On Snowdon Towers, Core Interior, RE1 Architecture, Projeto1 and
//! teste_export_2025, every named material that Revit's own IFC export
//! styles under that name has Revit's colour for it. Every layer material
//! read is the one Revit's export gives that layer, in Revit's order.
//!
//! The probe prints how many of the file's materials are named. Then, for
//! each material that a wall, floor, roof or ceiling type's layers use, it
//! prints the id, the shading colour, how many layers use it, and the name
//! (`-` when not read). `--all` prints every named material instead.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re58_material_names -- \
//!     MODEL.rvt [--all]

use rvt::RevitFile;
use rvt::partition_materials::{scan_material_appearances, scan_material_names};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [--all]");
    let all = args.any(|a| a == "--all");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names = scan_material_names(&mut rf, version, &declared)?;
    let looks = scan_material_appearances(&mut rf, version, &declared)?;
    let materials: BTreeSet<u32> = rvt::partition_type_records::scan_type_records(
        &mut rf,
        version,
        rvt::partition_type_records::OST_MATERIALS,
        &declared,
    )?
    .iter()
    .map(|record| record.element_id)
    .collect();
    let colour = |id: &u32| {
        looks
            .get(id)
            .map(|a| format!("{:02x}{:02x}{:02x}", a.rgb[0], a.rgb[1], a.rgb[2]))
            .unwrap_or_else(|| "------".into())
    };
    println!(
        "Revit {version}: {} of {} materials named",
        names.keys().filter(|id| materials.contains(id)).count(),
        materials.len()
    );
    if all {
        for (id, name) in &names {
            println!("{id}\t{}\t{name}", colour(id));
        }
        return Ok(());
    }
    let layers = rvt::partition_compound_structure::scan_type_layers(
        &mut rf, version, &declared, &materials, &declared,
    )?;
    let mut used: BTreeMap<u32, usize> = BTreeMap::new();
    for layer in layers.values().flatten() {
        if let Some(material) = layer.material {
            *used.entry(material).or_default() += 1;
        }
    }
    println!(
        "{} types with layers use {} materials; {} named",
        layers.len(),
        used.len(),
        used.keys().filter(|id| names.contains_key(id)).count()
    );
    for (id, count) in &used {
        let name = names.get(id).map_or("-", String::as_str);
        println!("{id}\t{}\t{count}\t{name}", colour(id));
    }
    Ok(())
}
