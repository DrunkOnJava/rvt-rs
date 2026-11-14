//! #1977: what the file says about a multistory stair, to place the copy Revit
//! writes on each storey above.
//!
//! FACT under test. Revit's own IFC4 export of Autodesk's Snowdon Towers
//! architectural sample writes each of its three multistory assembled stairs
//! (621690, 701090, 882250; L3 to L4) twice: the stair on L3 and a `:2` copy on
//! L4, with its own flights, stringers, landing and railings, 53 entities more
//! than rvt-rs writes. On the 30 stringer pairs the copy has the original's
//! extents and volume and sits exactly 4.064 m above it. Hypothesis: the file
//! names the stair that repeats (`MultistoryStairs.m_standardStairsId`), the
//! Level it starts from (`m_refLevelId`) and the Level it repeats up to
//! (`MultistoryStairsDataCell.m_instancesUpToLevelId`), so that the shift of
//! each copy is the elevation difference of those two Levels.
//!
//! Output, one record per line (`<ids>` are ElementIds):
//!   file <path> release <n>
//!   multistory <id> objects <Class:n ...>
//!   object <id> <Class> <scalar and id fields, or the pair set and the cell list as JSON>
//!   standard <id> stair <id> ref_level <id|-> excluded <ids|-> type <id|->
//!   cell <id> up_to <id|-> next_index <n|-> level_pairs <ids|->
//!   level <id> <Class> <fields>
//!   stair <id> <Class> <fields>
//!
//! A multistory stair is a record of the class `MultistoryStairs`; its
//! `MultistoryStairsDataCell` is read from the objects of the same record. The
//! Levels printed are every `Level` record of the file, with the objects of its
//! graph that name an elevation or a name, and the stairs the records
//! `m_standardStairsId` name, with the objects of their graphs that hold a base
//! or a top Level.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1977_multistory_stairs -- MODEL.rvt...

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use serde_json::Value;

/// The `n` of `{"m_id":{"m_id":n}}` or `{"m_id":{"m_id64":n}}`.
fn id_of(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner
        .get("m_id")
        .or_else(|| inner.get("m_id64"))
        .and_then(Value::as_i64)
}

/// Every ElementId held in `value`, at any depth.
fn collect_ids(value: &Value, out: &mut BTreeSet<i64>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = id_of(value).filter(|id| *id > 0) {
                out.insert(id);
            }
            map.values().for_each(|child| collect_ids(child, out));
        }
        Value::Array(items) => items.iter().for_each(|item| collect_ids(item, out)),
        _ => {}
    }
}

/// The ElementId in the first object of `graph` that has the field `name`.
fn field_id(graph: &Value, name: &str) -> Option<i64> {
    graph["objects"]
        .as_array()?
        .iter()
        .find_map(|object| id_of(object["fields"].get(name)?))
}

/// The ElementIds in the field `name` of every object of `graph`.
fn field_ids(graph: &Value, name: &str) -> BTreeSet<i64> {
    let mut out = BTreeSet::new();
    for object in graph["objects"].as_array().into_iter().flatten() {
        if let Some(value) = object["fields"].get(name) {
            collect_ids(value, &mut out);
        }
    }
    out
}

/// The class of each object of `graph`, with how many there are.
fn histogram(graph: &Value) -> String {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for object in graph["objects"].as_array().into_iter().flatten() {
        *counts
            .entry(object["class_name"].as_str().unwrap_or("-"))
            .or_default() += 1;
    }
    counts
        .iter()
        .map(|(class, count)| format!("{class}:{count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// `value` as JSON, cut at `max` characters.
fn clip(value: &Value, max: usize) -> String {
    let text = value.to_string();
    if text.chars().count() <= max {
        return text;
    }
    let head: String = text.chars().take(max).collect();
    format!("{head}...")
}

/// The ids as text, or `-` for none.
fn list<T: ToString>(ids: impl IntoIterator<Item = T>) -> String {
    let ids: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
    if ids.is_empty() {
        "-".to_string()
    } else {
        ids.join(" ")
    }
}

/// The integer in the first object of `graph` that has the field `name`.
fn field_int(graph: &Value, name: &str) -> Option<i64> {
    graph["objects"]
        .as_array()?
        .iter()
        .find_map(|object| object["fields"].get(name)?.as_i64())
}

/// The fields of `fields` that hold a number, a flag, a string or an ElementId, as
/// `name=value`, in order; pointers and nested objects are left out.
fn scalars(fields: &Value) -> String {
    let Some(map) = fields.as_object() else {
        return "-".to_string();
    };
    let mut out: Vec<String> = Vec::new();
    for (name, value) in map {
        match value {
            Value::Number(_) | Value::Bool(_) | Value::String(_) => {
                out.push(format!("{name}={value}"));
            }
            Value::Object(_) => {
                if let Some(id) = id_of(value) {
                    out.push(format!("{name}=#{id}"));
                }
            }
            _ => {}
        }
    }
    out.join(" ")
}

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    println!("file {path} release {}", rf.layout_release()?);

    // The multistory stairs, with every object of their records.
    let mut standard: BTreeSet<u64> = BTreeSet::new();
    let options = Options {
        selected_classes: ["MultistoryStairs".to_string()].into_iter().collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let graph = serde_json::to_value(graph)?;
        let id = record.identity.element_id;
        println!("multistory {id} objects {}", histogram(&graph));
        for object in graph["objects"].as_array().into_iter().flatten() {
            let class = object["class_name"].as_str().unwrap_or("-");
            if class.starts_with("MultistoryStairs") {
                println!("object {id} {class} {}", scalars(&object["fields"]));
            }
            if class == "ElementIdIndexPairSet" || class == "CellList" {
                println!("object {id} {class} {}", clip(&object["fields"], 1500));
            }
        }
        let stair = field_id(&graph, "m_standardStairsId");
        if let Some(stair) = stair.and_then(|id| u64::try_from(id).ok()) {
            standard.insert(stair);
        }
        println!(
            "standard {id} stair {} ref_level {} excluded {} type {}",
            stair.map_or("-".to_string(), |id| id.to_string()),
            field_id(&graph, "m_refLevelId").map_or("-".to_string(), |id| id.to_string()),
            list(field_ids(&graph, "m_excludedBaseLevelIds")),
            field_id(&graph, "m_typeId").map_or("-".to_string(), |id| id.to_string()),
        );
        println!(
            "cell {id} up_to {} next_index {} level_pairs {}",
            field_id(&graph, "m_instancesUpToLevelId").map_or("-".to_string(), |id| id.to_string()),
            field_int(&graph, "m_nextIndex").map_or("-".to_string(), |n| n.to_string()),
            list(field_ids(&graph, "m_levelIdIndexPairSet")),
        );
        Ok(())
    })?;

    // Every Level of the file: its name and elevation.
    let options = Options {
        selected_classes: ["Level".to_string()].into_iter().collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let graph = serde_json::to_value(graph)?;
        let id = record.identity.element_id;
        for object in graph["objects"].as_array().into_iter().flatten() {
            let class = object["class_name"].as_str().unwrap_or("-");
            if class == "Level" || class == "LevelAttributes" {
                println!("level {id} {class} {}", scalars(&object["fields"]));
            }
        }
        Ok(())
    })?;

    // The stairs the multistory records name: what their graphs hold.
    if !standard.is_empty() {
        let options = Options {
            selected_ids: standard,
            releases: NATIVE_RELEASES,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            let Some(graph) = &record.graph else {
                return Ok(());
            };
            let graph = serde_json::to_value(graph)?;
            let id = record.identity.element_id;
            for object in graph["objects"].as_array().into_iter().flatten() {
                let class = object["class_name"].as_str().unwrap_or("-");
                if class == "Stairs" || class == "StairsAttributes" || class == "StairsElement" {
                    println!("stair {id} {class} {}", scalars(&object["fields"]));
                }
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_1977_multistory_stairs MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
