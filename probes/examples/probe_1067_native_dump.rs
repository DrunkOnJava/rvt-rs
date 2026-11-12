//! #1067 (RE-267): what a wall's native record, and the records of the floors
//! and roofs it names, hold that tells a top join which cuts the wall from one
//! that leaves it whole.
//!
//! FACT under test: RE-258's rule (cut a wall's top at the lowest underside of
//! a floor or roof its `JoinToRoofGStep` names, over the wall's middle) cuts
//! walls that Revit leaves whole. Hypotheses: the join's own history tables,
//! the wall's curve and faces, the other steps of its record, its saved
//! parameters, or the floor's own record tell the two apart.
//!
//! Output, one record per line:
//!   wall <id> <class> root <fields json>
//!   wall <id> classes <class:count ...>
//!   wall <id> step <class> <fields json>
//!   wall <id> relation <class> <base|top|other|-> <ids>
//!   elem <id> <class> root <fields json>
//!   elem <id> classes <class:count ...>
//!   elem <id> step <class> <fields json>
//!   elem <id> refs <every element id the record holds>
//!   param <id> <parameter id> <value json> <source element id> <name>
//!
//! A `wall` is every `SWall` and `ArcWall` record; an `elem` is every floor
//! and roof record and every other element a wall's `JoinToRoofGStep`,
//! `CleanPairGStep`, `CutPairGStep`, `WallCutoutGStep` or `WallEmbedWallGStep`
//! names. A history table in a step (`m_faceHistTable`, `m_edgeHistTable`) is
//! printed as its length under `<name>#`, and a member that is a pointer is
//! left out. `tools/re/wall_top_cover_vs_ifc.py` is the matching scorer.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1067_native_dump -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{Options, Record, extract_graphs};
use rvt::research::native_element_parameters;
use serde_json::{Map, Value};

const WALLS: [&str; 2] = ["SWall", "ArcWall"];
const SLABS: [&str; 5] = [
    "Floor",
    "Roof",
    "FootPrintRoof",
    "ProfileRoof",
    "BuildingPad",
];
const KEPT: [&str; 6] = [
    "GLine",
    "GArc",
    "VWallDriver",
    "CDJoinInfo",
    "Plane",
    "GeomStepList",
];
const CLIP: usize = 8000;

#[derive(Default)]
struct State {
    walls: BTreeSet<u64>,
    slabs: BTreeSet<u64>,
    named: BTreeSet<u64>,
    seen: BTreeSet<u64>,
}

fn clip(text: String) -> String {
    if text.len() <= CLIP {
        text
    } else {
        text.chars().take(CLIP).collect()
    }
}

/// Every positive `m_id64` under `value`.
fn ids_under(value: &Value, out: &mut BTreeSet<u64>) {
    match value {
        Value::Object(members) => {
            for (name, member) in members {
                match member.as_u64() {
                    Some(id) if name == "m_id64" && id > 0 => {
                        out.insert(id);
                    }
                    _ => ids_under(member, out),
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                ids_under(item, out);
            }
        }
        _ => {}
    }
}

fn trimmed(fields: &Value) -> String {
    let Some(members) = fields.as_object() else {
        return clip(fields.to_string());
    };
    let mut kept = Map::new();
    for (name, member) in members {
        if name.ends_with("HistTable") {
            let count = member.as_array().map_or(0, Vec::len);
            kept.insert(format!("{name}#"), Value::from(count));
        } else if member.get("pointer_token").is_none() {
            kept.insert(name.clone(), member.clone());
        }
    }
    clip(Value::Object(kept).to_string())
}

fn dump(kind: &str, id: u64, class: &str, objects: &[(&str, &Value)], steps: bool) {
    if let Some((_, root)) = objects.first() {
        println!("{kind} {id} {class} root {}", trimmed(root));
    }
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for &(name, _) in objects {
        *counts.entry(name).or_default() += 1;
    }
    let histogram: Vec<String> = counts
        .iter()
        .map(|(name, n)| format!("{name}:{n}"))
        .collect();
    println!("{kind} {id} classes {}", histogram.join(" "));
    if !steps {
        return;
    }
    for &(name, fields) in objects.iter().skip(1) {
        if name.ends_with("GStep") || KEPT.contains(&name) {
            println!("{kind} {id} step {name} {}", trimmed(fields));
        }
    }
}

fn relations(id: u64, objects: &[(&str, &Value)], named: &mut BTreeSet<u64>) {
    for &(name, fields) in objects {
        let field = match name {
            "JoinToRoofGStep" => "m_ids",
            "CleanPairGStep" | "CutPairGStep" => "m_otherId",
            "WallCutoutGStep" | "WallEmbedWallGStep" => "m_instId",
            _ => continue,
        };
        let mut ids = BTreeSet::new();
        if let Some(value) = fields.get(field) {
            ids_under(value, &mut ids);
        }
        let end = if name == "JoinToRoofGStep" {
            match fields.get("m_baseOrTop").and_then(Value::as_i64) {
                Some(1) => "top",
                Some(0) => "base",
                _ => "other",
            }
        } else {
            "-"
        };
        let list: Vec<String> = ids.iter().map(u64::to_string).collect();
        println!("wall {id} relation {name} {end} {}", list.join(" "));
        named.extend(ids);
    }
}

fn visit(record: Record, state: &mut State) -> Result<()> {
    let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref()) else {
        return Ok(());
    };
    let id = record.identity.element_id;
    let objects: Vec<(&str, &Value)> = graph
        .objects
        .iter()
        .map(|object| (object.class_name.as_str(), &object.fields))
        .collect();
    state.seen.insert(id);
    if WALLS.contains(&class) {
        state.walls.insert(id);
        dump("wall", id, class, &objects, true);
        relations(id, &objects, &mut state.named);
        return Ok(());
    }
    let slab = SLABS.contains(&class);
    if slab {
        state.slabs.insert(id);
    }
    dump("elem", id, class, &objects, slab);
    let mut held = BTreeSet::new();
    for &(_, fields) in &objects {
        ids_under(fields, &mut held);
    }
    let list: Vec<String> = held.iter().take(4000).map(u64::to_string).collect();
    println!("elem {id} refs {}", list.join(" "));
    Ok(())
}

fn main() -> Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut state = State::default();
    let options = Options {
        selected_classes: WALLS
            .iter()
            .chain(SLABS.iter())
            .map(|name| (*name).to_string())
            .collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| visit(record, &mut state))?;
    let missing: BTreeSet<u64> = state.named.difference(&state.seen).copied().collect();
    if !missing.is_empty() {
        let options = Options {
            selected_ids: missing,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| visit(record, &mut state))?;
    }
    let mut ids = state.walls.clone();
    ids.extend(state.slabs.iter().copied());
    let saved = native_element_parameters::read(&mut rf, &ids)?;
    for (id, parameters) in &saved.elements {
        for parameter in parameters {
            println!(
                "param {id} {} {} {} {:?}",
                parameter.parameter_id,
                parameter.value,
                parameter.source_element_id,
                parameter.name.as_deref().unwrap_or("-"),
            );
        }
    }
    eprintln!(
        "{} walls, {} floors and roofs, {} elements seen",
        state.walls.len(),
        state.slabs.len(),
        state.seen.len()
    );
    Ok(())
}
