//! RE-156 (#35): an element's `SerialNumber` is Revit's.
//!
//! Revit's IFC4 exports of RE1 Electrical and Plumbing give every element
//! with a value of the shared parameter `Serial Number`
//! `Pset_ManufacturerOccurrence.SerialNumber` (`IfcIdentifier`), and a
//! building-element proxy the same value in
//! `Pset_PrecastConcreteElementGeneral.SerialNumber`: 31 elements on
//! Electrical, 1 on Plumbing. For every one of them rvt-rs writes with
//! Revit's `Tag`, rvt-rs's value must be Revit's, set for set.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `RE1-Electrical.rvt` and
//! `RE1-Plumbing.rvt` with their `RE1-*.ifc` exports. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// (element `Tag`, set) -> `SerialNumber` as written.
type Serials = BTreeMap<(String, String), String>;

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

/// Every `SerialNumber` in a property set on an element with a `Tag`.
fn serials(step: &str) -> Serials {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out = Serials::new();
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
                if unquote(pf.first()?) != "SerialNumber" {
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

/// Every element `Tag` in a STEP file.
fn tags(step: &str) -> Vec<String> {
    entities(step)
        .values()
        .filter_map(|(_, a)| {
            let tag = split_args(a).get(7)?.trim().trim_matches('\'').to_string();
            tag.parse::<u64>().is_ok().then_some(tag)
        })
        .collect()
}

#[test]
fn re1_serial_numbers_are_revits() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for (model, expected) in [("Electrical", 31), ("Plumbing", 1)] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference_ifc = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping RE1 {model}: model or reference export absent");
            continue;
        }
        let theirs = serials(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let step = write_step(&result.model);
        let written = tags(&step);
        let shared: Serials = theirs
            .into_iter()
            .filter(|((tag, _), _)| written.contains(tag))
            .collect();
        let elements: BTreeSet<&String> = shared.keys().map(|(tag, _)| tag).collect();
        assert_eq!(
            elements.len(),
            expected,
            "RE1 {model}: elements with Revit's SerialNumber that rvt-rs writes"
        );
        let ours = serials(&step);
        let off: Vec<(&(String, String), &String, Option<&String>)> = shared
            .iter()
            .filter(|(key, value)| ours.get(*key) != Some(*value))
            .map(|(key, value)| (key, value, ours.get(key)))
            .collect();
        assert!(
            off.is_empty(),
            "RE1 {model}: {} of Revit's {} SerialNumber values are not rvt-rs's ((tag, set), Revit's, rvt-rs's); first: {:?}",
            off.len(),
            shared.len(),
            off.iter().take(5).collect::<Vec<_>>()
        );
    }
}
