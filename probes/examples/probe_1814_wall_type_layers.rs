//! #1814 (probe): each wall's type and that type's layers, with their functions.
//!
//! FACT under test: where a wall's end meets a joined wall at a join the
//! reader resolves no reach for, Revit's end lies on a layer plane of the joined
//! wall (RE-447 §5: 0.049, 0.262, 0.492 or 0.590 ft inside a face). The
//! hypothesis is that the plane is the joined wall's core boundary, the face of
//! its run of structure layers (function 1), on the side away from the wall
//! whose end it is. `probe_re252_wall_ends` prints each wall's planes as offsets
//! with no function. This prints the function of each layer of each wall's type,
//! so the planes can be named.
//!
//! Output:
//!
//!   file <path> release <release>
//!   wall <id> type <type id|-> layers <layers|none>
//!
//! A layer prints as `<width mm>/<function>`, layers joined by `;`, exterior
//! first, as `partition_compound_structure::scan_type_layers` gives them
//! (functions: 1 structure, 2 substrate, 3 thermal or air, 4 finish 1,
//! 5 finish 2, 100 membrane, 200 structural deck).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1814_wall_type_layers -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_compound_structure as pcs;
use rvt::research::partition_type_records as ptr;
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(&mut rf, version, ptr::OST_MATERIALS, &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let mut type_of: BTreeMap<u32, u32> = BTreeMap::new();
    for wall in &mvp.walls {
        let Some(id) = wall.id else {
            continue;
        };
        for (name, value) in &wall.fields {
            if let (
                rvt::research::partition_schema_mvp::TYPE_ID_FIELD,
                InstanceField::ElementId { id: type_id, .. },
            ) = (name.as_str(), value)
            {
                type_of.insert(id, *type_id);
            }
        }
    }
    let used: BTreeSet<u32> = type_of.values().copied().collect();
    let layers = pcs::scan_type_layers(&mut rf, version, &used, &materials, &declared)?;
    for wall in &mvp.walls {
        let Some(id) = wall.id else {
            continue;
        };
        let type_id = type_of.get(&id);
        let printed =
            type_id
                .and_then(|type_id| layers.get(type_id))
                .map_or("none".to_string(), |layers| {
                    layers
                        .iter()
                        .map(|layer| format!("{:.1}/{}", layer.width_feet * 304.8, layer.function))
                        .collect::<Vec<_>>()
                        .join(";")
                });
        println!(
            "wall {id} type {} layers {printed}",
            type_id.map_or("-".to_string(), u32::to_string)
        );
    }
    Ok(())
}
