//! RE-170 (probe): every join the export writes, one line each, for diffing
//! two refs on files with no export of Revit's to score against (#528, B86).
//!
//! Prints each `IfcRelConnectsPorts` as its two ends, each the entity and
//! `Tag` of the element its port is tied to and the port's connector index,
//! sorted.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re170_join_list -- MODEL.rvt

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let step = write_step(&result.model);
    let mut entity_of: BTreeMap<String, String> = BTreeMap::new();
    let mut ports: BTreeMap<String, (String, String)> = BTreeMap::new();
    let mut rels = Vec::new();
    for line in step.lines() {
        let Some((id, body)) = line.strip_prefix('#').and_then(|rest| rest.split_once('=')) else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Some(tag) = args.rsplit('\'').nth(1) {
            entity_of
                .entry(tag.to_string())
                .or_insert_with(|| entity.to_string());
        }
        if entity == "IFCDISTRIBUTIONPORT" {
            let name = args.split('\'').nth(3).unwrap_or_default();
            if let Some((element, index)) = name
                .strip_prefix("Port_")
                .and_then(|rest| rest.split_once('_'))
            {
                ports.insert(id.to_string(), (element.to_string(), index.to_string()));
            }
        } else if entity == "IFCRELCONNECTSPORTS" {
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
    let end = |port: &String| {
        ports.get(port).map(|(element, index)| {
            let entity = entity_of.get(element).map_or("?", String::as_str);
            format!("{entity} {element} {index}")
        })
    };
    let joins: BTreeSet<String> = rels
        .iter()
        .filter_map(|(a, b)| {
            let mut ends = [end(a)?, end(b)?];
            ends.sort();
            Some(format!("{} <-> {}", ends[0], ends[1]))
        })
        .collect();
    println!("joins {}", joins.len());
    for join in &joins {
        println!("{join}");
    }
    Ok(())
}
