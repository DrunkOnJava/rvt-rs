//! B55 (#227): the opening a wall's edited elevation profile cuts.
//!
//! Revit's IFC4 export of `2024_Core_Interior.rvt` writes wall 55840, whose
//! elevation profile was edited into a stepped and slanted outline, with an
//! `IfcOpeningElement` tagged 55859 voiding it: the part of the wall's
//! elevation rectangle the outline leaves out, in the wall's vertical plane,
//! extruded through the wall's 0.5 ft thickness. It is the 63rd of the
//! export's unfilled openings (RE-151). rvt-rs must write that opening,
//! voiding the element tagged 55840, with the same outline area and depth.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR`: `2024_Core_Interior.rvt` with
//! `../IFC Exports/2024_Core_Interior_slim.ifc`. Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

const WALL: &str = "55840";
const OPENING: &str = "55859";

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

/// The last quoted string of an element's arguments: its `Tag`.
fn tag(args: &str) -> Option<&str> {
    args.rsplit('\'').nth(1)
}

/// Feet per unit of the file's length unit.
fn feet_per_unit(step: &str) -> f64 {
    let declares = |needle: &str| {
        step.lines()
            .any(|line| line.contains(".LENGTHUNIT.") && line.contains(needle))
    };
    if declares("FOOT") {
        1.0
    } else if declares(".MILLI.") {
        1.0 / 304.8
    } else {
        1.0 / 0.3048
    }
}

/// The numbers in a piece of argument text.
fn numbers(text: &str) -> Vec<f64> {
    text.split(|c: char| !(c.is_ascii_digit() || matches!(c, '.' | '-' | 'E' | 'e')))
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// The 2D points of a profile's outer curve: an `IfcPolyline` of
/// `IfcCartesianPoint`s or an `IfcIndexedPolyCurve` of a point list.
fn curve_points(ents: &BTreeMap<u64, (String, String)>, curve: u64) -> Vec<(f64, f64)> {
    let Some((entity, args)) = ents.get(&curve) else {
        return Vec::new();
    };
    match entity.as_str() {
        "IFCPOLYLINE" => refs(args)
            .iter()
            .filter_map(|point| {
                let values = numbers(&ents.get(point)?.1);
                Some((*values.first()?, *values.get(1)?))
            })
            .collect(),
        "IFCINDEXEDPOLYCURVE" => refs(args)
            .first()
            .and_then(|list| ents.get(list))
            .map(|(_, list)| {
                numbers(list)
                    .chunks_exact(2)
                    .map(|pair| (pair[0], pair[1]))
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// The opening tagged [`OPENING`]: the `Tag` of the element it voids, its
/// outline's area (square feet) and its extrusion depth (feet).
fn wall_opening(step: &str) -> Option<(Option<String>, f64, f64)> {
    let ents = entities(step);
    let scale = feet_per_unit(step);
    let (&opening, (_, args)) = ents
        .iter()
        .find(|(_, (entity, args))| entity == "IFCOPENINGELEMENT" && tag(args) == Some(OPENING))?;
    let host = ents.values().find_map(|(entity, rel)| {
        let all = refs(rel);
        (entity == "IFCRELVOIDSELEMENT" && all.last() == Some(&opening))
            .then(|| all.get(all.len().checked_sub(2)?).copied())
            .flatten()
            .and_then(|host| ents.get(&host))
            .and_then(|(_, host_args)| tag(host_args).map(str::to_string))
    });
    let shape = *refs(args).get(2)?;
    let mut stack = vec![shape];
    while let Some(id) = stack.pop() {
        let Some((entity, solid_args)) = ents.get(&id) else {
            continue;
        };
        if entity == "IFCEXTRUDEDAREASOLID" {
            let solid_refs = refs(solid_args);
            let depth = *numbers(solid_args.rsplit(',').next()?).first()?;
            let (_, profile_args) = ents.get(solid_refs.first()?)?;
            let points = curve_points(&ents, *refs(profile_args).first()?);
            let area = points
                .iter()
                .zip(points.iter().cycle().skip(1))
                .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
                .sum::<f64>()
                .abs()
                / 2.0;
            return Some((host, area * scale * scale, depth * scale));
        }
        stack.extend(refs(solid_args));
    }
    None
}

#[test]
fn core_interior_wall_profile_opening_is_revits() {
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
    let theirs = wall_opening(&std::fs::read_to_string(&reference).expect("reference IFC"))
        .expect("Revit's export writes opening 55859");
    assert_eq!(
        theirs.0.as_deref(),
        Some(WALL),
        "Revit's opening voids wall 55840"
    );
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = wall_opening(&write_step(&result.model));
    let Some((host, area, depth)) = ours else {
        panic!("rvt-rs writes no opening tagged {OPENING}; Revit's: {theirs:?}");
    };
    assert_eq!(host.as_deref(), Some(WALL), "the element the opening voids");
    assert!(
        (area - theirs.1).abs() <= 0.01,
        "outline area {area} sq ft, Revit's {}",
        theirs.1
    );
    assert!(
        (depth - theirs.2).abs() <= 0.001,
        "depth {depth} ft, Revit's {}",
        theirs.2
    );
}
