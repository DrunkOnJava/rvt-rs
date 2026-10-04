//! B78: each room is typed by an `IfcSpaceType` of its own, as Revit's export
//! types it.
//!
//! Revit's IFC4 export of `2024_Core_Interior.rvt` relates each of its 116
//! `IfcSpace`s to an `IfcSpaceType` of its own, named `<room name> <room
//! number>:<ElementId>`, its `Tag` the room's ElementId and its GlobalId the
//! room's as a sub-element. For every space Revit types, rvt-rs's space with
//! the same `Tag` must be typed by an `IfcSpaceType` with Revit's name, Tag
//! and GlobalId.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

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

/// Each space's type, by the space's `Tag`: the type's GlobalId, name and
/// `Tag`.
#[derive(Debug, PartialEq, Clone)]
struct SpaceType {
    global_id: String,
    name: String,
    tag: String,
}

fn space_types(step: &str) -> BTreeMap<String, SpaceType> {
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
        let Some((_, type_args)) = ents.get(&ty).filter(|(e, _)| e == "IFCSPACETYPE") else {
            continue;
        };
        let quoted: Vec<&str> = type_args.split('\'').skip(1).step_by(2).collect();
        let (Some(global_id), Some(name), Some(tag)) =
            (quoted.first(), quoted.get(1), quoted.get(2))
        else {
            continue;
        };
        let space_type = SpaceType {
            global_id: global_id.to_string(),
            name: name.to_string(),
            tag: tag.to_string(),
        };
        // An IfcSpace has no Tag; its type's Tag is the room's ElementId.
        if related
            .iter()
            .skip(1)
            .any(|element| ents.get(element).is_some_and(|(e, _)| e == "IFCSPACE"))
        {
            out.insert(space_type.tag.clone(), space_type);
        }
    }
    out
}

#[test]
fn core_interior_spaces_are_typed_as_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("2024_Core_Interior.rvt");
    let reference = dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no Core Interior model and reference export");
        return;
    }
    let theirs = space_types(&std::fs::read_to_string(&reference).expect("reference IFC"));
    assert_eq!(theirs.len(), 116, "Revit's typed spaces");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = space_types(&write_step(&result.model));
    let wrong: Vec<(&String, &SpaceType, Option<&SpaceType>)> = theirs
        .iter()
        .filter(|(tag, space_type)| ours.get(*tag) != Some(space_type))
        .map(|(tag, space_type)| (tag, space_type, ours.get(tag)))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of Revit's 116 space types are not rvt-rs's (room, Revit's, rvt-rs's), first 3: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(3)]
    );
}
