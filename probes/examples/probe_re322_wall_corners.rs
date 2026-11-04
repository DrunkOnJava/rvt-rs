//! RE-322 (#1194): what a wall's native record holds at a corner, where two
//! walls meet end to end, that says which of them Revit runs on and which it
//! stops.
//!
//! FACT under test: at an L join the wall that runs through reaches the other
//! wall's far face and the wall that stops ends at its near face (RE-70), but
//! the join order the record lists (RE-222 section 4) is no rule. Hypotheses:
//! `JoinEndGStep` (`m_exposedFaceMap`, `m_realJoined`, `m_geomJoined`,
//! `m_omittedTarget`, `m_flags`), `WallJoinTweakGStep`, `CleanPairGStep`,
//! `CutPairGStep`, `CDJoinInfo.m_wallJoinType` or the wall's own root fields
//! tell the two apart. Measured against Revit's body by
//! `tools/re/wall_corner_ends_vs_ifc.py`.
//!
//! Output, one JSON object per line:
//!   {"wall": id, "class": ..., "root": {...}, "curve": {"start": [x, y], "end": [x, y], "z": z} | null,
//!    "classes": {class: count}, "objects": [{"c": class, "f": {...}}, ...]}
//!   {"type": id, "root": {...}, "layers": [{"w": ft, "fn": n, "mat": id}, ...], "ext": n, "int": n}
//! `objects` holds the wall's `VWallDriver`, `CDJoinInfo`, `Plane` and every
//! `*Step` object. An id written `{"m_id": {"m_id64": n}}` is printed as `n`, a
//! history table (`*HistTable`) as its length under `<name>#`, and a member that
//! is a pointer is left out.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re322_wall_corners -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::{Map, Value, json};

const CLIP: usize = 6000;

fn compact(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if let Some(id) = map.get("m_id").and_then(|v| v.get("m_id64")) {
                if map.len() == 1 {
                    return id.clone();
                }
            }
            let mut out = Map::new();
            for (name, member) in map {
                if name.ends_with("HistTable") {
                    let count = member.as_array().map_or(0, Vec::len);
                    out.insert(format!("{name}#"), Value::from(count));
                } else if member.get("pointer_token").is_none() && name != "m_cleanData" {
                    out.insert(name.clone(), compact(member));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(compact).collect()),
        other => other.clone(),
    }
}

fn clipped(value: Value) -> Value {
    let text = value.to_string();
    if text.len() <= CLIP {
        value
    } else {
        Value::String(text.chars().take(CLIP).collect())
    }
}

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    Some([
        items.first()?.as_f64()?,
        items.get(1)?.as_f64()?,
        items.get(2)?.as_f64()?,
    ])
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: BTreeSet::from([
            "SWall".to_string(),
            "ArcWall".to_string(),
            "BasicWallType".to_string(),
        ]),
        ..Options::default()
    };
    let (mut walls, mut types) = (0usize, 0usize);
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        if class == "BasicWallType" {
            let structure = graph
                .objects
                .iter()
                .find(|object| object.class_name == "CompoundStructure");
            let layers: Vec<Value> = structure
                .and_then(|s| s.fields.get("m_layers")?.as_array())
                .into_iter()
                .flatten()
                .map(|layer| {
                    json!({
                        "w": layer.get("m_layerWidth"),
                        "fn": layer.get("m_layerFunction"),
                        "mat": layer.get("m_materialId").map(compact),
                    })
                })
                .collect();
            let field = |name: &str| structure.and_then(|s| s.fields.get(name)).cloned();
            types += 1;
            println!(
                "{}",
                json!({
                    "type": id,
                    "root": clipped(compact(&root.fields)),
                    "layers": layers,
                    "ext": field("m_numShellLayersExt"),
                    "int": field("m_numShellLayersInt"),
                })
            );
            return Ok(());
        }
        let curve = graph
            .objects
            .iter()
            .find(|object| object.class_name == "GLine")
            .and_then(|line| {
                let origin = triple(line.fields.get("m_origin"))?;
                let direction = triple(line.fields.get("m_dirVec"))?;
                let params = line.fields.get("m_endParams")?.as_array()?;
                let (a, b) = (params.first()?.as_f64()?, params.get(1)?.as_f64()?);
                let at = |t: f64| [origin[0] + direction[0] * t, origin[1] + direction[1] * t];
                Some(json!({"start": at(a), "end": at(b), "z": origin[2]}))
            });
        let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
        let mut objects = Vec::new();
        for object in graph.objects.iter().skip(1) {
            let name = object.class_name.as_str();
            *classes.entry(name).or_default() += 1;
            if name.ends_with("Step") || matches!(name, "VWallDriver" | "CDJoinInfo" | "Plane") {
                objects.push(json!({"c": name, "f": clipped(compact(&object.fields))}));
            }
        }
        walls += 1;
        println!(
            "{}",
            json!({
                "wall": id,
                "class": class,
                "root": clipped(compact(&root.fields)),
                "curve": curve,
                "classes": classes,
                "objects": objects,
            })
        );
        Ok(())
    })?;
    eprintln!("{walls} walls, {types} wall types");
    Ok(())
}
