//! RE-440: what the Revit 2023 "ArcWall" byte pattern matches, how many
//! Levels a file holds, and which walls are curtain walls, by the file's own
//! native records.
//!
//! FACT under test: the standard ArcWall pattern (RE-14.3, RE-15) finds walls.
//! On `Revit_IFC5_Einhoven` its 24 matches stand at four base elevations, which
//! was the reason a lone 2015 to 2023 Level was not trusted. Hypothesis: the
//! matches are not walls, and the file has one Level.
//!
//! Output, one record per line:
//!   match <stream> <offset> <ElementId|-> <native class|-> <base elevation ft>
//!   matches <count> without_id <count> walls <count> classes <class:count ...>
//!   levels <ElementId ...>
//!   curtain_wall <ElementId> type <type id> <type name>
//!   cell <wall ElementId> u <u> v <v> panel <panel ElementId> <panel class|->
//!
//! A `match` is every offset where `ArcWallRecord::decode_standard` succeeds in
//! a partition stream, with the ElementId its trailer gives and the class of
//! that ElementId's native record. `levels` is every native `Level` record. A
//! `curtain_wall` is every `SWall` or `ArcWall` whose type
//! (`m_WallAttributesId`) is a `NewCurtainWallType`; its cells are its
//! `CGridDivideFaceGStep.m_cellMap`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re440_curtain_wall_records -- MODEL.rvt...

use std::collections::{BTreeMap, BTreeSet};

use anyhow::Result;
use rvt::RevitFile;
use rvt::arc_wall_record::ArcWallRecord;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use serde_json::Value;

/// The classes of every record in `ids`, by ElementId.
fn classes_of(rf: &mut RevitFile, ids: &BTreeSet<u64>) -> Result<BTreeMap<u64, String>> {
    let mut out = BTreeMap::new();
    if ids.is_empty() {
        return Ok(out);
    }
    let options = Options {
        selected_ids: ids.clone(),
        graph_classes: Some(BTreeSet::new()),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(rf, &options, |record| {
        if let Some(class) = record.class_name {
            out.insert(record.identity.element_id, class);
        }
        Ok(())
    })?;
    Ok(out)
}

/// `{"m_id":{"m_id":n}}` or `{"m_id":{"m_id64":n}}` as `n`.
fn id_of(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner
        .get("m_id")
        .or_else(|| inner.get("m_id64"))
        .and_then(Value::as_i64)
}

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");

    // Every match of the standard pattern, with its trailer ElementId.
    let mut matches = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for offset in ArcWallRecord::scan_standard_for_revit_version(release, buf).offsets {
            let Ok(record) = ArcWallRecord::decode_standard(buf, offset) else {
                continue;
            };
            let trailer = ArcWallRecord::decode_trailer(buf, offset);
            let id = trailer
                .as_ref()
                .and_then(|t| t.element_id)
                .and_then(rvt::ElementId::as_u32)
                .map(u64::from);
            let base = trailer.as_ref().and_then(|t| t.base_elevation_feet);
            matches.push((stream.clone(), offset, id, base, record));
        }
    }
    let ids: BTreeSet<u64> = matches.iter().filter_map(|m| m.2).collect();
    let classes = classes_of(&mut rf, &ids)?;
    let mut histogram: BTreeMap<&str, usize> = BTreeMap::new();
    for (stream, offset, id, base, _) in &matches {
        let class = id
            .and_then(|id| classes.get(&id))
            .map_or("-", String::as_str);
        *histogram.entry(class).or_default() += 1;
        let id = id.map_or("-".to_string(), |id| id.to_string());
        let base = base.map_or("-".to_string(), |b| format!("{b:.3}"));
        println!("match {stream} {offset} {id} {class} {base}");
    }
    let walls = histogram.get("ArcWall").copied().unwrap_or(0)
        + histogram.get("SWall").copied().unwrap_or(0);
    let without_id = matches.iter().filter(|m| m.2.is_none()).count();
    let histogram: Vec<String> = histogram
        .iter()
        .map(|(class, count)| format!("{class}:{count}"))
        .collect();
    println!(
        "matches {} without_id {without_id} walls {walls} classes {}",
        matches.len(),
        histogram.join(" ")
    );

    // Every native Level, and every wall with the graph of its record.
    let mut levels = Vec::new();
    let mut walls: Vec<(u64, Value)> = Vec::new();
    let options = Options {
        selected_classes: BTreeSet::from([
            "Level".to_string(),
            "SWall".to_string(),
            "ArcWall".to_string(),
        ]),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        match record.class_name.as_deref() {
            Some("Level") => levels.push(record.identity.element_id),
            Some("SWall" | "ArcWall") => {
                if let Some(graph) = &record.graph {
                    walls.push((record.identity.element_id, serde_json::to_value(graph)?));
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let levels: Vec<String> = levels.iter().map(u64::to_string).collect();
    println!("levels {}", levels.join(" "));

    // The walls whose type is a curtain wall type, and their cells' panels.
    let type_of = |graph: &Value| -> Option<u64> {
        graph["objects"].as_array()?.iter().find_map(|object| {
            u64::try_from(id_of(object["fields"].get("m_WallAttributesId")?)?).ok()
        })
    };
    let types: BTreeSet<u64> = walls.iter().filter_map(|(_, g)| type_of(g)).collect();
    let mut type_names: BTreeMap<u64, Option<String>> = BTreeMap::new();
    if !types.is_empty() {
        let options = Options {
            selected_ids: types,
            releases: NATIVE_RELEASES,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            if record.class_name.as_deref() != Some("NewCurtainWallType") {
                return Ok(());
            }
            let name = record.graph.as_ref().and_then(|graph| {
                graph
                    .objects
                    .iter()
                    .find(|o| o.class_name == "SymbolInfo")?
                    .fields
                    .get("m_name")?
                    .as_str()
                    .map(str::to_string)
            });
            type_names.insert(record.identity.element_id, name);
            Ok(())
        })?;
    }
    let mut cells = Vec::new();
    for (wall, graph) in &walls {
        let Some(type_id) = type_of(graph) else {
            continue;
        };
        let Some(name) = type_names.get(&type_id) else {
            continue;
        };
        println!(
            "curtain_wall {wall} type {type_id} {}",
            name.as_deref().unwrap_or("-")
        );
        let objects = graph["objects"].as_array().cloned().unwrap_or_default();
        for object in &objects {
            if object["class_name"] != "CGridDivideFaceGStep" {
                continue;
            }
            for cell in object["fields"]["m_cellMap"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let (Some(u), Some(v)) =
                    (cell["first"]["m_u"].as_i64(), cell["first"]["m_v"].as_i64())
                else {
                    continue;
                };
                let Some(panel) = cell["second"]
                    .get("m_panelId")
                    .and_then(id_of)
                    .and_then(|id| u64::try_from(id).ok())
                else {
                    continue;
                };
                cells.push((*wall, u, v, panel));
            }
        }
    }
    let panels: BTreeSet<u64> = cells.iter().map(|c| c.3).collect();
    let panel_classes = classes_of(&mut rf, &panels)?;
    for (wall, u, v, panel) in cells {
        let class = panel_classes.get(&panel).map_or("-", String::as_str);
        println!("cell {wall} u {u} v {v} panel {panel} {class}");
    }
    Ok(())
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_re440_curtain_wall_records MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
