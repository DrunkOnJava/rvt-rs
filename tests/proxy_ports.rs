//! B83 (B54, #528): a family instance rvt-rs writes as an
//! `IfcBuildingElementProxy` has a port for each connector Revit's export
//! gives it, nested in it.
//!
//! Revit's export of RE1 Electrical writes 30 of its 53 ports on elements it
//! and rvt-rs write as proxies: panels, receptacles, switches, junction boxes
//! and sensors. That export is IFC2X3, which ties a port to any element with
//! `IfcRelConnectsPortToElement`. IFC4 restricts that relationship to
//! distribution elements, and Revit's own IFC4 export (revit-ifc
//! `ConnectorExporter`) nests every port in its element with an `IfcRelNests`
//! instead. rvt-rs writes IFC4, so each such port must be a port
//! `Port_<ElementId>_<index>` nested in the proxy with that `Tag`.
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

/// An element's `Tag`, the last string among its arguments.
fn tag(args: &str) -> Option<u32> {
    args.rsplit('\'').nth(1)?.parse().ok()
}

/// Every port's key, each element's entity by `Tag`, and the `Tag` of the
/// element each port is nested in.
type Read = (
    BTreeSet<(u32, u32)>,
    BTreeMap<u32, String>,
    BTreeMap<(u32, u32), u32>,
);

fn read(step: &str) -> Read {
    let ents = entities(step);
    let ports = ents
        .values()
        .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
        .filter_map(|(_, args)| port_key(args))
        .collect();
    let mut entity_of = BTreeMap::new();
    let mut nested = BTreeMap::new();
    for (entity, args) in ents.values() {
        if let Some(tag) = tag(args) {
            entity_of.entry(tag).or_insert_with(|| entity.clone());
        }
        if entity != "IFCRELNESTS" {
            continue;
        }
        let all = refs(args);
        let Some(host) = all
            .get(1)
            .and_then(|host| ents.get(host))
            .and_then(|(_, a)| tag(a))
        else {
            continue;
        };
        for part in all.iter().skip(2) {
            if let Some(key) = ents
                .get(part)
                .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
                .and_then(|(_, args)| port_key(args))
            {
                nested.insert(key, host);
            }
        }
    }
    (ports, entity_of, nested)
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let (theirs, _, _) = read(&std::fs::read_to_string(reference).expect("reference"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let (_, our_entities, nested) = read(&write_step(&result.model));
    let wanted: Vec<&(u32, u32)> = theirs
        .iter()
        .filter(|(element, _)| {
            our_entities
                .get(element)
                .is_some_and(|entity| entity == "IFCBUILDINGELEMENTPROXY")
        })
        .collect();
    let missing: Vec<&&(u32, u32)> = wanted
        .iter()
        .filter(|key| nested.get(key) != Some(&key.0))
        .collect();
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "{name}: {} of Revit's {} ports on proxies written, nested in their element",
        wanted.len() - missing.len(),
        wanted.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{name}: {} of Revit's {} ports on family instances rvt-rs writes as proxies are \
             not written nested in their element; first: {:?}",
            missing.len(),
            wanted.len(),
            missing.iter().take(8).collect::<Vec<_>>()
        ));
    }
    wanted.len()
}

#[test]
fn proxies_have_revits_ports_nested() {
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
        "no port on a proxy was compared"
    );
}
