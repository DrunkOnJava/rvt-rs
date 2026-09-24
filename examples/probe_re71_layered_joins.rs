//! RE-71: where each layer of a layered wall ends at a butt join.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, at a butt
//! join between two layered walls (RE-70), Revit cleans the join layer by
//! layer and the layers nest like concentric Ls. Counting both walls' layers
//! from the outside of the corner, layer `i` of the wall that runs through
//! reaches the outer edge of the partner's layer `i`, and layer `i` of the
//! wall that stops, its inner edge. Where both walls show the same layer
//! functions in the same order from the corner and share their base and
//! top, 521 of 528 ends are right in every layer in Revit's IFC4 bodies.
//!
//! This probe prints what `tools/re/wall_layer_joins_vs_ifc.py` needs to
//! measure that, one JSON object per wall with a centreline:
//! - `id`, the centreline's plan `start` and `end`, the type's `thickness`
//!   and the plan direction of the wall's `exterior` face;
//! - `layers`, exterior first, each `[width, function]` (1 structure,
//!   2 substrate, 3 thermal or air, 4 finish 1, 5 finish 2; membranes of
//!   no width left out);
//! - `joins`, at the start and the end, the partner wall and whether this
//!   wall runs through there (RE-70), or `null`;
//! - `box`, the wall's element-record box.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re71_layered_joins -- \
//!     MODEL.rvt > walls.jsonl

use rvt::RevitFile;
use rvt::partition_schema_mvp as mvp;
use rvt::walker::InstanceField;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let recovered =
        mvp::recover_partition_schema_mvp(&mut rf, version, rvt::walker::WalkerLimits::default())?;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let materials: BTreeSet<u32> = rvt::partition_type_records::scan_type_records(
        &mut rf,
        version,
        rvt::partition_type_records::OST_MATERIALS,
        &declared,
    )?
    .iter()
    .map(|record| record.element_id)
    .collect();
    let type_of = |fields: &[(String, InstanceField)]| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == mvp::TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = recovered
        .walls
        .iter()
        .filter_map(|w| type_of(&w.fields))
        .collect();
    let layers = rvt::partition_compound_structure::scan_type_layers(
        &mut rf, version, &types, &materials, &declared,
    )?;
    let boxes: BTreeMap<u32, [f64; 6]> = rvt::partition_element_records::scan_category_records(
        &mut rf,
        version,
        rvt::partition_element_records::OST_WALLS,
        &declared,
    )?
    .into_iter()
    .map(|record| (record.element_id, record.bbox_feet))
    .collect();
    let mut rows = 0;
    for wall in &recovered.walls {
        let (Some(id), Some(axis)) = (wall.id, mvp::wall_axis_from_fields(&wall.fields)) else {
            continue;
        };
        let float = |wanted: &str| {
            wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let [Some(nx), Some(ny)] = mvp::WALL_EXTERIOR_FIELDS.map(float) else {
            continue;
        };
        let Some(type_layers) = type_of(&wall.fields).and_then(|t| layers.get(&t)) else {
            continue;
        };
        let mut row = String::new();
        let _ = write!(
            row,
            "{{\"id\":{id},\"start\":[{},{}],\"end\":[{},{}],\"thickness\":{},\"exterior\":[{nx},{ny}],\"layers\":[",
            axis.start[0], axis.start[1], axis.end[0], axis.end[1], axis.thickness_feet
        );
        let kept: Vec<String> = type_layers
            .iter()
            .filter(|layer| layer.width_feet > 0.0)
            .map(|layer| format!("[{},{}]", layer.width_feet, layer.function))
            .collect();
        row.push_str(&kept.join(","));
        row.push_str("],\"joins\":[");
        for slot in 0..2 {
            if slot > 0 {
                row.push(',');
            }
            let partner = wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. }
                    if name == mvp::WALL_JOIN_PARTNER_FIELDS[slot] =>
                {
                    Some(*id)
                }
                _ => None,
            });
            let through = wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Bool(through) if name == mvp::WALL_JOIN_THROUGH_FIELDS[slot] => {
                    Some(*through)
                }
                _ => None,
            });
            match (partner, through) {
                (Some(partner), Some(through)) => {
                    let _ = write!(row, "[{partner},{through}]");
                }
                _ => row.push_str("null"),
            }
        }
        row.push(']');
        if let Some(b) = boxes.get(&id) {
            let _ = write!(
                row,
                ",\"box\":[{},{},{},{},{},{}]",
                b[0], b[1], b[2], b[3], b[4], b[5]
            );
        }
        row.push('}');
        println!("{row}");
        rows += 1;
    }
    eprintln!("Revit {version}: {rows} walls with a centreline, an exterior and layers");
    Ok(())
}
