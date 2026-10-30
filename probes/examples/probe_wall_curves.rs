//! W6: every wall's location curve and end joins from its native record.
//!
//! FACT under test: a wall's native `SWall` record holds the curve it is
//! drawn along (a `GLine` or `GArc`, `m_origin`, `m_dirVec`, `m_endParams`),
//! where its location line sits off that curve (`m_wallKeyRef`,
//! `m_keyRefOffset`, `m_isFlipped`), the walls joined at each end
//! (`VWallDriver.m_controlJoinsSet`, `m_noJoinAtEnd`), whether each end's
//! join changes its geometry (`JoinEndGStep`, one per end) and the type of
//! each joint along it (`CDJoinInfo.m_wallJoinType`). Measured against
//! Revit's body by `tools/re/wall_curve_ends_vs_ifc.py`, these say where
//! Revit ends a wall, on walls whose partition data gives no line.
//!
//! Output:
//!   curve <id> <class> start <x> <y> end <x> <y> z <z> thickness <ft> type <id>
//!       keyref <n> keyoffset <ft> flipped <bool>
//!   driver <id> nojoin <json> control <json>
//!   joinend <id> <end> real <bool> realjoined <json> geomjoined <json> omitted <json> flags <n>
//!   cdjoin <id> type <n> info <json>
//!   walltype <id> layers <ft,...> ext <n> int <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_wall_curves -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;

/// `{"m_id":{"m_id64":n}}` becomes `n`, and `"m_bTangent":false` goes, so
/// the joins print short.
fn compact(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            if let Some(id) = map.get("m_id").and_then(|v| v.get("m_id64")) {
                if map.len() == 1 {
                    return id.clone();
                }
            }
            Value::Object(
                map.iter()
                    .filter(|(k, v)| !(k.as_str() == "m_bTangent" && *v == &Value::Bool(false)))
                    .filter(|(k, _)| k.as_str() != "m_cleanData")
                    .map(|(k, v)| (k.clone(), compact(v)))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(compact).collect()),
        other => other.clone(),
    }
}

fn number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

fn triple(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    Some([
        items.first()?.as_f64()?,
        items.get(1)?.as_f64()?,
        items.get(2)?.as_f64()?,
    ])
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let thickness: BTreeMap<u64, f64> = iter_elements(&mut rf)?
        .filter(|e| e.class == "Wall")
        .filter_map(|e| {
            let id = u64::from(e.id?);
            e.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == "m_wall_thickness" => {
                    Some((id, *value))
                }
                _ => None,
            })
        })
        .collect();
    let options = Options {
        selected_classes: BTreeSet::from(["SWall".to_string(), "BasicWallType".to_string()]),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if record.class_name.as_deref() == Some("BasicWallType") {
            // The type's layers, exterior first, and how many of them lie
            // outside its core on each side (RE-222).
            if let Some(structure) = graph
                .objects
                .iter()
                .find(|object| object.class_name == "CompoundStructure")
            {
                let widths: Vec<String> = structure
                    .fields
                    .get("m_layers")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|layer| layer.get("m_layerWidth")?.as_f64())
                    .map(|w| format!("{w:.6}"))
                    .collect();
                println!(
                    "walltype {id} layers {} ext {} int {}",
                    widths.join(","),
                    structure
                        .fields
                        .get("m_numShellLayersExt")
                        .unwrap_or(&Value::Null),
                    structure
                        .fields
                        .get("m_numShellLayersInt")
                        .unwrap_or(&Value::Null),
                );
            }
            return Ok(());
        }
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let curve = graph
            .objects
            .iter()
            .find(|object| matches!(object.class_name.as_str(), "GLine" | "GArc"));
        if let Some(curve) = curve {
            let origin = triple(curve.fields.get("m_origin"));
            let params = curve.fields.get("m_endParams").and_then(Value::as_array);
            let (start, end) = match (curve.class_name.as_str(), origin, params) {
                ("GLine", Some(o), Some(params)) => {
                    let d = triple(curve.fields.get("m_dirVec")).unwrap_or([0.0; 3]);
                    let at = |t: f64| [o[0] + d[0] * t, o[1] + d[1] * t];
                    match (number(params.first()), number(params.get(1))) {
                        (Some(a), Some(b)) => (Some(at(a)), Some(at(b))),
                        _ => (None, None),
                    }
                }
                _ => (None, None),
            };
            if let (Some(s), Some(e), Some(o)) = (start, end, origin) {
                println!(
                    "curve {id} {} start {:.6} {:.6} end {:.6} {:.6} z {:.6} thickness {} type {} keyref {} keyoffset {} flipped {}",
                    curve.class_name,
                    s[0],
                    s[1],
                    e[0],
                    e[1],
                    o[2],
                    thickness
                        .get(&id)
                        .map_or("-".to_string(), |t| format!("{t:.6}")),
                    compact(
                        root.fields
                            .get("m_WallAttributesId")
                            .unwrap_or(&Value::Null)
                    ),
                    root.fields.get("m_wallKeyRef").unwrap_or(&Value::Null),
                    root.fields.get("m_keyRefOffset").unwrap_or(&Value::Null),
                    root.fields.get("m_isFlipped").unwrap_or(&Value::Null),
                );
            } else {
                println!("curve {id} {} unread", curve.class_name);
            }
        }
        for object in &graph.objects {
            let f = &object.fields;
            match object.class_name.as_str() {
                "VWallDriver" => println!(
                    "driver {id} nojoin {} control {}",
                    compact(f.get("m_noJoinAtEnd").unwrap_or(&Value::Null)),
                    compact(f.get("m_controlJoinsSet").unwrap_or(&Value::Null)),
                ),
                "JoinEndGStep" => println!(
                    "joinend {id} {} real {} realjoined {} geomjoined {} omitted {} flags {}",
                    f.get("m_end").unwrap_or(&Value::Null),
                    f.get("m_hasRealJoined").unwrap_or(&Value::Null),
                    compact(f.get("m_realJoined").unwrap_or(&Value::Null)),
                    compact(f.get("m_geomJoined").unwrap_or(&Value::Null)),
                    compact(f.get("m_omittedTarget").unwrap_or(&Value::Null)),
                    f.get("m_flags").unwrap_or(&Value::Null),
                ),
                "CDJoinInfo" => println!(
                    "cdjoin {id} type {} info {}",
                    f.get("m_wallJoinType").unwrap_or(&Value::Null),
                    compact(f.get("m_info").unwrap_or(&Value::Null)),
                ),
                _ => {}
            }
        }
        Ok(())
    })?;
    Ok(())
}
