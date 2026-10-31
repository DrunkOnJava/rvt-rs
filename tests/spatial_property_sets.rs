//! #35, #822: the property sets Revit's export gives storeys, the building
//! and spaces, from their names and the storey count.
//!
//! Revit's IFC2X3 exports (Coordination View 2.0) of the four RE1 models give
//! each storey `Pset_BuildingStoreyCommon` with `AboveGround` unknown
//! (`IfcLogical`), and the building `Pset_BuildingCommon` with
//! `NumberOfStoreys` (`IfcInteger`) and `IsLandmarked` unknown. Each of these
//! sets must be rvt-rs's on the same object, property for property and value
//! type for value type. They also give each storey and space
//! `Pset_AirSideSystemInformation` and `Pset_ProductRequirements`, each with
//! `Name` the object's name ([`IFC2X3_NAME_SETS`]): rvt-rs writes IFC4, which
//! has no `Pset_ProductRequirements` and applies
//! `Pset_AirSideSystemInformation` to spaces and zones only, and Revit's IFC4
//! export of the BIM4LCA office gives neither, so rvt-rs must write neither.
//! A storey or space is matched by its `Name`. `NumberOfStoreys` is left out
//! where a storey of Revit's export contains no element, as rvt-rs leaves it
//! out there: Revit's exporters before and since IFC 26.1 count it otherwise
//! (RE-263, the maintainer, 2026-10-29). rvt-rs also gives each storey its
//! Level's type name as `Reference` (RE-275), which Revit's IFC4 exports write
//! and IFC2X3's `Pset_BuildingStoreyCommon` does not have ([`IFC4_ONLY`]), so
//! it is left out of the comparison with these IFC2X3 exports.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-<discipline>.rvt` with
//! `RE1-<discipline>.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const SETS: &[&str] = &[
    "Pset_AirSideSystemInformation",
    "Pset_ProductRequirements",
    "Pset_BuildingStoreyCommon",
    "Pset_BuildingCommon",
];

/// The sets of [`SETS`] holding a storey's or space's name in Revit's IFC2X3
/// export, which rvt-rs's IFC4 export does not write.
const IFC2X3_NAME_SETS: [&str; 2] = ["Pset_AirSideSystemInformation", "Pset_ProductRequirements"];

/// Properties rvt-rs writes in IFC4 that IFC2X3's definition of their set does
/// not have, so Revit's IFC2X3 exports of RE1 cannot hold them.
const IFC4_ONLY: [(&str, &str); 1] = [("Pset_BuildingStoreyCommon", "Reference")];

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

/// Whether every storey of a STEP file contains an element through an
/// `IfcRelContainedInSpatialStructure` (its sixth argument, the structure).
fn every_storey_holds_an_element(step: &str) -> bool {
    let ents = entities(step);
    let holding: BTreeSet<u64> = ents
        .values()
        .filter(|(entity, _)| entity == "IFCRELCONTAINEDINSPATIALSTRUCTURE")
        .filter_map(|(_, args)| reference(split_args(args).get(5)?))
        .collect();
    ents.iter()
        .filter(|(_, (entity, _))| entity == "IFCBUILDINGSTOREY")
        .all(|(id, _)| holding.contains(id))
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
        let reference_step = std::fs::read_to_string(&reference_ifc).expect("reference IFC");
        let mut theirs = spatial_sets(&reference_step);
        assert!(
            theirs.contains_key(&("building".to_string(), "Pset_BuildingCommon".to_string())),
            "RE1 {discipline}: Revit's building has no Pset_BuildingCommon"
        );
        if !every_storey_holds_an_element(&reference_step) {
            let building = ("building".to_string(), "Pset_BuildingCommon".to_string());
            if let Some(props) = theirs.get_mut(&building) {
                props.remove("NumberOfStoreys");
            }
        }
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let mut ours = spatial_sets(&write_step(&result.model));
        for ((_, set), props) in ours.iter_mut() {
            for (only_set, property) in IFC4_ONLY {
                if set == only_set {
                    props.remove(property);
                }
            }
        }
        let differing: Vec<(&(String, String), &Props, Option<&Props>)> = theirs
            .iter()
            .filter(|((_, set), _)| !IFC2X3_NAME_SETS.contains(&set.as_str()))
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
        let name_sets: Vec<&(String, String)> = ours
            .keys()
            .filter(|(_, set)| IFC2X3_NAME_SETS.contains(&set.as_str()))
            .collect();
        assert!(
            name_sets.is_empty(),
            "RE1 {discipline}: rvt-rs writes IFC2X3's name sets on {} storeys and spaces: {:?}",
            name_sets.len(),
            name_sets.iter().take(5).collect::<Vec<_>>()
        );
    }
    eprintln!("checked {models} RE1 models");
}
