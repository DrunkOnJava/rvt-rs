//! B54 (#528): a family instance has a port for each of its connectors, as
//! Revit's export writes them.
//!
//! Revit's export of the RE1 MEP models writes a port for every connector of
//! a family instance (its symbol's connectors, cable tray and conduit ones
//! aside), joined or not: RE1 Electrical's lights, alarms and appliances,
//! Plumbing's fixtures and fittings, Mechanical's terminals and fittings. For
//! every such port on an element rvt-rs writes as a distribution element
//! (the only elements IFC4 lets a port be tied to), rvt-rs must write a port
//! `Port_<ElementId>_<index>` tied to the element with that `Tag`. Ducts and
//! pipes are `tests/duct_and_pipe_ports.rs`'s.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Electrical.rvt`,
//! `RE1-Plumbing.rvt` and `RE1-Mechanical.rvt` with Revit's `RE1-*.ifc`).
//! Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The IFC4 distribution element entities rvt-rs writes family instances as,
/// ducts and pipes aside.
const DISTRIBUTION_ENTITIES: [&str; 9] = [
    "IFCLIGHTFIXTURE",
    "IFCALARM",
    "IFCELECTRICAPPLIANCE",
    "IFCSANITARYTERMINAL",
    "IFCAIRTERMINAL",
    "IFCPIPEFITTING",
    "IFCDUCTFITTING",
    "IFCFLOWTERMINAL",
    "IFCFLOWFITTING",
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

/// Each port's key, with the `Tag` of the element it is tied to.
type Ports = BTreeMap<(u32, u32), Option<u32>>;

/// Each port's key, with the `Tag` of the element it is tied to, and each
/// element's entity by `Tag`.
fn ports_and_entities(step: &str) -> (Ports, BTreeMap<u32, String>) {
    let ents = entities(step);
    let mut ports: Ports = ents
        .values()
        .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
        .filter_map(|(_, args)| Some((port_key(args)?, None)))
        .collect();
    let mut entity_of = BTreeMap::new();
    for (entity, args) in ents.values() {
        if let Some(tag) = args.rsplit('\'').nth(1).and_then(|t| t.parse().ok()) {
            entity_of.entry(tag).or_insert_with(|| entity.clone());
        }
        if entity != "IFCRELCONNECTSPORTTOELEMENT" {
            continue;
        }
        let all = refs(args);
        let (Some(port), Some(element)) = (all.get(1), all.get(2)) else {
            continue;
        };
        let (Some((_, port_args)), Some((_, element_args))) = (ents.get(port), ents.get(element))
        else {
            continue;
        };
        if let Some(key) = port_key(port_args) {
            ports.insert(
                key,
                element_args
                    .rsplit('\'')
                    .nth(1)
                    .and_then(|t| t.parse().ok()),
            );
        }
    }
    (ports, entity_of)
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let (theirs, _) = ports_and_entities(&std::fs::read_to_string(reference).expect("reference"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let (ours, our_entities) = ports_and_entities(&write_step(&result.model));
    let wanted: BTreeSet<(u32, u32)> = theirs
        .keys()
        .filter(|(element, _)| {
            our_entities
                .get(element)
                .is_some_and(|entity| DISTRIBUTION_ENTITIES.contains(&entity.as_str()))
        })
        .copied()
        .collect();
    let missing: Vec<&(u32, u32)> = wanted
        .iter()
        .filter(|key| ours.get(key) != Some(&Some(key.0)))
        .collect();
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "{name}: {} of Revit's {} family-instance ports written",
        wanted.len() - missing.len(),
        wanted.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{name}: {} of Revit's {} ports on family instances rvt-rs writes as distribution \
             elements are not written, tied to their element; first: {:?}",
            missing.len(),
            wanted.len(),
            missing.iter().take(8).collect::<Vec<_>>()
        ));
    }
    wanted.len()
}

#[test]
fn family_instances_have_revits_ports() {
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
        compared > 0 || !dir.join("RE1-Electrical.rvt").exists(),
        "no family-instance port was compared"
    );
}
