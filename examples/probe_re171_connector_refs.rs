//! RE-171 (probe): an element's joins from its native `Connector` objects
//! (#528, B86).
//!
//! RE-169 found that a family instance's record holds a `Connector` object
//! per connector, each listing in `m_arrRefs` the elements joined to it
//! (`m_id`, `m_nIndex`, `m_connType`; type 1 beside the physical joins). This
//! reads every `FamilyInstance`, `Pipe` and `Duct` record through the native
//! record path, prints the fields of the first few `Connector` objects, and
//! scores the type-1 references against the `IfcRelConnectsPorts` of Revit's
//! export next to the model (`MODEL.ifc`), with a connector's own index taken
//! from each candidate: a field of the `Connector` object, or its position in
//! its manager's `m_connPtrArray`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re171_connector_refs -- MODEL.rvt

use rvt::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// Connector objects whose fields are printed whole.
const SHOWN: usize = 4;

type Join = [(u64, u32); 2];

fn revit_joins(step: &str) -> BTreeSet<Join> {
    let mut ports = BTreeMap::new();
    let mut rels = Vec::new();
    for line in step.lines() {
        let Some((id, body)) = line.strip_prefix('#').and_then(|rest| rest.split_once('=')) else {
            continue;
        };
        if let Some(args) = body.strip_prefix("IFCDISTRIBUTIONPORT(") {
            let name = args.split('\'').nth(3).unwrap_or_default();
            let rest = name
                .strip_prefix("InPort_")
                .or_else(|| name.strip_prefix("OutPort_"))
                .or_else(|| name.strip_prefix("Port_"));
            if let Some((element, index)) = rest.and_then(|rest| rest.split_once('_')) {
                if let (Ok(element), Ok(index)) = (element.parse(), index.parse()) {
                    ports.insert(id.trim().to_string(), (element, index));
                }
            }
        } else if let Some(args) = body.strip_prefix("IFCRELCONNECTSPORTS(") {
            let refs: Vec<String> = args
                .split('#')
                .skip(1)
                .map(|s| s.chars().take_while(char::is_ascii_digit).collect())
                .collect();
            if refs.len() >= 3 {
                rels.push((refs[1].clone(), refs[2].clone()));
            }
        }
    }
    rels.into_iter()
        .filter_map(|(a, b)| {
            let mut ends = [*ports.get(&a)?, *ports.get(&b)?];
            ends.sort();
            Some(ends)
        })
        .collect()
}

fn as_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.get("m_id").and_then(as_u64))
        .or_else(|| value.get("m_id64").and_then(as_u64))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let reference = std::path::Path::new(&path).with_extension("ifc");
    let theirs = std::fs::read_to_string(&reference)
        .map(|step| revit_joins(&step))
        .unwrap_or_default();
    let mut rf = RevitFile::open(&path)?;
    let options = native_document::Options {
        selected_classes: ["FamilyInstance", "Pipe", "Duct", "FlexPipe", "FlexDuct"]
            .into_iter()
            .map(String::from)
            .collect(),
        ..native_document::Options::default()
    };
    // candidate -> joins found
    let mut found: BTreeMap<String, BTreeSet<Join>> = BTreeMap::new();
    let mut shown = 0usize;
    let mut by_class: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut connector_fields: BTreeMap<String, usize> = BTreeMap::new();
    native_document::extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let class = record.class_name.clone().unwrap_or_default();
        let connectors: Vec<&_> = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "Connector")
            .collect();
        let counts = by_class.entry(class.clone()).or_default();
        counts.0 += 1;
        counts.1 += connectors.len();
        for (position, connector) in connectors.iter().enumerate() {
            if let Some(fields) = connector.fields.as_object() {
                for name in fields.keys() {
                    *connector_fields.entry(name.clone()).or_default() += 1;
                }
            }
            if shown < SHOWN {
                shown += 1;
                println!("{class} {id} connector {position}: {}", connector.fields);
            }
            let mut candidates: Vec<(String, u32)> = vec![
                ("position".into(), position as u32),
                ("position+1".into(), position as u32 + 1),
            ];
            if let Some(fields) = connector.fields.as_object() {
                for (name, value) in fields {
                    if let Some(index) = value.as_u64().and_then(|v| u32::try_from(v).ok()) {
                        candidates.push((format!("field {name}"), index));
                    }
                }
            }
            let refs = connector
                .fields
                .get("m_arrRefs")
                .and_then(|refs| refs.as_array())
                .cloned()
                .unwrap_or_default();
            for reference in refs {
                if reference.get("m_connType").and_then(|v| v.as_u64()) != Some(1) {
                    continue;
                }
                let (Some(other), Some(other_index)) = (
                    reference.get("m_id").and_then(as_u64),
                    reference.get("m_nIndex").and_then(|v| v.as_u64()),
                ) else {
                    continue;
                };
                for (name, own) in &candidates {
                    let mut ends = [(id, *own), (other, other_index as u32)];
                    ends.sort();
                    found.entry(name.clone()).or_default().insert(ends);
                }
            }
        }
        Ok(())
    })?;
    println!("records and connectors by class: {by_class:?}");
    println!("connector fields: {connector_fields:?}");
    println!("revit joins: {}", theirs.len());
    for (name, joins) in &found {
        let agree = joins.intersection(&theirs).count();
        println!(
            "candidate {name}: {} joins, {agree} of Revit's {}, {} not Revit's",
            joins.len(),
            theirs.len(),
            joins.len() - agree
        );
    }
    if let Some(best) = found
        .iter()
        .max_by_key(|(_, joins)| joins.intersection(&theirs).count())
    {
        for join in theirs.difference(best.1) {
            println!("  not found by {}: {join:?}", best.0);
        }
        for join in best.1.difference(&theirs).take(20) {
            println!("  found by {}, not Revit's: {join:?}", best.0);
        }
    }
    Ok(())
}
