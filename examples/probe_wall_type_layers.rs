//! W6: which wall types give their walls no centreline body, and why.
//!
//! FACT under test: on the BIM4LCA office 13 of the 17 US-1 exterior walls
//! (type 1495207) carry no centreline (`m_wall_axis_*`) although their
//! type's layers are read: the partition data gives them a line but no
//! orientation, and `attach_wall_layers` needs both (RE-222). This prints,
//! per wall type, how many walls carry each of the wall fields, and what
//! `scan_type_layers` reads for the type when any id may be a layer's
//! material. Run on `main` before RE-222: type 1495207 walls 17, axis 4,
//! layers 17 (Measure run 40204859227).
//!
//! Output:
//!   type <id> walls <n> axis <n> layers <n> exterior <n> thickness <n>
//!   layers <id> <count> widths <ft,...> functions <...> | none
//!
//! Usage:
//!   cargo run --profile ci --example probe_wall_type_layers -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_compound_structure as pcs;
use rvt::walker::{InstanceField, iter_elements};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mut by_type: BTreeMap<u32, [usize; 5]> = BTreeMap::new();
    for wall in iter_elements(&mut rf)?.filter(|e| e.class == "Wall") {
        let Some(type_id) = wall.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == "m_type_id" => Some(*id),
            _ => None,
        }) else {
            continue;
        };
        let has = |wanted: &str| wall.fields.iter().any(|(name, _)| name == wanted);
        let row = by_type.entry(type_id).or_default();
        row[0] += 1;
        row[1] += usize::from(has("m_wall_axis_start_x"));
        row[2] += usize::from(has("m_wall_layers"));
        row[3] += usize::from(has("m_wall_exterior_x"));
        row[4] += usize::from(has("m_wall_type_thickness"));
    }
    for (type_id, [n, axis, layers, exterior, thickness]) in &by_type {
        println!(
            "type {type_id} walls {n} axis {axis} layers {layers} exterior {exterior} thickness {thickness}"
        );
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let types: BTreeSet<u32> = by_type.keys().copied().collect();
    let read = pcs::scan_type_layers(&mut rf, version, &types, &declared, &declared)?;
    for type_id in &types {
        match read.get(type_id) {
            Some(layers) => println!(
                "layers {type_id} {} widths {} functions {:?}",
                layers.len(),
                layers
                    .iter()
                    .map(|l| format!("{:.6}", l.width_feet))
                    .collect::<Vec<_>>()
                    .join(","),
                layers.iter().map(|l| l.function).collect::<Vec<_>>()
            ),
            None => println!("layers {type_id} none"),
        }
    }
    Ok(())
}
