//! #1589: the saved mesh of every room, with its Level's room computation
//! height, to slice it where Revit takes its area.
//!
//! FACT under test: Revit takes a room's area from a horizontal section of its
//! solid at its Level's elevation plus the Level's
//! `m_roomComputationElevationOffset` (`probe_1589_level_fields`). On RE-374's
//! four stepped rooms of Snowdon Towers that height fits two and not the other
//! two. Measured over every room of a model against `Vim.Room.Area` (local
//! only), the section at that height must give Revit's area on every room
//! whose floor gives it, and say what happens where the height is at or below
//! the room's floor.
//!
//! Output, in feet and the document frame:
//!
//!   level <id> elevation <z> offset <ft> name <text>
//!   room <id> level <id> lower <ft> upper_level <id> upper <ft> height <ft>
//!   mesh <id> status <status> primitives <n> triangles <n> unbounded <n>
//!   tri <id> <ax> <ay> <az> <bx> <by> <bz> <cx> <cy> <cz>
//!
//! A Level's elevation is the z of its first `Plane`'s origin. A `tri` line is
//! each triangle of a room's saved mesh at detail 3, as `rvt-ifc` reads it,
//! visible branches only.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1589_room_sections -- MODEL.rvt

use std::collections::BTreeSet;

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;
use serde_json::Value;

const DETAIL: i64 = 3;

fn id_of(value: Option<&Value>) -> i64 {
    value
        .and_then(|v| v.pointer("/m_id/m_id64"))
        .and_then(Value::as_i64)
        .unwrap_or(-1)
}

fn number(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_f64)
        .map_or_else(|| "-".to_string(), |v| format!("{v:.6}"))
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1589_room_sections MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let levels = Options {
        selected_classes: BTreeSet::from(["Level".to_string()]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &levels, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let mut offset = None;
        let mut name = String::from("-");
        let mut elevation = None;
        for object in &graph.objects {
            let fields = object.fields.to_json();
            if object.class_name == "Level" {
                offset = fields.get("m_roomComputationElevationOffset").cloned();
                if let Some(text) = fields.get("m_text").and_then(Value::as_str) {
                    name = text.replace(char::is_whitespace, "_");
                }
            } else if object.class_name == "Plane" && elevation.is_none() {
                elevation = fields.pointer("/m_origin/2").cloned();
            }
        }
        println!(
            "level {id} elevation {} offset {} name {name}",
            number(elevation.as_ref()),
            number(offset.as_ref())
        );
        Ok(())
    })?;
    let rooms_options = Options {
        selected_classes: BTreeSet::from(["RoomElem".to_string()]),
        ..Options::default()
    };
    let mut rooms: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &rooms_options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(room) = graph.objects.iter().find(|o| o.class_name == "RoomElem") else {
            return Ok(());
        };
        let fields = room.fields.to_json();
        let id = record.identity.element_id;
        rooms.insert(id);
        println!(
            "room {id} level {} lower {} upper_level {} upper {} height {}",
            id_of(fields.get("m_assocLevelId")),
            number(fields.get("m_lowerOffset")),
            id_of(fields.get("m_upperLevelId")),
            number(fields.get("m_upperOffset")),
            number(fields.get("m_height")),
        );
        Ok(())
    })?;
    if rooms.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: rooms,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &options, DETAIL)?;
    for element in &scene.elements {
        let id = element.id;
        let meshes = &element.meshes;
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        println!(
            "mesh {id} status {} primitives {} triangles {triangles} unbounded {}",
            element.status,
            meshes.primitives.len(),
            meshes.unbounded_faces
        );
        for primitive in &meshes.primitives {
            for triangle in &primitive.triangles {
                let corners = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                let [Some(a), Some(b), Some(c)] = corners else {
                    continue;
                };
                println!(
                    "tri {id} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5} {:.5}",
                    a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]
                );
            }
        }
    }
    Ok(())
}
