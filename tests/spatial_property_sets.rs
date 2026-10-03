//! #35: the property sets Revit's export gives storeys, the building and
//! spaces, from their names and the storey count.
//!
//! Revit's IFC4 exports of the four RE1 models give each storey
//! `Pset_AirSideSystemInformation` and `Pset_ProductRequirements`, each with
//! `Name` (`IfcLabel`) the storey's name, and `Pset_BuildingStoreyCommon`
//! with `AboveGround` unknown (`IfcLogical`); the building
//! `Pset_BuildingCommon` with `NumberOfStoreys` (`IfcInteger`) and
//! `IsLandmarked` unknown; and each space the two `Name` sets, holding the
//! room's name. Every one of these sets rvt-rs writes on the same object
//! must be Revit's, property for property and value type for value type. A
//! storey or space is matched by its `Name`.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-<discipline>.rvt` with
//! `RE1-<discipline>.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SETS: &[&str] = &[
    "Pset_AirSideSystemInformation",
    "Pset_ProductRequirements",
    "Pset_BuildingStoreyCommon",
    "Pset_BuildingCommon",
];

/// A property set's properties: name -> value as written.
type Props = BTreeMap<String, String>;
/// (object, set) -> its properties.
type Sets = BTreeMap<(String, String), Props>;

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

/// Every set of [`SETS`] on a storey, the building or a space.
fn spatial_sets(step: &str) -> Sets {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out = Sets::new();
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
        let Some(set_name) = set_fields.get(2).map(|f| unquote(f)) else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" || !SETS.contains(&set_name.as_str()) {
            continue;
        }
        let list = set_fields.get(4).map(String::as_str).unwrap_or("");
        let props: Props = split_args(list.trim_start_matches('(').trim_end_matches(')'))
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
            let key = match e.as_str() {
                "IFCBUILDING" => "building".to_string(),
                "IFCBUILDINGSTOREY" | "IFCSPACE" => {
                    let Some(name) = split_args(a).get(2).map(|n| unquote(n)) else {
                        continue;
                    };
                    format!("{e} {name}")
                }
                _ => continue,
            };
            out.insert((key, set_name.clone()), props.clone());
        }
    }
    out
}

#[test]
fn re1_spatial_property_sets_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut models = 0;
    for discipline in ["Architecture", "Mechanical", "Plumbing", "Electrical"] {
        let rvt = dir.join(format!("RE1-{discipline}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{discipline}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {discipline}: no model and reference export");
            continue;
        }
        models += 1;
        let theirs = spatial_sets(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        assert!(
            theirs.contains_key(&("building".to_string(), "Pset_BuildingCommon".to_string())),
            "RE1 {discipline}: Revit's building has no Pset_BuildingCommon"
        );
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = spatial_sets(&write_step(&result.model));
        let differing: Vec<(&(String, String), &Props, Option<&Props>)> = theirs
            .iter()
            .filter(|(key, props)| ours.get(*key) != Some(*props))
            .map(|(key, props)| (key, props, ours.get(key)))
            .collect();
        assert!(
            differing.is_empty(),
            "RE1 {discipline}: {} of Revit's {} spatial property sets differ in rvt-rs's export \
             ((object, set), Revit's, rvt-rs's): {differing:?}",
            differing.len(),
            theirs.len()
        );
    }
    eprintln!("checked {models} RE1 models");
}
