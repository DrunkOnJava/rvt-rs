//! B84 (#528): every port is nested in its element, as Revit's IFC4 export
//! nests them.
//!
//! IFC4 relates a static port to its element with an `IfcRelNests`, and keeps
//! `IfcRelConnectsPortToElement` for a dynamic connection, restricted to
//! distribution elements. Revit's own IFC4 export (revit-ifc
//! `ConnectorExporter`) nests every port in its element, one `IfcRelNests`
//! per element. rvt-rs must write each port `Port_<ElementId>_<index>` nested
//! in the element with that `Tag`, by exactly one `IfcRelNests`, and tie no
//! port with `IfcRelConnectsPortToElement`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Electrical.rvt`,
//! `RE1-Plumbing.rvt` and `RE1-Mechanical.rvt`). Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
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

/// The `ElementId` a port's name `Port_<ElementId>_<index>` gives.
fn port_element(args: &str) -> Option<u32> {
    let name = args.split('\'').nth(3)?;
    name.strip_prefix("Port_")?.split_once('_')?.0.parse().ok()
}

fn check(rvt: &Path, failures: &mut Vec<String>) -> usize {
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ents = entities(&write_step(&result.model));
    let tag = |id: &u64| -> Option<u32> { ents.get(id)?.1.rsplit('\'').nth(1)?.parse().ok() };
    let ports: BTreeMap<u64, Option<u32>> = ents
        .iter()
        .filter(|(_, (entity, _))| entity == "IFCDISTRIBUTIONPORT")
        .map(|(id, (_, args))| (*id, port_element(args)))
        .collect();
    let mut nested_in: BTreeMap<u64, Vec<Option<u32>>> = BTreeMap::new();
    let mut tied = 0usize;
    for (entity, args) in ents.values() {
        match entity.as_str() {
            "IFCRELNESTS" => {
                let all = refs(args);
                let Some(host) = all.get(1) else { continue };
                for part in all.iter().skip(2).filter(|part| ports.contains_key(part)) {
                    nested_in.entry(*part).or_default().push(tag(host));
                }
            }
            "IFCRELCONNECTSPORTTOELEMENT" => tied += 1,
            _ => {}
        }
    }
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    let wrong: Vec<u64> = ports
        .iter()
        .filter(|&(id, element)| {
            nested_in.get(id).map(Vec::as_slice) != Some(std::slice::from_ref(element))
        })
        .map(|(id, _)| *id)
        .collect();
    eprintln!(
        "{name}: {} ports, {} nested once in their element, {tied} IfcRelConnectsPortToElement",
        ports.len(),
        ports.len() - wrong.len()
    );
    if !wrong.is_empty() {
        failures.push(format!(
            "{name}: {} of {} ports are not nested exactly once in the element their name gives",
            wrong.len(),
            ports.len()
        ));
    }
    if tied > 0 {
        failures.push(format!(
            "{name}: {tied} ports are tied with IfcRelConnectsPortToElement"
        ));
    }
    ports.len()
}

#[test]
fn every_port_is_nested_in_its_element() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut failures = Vec::new();
    let mut ports = 0;
    for model in ["Electrical", "Plumbing", "Mechanical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        if !rvt.exists() {
            eprintln!("skipping {}: absent", rvt.display());
            continue;
        }
        ports += check(&rvt, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        ports > 0 || !dir.join("RE1-Mechanical.rvt").exists(),
        "no port was read"
    );
}
