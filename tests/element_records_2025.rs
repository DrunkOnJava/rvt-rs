//! Revit 2025 partition element records against Revit's own export (RE-32).
//!
//! The oracle is `Drshelden/IFC-ECS` `data/RE1/` (MIT): Revit 2025
//! projects next to IFC exports Revit wrote from them. Fetch it with
//! `tools/fetch-corpus.sh` and point `RVT_PROJECT_CORPUS_DIR` at
//! `IFC-ECS/data/RE1`; without it the test skips. Walls and slabs must
//! reproduce the export's ElementId (`Tag`) sets exactly and rooms its
//! count. The doors and the curtain wall use the second prologue (RE-30);
//! their ElementIds come from their enclosing partition records (RE-35).

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

    // Seven walls, and a curtain wall that is `IfcCurtainWall` in both
    // exports since RE-46.
    let walls = tags(&reference, &["IFCWALL", "IFCWALLSTANDARDCASE"]);
    assert_eq!(walls.len(), 7);
    assert_eq!(tags(&step, &["IFCWALL"]), walls, "wall ElementIds");
    let curtain_walls = tags(&reference, &["IFCCURTAINWALL"]);
    assert_eq!(curtain_walls.len(), 1);
    assert_eq!(
        tags(&step, &["IFCCURTAINWALL"]),
        curtain_walls,
        "curtain wall ElementIds"
    );

    let slabs = tags(&reference, &["IFCSLAB"]);
    assert_eq!(slabs.len(), 2);
    assert_eq!(tags(&step, &["IFCSLAB"]), slabs, "slab ElementIds");

    assert_eq!(count(&reference, "IFCSPACE"), 11);
    assert_eq!(count(&step, "IFCSPACE"), 11, "rooms");

    // Every door Revit exported, plus 417199: a placed Single-Flush door of
    // another type in its own wall that Revit's export leaves out. Its
    // frame carries no design option or phase field that would say why
    // (RE-35 §5), so it is pinned as measured.
    let doors = tags(&reference, &["IFCDOOR"]);
    assert_eq!(doors.len(), 5);
    let exported_doors = tags(&step, &["IFCDOOR"]);
    assert!(exported_doors.is_superset(&doors), "door ElementIds");
    assert_eq!(
        exported_doors
            .difference(&doors)
            .copied()
            .collect::<Vec<_>>(),
        vec![417199]
    );

    // Nothing is left unattributed, so the readiness score is complete.
    assert!(
        !result
            .diagnostics
            .skipped
            .iter()
            .any(|item| item.reason == "element_record_without_element_id"),
        "no unattributed element records"
    );
    assert_eq!(result.diagnostics.confidence.unexported_element_records, 0);
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

/// RE-33 and RE-35 on the RE1 MEP models: the ducts and pipes rvt-rs
/// exports are exactly Revit's `IfcFlowSegment`s, and the duct and pipe
/// fittings exactly its `IfcFlowFitting`s (IFC2x3 references; rvt-rs writes
/// the IFC4 entities). Most of them sit in second-prologue frames whose
/// ElementId is their enclosing record's. Plumbing fixtures are every
/// `IfcFlowTerminal` Revit wrote on the Plumbing model plus one, 442378, a
/// floor-level fixture Revit's export leaves out (RE-35 §5).
#[test]
fn revit_2025_ducts_and_pipes_are_revits_own() {
    let mut checked = 0;
    for (model, ducts, duct_fittings, pipes, pipe_fittings) in
        [("Mechanical", 25, 32, 6, 3), ("Plumbing", 0, 0, 63, 54)]
    {
        let Some((rvt, ifc)) = re1(model) else {
            eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no RE1-{model}.rvt/.ifc");
            continue;
        };
        let (step, reference) = export_and_reference(&rvt, &ifc);
        for (ours, count) in [
            ("IFCDUCTSEGMENT", ducts),
            ("IFCPIPESEGMENT", pipes),
            ("IFCDUCTFITTING", duct_fittings),
            ("IFCPIPEFITTING", pipe_fittings),
        ] {
            assert_eq!(tags(&step, &[ours]).len(), count, "{model} {ours} count");
        }
        assert_eq!(
            tags(&step, &["IFCDUCTSEGMENT", "IFCPIPESEGMENT"]),
            tags(&reference, &["IFCFLOWSEGMENT"]),
            "{model} segment ElementIds"
        );
        assert_eq!(
            tags(&step, &["IFCDUCTFITTING", "IFCPIPEFITTING"]),
            tags(&reference, &["IFCFLOWFITTING"]),
            "{model} fitting ElementIds"
        );
        // RE-37: air terminals are Revit's `IfcFlowTerminal`s on the
        // Mechanical model, all 7 (IFC4 `IfcAirTerminal` here).
        if model == "Mechanical" {
            assert_eq!(
                tags(&step, &["IFCAIRTERMINAL"]),
                tags(&reference, &["IFCFLOWTERMINAL"]),
                "air terminal ElementIds"
            );
        }
        if model == "Plumbing" {
            let terminals = tags(&reference, &["IFCFLOWTERMINAL"]);
            let fixtures = tags(&step, &["IFCSANITARYTERMINAL"]);
            assert!(fixtures.is_superset(&terminals), "plumbing fixtures");
            assert_eq!(
                fixtures.difference(&terminals).copied().collect::<Vec<_>>(),
                vec![442378]
            );
        }
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no RE1 MEP model under RVT_PROJECT_CORPUS_DIR");
    }
}

/// RE-37 on RE1 Electrical: its 12 lighting fixtures, the only elements
/// rvt-rs recovers there, are exactly the lighting fixtures among Revit's
/// 14 `IfcFlowTerminal`s (the other two are data devices, not recovered).
#[test]
fn revit_2025_lighting_fixtures_are_revits_own() {
    let Some((rvt, ifc)) = re1("Electrical") else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no RE1-Electrical.rvt/.ifc");
        return;
    };
    let (step, reference) = export_and_reference(&rvt, &ifc);
    let fixtures = tags(&step, &["IFCLIGHTFIXTURE"]);
    assert_eq!(fixtures.len(), 12);
    let terminals = tags(&reference, &["IFCFLOWTERMINAL"]);
    assert_eq!(terminals.len(), 14);
    assert!(
        fixtures.is_subset(&terminals),
        "{:?} not in Revit's export",
        fixtures.difference(&terminals).collect::<Vec<_>>()
    );
    // Every entity carrying a numeric `Tag` is one of those fixtures
    // (IfcOwnerHistory's 8th attribute is its creation time, not a Tag).
    let entities: Vec<&str> = step
        .lines()
        .filter_map(|line| {
            line.split_once('=')?
                .1
                .split_once('(')
                .map(|(e, _)| e.trim())
        })
        .filter(|entity| *entity != "IFCOWNERHISTORY")
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(tags(&step, &entities), fixtures, "nothing else is exported");
}
