//! RE-134 (#35): a duct's length and type in its property sets are Revit's.
//!
//! Revit's IFC4 export of RE1 Mechanical gives each of its 25 rectangular
//! ducts `Pset_FlowSegmentDuctSegment.Length` and
//! `Pset_FlowSegmentPipeSegment.Length` (its length along its axis), and
//! `Pset_DistributionFlowElementCommon.Reference` and
//! `Pset_QuantityTakeOff.Reference` (its type's name). For every duct rvt-rs
//! writes with Revit's `Tag`, each of those must be in rvt-rs's export: the
//! lengths within 0.1 mm of Revit's (each file read in its own length unit),
//! the references as Revit writes them.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Mechanical.rvt` with
//! `RE1-Mechanical.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

const LENGTH_SETS: [&str; 2] = ["Pset_FlowSegmentDuctSegment", "Pset_FlowSegmentPipeSegment"];
const REFERENCE_SETS: [&str; 2] = ["Pset_DistributionFlowElementCommon", "Pset_QuantityTakeOff"];
const TOLERANCE_METRES: f64 = 1e-4;

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
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

/// Split a STEP argument list on top-level commas.
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut depth) = (false, 0usize);
    for c in args.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                current.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ')' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if !quoted && depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

fn reference(field: &str) -> Option<u64> {
    field.trim().strip_prefix('#')?.parse().ok()
}

fn list(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| reference(f))
        .collect()
}

/// Metres per the file's length unit: its `IfcSIUnit` of `.LENGTHUNIT.`.
fn metres_per_unit(ents: &BTreeMap<u64, (String, String)>) -> f64 {
    ents.values()
        .filter(|(entity, args)| entity == "IFCSIUNIT" && args.contains(".LENGTHUNIT."))
        .find_map(|(_, args)| {
            Some(match split_args(args).get(2)?.trim() {
                ".MILLI." => 0.001,
                ".CENTI." => 0.01,
                _ => 1.0,
            })
        })
        .unwrap_or(1.0)
}

/// A duct's lengths in metres and references as written, by set.
#[derive(Debug, Default)]
struct Duct {
    lengths: BTreeMap<String, f64>,
    references: BTreeMap<String, String>,
}

/// Every element of `entity`, by `Tag`.
fn ducts(step: &str, entity: &str) -> BTreeMap<String, Duct> {
    let ents = entities(step);
    let scale = metres_per_unit(&ents);
    let mut tags: BTreeMap<u64, String> = BTreeMap::new();
    for (id, (e, args)) in &ents {
        if e == entity {
            if let Some(tag) = split_args(args).get(7) {
                tags.insert(*id, tag.trim().trim_matches('\'').to_string());
            }
        }
    }
    let mut out: BTreeMap<String, Duct> = tags
        .values()
        .map(|t| (t.clone(), Duct::default()))
        .collect();
    for (e, args) in ents.values() {
        if e != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let Some((set_entity, set_args)) = f
            .get(5)
            .and_then(|s| reference(s))
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" {
            continue;
        }
        let sf = split_args(set_args);
        let set = sf
            .get(2)
            .map(|n| n.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        let props: Vec<(String, String)> = list(sf.get(4).map(String::as_str).unwrap_or(""))
            .iter()
            .filter_map(|p| ents.get(p))
            .filter_map(|(_, a)| {
                let pf = split_args(a);
                Some((
                    pf.first()?.trim().trim_matches('\'').to_string(),
                    pf.get(2)?.trim().to_string(),
                ))
            })
            .collect();
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some(duct) = tags.get(&object).and_then(|tag| out.get_mut(tag)) else {
                continue;
            };
            for (name, value) in &props {
                if name == "Length" && LENGTH_SETS.contains(&set.as_str()) {
                    let number = value
                        .split_once('(')
                        .and_then(|(_, n)| n.trim_end_matches(')').parse::<f64>().ok());
                    if let Some(number) = number {
                        duct.lengths.insert(set.clone(), number * scale);
                    }
                }
                if name == "Reference" && REFERENCE_SETS.contains(&set.as_str()) {
                    duct.references.insert(set.clone(), value.clone());
                }
            }
        }
    }
    out
}

#[test]
fn re1_mechanical_duct_lengths_and_references_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Mechanical.rvt");
    let reference_ifc = dir.join("RE1-Mechanical.ifc");
    if !rvt.exists() || !reference_ifc.exists() {
        eprintln!("skipping: no RE1 Mechanical model and reference export");
        return;
    }
    let step = std::fs::read_to_string(&reference_ifc).expect("reference IFC");
    // Revit writes ducts and pipes alike as IfcFlowSegment; its ducts are
    // the ones whose ObjectType is a Rectangular Duct type.
    let revit_ducts: Vec<String> = entities(&step)
        .values()
        .filter(|(e, a)| e == "IFCFLOWSEGMENT" && a.contains("'Rectangular Duct:"))
        .filter_map(|(_, a)| Some(split_args(a).get(7)?.trim().trim_matches('\'').to_string()))
        .collect();
    assert_eq!(revit_ducts.len(), 25, "Revit's ducts");
    let theirs = ducts(&step, "IFCFLOWSEGMENT");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = ducts(&write_step(&result.model), "IFCDUCTSEGMENT");
    let mut off: Vec<String> = Vec::new();
    for tag in &revit_ducts {
        let (Some(revit), Some(mine)) = (theirs.get(tag), ours.get(tag)) else {
            off.push(format!("{tag}: not a duct in rvt-rs's export"));
            continue;
        };
        for (set, length) in &revit.lengths {
            match mine.lengths.get(set) {
                Some(m) if (m - length).abs() <= TOLERANCE_METRES => {}
                other => off.push(format!(
                    "{tag} {set}.Length: Revit {length} m, rvt-rs {other:?}"
                )),
            }
        }
        for (set, value) in &revit.references {
            if mine.references.get(set) != Some(value) {
                off.push(format!(
                    "{tag} {set}.Reference: Revit {value}, rvt-rs {:?}",
                    mine.references.get(set)
                ));
            }
        }
    }
    assert!(
        off.is_empty(),
        "{} of the 25 ducts' Length and Reference values are not Revit's; first: {:?}",
        off.len(),
        off.iter().take(6).collect::<Vec<_>>()
    );
}
