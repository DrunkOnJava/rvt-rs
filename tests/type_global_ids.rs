//! B60: a type object's GlobalId is the one Revit's exporter gives it.
//!
//! Revit's exporter writes a family instance's type with the GlobalId of the
//! instance's original symbol (RE-167), which is the symbol itself for most
//! types and another FamilySymbol for many furniture, fittings, mullions
//! and panels, while the type's `Tag` stays the symbol's ElementId. For
//! every element both exports relate to a type (`IfcRelDefinesByType`,
//! matched by the element's `Tag`), rvt-rs's type must have the GlobalId of
//! Revit's. A door's or window's type is left out: Revit's is a hash of the
//! original symbol's GlobalId with the door's flip, not read yet (B72).
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: the four `RE1-*.rvt` models with
//! their `RE1-*.ifc` exports, and `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Revit's door and window type entities, IFC2X3 and IFC4 (B72).
const DOOR_AND_WINDOW_TYPES: [&str; 4] = [
    "IFCDOORSTYLE",
    "IFCDOORTYPE",
    "IFCWINDOWSTYLE",
    "IFCWINDOWTYPE",
];

/// `#id` -> (entity name, raw argument text) for every entity line.
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
        let Ok(id) = id.trim().parse() else {
            continue;
        };
        out.insert(id, (entity.trim().to_string(), args.to_string()));
    }
    out
}

/// The `#n` references in a piece of argument text.
fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// Each typed element's `Tag` -> (its type's entity, its type's GlobalId).
fn type_of(step: &str) -> BTreeMap<String, (String, String)> {
    let ents = entities(step);
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let Some((ty_entity, ty_args)) = ents.get(&ty) else {
            continue;
        };
        let Some(global_id) = ty_args.split('\'').nth(1) else {
            continue;
        };
        // The first reference is the owner history.
        for element in related.iter().skip(1) {
            let Some((_, element_args)) = ents.get(element) else {
                continue;
            };
            // An element's Tag is its last quoted string.
            let Some(tag) = element_args.rsplit('\'').nth(1) else {
                continue;
            };
            out.insert(tag.to_string(), (ty_entity.clone(), global_id.to_string()));
        }
    }
    out
}

fn check(rvt: &Path, reference: &Path, failures: &mut Vec<String>) -> usize {
    let theirs = type_of(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = type_of(&write_step(&result.model));
    let name = rvt.file_name().unwrap_or_default().to_string_lossy();
    let mut wrong = Vec::new();
    let mut compared = 0;
    let mut doors_and_windows = 0;
    for (tag, (entity, global_id)) in &theirs {
        let Some((_, ours)) = ours.get(tag) else {
            continue;
        };
        if DOOR_AND_WINDOW_TYPES.contains(&entity.as_str()) {
            doors_and_windows += 1;
            continue;
        }
        compared += 1;
        if ours != global_id {
            wrong.push(format!(
                "{tag} ({entity}): Revit {global_id}, rvt-rs {ours}"
            ));
        }
    }
    eprintln!(
        "{name}: {compared} elements typed in both, {} types' GlobalIds not Revit's; {doors_and_windows} doors and windows left out",
        wrong.len()
    );
    if !wrong.is_empty() {
        failures.push(format!(
            "{name}: {} of {compared} typed elements' types have another GlobalId than Revit's; first: {:?}",
            wrong.len(),
            wrong.iter().take(5).collect::<Vec<_>>()
        ));
    }
    compared
}

#[test]
fn type_objects_take_revits_global_ids() {
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
        "no typed element was compared"
    );
}
