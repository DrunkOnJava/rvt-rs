//! Family-instance materials match Revit's own export (#355).
//!
//! Revit's IFC export associates a material with every one of the 256
//! columns of `2024_Core_Interior.rvt`: "Default Wall" on the 149 that walls
//! are joined to, the material of those walls, and `<Unnamed>` on the 107
//! that no wall joins, whose type draws its geometry with no material of
//! its own. rvt-rs must give every column exactly Revit's set of material
//! names.
//!
//! A material set is read the IFC4 way: the names of every `IfcMaterial`
//! reachable from the `IfcRelAssociatesMaterial` that relates the element
//! (directly, or through a material list, constituent set, layer set or
//! layer set usage).
//!
//! Revit's export of `12_Rooms_Door_Windows.rvt` (puzzbobb's R2025 models,
//! #356) gives its three window trims of each kind and three muntin patterns
//! the materials their instance parameters name, "Wood - Stained", "Clad -
//! White" or both (RE-200): their types' geometry-material maps hold no
//! material, so `<Unnamed>` would be wrong on all nine.
//!
//! The walls of the R2025 models are drawn in a layer whose material is the
//! Walls category's, which a 2025 file keeps on the category's graphics
//! styles (RE-206). Revit's export writes "Default Wall" for every one, 5 to
//! 9 a model; each must have Revit's set.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (the model) and
//! `../IFC Exports/2024_Core_Interior_slim.ifc` (Revit's export), and
//! `RVT_PROJECT_CORPUS_DIR/r2025/` (the R2025 models and Revit's exports, as
//! `tools/ci/fetch-r2025-models.sh` lays them out). Skips what is absent.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// `#id` -> (entity name, raw argument text) for every entity line.
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
        let Ok(id) = id.trim().parse() else {
            continue;
        };
        out.insert(id, (entity.trim().to_string(), args.to_string()));
    }
    out
}

/// The `#n` references in a piece of argument text.
fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            let digits: String = s.chars().take_while(char::is_ascii_digit).collect();
            digits.parse().ok()
        })
        .collect()
}

/// The first quoted string of an argument list, with `''` unescaped.
fn first_string(args: &str) -> Option<String> {
    let start = args.find('\'')? + 1;
    let mut out = String::new();
    let mut chars = args[start..].chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\'' {
            if chars.peek() == Some(&'\'') {
                out.push('\'');
                chars.next();
            } else {
                return Some(out);
            }
        } else {
            out.push(c);
        }
    }
    None
}

/// Names of every `IfcMaterial` reachable from `id` through material
/// definitions.
fn material_names(ents: &BTreeMap<u64, (String, String)>, id: u64, out: &mut BTreeSet<String>) {
    let Some((entity, args)) = ents.get(&id) else {
        return;
    };
    match entity.as_str() {
        "IFCMATERIAL" => {
            if let Some(name) = first_string(args) {
                out.insert(name);
            }
        }
        "IFCMATERIALLIST"
        | "IFCMATERIALCONSTITUENTSET"
        | "IFCMATERIALCONSTITUENT"
        | "IFCMATERIALLAYERSET"
        | "IFCMATERIALLAYERSETUSAGE"
        | "IFCMATERIALLAYER"
        | "IFCMATERIALPROFILESET"
        | "IFCMATERIALPROFILE"
        | "IFCMATERIALPROFILESETUSAGE" => {
            for r in refs(args) {
                material_names(ents, r, out);
            }
        }
        _ => {}
    }
}

/// Element `Tag` -> material names, for elements of `class`.
fn materials_by_tag(step: &str, class: &str) -> BTreeMap<String, BTreeSet<String>> {
    let ents = entities(step);
    let tag_of: BTreeMap<u64, String> = ents
        .iter()
        .filter(|(_, (entity, _))| entity == class)
        .filter_map(|(id, (_, args))| {
            // The Tag is the 8th attribute, the last quoted string.
            let tag = args.rsplit('\'').nth(1)?.to_string();
            Some((*id, tag))
        })
        .collect();
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELASSOCIATESMATERIAL" {
            continue;
        }
        let all = refs(args);
        let Some((&material, related)) = all.split_last() else {
            continue;
        };
        // The first reference is the owner history.
        for element in related.iter().skip(1) {
            if let Some(tag) = tag_of.get(element) {
                material_names(&ents, material, out.entry(tag.clone()).or_default());
            }
        }
    }
    out
}

#[test]
fn core_interior_columns_take_revits_materials() {
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
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = materials_by_tag(&write_step(&result.model), "IFCCOLUMN");
    let theirs = materials_by_tag(
        &std::fs::read_to_string(&reference).expect("reference IFC"),
        "IFCCOLUMN",
    );
    assert_eq!(theirs.len(), 256, "Revit's export relates 256 columns");
    let same = theirs
        .iter()
        .filter(|(tag, names)| ours.get(*tag) == Some(*names))
        .count();
    let wrong: Vec<_> = theirs
        .iter()
        .filter_map(|(tag, names)| {
            let mine = ours.get(tag).filter(|m| !m.is_empty())?;
            (mine != names).then(|| (tag.clone(), mine.clone(), names.clone()))
        })
        .take(5)
        .collect();
    assert_eq!(
        same,
        256,
        "every column has Revit's material set; {} have none from rvt-rs, first differing: {wrong:?}",
        theirs.keys().filter(|t| !ours.contains_key(*t)).count()
    );
}

#[test]
fn r2025_window_trims_take_their_instance_materials() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("r2025/12_Rooms_Door_Windows.rvt");
    let reference = dir.join("r2025/12_Rooms_Door_Windows.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no R2025 model 12 and reference export");
        return;
    }
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = materials_by_tag(&write_step(&result.model), "IFCBUILDINGELEMENTPROXY");
    let theirs = materials_by_tag(
        &std::fs::read_to_string(&reference).expect("reference IFC"),
        "IFCBUILDINGELEMENTPROXY",
    );
    assert_eq!(
        theirs.len(),
        9,
        "Revit's export relates 9 trims and muntins"
    );
    let differing: Vec<_> = theirs
        .iter()
        .filter(|(tag, names)| ours.get(*tag) != Some(*names))
        .map(|(tag, names)| (tag.clone(), ours.get(tag).cloned(), names.clone()))
        .collect();
    assert!(
        differing.is_empty(),
        "every trim and muntin has Revit's material set; differing (tag, rvt-rs, Revit): {differing:?}"
    );
}

#[test]
fn r2025_walls_take_their_category_material() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let Ok(listing) = std::fs::read_dir(dir.join("r2025")) else {
        eprintln!("skipping: no R2025 models");
        return;
    };
    let mut models: Vec<PathBuf> = listing
        .filter_map(|entry| Some(entry.ok()?.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .filter(|path| path.with_extension("ifc").exists())
        .collect();
    models.sort();
    if models.is_empty() {
        eprintln!("skipping: no R2025 model with a reference export");
        return;
    }
    let (mut walls, mut differing) = (0usize, Vec::new());
    for rvt in &models {
        let mut rf = RevitFile::open(rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = materials_by_tag(&write_step(&result.model), "IFCWALL");
        let theirs = materials_by_tag(
            &std::fs::read_to_string(rvt.with_extension("ifc")).expect("reference IFC"),
            "IFCWALL",
        );
        for (tag, names) in &theirs {
            walls += 1;
            if ours.get(tag) != Some(names) {
                differing.push((
                    rvt.file_name().map(|n| n.to_string_lossy().into_owned()),
                    tag.clone(),
                    ours.get(tag).cloned(),
                    names.clone(),
                ));
            }
        }
    }
    assert!(walls > 0, "Revit's exports relate materials to walls");
    assert!(
        differing.is_empty(),
        "every R2025 wall has Revit's material set; {} of {walls} differ (model, tag, rvt-rs, Revit): {differing:?}",
        differing.len()
    );
}
