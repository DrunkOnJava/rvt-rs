//! Revit 2025 partition element records against Revit's own export (RE-32).
//!
//! The oracle is `Drshelden/IFC-ECS` `data/RE1/` (MIT): Revit 2025
//! projects next to IFC exports Revit wrote from them. Fetch it with
//! `tools/fetch-corpus.sh` and point `RVT_PROJECT_CORPUS_DIR` at
//! `IFC-ECS/data/RE1`; without it the test skips. Walls and slabs must
//! reproduce the export's ElementId (`Tag`) sets exactly, rooms its count,
//! and the doors, whose records use the second prologue (RE-30), must be
//! reported as unattributed rather than dropped silently.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn re1_architecture() -> Option<(PathBuf, PathBuf)> {
    let dir = PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?);
    let rvt = dir.join("RE1-Architecture.rvt");
    let ifc = dir.join("RE1-Architecture.ifc");
    (rvt.exists() && ifc.exists()).then_some((rvt, ifc))
}

/// `Tag` (the Revit ElementId) of every `ifc_types` entity in a STEP file.
fn tags(step: &str, ifc_types: &[&str]) -> BTreeSet<u64> {
    let mut out = BTreeSet::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if !ifc_types.contains(&entity.trim()) {
            continue;
        }
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        // Tag is the 8th attribute; the preceding seven never contain a
        // quoted comma on these files, so a quote-aware split suffices.
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut quoted = false;
        for c in args.chars() {
            match c {
                '\'' => {
                    quoted = !quoted;
                    current.push(c);
                }
                ',' if !quoted => fields.push(std::mem::take(&mut current)),
                _ => current.push(c),
            }
        }
        fields.push(current);
        if let Some(tag) = fields.get(7) {
            if let Ok(id) = tag.trim_matches('\'').parse() {
                out.insert(id);
            }
        }
    }
    out
}

fn count(step: &str, ifc_type: &str) -> usize {
    step.matches(&format!("={ifc_type}(")).count()
}

#[test]
fn revit_2025_walls_slabs_and_rooms_match_revits_export() {
    let Some((rvt, ifc)) = re1_architecture() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no RE1-Architecture.rvt/.ifc");
        return;
    };
    let reference = std::fs::read_to_string(&ifc).expect("reference IFC");
    let mut rf = RevitFile::open(&rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let step = write_step(&result.model);

    let walls = tags(&reference, &["IFCWALL", "IFCWALLSTANDARDCASE"]);
    assert_eq!(walls.len(), 7);
    assert_eq!(tags(&step, &["IFCWALL"]), walls, "wall ElementIds");

    let slabs = tags(&reference, &["IFCSLAB"]);
    assert_eq!(slabs.len(), 2);
    assert_eq!(tags(&step, &["IFCSLAB"]), slabs, "slab ElementIds");

    assert_eq!(count(&reference, "IFCSPACE"), 11);
    assert_eq!(count(&step, "IFCSPACE"), 11, "rooms");

    let unattributed = result
        .diagnostics
        .skipped
        .iter()
        .find(|item| item.reason == "element_record_without_element_id")
        .expect("second-prologue door records are counted");
    assert_eq!(unattributed.classes.get("Door"), Some(&17));

    // The readiness score must not read as complete while records are missing.
    let confidence = &result.diagnostics.confidence;
    assert_eq!(confidence.unexported_element_records, unattributed.count);
    assert!(confidence.score < 0.75, "score {}", confidence.score);
}
