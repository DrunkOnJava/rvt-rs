//! RE-73: where a wall that ends part way along another wall stops.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a wall
//! whose centreline ends on another wall's centreline, away from that
//! wall's ends and with no join list naming it there (RE-70), makes a T
//! joint: the other wall runs through and this one stops against it layer
//! by layer, by Revit's layer priorities (1 structure, 2 substrate,
//! 3 thermal or air, 4 finish 1, 5 finish 2). Counted from the face it
//! meets, each layer passes the other wall's layers of a larger function
//! number and stops at the first of an equal or smaller one; a single-layer
//! wall stops at the face. `tools/re/wall_tee_joins_vs_ifc.py` scores that
//! against Revit's IFC4 bodies.
//!
//! This probe prints, one JSON object per wall with a centreline, what the
//! exporter decided:
//! - `id`, the centreline's plan `start` and `end`, the type's `thickness`
//!   and the plan direction of the wall's `exterior` face;
//! - `layers`, exterior first, each `[width, function]` (membranes of no
//!   width left out);
//! - `ends`, at the start and the end, the wall it joins, whether it runs
//!   through, and how far past the centreline's end its body (`reach`) or
//!   each of its layers reaches, feet: `layer_reaches`, exterior first, one
//!   pair per layer for its exterior-side and interior-side edge (equal
//!   where the end is square, RE-74). `null` where no join is decided;
//! - `box`, the wall's element-record box.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re73_tee_joins -- \
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
        row.push_str("],\"ends\":[");
        let layer_ends = mvp::wall_layer_ends_from_fields(&wall.fields);
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
            let (Some(partner), Some(through)) = (partner, through) else {
                row.push_str("null");
                continue;
            };
            let reach = axis.join_reach_feet[slot].map_or("null".to_owned(), |r| r.to_string());
            let layer_reaches = layer_ends
                .as_ref()
                .and_then(|ends| ends.reach_feet[slot].as_ref())
                .map_or("null".to_owned(), |reaches| {
                    let items: Vec<String> = reaches
                        .iter()
                        .map(|[outer, inner]| format!("[{outer},{inner}]"))
                        .collect();
                    format!("[{}]", items.join(","))
                });
            let _ = write!(
                row,
                "{{\"wall\":{partner},\"through\":{through},\"reach\":{reach},\"layer_reaches\":{layer_reaches}}}"
            );
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
