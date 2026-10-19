//! B29 (#35, #822): every `Reference` Revit's export writes on the RE1 MEP
//! models' elements is rvt-rs's.
//!
//! Revit's IFC2X3 exports (Coordination View 2.0) of RE1 Mechanical,
//! Plumbing and Electrical give each element with a type
//! `Pset_QuantityTakeOff.Reference` and its common set's `Reference`
//! (`IfcIdentifier`), the type's name (RE-154). That includes electrical
//! equipment Revit names by its panel name (`262416-PANEL:RE-1:428352`) and
//! pipes whose type the reference list does not give (RE-130). rvt-rs writes
//! IFC4, where those values are in `Pset_EnvironmentalImpactIndicators` and
//! a distribution element's own type set ([`ifc4_set`]). For every element
//! rvt-rs writes with Revit's `Tag`, each of those values must be rvt-rs's,
//! in that IFC4 set.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-<discipline>.rvt` with
//! `RE1-<discipline>.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// (element `Tag`, set) -> `Reference` as written.
type Values = BTreeMap<(String, String), String>;

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

/// Every `Reference` in a property set on an element with a `Tag`.
fn references(step: &str) -> Values {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out = Values::new();
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
        let set = sf.get(2).map(|n| unquote(n)).unwrap_or_default();
        let value = list(sf.get(4).map(String::as_str).unwrap_or(""))
            .iter()
            .filter_map(|p| ents.get(p))
            .find_map(|(_, a)| {
                let pf = split_args(a);
                if unquote(pf.first()?) != "Reference" {
                    return None;
                }
                Some(pf.get(2)?.trim().to_string())
            });
        let Some(value) = value else {
            continue;
        };
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some(tag) = ents
                .get(&object)
                .and_then(|(_, a)| split_args(a).get(7).map(|t| unquote(t)))
                .filter(|t| !t.is_empty() && t != "$")
            else {
                continue;
            };
            out.insert((tag, set.clone()), value.clone());
        }
    }
    out
}

/// Every element `Tag` in a STEP file, with the entity it is written as.
fn tags(step: &str) -> BTreeMap<String, String> {
    entities(step)
        .into_values()
        .filter_map(|(entity, a)| {
            let tag = split_args(&a).get(7)?.trim().trim_matches('\'').to_string();
            tag.parse::<u64>().is_ok().then_some((tag, entity))
        })
        .collect()
}

/// The IFC4 set holding a `Reference` that Revit's IFC2X3 export writes in
/// `set`, on an element rvt-rs writes as `entity`: `set` itself, except for
/// IFC2X3's `Pset_QuantityTakeOff` (IFC4's `Pset_EnvironmentalImpactIndicators`)
/// and `Pset_DistributionFlowElementCommon` (the entity's IFC4 type set, none
/// for an entity IFC4 gives none), as Revit's IFC4 export of the BIM4LCA
/// office writes them.
fn ifc4_set(set: &str, entity: &str) -> Option<String> {
    let set = match set {
        "Pset_QuantityTakeOff" => "Pset_EnvironmentalImpactIndicators",
        "Pset_DistributionFlowElementCommon" => match entity {
            "IFCDUCTSEGMENT" => "Pset_DuctSegmentTypeCommon",
            "IFCPIPESEGMENT" => "Pset_PipeSegmentTypeCommon",
            "IFCDUCTFITTING" => "Pset_DuctFittingTypeCommon",
            "IFCPIPEFITTING" => "Pset_PipeFittingTypeCommon",
            "IFCAIRTERMINAL" => "Pset_AirTerminalTypeCommon",
            "IFCSANITARYTERMINAL" => "Pset_SanitaryTerminalTypeCommon",
            "IFCLIGHTFIXTURE" => "Pset_LightFixtureTypeCommon",
            "IFCELECTRICAPPLIANCE" => "Pset_ElectricApplianceTypeCommon",
            _ => return None,
        },
        other => other,
    };
    Some(set.to_string())
}

#[test]
fn re1_mep_references_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for model in ["Mechanical", "Plumbing", "Electrical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let theirs = references(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let step = write_step(&result.model);
        let written = tags(&step);
        let shared: Values = theirs
            .into_iter()
            .filter_map(|((tag, set), value)| {
                let set = ifc4_set(&set, written.get(&tag)?)?;
                Some(((tag, set), value))
            })
            .collect();
        assert!(
            !shared.is_empty(),
            "RE1 {model}: no element with Revit's Reference is written by rvt-rs"
        );
        let ours = references(&step);
        let off: Vec<(&(String, String), &String, Option<&String>)> = shared
            .iter()
            .filter(|(key, value)| ours.get(*key) != Some(*value))
            .map(|(key, value)| (key, value, ours.get(key)))
            .collect();
        assert!(
            off.is_empty(),
            "RE1 {model}: {} of Revit's {} Reference values are not rvt-rs's ((tag, set), Revit's, rvt-rs's); first: {:?}",
            off.len(),
            shared.len(),
            off.iter().take(5).collect::<Vec<_>>()
        );
    }
}
