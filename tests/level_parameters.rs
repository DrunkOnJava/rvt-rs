//! RE-172: the Level parameters in `Constraints` are Revit's.
//!
//! Autodesk's public Revit 2024 sample `racadvancedsampleproject.rvt` with
//! Revit's own IFC4 export of it. For every element `Tag` that both files
//! carry, each of `Level`, `Base Level`, `Base Constraint`, `Top Constraint`
//! that rvt-rs writes must equal Revit's value, and rvt-rs must write a
//! substantial number of them (measured: 5214 `Level`, 191 `Base Level`, 180
//! `Base Constraint`, 177 `Top Constraint`).
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR`, holding the `.rvt` and its
//! `.ifc` export. Skips when it is not set or the files are absent.

use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Each checked property with the least number rvt-rs must write.
const NAMES: [(&str, usize); 4] = [
    ("Level", 5000),
    ("Base Level", 150),
    ("Base Constraint", 150),
    ("Top Constraint", 150),
];

/// (element `Tag`, property) -> value as written, from every `Constraints` set.
fn constraints(step: &str) -> BTreeMap<(String, String), String> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let f = split_args(args);
        let Some((set_entity, set_args)) = f
            .get(5)
            .and_then(|s| s.trim().strip_prefix('#')?.parse().ok())
            .and_then(|s: u64| ents.get(&s))
        else {
            continue;
        };
        if set_entity != "IFCPROPERTYSET" {
            continue;
        }
        let sf = split_args(set_args);
        if sf.get(2).map(|n| unquote(n)).as_deref() != Some("Constraints") {
            continue;
        }
        let values: Vec<(String, String)> = list(sf.get(4).map(String::as_str).unwrap_or(""))
            .iter()
            .filter_map(|p| ents.get(p))
            .filter_map(|(_, a)| {
                let pf = split_args(a);
                let name = unquote(pf.first()?);
                let value = pf.get(2)?.trim().to_string();
                NAMES
                    .iter()
                    .any(|(n, _)| *n == name)
                    .then_some((name, value))
            })
            .collect();
        for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some(tag) = ents
                .get(&object)
                .and_then(|(_, a)| split_args(a).get(7).map(|t| unquote(t)))
                .filter(|t| !t.is_empty() && t != "$")
            else {
                continue;
            };
            for (name, value) in &values {
                out.insert((tag.clone(), name.clone()), value.clone());
            }
        }
    }
    out
}

#[test]
fn racadvanced_level_parameters_are_revits() {
    let Some(dir) = std::env::var_os("RVT_REVIT2024_SAMPLES_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_REVIT2024_SAMPLES_DIR is not set");
        return;
    };
    let rvt = dir.join("racadvancedsampleproject.rvt");
    let reference_ifc = dir.join("racadvancedsampleproject.ifc");
    if !rvt.exists() || !reference_ifc.exists() {
        eprintln!("skipping: model or reference export absent");
        return;
    }
    let theirs = constraints(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = constraints(&write_step(&result.model));
    for (name, floor) in NAMES {
        let written: Vec<_> = ours.iter().filter(|((_, n), _)| n == name).collect();
        let wrong: Vec<_> = written
            .iter()
            .filter(|(key, value)| theirs.get(*key).is_some_and(|t| t != *value))
            .collect();
        assert!(
            wrong.is_empty(),
            "{name}: differs from Revit's on {wrong:?}"
        );
        assert!(
            written.len() >= floor,
            "{name}: {} written, expected at least {floor}",
            written.len()
        );
    }
}
