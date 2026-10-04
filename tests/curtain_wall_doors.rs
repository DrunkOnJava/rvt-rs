//! B64: a door or window set in a curtain wall is one of the curtain wall's
//! parts.
//!
//! Revit's IFC export aggregates a curtain wall's panel doors and windows
//! under its `IfcCurtainWall`, with its panels and mullions, and gives them
//! no opening and no host wall: on RE1 Architecture door 445975
//! (`Door-Curtain-Wall-Double-Glas`) under curtain wall 445961. For every
//! door and window Revit's export aggregates under a curtain wall, rvt-rs
//! must aggregate the element of the same `Tag` under the curtain wall of
//! the same `Tag`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Architecture.rvt` and Revit's
//! `RE1-Architecture.ifc`). Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

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

/// An element's `Tag`: the 8th attribute, the last quoted string of the
/// elements compared here.
fn tag(args: &str) -> Option<String> {
    Some(args.rsplit('\'').nth(1)?.to_string())
}

/// Each door's and window's `Tag` -> the `Tag` of the curtain wall that
/// aggregates it.
fn curtain_wall_of(step: &str) -> BTreeMap<String, String> {
    let ents = entities(step);
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELAGGREGATES" {
            continue;
        }
        let all = refs(args);
        // Owner history, the whole, then its parts.
        let Some((whole_entity, whole_args)) = all.get(1).and_then(|w| ents.get(w)) else {
            continue;
        };
        if whole_entity != "IFCCURTAINWALL" {
            continue;
        }
        let Some(whole) = tag(whole_args) else {
            continue;
        };
        for part in all.iter().skip(2) {
            let Some((part_entity, part_args)) = ents.get(part) else {
                continue;
            };
            if part_entity != "IFCDOOR" && part_entity != "IFCWINDOW" {
                continue;
            }
            if let Some(part) = tag(part_args) {
                out.insert(part, whole.clone());
            }
        }
    }
    out
}

#[test]
fn re1_curtain_wall_door_is_a_part_of_its_curtain_wall() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Architecture.rvt");
    let reference = dir.join("RE1-Architecture.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no RE1 Architecture model and reference export");
        return;
    }
    let theirs = curtain_wall_of(&std::fs::read_to_string(&reference).expect("reference IFC"));
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = curtain_wall_of(&write_step(&result.model));
    assert_eq!(
        theirs,
        BTreeMap::from([("445975".to_string(), "445961".to_string())]),
        "Revit's export aggregates door 445975 under curtain wall 445961"
    );
    let wrong: Vec<String> = theirs
        .iter()
        .filter(|(part, whole)| ours.get(*part) != Some(*whole))
        .map(|(part, whole)| {
            format!(
                "{part}: Revit's curtain wall {whole}, rvt-rs's {:?}",
                ours.get(part)
            )
        })
        .collect();
    assert!(
        wrong.is_empty(),
        "curtain wall doors and windows not aggregated as in Revit's export: {wrong:?}"
    );
}
