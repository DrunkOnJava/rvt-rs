//! RE-112: a Revit 2023 wall, floor or roof type keeps its layers in its
//! element data, as 2024's does (RE-53, RE-57), in 29-byte records.
//!
//! FACT: after a 2023 system-family type's name (RE-111) comes its layer
//! count, framed by `ff ff ff ff 6f 10` on a wall type and by the name and
//! `u32 0` on a floor or roof type, then one record per layer: `f64` width
//! in feet, `u32` function, four bytes, `u32` material ElementId (`ff` × 4 =
//! the category's), `u32` deck profile ElementId (`ff` × 4 = none) and five
//! more bytes. A material names itself in its own element data.
//!
//! The probe prints, for each type RE-111 gives a 2023 wall, floor or roof,
//! its layers in metres with their function and material; compare them with
//! the `IfcMaterialLayerSet` Revit's own export gives the type's elements.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re112_layers_2023 -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use std::collections::{BTreeMap, BTreeSet};

fn main() {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().expect("BasicFileInfo").version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())
        .expect("partition records");
    let types: BTreeSet<u32> = mvp
        .walls
        .iter()
        .chain(&mvp.slabs)
        .chain(&mvp.products)
        .flat_map(|element| &element.fields)
        .filter_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
        .collect();
    let table = rvt::elem_table::parse_records(&mut rf).expect("Global/ElemTable");
    let declared = rvt::elem_table::declared_ids(&table);
    let layers = rvt::partition_compound_structure::scan_type_layers(
        &mut rf, version, &types, &declared, &declared,
    )
    .expect("layers");
    let materials: BTreeSet<u32> = layers
        .values()
        .flatten()
        .filter_map(|layer| layer.material)
        .collect();
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&stream) {
            names.extend(rvt::partition_names::find_element_data_names_2023(
                inflated.bytes(),
                &materials,
            ));
        }
    }
    for (type_id, type_layers) in &layers {
        println!("type {type_id}");
        for layer in type_layers {
            let material = match layer.material {
                Some(id) => format!("{id} {:?}", names.get(&id)),
                None => "by category".into(),
            };
            println!(
                "  {:.4} m  function {}  material {material}",
                layer.width_feet * 0.3048,
                layer.function
            );
        }
    }
}
