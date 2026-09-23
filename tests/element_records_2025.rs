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

fn re1(model: &str) -> Option<(PathBuf, PathBuf)> {
    let dir = PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?);
    let rvt = dir.join(format!("RE1-{model}.rvt"));
    let ifc = dir.join(format!("RE1-{model}.ifc"));
    (rvt.exists() && ifc.exists()).then_some((rvt, ifc))
}

fn re1_architecture() -> Option<(PathBuf, PathBuf)> {
    re1("Architecture")
}

/// Export `rvt` and return its STEP text next to the reference's.
fn export_and_reference(rvt: &PathBuf, ifc: &PathBuf) -> (String, String) {
    let reference = std::fs::read_to_string(ifc).expect("reference IFC");
    let mut rf = RevitFile::open(rvt).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    (write_step(&result.model), reference)
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

    // Revit's export holds 5 doors and 1 curtain wall that rvt-rs cannot
    // attribute: their frames carry no ElementId at +0x00 (RE-30). The
    // count is 6 door frames and 1 wall frame that pass the instance
    // rule; the other second-prologue frames are container members or
    // type symbols and do not count (RE-33).
    let unattributed = result
        .diagnostics
        .skipped
        .iter()
        .find(|item| item.reason == "element_record_without_element_id")
        .expect("second-prologue door records are counted");
    assert_eq!(unattributed.classes.get("Door"), Some(&6));
    assert_eq!(unattributed.classes.get("Wall"), Some(&1));
    assert_eq!(unattributed.count, 7);

    // The readiness score must not read as complete while records are missing.
    let confidence = &result.diagnostics.confidence;
    assert_eq!(confidence.unexported_element_records, unattributed.count);
    assert!(confidence.score < 1.0, "score {}", confidence.score);
}

/// RE-33: the other product categories whose element records decode on
/// RE1 Architecture. Each exported set must equal the reference export's
/// set for the entity Revit chose (an IFC2x3 export, so furniture and
/// casework are `IfcFurnishingElement` there and plumbing fixtures
/// `IfcFlowTerminal`; rvt-rs writes the IFC4 types).
#[test]
fn revit_2025_product_categories_match_revits_export() {
    let Some((rvt, ifc)) = re1_architecture() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no RE1-Architecture.rvt/.ifc");
        return;
    };
    let (step, reference) = export_and_reference(&rvt, &ifc);
    for (ours, theirs, expected) in [
        ("IFCFURNITURE", "IFCFURNISHINGELEMENT", 23),
        ("IFCSANITARYTERMINAL", "IFCFLOWTERMINAL", 7),
        ("IFCBUILDINGELEMENTPROXY", "IFCBUILDINGELEMENTPROXY", 9),
        ("IFCCOVERING", "IFCCOVERING", 6),
        ("IFCMEMBER", "IFCMEMBER", 10),
        ("IFCPLATE", "IFCPLATE", 2),
        ("IFCRAILING", "IFCRAILING", 1),
    ] {
        let reference_tags = tags(&reference, &[theirs]);
        assert_eq!(reference_tags.len(), expected, "reference {theirs}");
        assert_eq!(tags(&step, &[ours]), reference_tags, "{ours} ElementIds");
    }
}

/// RE-33 on the RE1 MEP models: every duct, duct fitting, pipe and pipe
/// fitting rvt-rs exports is one Revit exported. Most of them sit in
/// second-prologue frames whose ElementId RE-34 infers from the reference
/// order; the rest stay counted as unattributed. The recall is pinned so
/// a change to it is measured rather than silent.
#[test]
fn revit_2025_ducts_and_pipes_are_revits_own() {
    let mut checked = 0;
    for (model, ducts, duct_fittings, pipes, pipe_fittings) in
        [("Mechanical", 21, 18, 6, 3), ("Plumbing", 0, 0, 16, 23)]
    {
        let Some((rvt, ifc)) = re1(model) else {
            eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no RE1-{model}.rvt/.ifc");
            continue;
        };
        let (step, reference) = export_and_reference(&rvt, &ifc);
        let segments = tags(&reference, &["IFCFLOWSEGMENT"]);
        let fittings = tags(&reference, &["IFCFLOWFITTING"]);
        for (ours, reference_set, expected) in [
            ("IFCDUCTSEGMENT", &segments, ducts),
            ("IFCPIPESEGMENT", &segments, pipes),
            ("IFCDUCTFITTING", &fittings, duct_fittings),
            ("IFCPIPEFITTING", &fittings, pipe_fittings),
        ] {
            let exported = tags(&step, &[ours]);
            assert_eq!(exported.len(), expected, "{model} {ours} count");
            assert!(
                exported.is_subset(reference_set),
                "{model} {ours}: {:?} not in Revit's export",
                exported.difference(reference_set).collect::<Vec<_>>()
            );
        }
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no RE1 MEP model under RVT_PROJECT_CORPUS_DIR");
    }
}
