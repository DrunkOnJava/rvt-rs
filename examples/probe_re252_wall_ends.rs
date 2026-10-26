//! RE-252 (#1046, W7): each wall's centreline and the joins its native
//! record names at its ends, for finding how Revit's wall Length follows
//! from them.
//!
//! FACT under test: revit-ifc's `LengthCalculator` (CLEANROOM.md source 5)
//! takes a wall's `Qto_WallBaseQuantities.Length` from its Length
//! parameter, which on the BIM4LCA office differs from the centreline's
//! length on about 186 of 607 walls and is stored nowhere in the wall's
//! record (RE-251). The record's `CDJoinInfo` objects each pair one of this
//! wall's ends (`m_joinedEnd` 0 or 1, 3 where another wall meets its middle)
//! with another element and that element's end, and each `JoinEndGStep` holds
//! one end's join step. Hypothesis: the Length is the centreline less, or
//! plus, a share of the thickness of the walls joined at its ends.
//! `tools/re/wall_length_vs_ends.py` tests it against Revit's export.
//!
//! Output, one JSON object per wall with a centreline:
//! `id`, plan `start` and `end` and the type's `thickness` (feet), and
//! `joins`, each `CDJoinInfo` as `[[this end, other id, other end], ...,
//! wallJoinType]`, and `ends`, each `JoinEndGStep` as `[end, weakIdx,
//! omitted id, omitted end]`; `control`, `VWallDriver.m_controlJoinsSet` as
//! `[m_index, [[id, end], ...]]`; `keyref`, `SWall.m_wallKeyRef`,
//! `m_locLineOffset` and `m_keyRefOffset`; `exterior`, the unit plan direction
//! to the exterior face (RE-53); and `tweaks`, the ElementIds
//! `WallJoinTweakGStep`'s `m_affectedEdgesTags` name.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re252_wall_ends -- MODEL.rvt > ends.jsonl

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

fn id_of(value: &Value) -> Option<i64> {
    value.pointer("/m_id/m_id64").and_then(Value::as_i64)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    type Axis = (rvt::research::partition_schema_mvp::WallAxis, String);
    let axes: BTreeMap<u64, Axis> = mvp
        .walls
        .iter()
        .filter_map(|wall| {
            let float = |wanted: &str| {
                wall.fields.iter().find_map(|(name, value)| match value {
                    rvt::walker::InstanceField::Float { value, .. } if name == wanted => {
                        Some(*value)
                    }
                    _ => None,
                })
            };
            let exterior =
                match rvt::research::partition_schema_mvp::WALL_EXTERIOR_FIELDS.map(float) {
                    [Some(x), Some(y)] => format!("[{x},{y}]"),
                    _ => "null".into(),
                };
            Some((
                u64::from(wall.id?),
                (
                    rvt::research::partition_schema_mvp::wall_axis_from_fields(&wall.fields)?,
                    exterior,
                ),
            ))
        })
        .collect();
    let ids: BTreeSet<u64> = axes.keys().copied().collect();
    type Row = (Vec<String>, Vec<String>, String, BTreeSet<i64>, Vec<String>);
    let mut rows: BTreeMap<u64, Row> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: ids.clone(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let mut joins = Vec::new();
            let mut ends = Vec::new();
            let mut keyref = String::from("null");
            let mut tweaks: BTreeSet<i64> = BTreeSet::new();
            let mut control = Vec::new();
            for object in &graph.objects {
                match object.class_name.as_str() {
                    "CDJoinInfo" => {
                        let mut entry = String::from("[");
                        if let Some(info) = object.fields.get("m_info").and_then(Value::as_array) {
                            for item in info {
                                let end = item.get("m_joinedEnd").and_then(Value::as_i64);
                                let other = item.get("m_elemId").and_then(id_of);
                                let _ = write!(
                                    entry,
                                    "[{},{}],",
                                    other.map_or("null".into(), |v| v.to_string()),
                                    end.map_or("null".into(), |v| v.to_string())
                                );
                            }
                        }
                        let kind = object.fields.get("m_wallJoinType").and_then(Value::as_i64);
                        let _ = write!(entry, "{}]", kind.map_or("null".into(), |v| v.to_string()));
                        joins.push(entry);
                    }
                    "JoinEndGStep" => {
                        let field = |name: &str| object.fields.get(name).and_then(Value::as_i64);
                        let omitted = object.fields.get("m_omittedTarget");
                        let omitted_id = omitted.and_then(|o| o.get("m_elemId")).and_then(id_of);
                        let omitted_end = omitted
                            .and_then(|o| o.get("m_joinedEnd"))
                            .and_then(Value::as_i64);
                        let show = |v: Option<i64>| v.map_or("null".into(), |v| v.to_string());
                        ends.push(format!(
                            "[{},{},{},{}]",
                            show(field("m_end")),
                            show(field("m_weakIdx")),
                            show(omitted_id),
                            show(omitted_end)
                        ));
                    }
                    "SWall" => {
                        let num = |name: &str| {
                            object
                                .fields
                                .get(name)
                                .and_then(Value::as_f64)
                                .map_or("null".into(), |v| v.to_string())
                        };
                        keyref = format!(
                            "[{},{},{}]",
                            num("m_wallKeyRef"),
                            num("m_locLineOffset"),
                            num("m_keyRefOffset")
                        );
                    }
                    "VWallDriver" => {
                        for set in object
                            .fields
                            .get("m_controlJoinsSet")
                            .and_then(Value::as_array)
                            .into_iter()
                            .flatten()
                        {
                            let index = set.get("m_index").and_then(Value::as_i64);
                            let mut entry = String::new();
                            for item in set
                                .pointer("/m_JoinInfo/m_info")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                            {
                                let other = item.get("m_elemId").and_then(id_of);
                                let end = item.get("m_joinedEnd").and_then(Value::as_i64);
                                let show =
                                    |v: Option<i64>| v.map_or("null".into(), |v| v.to_string());
                                if !entry.is_empty() {
                                    entry.push(',');
                                }
                                let _ = write!(entry, "[{},{}]", show(other), show(end));
                            }
                            control.push(format!(
                                "[{},[{entry}]]",
                                index.map_or("null".into(), |v| v.to_string())
                            ));
                        }
                    }
                    "WallJoinTweakGStep" => {
                        if let Some(tags) = object
                            .fields
                            .get("m_affectedEdgesTags")
                            .and_then(Value::as_array)
                        {
                            for tag in tags {
                                if let Some(id) = tag.get("first").and_then(id_of) {
                                    tweaks.insert(id);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
            rows.insert(
                record.identity.element_id,
                (joins, ends, keyref, tweaks, control),
            );
            Ok(())
        },
    )?;
    for (id, (axis, exterior)) in &axes {
        let (joins, ends, keyref, tweaks, control) = rows.get(id).cloned().unwrap_or_default();
        let keyref = if keyref.is_empty() {
            "null".into()
        } else {
            keyref
        };
        let tweaks: Vec<String> = tweaks.iter().map(i64::to_string).collect();
        println!(
            "{{\"id\":{id},\"start\":[{},{}],\"end\":[{},{}],\"thickness\":{},\"joins\":[{}],\"ends\":[{}],\"keyref\":{keyref},\"exterior\":{exterior},\"control\":[{}],\"tweaks\":[{}]}}",
            axis.start[0],
            axis.start[1],
            axis.end[0],
            axis.end[1],
            axis.thickness_feet,
            joins.join(","),
            ends.join(","),
            control.join(","),
            tweaks.join(",")
        );
    }
    eprintln!(
        "{} walls with a centreline, {} records",
        axes.len(),
        rows.len()
    );
    Ok(())
}
