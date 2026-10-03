//! RE-154 (#35): the `Reference` of Revit's common property sets.
//!
//! Revit's IFC4 export gives every element of the RE1 models
//! `Pset_QuantityTakeOff.Reference` and, where its entity has one, its
//! `Pset_<Entity>Common.Reference` (and on walls and slabs
//! `Pset_ReinforcementBarPitchOf{Wall,Slab}.Reference`), each its type's name.
//! For every element rvt-rs writes as the same entity, with the same `Tag` and
//! `Name` (`Family:Type:ElementId`) as Revit's, each of those `Reference`
//! values must be in rvt-rs's export too, with Revit's value type, and rvt-rs
//! must write no `Reference` Revit does not.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: the four `RE1-*.rvt` models with
//! their `RE1-*.ifc` exports. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
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

fn list(field: &str) -> Vec<u64> {
    split_args(field.trim().trim_start_matches('(').trim_end_matches(')'))
        .iter()
        .filter_map(|f| reference(f))
        .collect()
}

/// The entity an element is compared as: Revit's export writes walls as
/// `IfcWallStandardCase`, and flow, furnishing and control elements as the
/// supertypes of the IFC4 entities rvt-rs writes.
fn supertype(entity: &str) -> &str {
    match entity {
        "IFCWALLSTANDARDCASE" => "IFCWALL",
        "IFCDUCTSEGMENT" | "IFCPIPESEGMENT" => "IFCFLOWSEGMENT",
        "IFCDUCTFITTING" | "IFCPIPEFITTING" => "IFCFLOWFITTING",
        "IFCAIRTERMINAL" | "IFCSANITARYTERMINAL" | "IFCLIGHTFIXTURE" | "IFCELECTRICAPPLIANCE" => {
            "IFCFLOWTERMINAL"
        }
        "IFCFURNITURE" => "IFCFURNISHINGELEMENT",
        "IFCALARM" => "IFCDISTRIBUTIONCONTROLELEMENT",
        other => other,
    }
}

/// One element: its entity ([`supertype`]), `Name` as written, and each
/// property set's `Reference` as written (`IFCIDENTIFIER('-')`).
#[derive(Debug, Default)]
struct Element {
    entity: String,
    name: String,
    references: BTreeMap<String, String>,
}

/// Every element with a numeric `Tag`, by `Tag`.
fn elements(step: &str) -> BTreeMap<String, Element> {
    let ents = entities(step);
    let mut out: BTreeMap<String, Element> = BTreeMap::new();
    let mut by_id: BTreeMap<u64, String> = BTreeMap::new();
    for (id, (entity, args)) in &ents {
        if entity.ends_with("TYPE") || entity.ends_with("STYLE") || entity == "IFCOPENINGELEMENT" {
            continue;
        }
        let f = split_args(args);
        let Some(tag) = f.get(7).map(|t| t.trim().trim_matches('\'').to_string()) else {
            continue;
        };
        if tag.is_empty() || !tag.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let entity = supertype(entity).to_string();
        by_id.insert(*id, tag.clone());
        out.insert(
            tag,
            Element {
                entity,
                name: f.get(2).cloned().unwrap_or_default(),
                references: BTreeMap::new(),
            },
        );
    }
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let Some((set_entity, set_args)) = f
            .get(5)
            .and_then(|s| reference(s))
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" {
            continue;
        }
        let sf = split_args(set_args);
        let set = sf
            .get(2)
            .map(|n| n.trim().trim_matches('\'').to_string())
            .unwrap_or_default();
        let value = list(sf.get(4).map(String::as_str).unwrap_or(""))
            .iter()
            .filter_map(|p| ents.get(p))
            .find_map(|(_, a)| {
                let pf = split_args(a);
                (pf.first()?.trim() == "'Reference'")
                    .then(|| pf.get(2).map(|v| v.trim().to_string()))?
            });
        let Some(value) = value else {
            continue;
        };
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            if let Some(element) = by_id.get(&object).and_then(|tag| out.get_mut(tag)) {
                element.references.insert(set.clone(), value.clone());
            }
        }
    }
    out
}

#[test]
fn re1_common_references_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut compared = 0;
    for model in ["Architecture", "Mechanical", "Plumbing", "Electrical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let theirs = elements(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = elements(&write_step(&result.model));
        let mut missing: Vec<(String, String, String)> = Vec::new();
        let mut wrong: Vec<(String, String, String, Option<String>)> = Vec::new();
        for (tag, element) in &ours {
            let Some(revit) = theirs.get(tag) else {
                for (set, value) in &element.references {
                    wrong.push((tag.clone(), set.clone(), value.clone(), None));
                }
                continue;
            };
            for (set, value) in &element.references {
                if revit.references.get(set) != Some(value) {
                    wrong.push((
                        tag.clone(),
                        set.clone(),
                        value.clone(),
                        revit.references.get(set).cloned(),
                    ));
                }
            }
            if revit.entity != element.entity || revit.name != element.name {
                continue;
            }
            compared += 1;
            for (set, value) in &revit.references {
                if set == "Pset_SpaceCommon" {
                    continue;
                }
                if element.references.get(set) != Some(value) {
                    missing.push((tag.clone(), set.clone(), value.clone()));
                }
            }
        }
        let sets: BTreeSet<&String> = missing.iter().map(|(_, set, _)| set).collect();
        assert!(
            missing.is_empty(),
            "RE1 {model}: {} of Revit's Reference values on elements rvt-rs writes as Revit does are not in rvt-rs's export, in {sets:?}; first: {:?}",
            missing.len(),
            missing.iter().take(5).collect::<Vec<_>>()
        );
        assert!(
            wrong.is_empty(),
            "RE1 {model}: Reference values Revit's export does not hold (tag, set, rvt-rs's, Revit's): {:?}",
            wrong.iter().take(5).collect::<Vec<_>>()
        );
    }
    eprintln!("{compared} elements compared");
}
