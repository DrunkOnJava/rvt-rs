//! RE-153 (#35): a room's `Pset_SpaceCommon` is Revit's.
//!
//! Revit's IFC2X3 export (Coordination View 2.0) of RE1 Architecture gives
//! each of its 11 rooms a `Pset_SpaceCommon` with `Reference`
//! (`IfcIdentifier`, the room's name and number) and `FloorCovering`
//! (`IfcLabel`, its Floor Finish). rvt-rs writes IFC4, whose
//! `Pset_SpaceCommon` has no covering properties: it writes `FloorCovering`
//! in `Pset_SpaceCoveringRequirements`, where IFC4 puts it, and its
//! `Pset_SpaceCommon` has the `IsExternal` IFC2X3's lacks (RE-193). Every
//! space rvt-rs writes must carry, without `IsExternal`, the properties of
//! Revit's set, each in the set IFC4 puts it in, property for property and
//! value type for value type, matched by the space's `Name` (its number).
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Architecture.rvt` with
//! `RE1-Architecture.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// A space's properties: `<set>.<property>` -> value as written.
type Props = BTreeMap<String, String>;

/// The properties of IFC2X3's `Pset_SpaceCommon` that IFC4 moved to
/// `Pset_SpaceCoveringRequirements` (buildingSMART's IFC2X3 and IFC4 property
/// set definitions).
const IFC2X3_SPACE_COVERINGS: [&str; 6] = [
    "FloorCovering",
    "WallCovering",
    "CeilingCovering",
    "SkirtingBoard",
    "ConcealedFlooring",
    "ConcealedCeiling",
];

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

/// Space `Name` -> `<set>.<property>` -> its value as written
/// (`IFCLABEL('Wood')`), for each space, across its sets named in `sets`.
fn space_sets(step: &str, sets: &[&str]) -> BTreeMap<String, Props> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out: BTreeMap<String, Props> = BTreeMap::new();
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
        let set_name = set_fields.get(2).map(|f| unquote(f)).unwrap_or_default();
        if set_entity != "IFCPROPERTYSET" || !sets.contains(&set_name.as_str()) {
            continue;
        }
        let list = set_fields.get(4).map(String::as_str).unwrap_or("");
        let props: Props = split_args(list.trim_start_matches('(').trim_end_matches(')'))
            .iter()
            .filter_map(|f| ents.get(&reference(f)?))
            .filter_map(|(_, a)| {
                let p = split_args(a);
                Some((
                    format!("{set_name}.{}", unquote(p.first()?)),
                    p.get(2)?.trim().to_string(),
                ))
            })
            .collect();
        for object in split_args(objects.trim_start_matches('(').trim_end_matches(')')) {
            let Some((e, a)) = reference(&object).and_then(|id| ents.get(&id)) else {
                continue;
            };
            if e == "IFCSPACE" {
                if let Some(name) = split_args(a).get(2) {
                    out.entry(unquote(name)).or_default().extend(props.clone());
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
    let theirs: BTreeMap<String, Props> = space_sets(
        &std::fs::read_to_string(&reference_ifc).expect("reference IFC"),
        &["Pset_SpaceCommon"],
    )
    .into_iter()
    .map(|(space, props)| {
        let props: Props = props
            .into_iter()
            .map(|(key, value)| match key.strip_prefix("Pset_SpaceCommon.") {
                Some(property) if IFC2X3_SPACE_COVERINGS.contains(&property) => {
                    (format!("Pset_SpaceCoveringRequirements.{property}"), value)
                }
                _ => (key, value),
            })
            .collect();
        (space, props)
    })
    .collect();
    assert_eq!(theirs.len(), 11, "Revit's spaces with Pset_SpaceCommon");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours: BTreeMap<String, Props> = space_sets(
        &write_step(&result.model),
        &["Pset_SpaceCommon", "Pset_SpaceCoveringRequirements"],
    )
    .into_iter()
    .map(|(space, mut props)| {
        props.remove("Pset_SpaceCommon.IsExternal");
        (space, props)
    })
    .collect();
    let differing: Vec<(&String, &Props, Option<&Props>)> = theirs
        .iter()
        .filter(|(space, props)| ours.get(*space) != Some(*props))
        .map(|(space, props)| (space, props, ours.get(space)))
        .collect();
    assert!(
        differing.is_empty(),
        "{} of Revit's {} spaces' Pset_SpaceCommon properties differ in rvt-rs's export, each in its IFC4 set (space, Revit's, rvt-rs's): {differing:?}",
        differing.len(),
        theirs.len()
    );
    let extra: Vec<&String> = ours
        .keys()
        .filter(|space| !theirs.contains_key(*space))
        .collect();
    assert!(
        extra.is_empty(),
        "Pset_SpaceCommon or Pset_SpaceCoveringRequirements on spaces Revit gives no Pset_SpaceCommon: {extra:?}"
    );
}
