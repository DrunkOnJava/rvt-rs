//! B86 (B54, #528): a join between an element rvt-rs writes as a proxy and
//! another element is written as Revit's export writes it.
//!
//! Revit's export of RE1 Mechanical joins its air handling unit (427568,
//! which it and rvt-rs write as an `IfcBuildingElementProxy`) to five ducts
//! and pipes with `IfcRelConnectsPorts`. Since B83 a proxy has its ports,
//! nested in it, so every connection of Revit's export with a proxy at one
//! end must be written between the ports `Port_<ElementId>_<index>` of its
//! two ends, and rvt-rs must write no connection with a proxy at one end that
//! Revit's export does not hold.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Electrical.rvt`,
//! `RE1-Plumbing.rvt` and `RE1-Mechanical.rvt` with Revit's `RE1-*.ifc`).
//! Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// `#id -> (entity, args)` for every line of a STEP file.
fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

/// `(ElementId, connector index)` from a port's name.
fn port_key(args: &str) -> Option<(u32, u32)> {
    let name = args.split('\'').nth(3)?;
    let rest = name
        .strip_prefix("InPort_")
        .or_else(|| name.strip_prefix("OutPort_"))
        .or_else(|| name.strip_prefix("Port_"))?;
    let (id, index) = rest.split_once('_')?;
    Some((id.parse().ok()?, index.parse().ok()?))
}

/// A connection: the two ends' keys, in order.
type Connection = [(u32, u32); 2];

/// Every port-to-port connection, and each element's entity by `Tag`.
fn read(step: &str) -> (BTreeSet<Connection>, BTreeMap<u32, String>) {
    let ents = entities(step);
    let key = |id: &u64| {
        ents.get(id)
            .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
            .and_then(|(_, args)| port_key(args))
    };
    let mut connections = BTreeSet::new();
    let mut entity_of = BTreeMap::new();
    for (entity, args) in ents.values() {
        if let Some(tag) = args.rsplit('\'').nth(1).and_then(|t| t.parse().ok()) {
            entity_of.entry(tag).or_insert_with(|| entity.clone());
        }
        if entity != "IFCRELCONNECTSPORTS" {
            continue;
        }
        let all = refs(args);
        if let (Some(a), Some(b)) = (all.get(1).and_then(&key), all.get(2).and_then(&key)) {
            let mut ends = [a, b];
            ends.sort();
            connections.insert(ends);
        }
    }
    (connections, entity_of)
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let (theirs, _) = read(&std::fs::read_to_string(reference).expect("reference"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let (ours, our_entities) = read(&write_step(&result.model));
    let proxy = |connection: &Connection| {
        connection.iter().any(|(element, _)| {
            our_entities
                .get(element)
                .is_some_and(|entity| entity == "IFCBUILDINGELEMENTPROXY")
        })
    };
    let written = |connection: &Connection| {
        connection
            .iter()
            .all(|(element, _)| our_entities.contains_key(element))
    };
    let wanted: Vec<&Connection> = theirs
        .iter()
        .filter(|connection| proxy(connection) && written(connection))
        .collect();
    let missing: Vec<&&Connection> = wanted
        .iter()
        .filter(|connection| !ours.contains(**connection))
        .collect();
    let extra: Vec<&Connection> = ours
        .iter()
        .filter(|connection| proxy(connection) && !theirs.contains(*connection))
        .collect();
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "{name}: {} of Revit's {} connections with a proxy at one end written, {} not in \
         Revit's export",
        wanted.len() - missing.len(),
        wanted.len(),
        extra.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{name}: {} of Revit's {} connections with a proxy at one end are not written: {:?}",
            missing.len(),
            wanted.len(),
            missing
        ));
    }
    if !extra.is_empty() {
        failures.push(format!(
            "{name}: {} connections with a proxy at one end are not in Revit's export: {:?}",
            extra.len(),
            extra
        ));
    }
    wanted.len()
}

#[test]
fn joins_to_proxies_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut failures = Vec::new();
    let mut compared = 0;
    for model in ["Electrical", "Plumbing", "Mechanical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            eprintln!("skipping {}: model or reference absent", rvt.display());
            continue;
        }
        compared += check(&rvt, &reference, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        compared > 0 || !dir.join("RE1-Mechanical.rvt").exists(),
        "no connection with a proxy at one end was compared"
    );
}
