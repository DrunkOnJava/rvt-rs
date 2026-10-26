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
//! omitted id, omitted end]`.
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
    let axes: BTreeMap<u64, rvt::research::partition_schema_mvp::WallAxis> = mvp
        .walls
        .iter()
        .filter_map(|wall| {
            Some((
                u64::from(wall.id?),
                rvt::research::partition_schema_mvp::wall_axis_from_fields(&wall.fields)?,
            ))
        })
        .collect();
    let ids: BTreeSet<u64> = axes.keys().copied().collect();
    let mut rows: BTreeMap<u64, (Vec<String>, Vec<String>)> = BTreeMap::new();
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
                    _ => {}
                }
            }
            rows.insert(record.identity.element_id, (joins, ends));
            Ok(())
        },
    )?;
    for (id, axis) in &axes {
        let (joins, ends) = rows.get(id).cloned().unwrap_or_default();
        println!(
            "{{\"id\":{id},\"start\":[{},{}],\"end\":[{},{}],\"thickness\":{},\"joins\":[{}],\"ends\":[{}]}}",
            axis.start[0],
            axis.start[1],
            axis.end[0],
            axis.end[1],
            axis.thickness_feet,
            joins.join(","),
            ends.join(",")
        );
    }
    eprintln!(
        "{} walls with a centreline, {} records",
        axes.len(),
        rows.len()
    );
    Ok(())
}
