//! B54 (#528): every duct and pipe has a port at each end, as Revit's export
//! writes them.
//!
//! Revit's export of RE1 Mechanical and Plumbing writes an
//! `IfcDistributionPort` for connectors 0 and 1 of 92 of the 94 ducts and
//! pipes, named `<In or Out>Port_<ElementId>_<index>` and tied to the duct or
//! pipe, whether or not anything is connected there. rvt-rs must write a
//! port `Port_<ElementId>_<index>` tied to the element with that `Tag` for
//! each of them.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Mechanical.rvt` and
//! `RE1-Plumbing.rvt` with Revit's `RE1-*.ifc`). Skips what is absent.

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

/// Each port's key, with the `Tag` of the element it is tied to.
fn ports(step: &str) -> BTreeMap<(u32, u32), Option<u32>> {
    let ents = entities(step);
    let mut out: BTreeMap<(u32, u32), Option<u32>> = ents
        .values()
        .filter(|(entity, _)| entity == "IFCDISTRIBUTIONPORT")
        .filter_map(|(_, args)| Some((port_key(args)?, None)))
        .collect();
    for (entity, args) in ents.values() {
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
            let tag = element_args
                .rsplit('\'')
                .nth(1)
                .and_then(|t| t.parse().ok());
            out.insert(key, tag);
        }
    }
    out
}

/// The `Tag`s of Revit's ducts and pipes (`IfcFlowSegment` in IFC2X3, its
/// duct and pipe subtypes in IFC4).
fn segments(step: &str) -> BTreeSet<u32> {
    entities(step)
        .values()
        .filter(|(entity, _)| {
            matches!(
                entity.as_str(),
                "IFCFLOWSEGMENT" | "IFCDUCTSEGMENT" | "IFCPIPESEGMENT"
            )
        })
        .filter_map(|(_, args)| args.rsplit('\'').nth(1)?.parse().ok())
        .collect()
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let step = std::fs::read_to_string(reference).expect("reference IFC");
    let segment_tags = segments(&step);
    let theirs = ports(&step);
    let mut by_segment: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (element, index) in theirs.keys() {
        if segment_tags.contains(element) {
            by_segment.entry(*element).or_default().insert(*index);
        }
    }
    let wanted: Vec<(u32, u32)> = by_segment
        .iter()
        .filter(|(_, indices)| indices.len() == 2)
        .flat_map(|(element, indices)| indices.iter().map(move |index| (*element, *index)))
        .collect();
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = ports(&write_step(&result.model));
    let missing: Vec<&(u32, u32)> = wanted
        .iter()
        .filter(|key| ours.get(key) != Some(&Some(key.0)))
        .collect();
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    eprintln!(
        "{name}: {} of Revit's {} duct and pipe end ports written",
        wanted.len() - missing.len(),
        wanted.len()
    );
    if !missing.is_empty() {
        failures.push(format!(
            "{name}: {} of Revit's {} duct and pipe end ports are not written, tied to their element; first: {:?}",
            missing.len(),
            wanted.len(),
            missing.iter().take(8).collect::<Vec<_>>()
        ));
    }
    wanted.len()
}

#[test]
fn every_duct_and_pipe_has_revits_end_ports() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut failures = Vec::new();
    let mut compared = 0;
    for model in ["Mechanical", "Plumbing"] {
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
        compared > 0 || !dir.join("RE1-Plumbing.rvt").exists(),
        "no duct or pipe port was compared"
    );
}
