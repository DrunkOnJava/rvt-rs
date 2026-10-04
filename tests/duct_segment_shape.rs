//! B43 (#35): a duct's `Pset_DuctSegmentTypeCommon.Shape` is Revit's.
//!
//! Revit's export of RE1 Mechanical gives each of its 25 ducts
//! `Pset_DuctSegmentTypeCommon.Shape` as an `IfcPropertyEnumeratedValue`
//! holding `IfcLabel('RECTANGULAR')`, the shape of the duct's type. For every
//! duct Revit's export writes the property on, rvt-rs must write the same
//! enumerated value on the element with that `Tag`.
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

/// A `Shape` property: its entity and its value text.
type Shape = (String, String);

/// Each element's `Pset_DuctSegmentTypeCommon.Shape`, by its `Tag`.
fn shapes(step: &str) -> BTreeMap<String, Shape> {
    let ents = entities(step);
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYPROPERTIES" {
            continue;
        }
        let all = refs(args);
        let Some((&set, related)) = all.split_last() else {
            continue;
        };
        let Some((_, set_args)) = ents
            .get(&set)
            .filter(|(e, a)| e == "IFCPROPERTYSET" && a.contains("'Pset_DuctSegmentTypeCommon'"))
        else {
            continue;
        };
        let shape = refs(set_args).into_iter().skip(1).find_map(|p| {
            let (e, a) = ents.get(&p)?;
            a.starts_with("'Shape'").then(|| {
                let value = a.split_once(",$,").map_or("", |(_, rest)| rest);
                (
                    e.clone(),
                    value
                        .trim_end_matches(';')
                        .trim_end_matches(')')
                        .trim_end_matches(",$")
                        .to_string(),
                )
            })
        });
        let Some(shape) = shape else {
            continue;
        };
        for element in related.iter().skip(1) {
            if let Some(tag) = ents.get(element).and_then(|(_, a)| a.rsplit('\'').nth(1)) {
                out.insert(tag.to_string(), shape.clone());
            }
        }
    }
    out
}

#[test]
fn re1_duct_shapes_are_revits() {
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
    let step = std::fs::read_to_string(&reference).expect("reference IFC");
    // Revit also writes the set on Mechanical's pipes; the ducts are the
    // elements whose ObjectType is a Rectangular Duct type.
    let ducts: Vec<String> = entities(&step)
        .values()
        .filter(|(e, a)| e == "IFCFLOWSEGMENT" && a.contains("'Rectangular Duct:"))
        .filter_map(|(_, a)| a.rsplit('\'').nth(1).map(str::to_string))
        .collect();
    assert_eq!(ducts.len(), 25, "Revit's ducts");
    let theirs = shapes(&step);
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = shapes(&write_step(&result.model));
    let wrong: Vec<(&String, Option<&Shape>, Option<&Shape>)> = ducts
        .iter()
        .filter(|tag| theirs.get(*tag) != ours.get(*tag))
        .map(|tag| (tag, theirs.get(tag), ours.get(tag)))
        .collect();
    assert!(
        wrong.is_empty(),
        "{} of 25 ducts' Shape is not Revit's (Tag, Revit's, rvt-rs's), first 3: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(3)]
    );
}
