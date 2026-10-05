//! B86 (B54, #528): every join of Revit's export between two elements rvt-rs
//! writes with ports is written, and every join rvt-rs writes between two
//! elements Revit's export holds is Revit's.
//!
//! Revit's export of the RE1 MEP models writes each join as an
//! `IfcRelConnectsPorts` between the ports `<In or Out>Port_<ElementId>_<index>`
//! of its two ends. rvt-rs writes ports on distribution elements and, since
//! B83, on proxies, named `Port_<ElementId>_<index>`. Wherever rvt-rs writes
//! both elements of one of Revit's joins as such elements, it must write that
//! join between those two ports: RE1 Mechanical's air handling unit (a proxy)
//! to its ducts and pipes, its air terminals to their duct fittings, and RE1
//! Plumbing's fixtures to their pipes among them. A join rvt-rs writes between
//! two elements Revit's export holds must be one of Revit's.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Electrical.rvt`,
//! `RE1-Plumbing.rvt` and `RE1-Mechanical.rvt` with Revit's `RE1-*.ifc`).
//! Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The entities rvt-rs ties ports to.
const PORTED_ENTITIES: [&str; 11] = [
    "IFCDUCTSEGMENT",
    "IFCPIPESEGMENT",
    "IFCDUCTFITTING",
    "IFCPIPEFITTING",
    "IFCAIRTERMINAL",
    "IFCSANITARYTERMINAL",
    "IFCLIGHTFIXTURE",
    "IFCALARM",
    "IFCELECTRICAPPLIANCE",
    "IFCBUILDINGELEMENTPROXY",
    "IFCFLOWTERMINAL",
];

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

/// A join: its two ends' keys, in order.
type Join = [(u32, u32); 2];

/// Every port-to-port join, and each element's entity by `Tag`.
fn read(step: &str) -> (BTreeSet<Join>, BTreeMap<u32, String>) {
    let ents = entities(step);
    let key = |id: &u64| {
        ents.get(id)
            .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
            .and_then(|(_, args)| port_key(args))
    };
    let mut joins = BTreeSet::new();
    let mut entity_of = BTreeMap::new();
    for (entity, args) in ents.values() {
        if let Some(tag) = args.rsplit('\'').nth(1).and_then(|t| t.parse().ok()) {
            entity_of.entry(tag).or_insert_with(|| entity.clone());
        }
        if entity != "IFCRELCONNECTSPORTS" {
            continue;
        }
        let all = refs(args);
        if let (Some(a), Some(b)) = (all.get(1).and_then(key), all.get(2).and_then(key)) {
            let mut ends = [a, b];
            ends.sort();
            joins.insert(ends);
        }
    }
    (joins, entity_of)
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let (theirs, their_entities) = read(&std::fs::read_to_string(reference).expect("reference"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let (ours, our_entities) = read(&write_step(&result.model));
    let ported = |join: &Join| {
        join.iter().all(|(element, _)| {
            our_entities
                .get(element)
                .is_some_and(|entity| PORTED_ENTITIES.contains(&entity.as_str()))
        })
    };
    let held = |join: &Join| {
        join.iter()
            .all(|(element, _)| their_entities.contains_key(element))
    };
    let wanted: Vec<&Join> = theirs.iter().filter(|join| ported(join)).collect();
    let missing: Vec<&Join> = wanted
        .iter()
        .copied()
        .filter(|join| !ours.contains(*join))
        .collect();
    let extra: Vec<&Join> = ours
        .iter()
        .filter(|join| held(join) && !theirs.contains(*join))
        .collect();
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "{name}: {} of Revit's {} joins between elements with ports written, {} written that \
         are not Revit's",
        wanted.len() - missing.len(),
        wanted.len(),
        extra.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{name}: {} of Revit's {} joins between elements rvt-rs writes with ports are not \
             written: {missing:?}",
            missing.len(),
            wanted.len()
        ));
    }
    if !extra.is_empty() {
        failures.push(format!(
            "{name}: {} joins written between elements Revit's export holds are not Revit's: \
             {extra:?}",
            extra.len()
        ));
    }
    wanted.len()
}

#[test]
fn every_join_revit_writes_is_written() {
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
        "no join was compared"
    );
}
