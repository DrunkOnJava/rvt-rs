//! #1409, #1410, #1411 (RE-348): what a wall's native record holds of the steps
//! Revit draws a wall's end in, and of the wall beneath a stepped end.
//!
//! FACT under test: RE-337 (#1396) draws the end of a profile-edited wall to the
//! face of the one wall beneath it and omits the steps Revit goes on to above,
//! because which of the walls that meet the end it follows at each height is its
//! join order, which was not read. Hypothesis: `JoinEndGStep.m_realJoined` (or
//! `m_geomJoined`) lists the walls in the order Revit follows them, so that at
//! each height the end is the face of the first wall named that stands there.
//! `tools/re/wall_heights_vs_ifc.py`, `wall_top_cover_vs_ifc.py` and
//! `wall_quantities_vs_ifc.py` test it against Revit's own export.
//!
//! Output, one record per line:
//!   end <id> <m_end> real <id:end ...> geom <id:end ...> omitted <id:end> weak <n> exposed <id:tag ...>
//!   ctl <id> <m_index> <id:end ...> type <m_wallJoinType>
//!   full <id> <class> <fields json>
//! An `end` line is each `JoinEndGStep` of a wall and a `ctl` line each entry
//! of its `VWallDriver.m_controlJoinsSet`, for every `SWall` and `ArcWall`.
//! A `full` line is every object of the records of the walls named in `FOCUS`,
//! the walls RE-337's follow-ups are about and the walls its joins name, with
//! each history table printed as its length and pointer members left out.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1410_wall_end_steps -- MODEL.rvt

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, Record, extract_graphs};
use serde_json::Map;

const WALLS: [&str; 2] = ["SWall", "ArcWall"];

/// The walls whose records are printed whole: the four walls of #1409 to #1411,
/// the walls beneath and above their ends and the wall beneath them at permit.
const FOCUS: [u64; 11] = [
    2057668, 2057670, 2058499, 2058501, 1347201, 1493966, 3490093, 1344124, 1347199, 1347200,
    1347202,
];

const CLIP: usize = 60000;

fn clip(text: String) -> String {
    if text.len() <= CLIP {
        text
    } else {
        text.chars().take(CLIP).collect()
    }
}

fn trimmed(fields: &Fv) -> String {
    let Some(members) = fields.as_object() else {
        return clip(fields.to_string());
    };
    let mut kept = Map::new();
    for (name, member) in members {
        if name.ends_with("HistTable") {
            let count = member.as_array().map_or(0, Vec::len);
            kept.insert(format!("{name}#"), Fv::from(count));
        } else if member.get("pointer_token").is_none() {
            kept.insert(name.clone(), member.clone());
        }
    }
    clip(Fv::Object(kept).to_string())
}

/// `m_elemId` and `m_joinedEnd` of one join entry as `id:end`.
fn entry(item: &Fv) -> String {
    let id = item
        .get("m_elemId")
        .and_then(|e| e.get("m_id"))
        .and_then(|e| e.get("m_id64"))
        .and_then(Fv::as_i64)
        .map_or_else(|| "?".to_string(), |id| id.to_string());
    let end = item
        .get("m_joinedEnd")
        .and_then(Fv::as_i64)
        .map_or_else(|| "?".to_string(), |end| end.to_string());
    format!("{id}:{end}")
}

fn entries(list: Option<&Fv>) -> String {
    list.and_then(Fv::as_array)
        .map(|items| items.iter().map(entry).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()
}

fn exposed(list: Option<&Fv>) -> String {
    list.and_then(Fv::as_array)
        .map(|items| {
            items
                .iter()
                .map(|item| {
                    let second = item.get("second");
                    let id = second
                        .and_then(|s| s.get("m_elemId"))
                        .and_then(|e| e.get("m_id"))
                        .and_then(|e| e.get("m_id64"))
                        .and_then(Fv::as_i64)
                        .unwrap_or(-1);
                    let tag = second
                        .and_then(|s| s.get("m_tag"))
                        .and_then(Fv::as_i64)
                        .unwrap_or(-1);
                    format!("{id}:{tag}")
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

fn visit(record: Record) -> Result<()> {
    let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref()) else {
        return Ok(());
    };
    if !WALLS.contains(&class) {
        return Ok(());
    }
    let id = record.identity.element_id;
    for object in &graph.objects {
        let fields = &object.fields;
        match object.class_name.as_str() {
            "JoinEndGStep" => println!(
                "end {id} {} real {} geom {} omitted {} weak {} exposed {}",
                fields
                    .get("m_end")
                    .and_then(Fv::as_i64)
                    .map_or_else(|| "?".to_string(), |end| end.to_string()),
                entries(fields.get("m_realJoined")),
                entries(fields.get("m_geomJoined")),
                fields
                    .get("m_omittedTarget")
                    .map_or_else(String::new, entry),
                fields
                    .get("m_weakIdx")
                    .and_then(Fv::as_i64)
                    .map_or_else(|| "?".to_string(), |n| n.to_string()),
                exposed(fields.get("m_exposedFaceMap")),
            ),
            "VWallDriver" => {
                let sets = fields
                    .get("m_controlJoinsSet")
                    .and_then(Fv::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default();
                for set in sets {
                    let info = set.get("m_JoinInfo");
                    println!(
                        "ctl {id} {} {} type {}",
                        set.get("m_index")
                            .and_then(Fv::as_i64)
                            .map_or_else(|| "?".to_string(), |n| n.to_string()),
                        entries(info.and_then(|i| i.get("m_info"))),
                        info.and_then(|i| i.get("m_wallJoinType"))
                            .and_then(Fv::as_i64)
                            .map_or_else(|| "?".to_string(), |n| n.to_string()),
                    );
                }
            }
            _ => {}
        }
    }
    if FOCUS.contains(&id) {
        for object in &graph.objects {
            println!(
                "full {id} {} {}",
                object.class_name,
                trimmed(&object.fields)
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: WALLS.iter().map(|name| (*name).to_string()).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, visit)?;
    Ok(())
}
