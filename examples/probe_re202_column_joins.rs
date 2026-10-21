//! RE-202 and RE-205: an off-centre column's material is a layer of the
//! wall it is joined to; rendered content is `<Unnamed>`.
//!
//! FACT: a column whose type draws with no material takes, in Revit's
//! export, one layer of the first wall it was joined to (its `CutPairGStep`
//! objects by `m_id`, `m_otherId`) among the walls that overlap it: the
//! exterior layer where the column stands out past both faces of the wall or
//! its box centre lies on the wall's exterior side, else the interior layer.
//! A wall's exterior lies left of its location line (`VWallDriver.m_pCrv`)
//! unless `SWall.m_isFlipped`. 108 of 108 of Snowdon Towers' off-centre
//! columns. A family whose `Family.m_eRenderModelType` is 4 (RPC, planting)
//! is exported with `<Unnamed>` alone: 127 of 127 there.
//!
//! The probe prints one JSON line per column of the partition decode: its
//! joins in order, the wall taken, the side and the layer taken, as
//! `partition_schema_mvp` exports them, and one line per rendered-content
//! type. Verify with `tools/re/element_materials_vs_ifc.py`.
//!
//! Usage:
//!   cargo run --release --example probe_re202_column_joins -- MODEL.rvt

use rvt::RevitFile;
use rvt::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD;
use rvt::partition_schema_mvp::{TYPE_ID_FIELD, TYPE_MATERIAL_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::json;
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: probe_re202_column_joins MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let ids: BTreeSet<u64> = decoded
        .columns
        .iter()
        .filter_map(|c| c.id.map(u64::from))
        .collect();
    let joins = rvt::native_column_joins::joins(&mut rf, &ids)?;
    let walls: BTreeSet<u64> = joins.values().flatten().copied().collect();
    let sides = rvt::native_column_joins::wall_sides(&mut rf, &walls)?;
    let mut joined = 0usize;
    for column in &decoded.columns {
        let Some(id) = column.id.map(u64::from) else {
            continue;
        };
        let overlapping: BTreeSet<u64> = column
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == COLUMN_JOINED_WALL_FIELD => {
                    Some(u64::from(*id))
                }
                _ => None,
            })
            .collect();
        let order = joins.get(&id).cloned().unwrap_or_default();
        let wall = order
            .iter()
            .find(|w| overlapping.contains(w))
            .or_else(|| order.first())
            .copied();
        let float = |wanted: &str| {
            column.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let side = match (
            wall.and_then(|w| sides.get(&w)),
            float("m_locationX"),
            float("m_locationY"),
        ) {
            (Some(side), Some(x), Some(y)) => Some(if side.is_exterior([x, y]) {
                "exterior"
            } else {
                "interior"
            }),
            _ => None,
        };
        let materials: Vec<&str> = column
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                InstanceField::String(s) if name == TYPE_MATERIAL_FIELD => Some(s.as_str()),
                _ => None,
            })
            .collect();
        joined += usize::from(wall.is_some());
        println!(
            "{}",
            json!({"id": id, "joins": order, "wall": wall, "centre_side": side, "materials": materials})
        );
    }
    let types: BTreeSet<u64> = decoded
        .products
        .iter()
        .filter_map(|p| {
            p.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => {
                    Some(u64::from(*id))
                }
                _ => None,
            })
        })
        .collect();
    let rendered = rvt::native_rendered_content::rendered_content_types(&mut rf, &types)?;
    for type_id in &rendered {
        println!("{}", json!({"rendered_content_type": type_id}));
    }
    println!(
        "{}",
        json!({"columns": decoded.columns.len(), "joined": joined, "rendered_content_types": rendered.len()})
    );
    Ok(())
}
