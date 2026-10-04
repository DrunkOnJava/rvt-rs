//! B44 (#35): a pipe fitting's `Pset_PipeFittingTypeCommon.NominalDiameter`.
//!
//! Revit's export of RE1 Plumbing gives each of its 42 pipe fittings a
//! `NominalDiameter`, an `IfcPropertyListValue` of one
//! `IfcPositiveLengthMeasure` (10 to 50 mm). For every element Revit's export
//! gives one, rvt-rs must write the same list on the element of the same `Tag`,
//! each length within a relative 1e-5.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Plumbing.rvt` and
//! `RE1-Mechanical.rvt` with their `RE1-*.ifc` exports. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

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

fn references(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| f.trim().strip_prefix('#')?.parse().ok())
        .collect()
}

/// Metres per the file's length unit.
fn metres_per_unit(ents: &BTreeMap<u64, (String, String)>) -> f64 {
    ents.values()
        .filter(|(e, a)| e == "IFCSIUNIT" && a.contains(".LENGTHUNIT."))
        .find_map(|(_, a)| {
            Some(match split_args(a).get(2)?.trim() {
                ".MILLI." => 0.001,
                _ => 1.0,
            })
        })
        .unwrap_or(1.0)
}

/// Each element's `Tag` and its `Pset_PipeFittingTypeCommon.NominalDiameter`
/// list, in metres; `None` where the property is not a list of lengths.
fn nominal_diameters(step: &str) -> BTreeMap<String, Option<Vec<f64>>> {
    let ents = entities(step);
    let scale = metres_per_unit(&ents);
    let mut out = BTreeMap::new();
    for (e, a) in ents.values() {
        if e != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(a);
        let Some((set_entity, set_args)) = f
            .get(5)
            .and_then(|s| references(s).first().copied())
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        let sf = split_args(set_args);
        if set_entity != "IFCPROPERTYSET"
            || sf.get(2).map(|n| n.trim()) != Some("'Pset_PipeFittingTypeCommon'")
        {
            continue;
        }
        let Some((property_entity, property_args)) =
            references(sf.get(4).map(String::as_str).unwrap_or(""))
                .iter()
                .filter_map(|p| ents.get(p))
                .find(|(_, pa)| {
                    split_args(pa).first().map(|n| n.trim()) == Some("'NominalDiameter'")
                })
        else {
            continue;
        };
        let values = (property_entity == "IFCPROPERTYLISTVALUE").then(|| {
            let list = split_args(property_args)
                .get(2)
                .cloned()
                .unwrap_or_default();
            split_args(list.trim().trim_start_matches('(').trim_end_matches(')'))
                .iter()
                .filter_map(|item| {
                    let inner = item.trim().strip_prefix("IFCPOSITIVELENGTHMEASURE(")?;
                    inner.trim_end_matches(')').parse::<f64>().ok()
                })
                .map(|v| v * scale)
                .collect::<Vec<f64>>()
        });
        for object in references(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some((_, oa)) = ents.get(&object) else {
                continue;
            };
            let tag = split_args(oa)
                .get(7)
                .map(|t| t.trim().trim_matches('\'').to_string())
                .unwrap_or_default();
            if !tag.is_empty() {
                out.insert(tag, values.clone());
            }
        }
    }
    out
}

#[test]
fn re1_fitting_nominal_diameters_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut compared = 0;
    let mut failures = Vec::new();
    for (model, expected) in [("Plumbing", 42), ("Mechanical", 0)] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let theirs =
            nominal_diameters(&std::fs::read_to_string(&reference).expect("reference IFC"));
        assert_eq!(
            theirs.values().filter(|v| v.is_some()).count(),
            expected,
            "RE1 {model}: pipe fittings with Revit's NominalDiameter in the reference export"
        );
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = nominal_diameters(&write_step(&result.model));
        let mut wrong = Vec::new();
        for (tag, revit) in &theirs {
            let Some(revit) = revit else {
                continue;
            };
            compared += 1;
            let same = ours.get(tag).cloned().flatten().is_some_and(|ours| {
                ours.len() == revit.len()
                    && ours
                        .iter()
                        .zip(revit)
                        .all(|(a, b)| (a - b).abs() <= 1e-5 * a.abs().max(b.abs()).max(1.0))
            });
            if !same {
                wrong.push(format!(
                    "{tag}: Revit {revit:?}, rvt-rs {:?}",
                    ours.get(tag)
                ));
            }
        }
        eprintln!(
            "RE1 {model}: {} fittings with a NominalDiameter in Revit's export, {} not Revit's in rvt-rs's",
            theirs.values().filter(|v| v.is_some()).count(),
            wrong.len()
        );
        if !wrong.is_empty() {
            failures.push(format!(
                "RE1 {model}: {} NominalDiameter lists are not Revit's; first: {:?}",
                wrong.len(),
                wrong.iter().take(5).collect::<Vec<_>>()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    eprintln!("{compared} fittings compared");
}
