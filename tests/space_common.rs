//! RE-153 (#35): a room's `Pset_SpaceCommon` is Revit's.
//!
//! Revit's IFC4 export of RE1 Architecture gives each of its 11 rooms a
//! `Pset_SpaceCommon` with `Reference` (`IfcIdentifier`, the room's name and
//! number) and `FloorCovering` (`IfcLabel`, its Floor Finish). Every space
//! rvt-rs writes must carry the same set, property for property and value
//! type for value type, matched by the space's `Name` (its number).
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Architecture.rvt` with
//! `RE1-Architecture.ifc`. Skips what is absent.

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

fn reference(field: &str) -> Option<u64> {
    field.trim().strip_prefix('#')?.parse().ok()
}

/// Space `Name` -> property -> its value as written (`IFCLABEL('Wood')`), for
/// each space's `Pset_SpaceCommon`.
fn space_common(step: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let fields = split_args(args);
        let (Some(objects), Some(set)) = (fields.get(4), fields.get(5).and_then(|f| reference(f)))
        else {
            continue;
        };
        let Some((set_entity, set_args)) = ents.get(&set) else {
            continue;
        };
        let set_fields = split_args(set_args);
        if set_entity != "IFCPROPERTYSET"
            || set_fields.get(2).map(|f| unquote(f)).as_deref() != Some("Pset_SpaceCommon")
        {
            continue;
        }
        let list = set_fields.get(4).map(String::as_str).unwrap_or("");
        let props: BTreeMap<String, String> =
            split_args(list.trim_start_matches('(').trim_end_matches(')'))
                .iter()
                .filter_map(|f| ents.get(&reference(f)?))
                .filter_map(|(_, a)| {
                    let p = split_args(a);
                    Some((unquote(p.first()?), p.get(2)?.trim().to_string()))
                })
                .collect();
        for object in split_args(objects.trim_start_matches('(').trim_end_matches(')')) {
            let Some((e, a)) = reference(&object).and_then(|id| ents.get(&id)) else {
                continue;
            };
            if e == "IFCSPACE" {
                if let Some(name) = split_args(a).get(2) {
                    out.insert(unquote(name), props.clone());
                }
            }
        }
    }
    out
}

#[test]
fn re1_architecture_space_common_is_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Architecture.rvt");
    let reference_ifc = dir.join("RE1-Architecture.ifc");
    if !rvt.exists() || !reference_ifc.exists() {
        eprintln!("skipping: no RE1 Architecture model and reference export");
        return;
    }
    let theirs = space_common(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
    assert_eq!(theirs.len(), 11, "Revit's spaces with Pset_SpaceCommon");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = space_common(&write_step(&result.model));
    let differing: Vec<(
        &String,
        &BTreeMap<String, String>,
        Option<&BTreeMap<String, String>>,
    )> = theirs
        .iter()
        .filter(|(space, props)| ours.get(*space) != Some(*props))
        .map(|(space, props)| (space, props, ours.get(space)))
        .collect();
    assert!(
        differing.is_empty(),
        "{} of Revit's {} Pset_SpaceCommon sets differ in rvt-rs's export (space, Revit's, rvt-rs's): {differing:?}",
        differing.len(),
        theirs.len()
    );
    let extra: Vec<&String> = ours
        .keys()
        .filter(|space| !theirs.contains_key(*space))
        .collect();
    assert!(
        extra.is_empty(),
        "Pset_SpaceCommon on spaces Revit gives none: {extra:?}"
    );
}
