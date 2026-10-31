//! #1046 step 2 (W7, RE-268): the mullion joins a curtain wall's record lists.
//!
//! FACT under test: a curtain wall's `WallCGDriver.m_mullionJoins` lists, per
//! grid vertex, the mullion ends that meet there (`m_gridJoinSet`: the
//! `MullionFamRef.m_edgeTag` of each mullion and which end of it,
//! `m_end` 0 or 1), and Revit's `Qto_MemberBaseQuantities.NetSurfaceArea`
//! of a mullion (its section's perimeter times its length plus 0, 1 or 2 of
//! its section's areas, RE-248) counts the ends that no join covers.
//! `tools/re/mullion_joins_vs_ifc.py` scores the lines against Revit's
//! export.
//!
//! Output, one line per item:
//!   mul <id> <host> <type> <edge>        every member and plate the export writes
//!   drv <host> <joinid> <joins> <mullions> <CurtainGrid.m_joinTypes as JSON>
//!   join <host> <id> <butt> <edge>:<end>,...   one line per m_mullionJoins entry
//!   byedge <host> <edge>=<mullion>,...   CGDriverMullionData.m_mullionsByEdge
//!   full <host> <class> <JSON>           whole objects of a few small hosts
//!   type <id> <class> <JSON>             the objects of each host type
//!   gl <id> <owner> <JSON>               each CurtainGridLine's own object
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1046_mullion_joins -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::{GraphObject, ObjectGraph};
use serde_json::Value;

const FULL_CHARS: usize = 30_000;
const SAMPLE_HOSTS: usize = 3;

fn link(value: &Value) -> Option<i64> {
    value.pointer("/m_id/m_id64").and_then(Value::as_i64)
}

fn clip(text: String) -> String {
    if text.chars().count() <= FULL_CHARS {
        text
    } else {
        let head: String = text.chars().take(FULL_CHARS).collect();
        format!("{head}...")
    }
}

fn find<'a>(graph: &'a ObjectGraph, name: &str) -> Option<&'a Value> {
    graph
        .objects
        .iter()
        .find_map(|object| object.fields.get(name))
}

fn object<'a>(graph: &'a ObjectGraph, class: &str) -> Option<&'a GraphObject> {
    graph
        .objects
        .iter()
        .find(|object| object.class_name == class)
}

fn id_of(value: Option<&Value>) -> Option<u64> {
    value.and_then(link).and_then(|id| u64::try_from(id).ok())
}

fn list<'a>(value: &'a Value, key: &str) -> &'a [Value] {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

fn number(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map_or("-".to_string(), |n| n.to_string())
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCMEMBER")
                || ifc_type.eq_ignore_ascii_case("IFCPLATE") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect();

    let mut by_host: BTreeMap<u64, usize> = BTreeMap::new();
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
            let id = record.identity.element_id;
            let host = id_of(find(graph, "m_hostId"));
            if let Some(host) = host {
                *by_host.entry(host).or_default() += 1;
            }
            println!(
                "mul {id} {} {} {}",
                host.map_or("-".to_string(), |h| h.to_string()),
                id_of(find(graph, "m_masterSymbolId")).map_or("-".to_string(), |t| t.to_string()),
                find(graph, "m_edgeTag")
                    .and_then(Value::as_i64)
                    .map_or("-".to_string(), |e| e.to_string()),
            );
            Ok(())
        },
    )?;

    let hosts: BTreeSet<u64> = by_host.keys().copied().collect();
    let mut host_types: BTreeSet<u64> = BTreeSet::new();
    let mut joins_of: BTreeMap<u64, usize> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: hosts,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            if let Some(kind) = id_of(find(graph, "m_WallAttributesId")) {
                host_types.insert(kind);
            }
            let Some(driver) = object(graph, "WallCGDriver") else {
                println!("drv {id} - - - -");
                return Ok(());
            };
            let joins = list(&driver.fields, "m_mullionJoins");
            joins_of.insert(id, joins.len());
            let join_types = object(graph, "CurtainGrid")
                .and_then(|grid| grid.fields.get("m_joinTypes"))
                .map_or("-".to_string(), Value::to_string);
            println!(
                "drv {id} {} {} {} {join_types}",
                number(&driver.fields, "m_mullionJoinId"),
                joins.len(),
                by_host.get(&id).copied().unwrap_or(0),
            );
            for join in joins {
                let set: Vec<String> = list(join, "m_gridJoinSet")
                    .iter()
                    .map(|member| {
                        format!(
                            "{}:{}",
                            number(member, "m_edgeTag"),
                            number(member, "m_end")
                        )
                    })
                    .collect();
                println!(
                    "join {id} {} {} {}",
                    number(join, "m_id"),
                    join.get("m_constULinesButt")
                        .map_or("-".to_string(), Value::to_string),
                    set.join(",")
                );
            }
            if let Some(data) = object(graph, "CGDriverMullionData") {
                let pairs: Vec<String> = list(&data.fields, "m_mullionsByEdge")
                    .iter()
                    .map(|entry| {
                        format!(
                            "{}={}",
                            number(entry, "m_edgeTag"),
                            entry
                                .get("m_mullionId")
                                .and_then(link)
                                .map_or("-".to_string(), |m| m.to_string())
                        )
                    })
                    .collect();
                println!("byedge {id} {}", pairs.join(","));
            }
            Ok(())
        },
    )?;

    let mut ranked: Vec<(usize, u64)> = by_host
        .iter()
        .filter(|(host, mullions)| {
            **mullions >= 8 && joins_of.get(*host).copied().unwrap_or(0) >= 4
        })
        .map(|(host, mullions)| (*mullions, *host))
        .collect();
    ranked.sort();
    let samples: BTreeSet<u64> = ranked
        .iter()
        .take(SAMPLE_HOSTS)
        .map(|(_, host)| *host)
        .collect();
    const FULL_CLASSES: [&str; 7] = [
        "WallCGDriver",
        "CurtainGrid",
        "CGDriverMullionData",
        "CGridDivideFaceGStep",
        "CWallGridFaceGStep",
        "HostObjMiscData",
        "SWall",
    ];
    let mut wanted = BTreeSet::from(["CurtainGridLine".to_string()]);
    wanted.insert("NewCurtainWallType".to_string());
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: samples.iter().chain(host_types.iter()).copied().collect(),
            selected_classes: wanted,
            ..Options::default()
        },
        |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let id = record.identity.element_id;
            if class == "CurtainGridLine" {
                let root = &graph.objects[0];
                let mut fields = root.fields.clone();
                if let Some(map) = fields.as_object_mut() {
                    map.retain(|_, value| !value.is_object() || link(value).is_some());
                }
                println!(
                    "gl {id} {} {}",
                    record.identity.owning_element_id,
                    clip(fields.to_string())
                );
                for object in &graph.objects {
                    if object.class_name == "GLine" {
                        println!("glline {id} {}", clip(object.fields.to_string()));
                    }
                }
            } else if host_types.contains(&id) {
                for object in &graph.objects {
                    println!(
                        "type {id} {} {}",
                        object.class_name,
                        clip(object.fields.to_string())
                    );
                }
            }
            if samples.contains(&id) {
                for object in &graph.objects {
                    if FULL_CLASSES.contains(&object.class_name.as_str()) {
                        println!(
                            "full {id} {} {}",
                            object.class_name,
                            clip(object.fields.to_string())
                        );
                    }
                }
            }
            Ok(())
        },
    )?;
    eprintln!(
        "{} members and plates, {} hosts, {} sample hosts",
        ids.len(),
        by_host.len(),
        samples.len()
    );
    Ok(())
}
