//! B59, B43: a duct is named and typed by its type's shape, as Revit's
//! export names it.
//!
//! Revit names a duct `<system family>:<type>:<ElementId>`, the system family
//! following its type's shape: `Rectangular Duct`, `Round Duct` or `Oval
//! Duct`. Revit's export of RE1 Mechanical writes its 25 ducts as
//! `Rectangular Duct:233113-DUCT-Tees:<id>`, with that ObjectType and a type
//! object named `Rectangular Duct:233113-DUCT-Tees`. rvt-rs must give each
//! duct Revit's Name and ObjectType, and type it by a type object of Revit's
//! name.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-Mechanical.rvt` with Revit's
//! `RE1-Mechanical.ifc`). Skips what is absent.

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

/// What an export says of each duct, by its `Tag`: its Name, its ObjectType
/// and its type object's name.
#[derive(Debug, PartialEq, Default, Clone)]
struct Duct {
    name: String,
    object_type: String,
    type_name: Option<String>,
}

/// Every element of `entity` whose ObjectType starts with `prefix`, or every
/// one when `prefix` is empty, by `Tag`.
fn ducts(step: &str, entity: &str, prefix: &str) -> BTreeMap<String, Duct> {
    let ents = entities(step);
    let mut by_id: BTreeMap<u64, String> = BTreeMap::new();
    let mut out = BTreeMap::new();
    for (id, (e, args)) in &ents {
        if e != entity {
            continue;
        }
        let quoted: Vec<&str> = args.split('\'').skip(1).step_by(2).collect();
        let (Some(name), Some(tag)) = (quoted.get(1), quoted.last()) else {
            continue;
        };
        // GlobalId, Name, then ObjectType after the description ($).
        let object_type = quoted.get(2).copied().unwrap_or_default();
        if !object_type.starts_with(prefix) {
            continue;
        }
        by_id.insert(*id, tag.to_string());
        out.insert(
            tag.to_string(),
            Duct {
                name: name.to_string(),
                object_type: object_type.to_string(),
                type_name: None,
            },
        );
    }
    for (e, args) in ents.values() {
        if e != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let type_name = ents
            .get(&ty)
            .and_then(|(_, a)| a.split('\'').nth(3))
            .map(str::to_string);
        for element in related.iter().skip(1) {
            if let Some(duct) = by_id.get(element).and_then(|tag| out.get_mut(tag)) {
                duct.type_name = type_name.clone();
            }
        }
    }
    out
}

#[test]
fn re1_ducts_are_named_and_typed_as_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("RE1-Mechanical.rvt");
    let reference = dir.join("RE1-Mechanical.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no RE1 Mechanical model and reference export");
        return;
    }
    // Revit writes ducts and pipes alike as IfcFlowSegment.
    let theirs = ducts(
        &std::fs::read_to_string(&reference).expect("reference IFC"),
        "IFCFLOWSEGMENT",
        "Rectangular Duct:",
    );
    assert_eq!(theirs.len(), 25, "Revit's ducts");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = ducts(&write_step(&result.model), "IFCDUCTSEGMENT", "");
    let wrong: Vec<(&String, &Duct, Option<&Duct>)> = theirs
        .iter()
        .filter(|(tag, duct)| ours.get(*tag) != Some(duct))
        .map(|(tag, duct)| (tag, duct, ours.get(tag)))
        .take(5)
        .collect();
    assert!(
        wrong.is_empty(),
        "ducts not named and typed as Revit's (Tag, Revit's, rvt-rs's), first 5: {wrong:?}"
    );
}
