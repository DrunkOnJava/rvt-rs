//! RE-157 (B26, #35): a pipe's `InvertElevation` is Revit's.
//!
//! Revit's IFC4 export of the RE1 Mechanical and Plumbing models gives every
//! pipe `Pset_FlowSegmentPipeSegment.InvertElevation` (`IfcLengthMeasure`):
//! the height above its storey of a horizontal pipe's inside bottom, or of a
//! vertical pipe's lower end (69 pipes). For every pipe rvt-rs writes with
//! Revit's `Tag`, it must be in rvt-rs's export, within 0.1 mm of Revit's,
//! each file's values read in its own length unit.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Mechanical.rvt` and
//! `RE1-Plumbing.rvt` with their `RE1-*.ifc` exports. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SETS: [&str; 1] = ["Pset_FlowSegmentPipeSegment"];
const PROPERTY: &str = "'InvertElevation'";
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
            let prefix = split_args(args).get(2)?.trim().to_string();
            Some(match prefix.as_str() {
                ".MILLI." => 0.001,
                ".CENTI." => 0.01,
                _ => 1.0,
            })
        })
        .unwrap_or(1.0)
}

/// Pipe `Tag` -> set -> [`PROPERTY`] in metres, for each element of `entity`.
fn lengths(step: &str, entity: &str) -> BTreeMap<String, BTreeMap<String, f64>> {
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
    let mut out: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
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
        let sf = split_args(set_args);
        let set = sf.get(2).map(|n| n.trim().trim_matches('\'')).unwrap_or("");
        if set_entity != "IFCPROPERTYSET" || !SETS.contains(&set) {
            continue;
        }
        let length = list(sf.get(4).map(String::as_str).unwrap_or(""))
            .iter()
            .filter_map(|p| ents.get(p))
            .find_map(|(_, a)| {
                let pf = split_args(a);
                (pf.first()?.trim() == PROPERTY).then_some(())?;
                let value = pf.get(2)?.trim();
                let number = value.split_once('(')?.1.trim_end_matches(')');
                number.parse::<f64>().ok()
            });
        let Some(length) = length else {
            continue;
        };
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            if let Some(tag) = tags.get(&object) {
                out.entry(tag.clone())
                    .or_default()
                    .insert(set.to_string(), length * scale);
            }
        }
    }
    out
}

#[test]
fn re1_pipe_invert_elevations_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for model in ["Mechanical", "Plumbing"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let theirs = lengths(
            &std::fs::read_to_string(&reference_ifc).expect("reference IFC"),
            "IFCFLOWSEGMENT",
        );
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let step = write_step(&result.model);
        let ents = entities(&step);
        let pipes: Vec<String> = ents
            .values()
            .filter(|(e, _)| e == "IFCPIPESEGMENT")
            .filter_map(|(_, a)| Some(split_args(a).get(7)?.trim().trim_matches('\'').to_string()))
            .filter(|tag| theirs.contains_key(tag))
            .collect();
        assert!(
            !pipes.is_empty(),
            "RE1 {model}: no pipe rvt-rs writes is in Revit's export"
        );
        let ours = lengths(&step, "IFCPIPESEGMENT");
        let mut off: Vec<(String, String, f64, Option<f64>)> = Vec::new();
        for tag in &pipes {
            for (set, revit) in &theirs[tag] {
                let mine = ours.get(tag).and_then(|sets| sets.get(set)).copied();
                if mine.is_none_or(|m| (m - revit).abs() > TOLERANCE_METRES) {
                    off.push((tag.clone(), set.clone(), *revit, mine));
                }
            }
        }
        assert!(
            off.is_empty(),
            "RE1 {model}: {} of the InvertElevations of the {} pipes both exports hold are not Revit's (tag, set, Revit's m, rvt-rs's m); first: {:?}",
            off.len(),
            pipes.len(),
            off.iter().take(5).collect::<Vec<_>>()
        );
    }
}
