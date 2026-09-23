//! RE-48: rvt-rs writes the GlobalId Revit's own exporter gives each
//! element, rebuilt from `Global/History` and `Global/ElemTable`.
//!
//! Every element of Core Interior (Revit 2024) and of the RE1 models (Revit
//! 2025) rvt-rs exports has the GlobalId Revit's export gives the same
//! `Tag`. Corpus-gated through `RVT_PROJECT_CORPUS_DIR`.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Top-level STEP arguments, keeping quoted strings and aggregates whole.
fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let mut quoted = false;
    for ch in args.chars() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                out.push(std::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.push(ch);
    }
    out.push(current);
    out
}

/// GlobalId of the first element carrying each `Tag` (openings and types
/// excluded: their GlobalIds are not the element's).
fn global_ids_by_tag(step: &str) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        let entity = entity.trim();
        if !entity.starts_with("IFC") || entity.ends_with("TYPE") || entity == "IFCOPENINGELEMENT" {
            continue;
        }
        let args = args.trim_end().trim_end_matches(';');
        let args = split_args(args.strip_suffix(')').unwrap_or(args));
        let (Some(global_id), Some(tag)) = (args.first(), args.get(7)) else {
            continue;
        };
        let Ok(tag) = tag.trim_matches('\'').parse() else {
            continue;
        };
        out.entry(tag)
            .or_insert_with(|| global_id.trim_matches('\'').to_string());
    }
    out
}

/// Elements whose GlobalId equals Revit's, and every one that does not.
fn check(rvt: &Path, reference: &Path) -> (usize, Vec<(u32, String, String)>) {
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let ours = global_ids_by_tag(&write_step(&result.model));
    let theirs = global_ids_by_tag(&std::fs::read_to_string(reference).expect("reference IFC"));
    let mut same = 0;
    let mut different = Vec::new();
    for (tag, global_id) in &ours {
        match theirs.get(tag) {
            Some(revit) if revit == global_id => same += 1,
            Some(revit) => different.push((*tag, global_id.clone(), revit.clone())),
            None => {}
        }
    }
    (same, different)
}

/// Every GlobalId of `entity` in a STEP file.
fn global_ids_of(step: &str, entity: &str) -> std::collections::BTreeSet<String> {
    let needle = format!("={entity}('");
    step.lines()
        .filter_map(|line| {
            let at = line.find(&needle)? + needle.len();
            line.get(at..at + 22).map(str::to_string)
        })
        .collect()
}

fn corpus() -> Option<PathBuf> {
    std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)
}

#[test]
fn core_interior_global_ids_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let rvt = dir.join("2024_Core_Interior.rvt");
    let reference = dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !rvt.exists() || !reference.exists() {
        eprintln!("skipping: no Core Interior model and reference export");
        return;
    }
    let (same, different) = check(&rvt, &reference);
    assert_eq!(different, []);
    // 360 walls, 256 columns, 132 doors, 80 slabs, 20 shading devices, 6
    // windows.
    assert_eq!(same, 854);

    // Rooms and storeys carry no Tag; their GlobalIds are Revit's too.
    let mut rf = RevitFile::open(&rvt).expect("open");
    let ours = write_step(
        &RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export")
            .model,
    );
    let theirs = std::fs::read_to_string(&reference).expect("reference IFC");
    for (entity, count) in [("IFCSPACE", 116), ("IFCBUILDINGSTOREY", 15)] {
        let revit = global_ids_of(&theirs, entity);
        assert_eq!(revit.len(), count, "{entity}");
        assert_eq!(global_ids_of(&ours, entity), revit, "{entity}");
    }
}

#[test]
fn re1_global_ids_are_revits() {
    let Some(dir) = corpus() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut checked = 0;
    for (model, expected) in [
        ("Architecture", 73),
        ("Mechanical", 73),
        ("Plumbing", 123),
        ("Electrical", 12),
    ] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            continue;
        }
        let (same, different) = check(&rvt, &reference);
        assert_eq!(different, [], "RE1 {model}");
        assert_eq!(same, expected, "RE1 {model}");
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no RE1 model under RVT_PROJECT_CORPUS_DIR");
    }
}
