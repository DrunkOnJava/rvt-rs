//! RE-172: the Level parameters in `Constraints` are Revit's.
//!
//! Autodesk's public Revit 2024 sample `racadvancedsampleproject.rvt` with
//! Revit's own IFC4 export of it. For every element `Tag` that both files
//! carry, each of `Level`, `Base Level`, `Base Constraint`, `Top Constraint`
//! that rvt-rs writes must equal Revit's value, and rvt-rs must write a
//! substantial number of them (measured: 5254 `Level`, 196 `Base Level`, 180
//! `Base Constraint`, 177 `Top Constraint`, 35 beam `Reference Level`, 5 stair
//! `Top Level`; RE-174).
//!
//! Runs against `RVT_REVIT2024_SAMPLES_DIR`, holding the `.rvt` and its
//! `.ifc` export. Skips when it is not set or the files are absent.

use crate::is_external::{entities, list, split_args};
use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Each checked property with the least number rvt-rs must write.
const NAMES: [(&str, usize); 6] = [
    ("Level", 5000),
    ("Base Level", 150),
    ("Base Constraint", 150),
    ("Top Constraint", 150),
    ("Reference Level", 30),
    ("Top Level", 4),
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

/// Revit's storey name per element `Tag`: from
/// `IfcRelContainedInSpatialStructure`; for an element contained in a room
/// (`IfcSpace`), the storey the room is aggregated into; and for a part (a
/// curtain wall's mullion or panel) the storey of what it is aggregated into.
fn revit_storeys(step: &str) -> BTreeMap<String, String> {
    let ents = entities(step);
    let unquote = |raw: &str| raw.trim().trim_matches('\'').to_string();
    let reference = |s: &str| s.trim().strip_prefix('#')?.parse::<u64>().ok();
    let mut storey_of: BTreeMap<u64, String> = BTreeMap::new();
    let mut parent_of: BTreeMap<u64, u64> = BTreeMap::new();
    for (id, (entity, args)) in &ents {
        let f = split_args(args);
        match entity.as_str() {
            "IFCRELCONTAINEDINSPATIALSTRUCTURE" => {
                let Some((storey_entity, storey_args)) = f
                    .get(5)
                    .and_then(|s| reference(s))
                    .and_then(|s| ents.get(&s))
                else {
                    continue;
                };
                if storey_entity == "IFCSPACE" {
                    // Revit contains furniture and fixtures in their room; the
                    // room is aggregated into its storey.
                    let space = f.get(5).and_then(|s| reference(s)).unwrap_or(0);
                    for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
                        parent_of.insert(object, space);
                    }
                    continue;
                }
                if storey_entity != "IFCBUILDINGSTOREY" {
                    continue;
                }
                let Some(storey) = split_args(storey_args).get(2).map(|n| unquote(n)) else {
                    continue;
                };
                for object in list(f.get(4).map(String::as_str).unwrap_or("")) {
                    storey_of.insert(object, storey.clone());
                }
            }
            "IFCRELAGGREGATES" => {
                let Some(parent) = f.get(4).and_then(|s| reference(s)) else {
                    continue;
                };
                for part in list(f.get(5).map(String::as_str).unwrap_or("")) {
                    parent_of.insert(part, parent);
                }
            }
            "IFCBUILDINGSTOREY" => {
                if let Some(name) = f.get(2).map(|n| unquote(n)) {
                    storey_of.insert(*id, name);
                }
            }
            _ => {}
        }
    }
    let mut out = BTreeMap::new();
    for (&id, (_, args)) in &ents {
        let mut at = id;
        for _ in 0..4 {
            if let Some(storey) = storey_of.get(&at) {
                if let Some(tag) = split_args(args).get(7).map(|t| unquote(t)) {
                    out.insert(tag, storey.clone());
                }
                break;
            }
            match parent_of.get(&at) {
                Some(&parent) => at = parent,
                None => break,
            }
        }
    }
    out
}

/// BIM4LCA / Nordic LCA, `ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit`
/// (Revit 2024; VTT, Granlund, Gravicon, Insinööritoimisto Kallinen; CC BY-SA
/// 4.0, <https://cris.vtt.fi/en/publications/bim-files-to-download/>) with
/// Revit's own IFC4 export, which carries no Revit level parameters. The oracle
/// is Revit's storey containment.
///
/// Measured: rvt-rs exports 507 building elements on 4 storeys and writes a
/// level for 467 of them (`Level_01`); all 467 are Revit's storey.
/// (Housing_Concrete_BuildingPermit: 1388 of 1401 agree once elements Revit
/// contains in a room take the room's storey; the 13 others are openings,
/// which Revit puts on their host's storey.) Housing_Timber_*
/// recovers no Level ids, so it writes none.
///
/// Runs against `RVT_NORDIC_LCA_DIR`, a folder holding the `.rvt` and the
/// `_Revit.ifc` of that name. Skips when absent.
#[test]
fn nordic_housing_terrain_levels_are_revits_storeys() {
    const NAME: &str = "ARK_NordicLCA_Housing_Terrain-Concrete_BuildingPermit";
    let Some(dir) = std::env::var_os("RVT_NORDIC_LCA_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_NORDIC_LCA_DIR is not set");
        return;
    };
    let rvt = dir.join(format!("{NAME}.rvt"));
    let reference_ifc = dir.join(format!("{NAME}_Revit.ifc"));
    if !rvt.exists() || !reference_ifc.exists() {
        eprintln!("skipping: model or reference export absent");
        return;
    }
    let storeys = revit_storeys(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
    let mut rf = RevitFile::open(&rvt).expect("open");
    let levels: Vec<String> = rvt::partition_level_records::recover_partition_levels(&mut rf, 2024)
        .expect("levels")
        .into_iter()
        .map(|level| level.name)
        .collect();
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = constraints(&write_step(&result.model));
    let (mut written, mut agree) = (0usize, 0usize);
    for ((tag, name), value) in &ours {
        if !["Level", "Base Level", "Base Constraint"].contains(&name.as_str()) {
            continue;
        }
        let level = value
            .trim_start_matches("IFCLABEL('")
            .trim_end_matches("')");
        assert!(
            levels.iter().any(|l| l == level),
            "{tag}: {name} {level:?} is not a recovered Level ({levels:?})"
        );
        written += 1;
        agree += usize::from(storeys.get(tag).is_some_and(|s| s == level));
    }
    eprintln!("{agree} of {written} levels agree with Revit's storey");
    assert!(
        written >= 400,
        "{written} levels written, expected at least 400"
    );
    assert!(
        agree * 100 >= written * 99,
        "{agree} of {written} agree with Revit's storey, expected 99%"
    );
}
