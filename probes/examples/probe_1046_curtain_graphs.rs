//! #1046 step 2 (W7, RE-268): what a curtain wall's records hold about its
//! grid and the joins of its mullions, for finding which ends of a mullion
//! Revit's solid keeps (RE-248 §2).
//!
//! FACT under test: a curtain mullion's `NetSurfaceArea` counts 0, 1 or 2 of
//! its section's end faces, and the mullion's own record (`MullionFamRef`
//! names only its host, a driver and an edge tag) holds no join. Hypothesis:
//! the joins are held by the host curtain wall's record, its type's, or by
//! records of classes named for the curtain grid, which this lists.
//!
//! Output, one line per item:
//!   class <name> <records>              every record class of the file
//!   mul <id> ...                        every member and plate the export writes
//!   host <id> ...                       every host: object classes of its graph
//!   hostdump / typedump / mulldump / griddump <id> <class> obj<n> <class> <fields as JSON>
//!                                       a graph's objects, the first three of each class
//!   cls <class> <id> ...                records of classes named for a grid
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1046_curtain_graphs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use serde_json::Value;

const FIELD_CHARS: usize = 400;
const FULL_PER_CLASS: usize = 3;
const COMPACT_PER_CLASS: usize = 60;

fn link(value: &Value) -> Option<i64> {
    value.pointer("/m_id/m_id64").and_then(Value::as_i64)
}

fn clip(text: String) -> String {
    if text.chars().count() <= FIELD_CHARS {
        text
    } else {
        let head: String = text.chars().take(FIELD_CHARS).collect();
        format!("{head}...")
    }
}

fn find<'a>(graph: &'a ObjectGraph, name: &str) -> Option<&'a Value> {
    graph
        .objects
        .iter()
        .find_map(|object| object.fields.get(name))
}

fn show(value: Option<&Value>) -> String {
    match value {
        None => "-".to_string(),
        Some(v) => match link(v) {
            Some(id) => format!("@{id}"),
            None => clip(v.to_string()),
        },
    }
}

fn id_of(value: Option<&Value>) -> Option<u64> {
    value.and_then(link).and_then(|id| u64::try_from(id).ok())
}

fn dump(tag: &str, id: u64, class: &str, graph: &ObjectGraph) {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for (index, object) in graph.objects.iter().enumerate() {
        let n = seen.entry(object.class_name.as_str()).or_default();
        *n += 1;
        if *n > FULL_PER_CLASS {
            continue;
        }
        let fields = clip(object.fields.to_string());
        println!(
            "{tag} {id} {class} obj{index} {} {fields}",
            object.class_name
        );
    }
    let census: Vec<String> = seen.iter().map(|(c, n)| format!("{c}x{n}")).collect();
    println!("{tag} {id} {class} classes {}", census.join(","));
    let mut edges: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for edge in &graph.edges {
        let from = graph.objects[edge.source_object_index].class_name.as_str();
        let to = graph.objects[edge.target_object_index].class_name.as_str();
        *edges.entry((from, to)).or_default() += 1;
    }
    for ((from, to), n) in &edges {
        println!("{tag} {id} {class} edge {from}->{to} {n}");
    }
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

    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    let mut census: BTreeMap<String, usize> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            graph_classes: Some(BTreeSet::new()),
            ..Options::default()
        },
        |record| {
            let class = record.class_name.unwrap_or_else(|| "-".to_string());
            *census.entry(class.clone()).or_default() += 1;
            classes.insert(record.identity.element_id, class);
            Ok(())
        },
    )?;
    for (class, n) in &census {
        println!("class {class} {n}");
    }

    let mut mullions: BTreeMap<u64, Option<u64>> = BTreeMap::new();
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
            let owner = record.identity.owning_element_id;
            let owner_class = u64::try_from(owner)
                .ok()
                .and_then(|owner| classes.get(&owner))
                .map_or("-", String::as_str);
            let host = id_of(find(graph, "m_hostId"));
            mullions.insert(id, host);
            println!(
                "mul {id} {} owner={owner} ownerclass={owner_class} host={} type={} edge={} driver={} param={} elev={} flippedY={} vertical={} mirrored={} cell0={} cell1={} objects={}",
                record.class_name.as_deref().unwrap_or("-"),
                show(find(graph, "m_hostId")),
                show(find(graph, "m_masterSymbolId")),
                show(find(graph, "m_edgeTag")),
                show(find(graph, "m_driverId")),
                show(find(graph, "m_hostParam")),
                show(find(graph, "m_elevation")),
                show(find(graph, "m_flippedY")),
                show(find(graph, "m_bVertical")),
                show(find(graph, "m_refPntMirrored")),
                show(find(graph, "m_gridId0")),
                show(find(graph, "m_gridId1")),
                graph.objects.len(),
            );
            Ok(())
        },
    )?;

    let mut by_host: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    for (id, host) in &mullions {
        if let Some(host) = host {
            by_host.entry(*host).or_default().push(*id);
        }
    }
    let mut ranked: Vec<(usize, u64)> = by_host.iter().map(|(h, m)| (m.len(), *h)).collect();
    ranked.sort();
    let mut sample_hosts: BTreeSet<u64> = BTreeSet::new();
    if let Some(small) = ranked.iter().find(|(n, _)| *n >= 6) {
        sample_hosts.insert(small.1);
    }
    if let Some(middle) = ranked.get(ranked.len() / 2) {
        sample_hosts.insert(middle.1);
    }
    if let Some(large) = ranked.last() {
        sample_hosts.insert(large.1);
    }
    let sample_mullions: BTreeSet<u64> = sample_hosts
        .iter()
        .flat_map(|host| by_host[host].iter().take(2).copied())
        .collect();

    let hosts: BTreeSet<u64> = by_host.keys().copied().collect();
    let mut host_types: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: hosts,
            ..Options::default()
        },
        |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                *seen.entry(object.class_name.as_str()).or_default() += 1;
            }
            let census: Vec<String> = seen.iter().map(|(c, n)| format!("{c}x{n}")).collect();
            let kind = id_of(find(graph, "m_WallAttributesId"));
            if let Some(kind) = kind {
                host_types.insert(kind);
            }
            println!(
                "host {id} {class} mullions={} owner={} type={} objects={} edges={} {}",
                by_host.get(&id).map_or(0, Vec::len),
                record.identity.owning_element_id,
                kind.map_or("-".to_string(), |k| k.to_string()),
                graph.objects.len(),
                graph.edges.len(),
                census.join(","),
            );
            if sample_hosts.contains(&id) {
                dump("hostdump", id, class, graph);
            }
            Ok(())
        },
    )?;

    let wanted: BTreeSet<String> = census
        .keys()
        .filter(|class| {
            let lower = class.to_lowercase();
            ["curtain", "grid", "mullion", "panel", "join"]
                .iter()
                .any(|word| lower.contains(word))
        })
        .cloned()
        .collect();
    let mut shown: BTreeMap<String, usize> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: host_types
                .iter()
                .chain(sample_mullions.iter())
                .copied()
                .collect(),
            selected_classes: wanted,
            ..Options::default()
        },
        |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let id = record.identity.element_id;
            if host_types.contains(&id) {
                dump("typedump", id, class, graph);
            } else if sample_mullions.contains(&id) {
                dump("mulldump", id, class, graph);
            } else {
                let n = shown.entry(class.to_string()).or_default();
                *n += 1;
                if *n <= FULL_PER_CLASS {
                    dump("griddump", id, class, graph);
                }
                if *n <= COMPACT_PER_CLASS {
                    println!(
                        "cls {class} {id} owner={} objects={} edges={}",
                        record.identity.owning_element_id,
                        graph.objects.len(),
                        graph.edges.len(),
                    );
                }
            }
            Ok(())
        },
    )?;
    eprintln!(
        "{} members and plates, {} hosts, {} record classes",
        ids.len(),
        by_host.len(),
        census.len()
    );
    Ok(())
}
