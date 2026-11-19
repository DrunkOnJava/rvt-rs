//! RE-462: what the walls of a curtain wall type hold, to tell the ones
//! Revit's export writes from the ones it leaves out (#1960).
//!
//! FACT under test. The Revit 2023 rule (RE-440) makes a wall a curtain wall by
//! its type, and the 2024 rule (RE-46, RE-72) by a mullion that names it or a
//! grid found. They disagree on rme_basic's walls 573660 and 573664 (one cell,
//! one panel, no mullion found). Revit's own export writes rac_basic's
//! one-cell wall 708088 (with four mullions and one panel) and leaves out the
//! 23 one-cell walls of Snowdon Towers' type "Solar Wall Window Frame", wall
//! and panel both. Hypothesis: those 23 are curtain walls nested in another
//! curtain wall's grid as a host panel, which Revit's exporter folds into the
//! owner, and rme_basic's two are not. This probe prints what each wall of a
//! curtain wall type holds, so that the two kinds can be told apart by their
//! own records.
//!
//! Output, one record per line (`<ids>` are ElementIds):
//!   file <path> release <n>
//!   wall <id> <class> type <id> <name> cells <n> gridlines <n> listed_mullions <n> mullion_records <n> objects <Class:n ...>
//!   wallnames <wall> curtain <ids|-> others <id:Class ...>
//!   object <wall> <Class>#<token> <fields>
//!   cell <wall> u <u> v <v> panel <id> <class>
//!   cellvalue <wall> u <u> v <v> <the cell's value>
//!   gridline <wall> <id> <class> mullions <n>
//!   panel <id> of <wall> <class> objects <Class:n ...> names <ids|->
//!   panelobject <panel> <Class>#<token> <fields>
//!   panelnames <panel> <id:Class ...>
//!   nested <wall> is named by the panel <id> of <wall>
//!
//! A wall of a curtain wall type is an `SWall` or `ArcWall` whose type
//! (`m_WallAttributesId`) is a `NewCurtainWallType`. Its cells are its
//! `CGridDivideFaceGStep.m_cellMap`. A grid line is a `CurtainWallGridLine` or
//! `CurtainGridLine` whose `m_wallId` or `m_hostId` is the wall, a mullion a
//! `Mullion` record on one of those lines or naming the wall as its host. The
//! objects, cell values and panel records are printed for the walls with one or
//! two cells and for the first wall of each type (the others only count), and
//! `nested` for every panel of any wall whose record names another wall of a
//! curtain wall type.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re462_curtain_wall_structure -- MODEL.rvt...

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

/// One cell of a wall's grid: its u and v, its panel and the cell's value.
struct Cell {
    u: i64,
    v: i64,
    panel: u64,
    value: Value,
}

/// A grid line record, with the wall it names and how many mullions it lists.
struct GridLine {
    id: u64,
    class: String,
    owner: Option<i64>,
    listed: usize,
}

/// A mullion record, with the grid line and the host it names.
struct Mullion {
    grid_line: Option<i64>,
    host: Option<i64>,
}

/// A wall of a curtain wall type.
struct CurtainWall {
    id: u64,
    class: String,
    type_id: u64,
    type_name: Option<String>,
    graph: Value,
    cells: Vec<Cell>,
}

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    println!("file {path} release {}", rf.layout_release()?);

    // Every wall with the graph of its record, and the grid lines and
    // mullions that name a wall.
    let mut walls: Vec<(u64, String, Value)> = Vec::new();
    let mut lines: Vec<GridLine> = Vec::new();
    let mut mullions: Vec<Mullion> = Vec::new();
    let options = Options {
        selected_classes: [
            "SWall",
            "ArcWall",
            "CurtainWallGridLine",
            "CurtainGridLine",
            "Mullion",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), &record.graph) else {
            return Ok(());
        };
        let graph = serde_json::to_value(graph)?;
        let id = record.identity.element_id;
        match class {
            "SWall" | "ArcWall" => walls.push((id, class.to_string(), graph)),
            "CurtainWallGridLine" | "CurtainGridLine" => lines.push(GridLine {
                id,
                class: class.to_string(),
                owner: field_id(&graph, "m_wallId").or_else(|| field_id(&graph, "m_hostId")),
                listed: field_ids(&graph, "m_mullionIds").len(),
            }),
            "Mullion" => mullions.push(Mullion {
                grid_line: field_id(&graph, "m_gridLineId"),
                host: field_id(&graph, "m_hostId"),
            }),
            _ => {}
        }
        Ok(())
    })?;

    // The walls whose type is a curtain wall type.
    let type_of = |graph: &Value| -> Option<u64> {
        graph["objects"].as_array()?.iter().find_map(|object| {
            u64::try_from(id_of(object["fields"].get("m_WallAttributesId")?)?).ok()
        })
    };
    let types: BTreeSet<u64> = walls.iter().filter_map(|(_, _, g)| type_of(g)).collect();
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
    let mut curtain: Vec<CurtainWall> = Vec::new();
    for (id, class, graph) in walls {
        let Some(type_id) = type_of(&graph) else {
            continue;
        };
        let Some(type_name) = type_names.get(&type_id) else {
            continue;
        };
        let mut cells = Vec::new();
        for object in graph["objects"].as_array().into_iter().flatten() {
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
                cells.push(Cell {
                    u,
                    v,
                    panel,
                    value: cell["second"].clone(),
                });
            }
        }
        curtain.push(CurtainWall {
            id,
            class,
            type_id,
            type_name: type_name.clone(),
            graph,
            cells,
        });
    }
    curtain.sort_by_key(|wall| wall.id);
    let curtain_ids: BTreeSet<i64> = curtain.iter().map(|wall| wall.id as i64).collect();

    // The walls printed in full: one or two cells, and the first of each type.
    let mut seen_types = BTreeSet::new();
    let detail: BTreeSet<u64> = curtain
        .iter()
        .filter(|wall| wall.cells.len() <= 2 || seen_types.insert(wall.type_id))
        .map(|wall| wall.id)
        .collect();

    // The panels of every wall: what each names, and which name another wall.
    let panel_wall: BTreeMap<u64, u64> = curtain
        .iter()
        .flat_map(|wall| wall.cells.iter().map(|cell| (cell.panel, wall.id)))
        .collect();
    struct Panel {
        class: String,
        graph: Value,
        names: BTreeSet<i64>,
    }
    let mut panels: BTreeMap<u64, Panel> = BTreeMap::new();
    if !panel_wall.is_empty() {
        let options = Options {
            selected_ids: panel_wall.keys().copied().collect(),
            releases: NATIVE_RELEASES,
            ..Options::default()
        };
        extract_graphs(&mut rf, &options, |record| {
            let Some(graph) = &record.graph else {
                return Ok(());
            };
            let graph = serde_json::to_value(graph)?;
            let mut names = BTreeSet::new();
            collect_ids(&graph, &mut names);
            let id = record.identity.element_id;
            let keep = panel_wall
                .get(&id)
                .is_some_and(|wall| detail.contains(wall));
            panels.insert(
                id,
                Panel {
                    class: record.class_name.clone().unwrap_or_else(|| "-".into()),
                    graph: if keep { graph } else { Value::Null },
                    names,
                },
            );
            Ok(())
        })?;
    }

    // The classes of the ids that the printed walls and panels name.
    let mut named: BTreeSet<u64> = BTreeSet::new();
    for wall in curtain.iter().filter(|wall| detail.contains(&wall.id)) {
        let mut ids = BTreeSet::new();
        collect_ids(&wall.graph, &mut ids);
        named.extend(ids.iter().filter_map(|id| u64::try_from(*id).ok()));
    }
    for (id, panel) in &panels {
        if panel_wall.get(id).is_some_and(|wall| detail.contains(wall)) {
            named.extend(panel.names.iter().filter_map(|id| u64::try_from(*id).ok()));
        }
    }
    let classes = classes_of(&mut rf, &named)?;
    let describe = |ids: &BTreeSet<i64>, skip: &[i64]| -> String {
        list(
            ids.iter()
                .filter(|id| !skip.contains(id) && !curtain_ids.contains(id))
                .map(
                    |id| match u64::try_from(*id).ok().and_then(|id| classes.get(&id)) {
                        Some(class) => format!("{id}:{class}"),
                        None => id.to_string(),
                    },
                ),
        )
    };

    for wall in &curtain {
        let own = wall.id as i64;
        let wall_lines: Vec<&GridLine> = lines.iter().filter(|l| l.owner == Some(own)).collect();
        let line_ids: BTreeSet<i64> = wall_lines.iter().map(|l| l.id as i64).collect();
        let listed: usize = wall_lines.iter().map(|l| l.listed).sum();
        let mullion_records = mullions
            .iter()
            .filter(|m| {
                m.host == Some(own) || m.grid_line.is_some_and(|line| line_ids.contains(&line))
            })
            .count();
        println!(
            "wall {} {} type {} {} cells {} gridlines {} listed_mullions {listed} mullion_records {mullion_records} objects {}",
            wall.id,
            wall.class,
            wall.type_id,
            wall.type_name.as_deref().unwrap_or("-"),
            wall.cells.len(),
            wall_lines.len(),
            histogram(&wall.graph),
        );
        let mut ids = BTreeSet::new();
        collect_ids(&wall.graph, &mut ids);
        let others: BTreeSet<i64> = ids.iter().copied().filter(|id| *id != own).collect();
        let curtain_named: Vec<i64> = others
            .iter()
            .copied()
            .filter(|id| curtain_ids.contains(id))
            .collect();
        for cell in &wall.cells {
            let class = panels.get(&cell.panel).map_or("-", |p| p.class.as_str());
            println!(
                "cell {} u {} v {} panel {} {class}",
                wall.id, cell.u, cell.v, cell.panel
            );
        }
        for (panel, owner) in &panel_wall {
            if *owner != wall.id {
                continue;
            }
            let Some(info) = panels.get(panel) else {
                continue;
            };
            for other in info.names.iter().filter(|id| curtain_ids.contains(id)) {
                if *other != own {
                    println!(
                        "nested {other} is named by the panel {panel} of {}",
                        wall.id
                    );
                }
            }
        }
        if !detail.contains(&wall.id) {
            continue;
        }
        println!(
            "wallnames {} curtain {} others {}",
            wall.id,
            list(&curtain_named),
            describe(&others, &[own]),
        );
        for object in wall.graph["objects"].as_array().into_iter().flatten() {
            println!(
                "object {} {}#{} {}",
                wall.id,
                object["class_name"].as_str().unwrap_or("-"),
                object["token"],
                clip(&object["fields"], 300),
            );
        }
        for cell in &wall.cells {
            println!(
                "cellvalue {} u {} v {} {}",
                wall.id,
                cell.u,
                cell.v,
                clip(&cell.value, 240)
            );
        }
        for line in &wall_lines {
            println!(
                "gridline {} {} {} mullions {}",
                wall.id, line.id, line.class, line.listed
            );
        }
        for cell in &wall.cells {
            let Some(info) = panels.get(&cell.panel) else {
                continue;
            };
            let names: Vec<i64> = info
                .names
                .iter()
                .copied()
                .filter(|id| curtain_ids.contains(id))
                .collect();
            println!(
                "panel {} of {} {} objects {} names {}",
                cell.panel,
                wall.id,
                info.class,
                histogram(&info.graph),
                list(&names),
            );
            for object in info.graph["objects"].as_array().into_iter().flatten() {
                let class = object["class_name"].as_str().unwrap_or("-");
                if class.contains("Curtain") || class.contains("Panel") || class.contains("Mullion")
                {
                    println!(
                        "panelobject {} {class}#{} {}",
                        cell.panel,
                        object["token"],
                        clip(&object["fields"], 300),
                    );
                }
            }
            println!(
                "panelnames {} {}",
                cell.panel,
                describe(&info.names, &[cell.panel as i64, own])
            );
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_re462_curtain_wall_structure MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
