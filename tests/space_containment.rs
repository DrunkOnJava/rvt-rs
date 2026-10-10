//! B63: the elements Revit's export contains in a space.
//!
//! Revit's IFC export contains a room's furniture, fixtures and equipment in
//! the room's `IfcSpace` (`IfcRelContainedInSpatialStructure`), and walls,
//! doors, slabs and the rest in their storey. For every element both exports
//! hold (by `Tag`, its ElementId), rvt-rs must contain it in a space exactly
//! where Revit does, in the space of the same `Name` (the room number).
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: the four `RE1-*.rvt` models with
//! their `RE1-*.ifc` exports, and `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

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

/// Each contained element's `Tag` and its container: `Some(space name)` for
/// an `IfcSpace`, `None` for a storey, the building or the site.
fn containers(step: &str) -> BTreeMap<String, Option<String>> {
    let ents = entities(step);
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELCONTAINEDINSPATIALSTRUCTURE" {
            continue;
        }
        let f = split_args(args);
        let Some((structure, structure_args)) = f
            .get(5)
            .and_then(|s| references(s).first().copied())
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        let space = (structure == "IFCSPACE").then(|| {
            split_args(structure_args)
                .get(2)
                .map(|n| n.trim().to_string())
                .unwrap_or_default()
        });
        for element in references(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some((_, element_args)) = ents.get(&element) else {
                continue;
            };
            let tag = split_args(element_args)
                .get(7)
                .map(|t| t.trim().trim_matches('\'').to_string())
                .unwrap_or_default();
            if !tag.is_empty() && tag.chars().all(|c| c.is_ascii_digit()) {
                out.insert(tag, space.clone());
            }
        }
    }
    out
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let theirs = containers(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = containers(&write_step(&result.model));
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    let mut wrong = Vec::new();
    let mut compared = 0;
    for (tag, revit_space) in &theirs {
        let Some(our_space) = ours.get(tag) else {
            continue;
        };
        compared += 1;
        if our_space != revit_space {
            wrong.push(format!(
                "{tag}: Revit {}, rvt-rs {}",
                revit_space.as_deref().unwrap_or("storey"),
                our_space.as_deref().unwrap_or("storey")
            ));
        }
    }
    let in_spaces = theirs.values().filter(|space| space.is_some()).count();
    eprintln!(
        "{name}: {compared} elements in both, {in_spaces} in a space in Revit's, {} contained differently",
        wrong.len()
    );
    if !wrong.is_empty() {
        failures.push(format!(
            "{name}: {} of {compared} elements are contained differently from Revit's export; first: {:?}",
            wrong.len(),
            wrong.iter().take(5).collect::<Vec<_>>()
        ));
    }
    compared
}

#[test]
fn elements_are_contained_in_revits_spaces() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut pairs: Vec<(PathBuf, PathBuf)> =
        ["Architecture", "Mechanical", "Plumbing", "Electrical"]
            .iter()
            .map(|model| {
                (
                    dir.join(format!("RE1-{model}.rvt")),
                    dir.join(format!("RE1-{model}.ifc")),
                )
            })
            .collect();
    pairs.push((
        dir.join("2024_Core_Interior.rvt"),
        dir.join("../IFC Exports/2024_Core_Interior_slim.ifc"),
    ));
    let mut failures = Vec::new();
    let mut compared = 0;
    for (rvt, reference) in &pairs {
        if !rvt.exists() || !reference.exists() {
            eprintln!(
                "skipping {}: model or reference export absent",
                rvt.display()
            );
            continue;
        }
        compared += check(rvt, reference, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        compared > 0 || !dir.join("RE1-Architecture.rvt").exists(),
        "no element was compared"
    );
}

/// Every `IfcSpace` is an aggregated part of an `IfcBuildingStorey` or the
/// `IfcBuilding` and is never itself in `IfcRelContainedInSpatialStructure`:
/// a space is a spatial element (buildingSMART validation rules SPS002 and
/// SPS007). Returns the number of spaces checked.
fn check_space_aggregation(rvt: &Path, failures: &mut Vec<String>) -> usize {
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ents = entities(&write_step(&result.model));
    let spaces: Vec<u64> = ents
        .iter()
        .filter(|(_, (entity, _))| entity == "IFCSPACE")
        .map(|(id, _)| *id)
        .collect();
    let mut aggregated = std::collections::BTreeSet::new();
    let mut contained = std::collections::BTreeSet::new();
    for (entity, args) in ents.values() {
        let f = split_args(args);
        match entity.as_str() {
            "IFCRELAGGREGATES" => {
                let parent = f
                    .get(4)
                    .and_then(|s| references(s).first().copied())
                    .and_then(|p| ents.get(&p))
                    .map(|(e, _)| e.as_str());
                if matches!(parent, Some("IFCBUILDINGSTOREY" | "IFCBUILDING")) {
                    aggregated.extend(references(f.get(5).map(String::as_str).unwrap_or("")));
                }
            }
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                contained.extend(references(f.get(4).map(String::as_str).unwrap_or("")));
            }
            _ => {}
        }
    }
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    let loose = spaces.iter().filter(|s| !aggregated.contains(s)).count();
    let wrongly_contained = spaces.iter().filter(|s| contained.contains(s)).count();
    eprintln!(
        "{name}: {} spaces, {loose} not aggregated into a storey or the building, {wrongly_contained} contained",
        spaces.len()
    );
    if loose + wrongly_contained > 0 {
        failures.push(format!(
            "{name}: {loose} of {} spaces not aggregated, {wrongly_contained} in IfcRelContainedInSpatialStructure",
            spaces.len()
        ));
    }
    spaces.len()
}

#[test]
fn spaces_are_aggregated_into_their_storey() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut failures = Vec::new();
    let mut checked = 0;
    for model in ["2024_Core_Interior.rvt", "RE1-Architecture.rvt"] {
        let rvt = dir.join(model);
        if !rvt.exists() {
            eprintln!("skipping {}: absent", rvt.display());
            continue;
        }
        checked += check_space_aggregation(&rvt, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        checked > 0 || !dir.join("2024_Core_Interior.rvt").exists(),
        "no space was checked"
    );
}
