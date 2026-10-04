//! B72: a door's or window's type object has the GlobalId Revit's exporter
//! gives it.
//!
//! Revit's exporter writes a door's or window's type with the GlobalId MD5
//! makes of `<original symbol's GlobalId>Sub-element:Flipped: <bool>
//! InAssembly: False` (RE-167), the flip from which side of its host wall's
//! axis the door's box lies on against where its own Y axis points
//! (revit-ifc `DoorWindowInfo`). For every door and window both exports
//! relate to a type (matched by the element's `Tag`), rvt-rs's type must have
//! the GlobalId of Revit's.
//!
//! A type's GlobalId is the type's identity, so an export that writes no
//! geometry (`typed-no-geometry`) must give it the same one.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: the four `RE1-*.rvt` models with
//! their `RE1-*.ifc` exports, and `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{ExportQualityMode, RvtDocExporter, write_step};
use rvt::walker::WalkerLimits;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Revit's door and window type entities, IFC2X3 and IFC4.
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

fn check(
    rvt: &Path,
    reference: &Path,
    mode: ExportQualityMode,
    failures: &mut Vec<String>,
) -> usize {
    let theirs = type_of(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut rf = RevitFile::open(rvt).expect("open");
    let model = RvtDocExporter
        .export_with_mode_and_limits(&mut rf, mode, WalkerLimits::default())
        .expect("export");
    let ours = type_of(&write_step(&model));
    let name = format!(
        "{} ({})",
        rvt.file_name().unwrap_or_default().to_string_lossy(),
        mode.as_str()
    );
    let mut wrong = Vec::new();
    let mut compared = 0;
    for (tag, (entity, global_id)) in &theirs {
        let Some((_, ours)) = ours.get(tag) else {
            continue;
        };
        if !DOOR_AND_WINDOW_TYPES.contains(&entity.as_str()) {
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
        "{name}: {compared} doors and windows typed in both, {} types' GlobalIds not Revit's",
        wrong.len()
    );
    if !wrong.is_empty() {
        failures.push(format!(
            "{name}: {} of {compared} doors' and windows' types have another GlobalId than Revit's; first: {:?}",
            wrong.len(),
            wrong.iter().take(5).collect::<Vec<_>>()
        ));
    }
    compared
}

fn check_corpus(mode: ExportQualityMode) {
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
        compared += check(rvt, reference, mode, &mut failures);
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    assert!(
        compared > 0 || !dir.join("RE1-Architecture.rvt").exists(),
        "no door or window was compared"
    );
}

#[test]
fn door_and_window_types_take_revits_global_ids() {
    check_corpus(ExportQualityMode::Scaffold);
}

#[test]
fn door_and_window_types_take_revits_global_ids_without_geometry() {
    check_corpus(ExportQualityMode::TypedNoGeometry);
}
