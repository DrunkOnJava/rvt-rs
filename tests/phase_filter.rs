//! RE-184 (#328): an element that does not exist in the export phase is not
//! exported, as Revit's export leaves it out.
//!
//! `01_Walls_Phases` has three phases (Existing, New Construction, Phase 3)
//! and five walls, one created in Phase 3; Revit's export, with no phase
//! chosen, holds all five. `01_Walls_Phases_Demo` is the same model with wall
//! 2892 demolished in Phase 3, and Revit's export leaves that wall out. For
//! each, the walls rvt-rs writes (by `Tag`) must be exactly Revit's, and the
//! diagnostics must count each wall left out by phase.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR/r2025/`: `<model>.rvt` with
//! `<model>.ifc`. Skips what is absent.
//!
//! CI does not run this test yet. The two models and Revit's exports of them
//! are not published or pinned, so no CI job has them, and the test skips
//! there and passes. It runs only where both models and their exports are
//! under `RVT_PROJECT_CORPUS_DIR/r2025/`, until they are published and
//! fetched (`reports/element-framing/RE-184-phase-order.md`, section 6).

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The `Tag` of every `IFCWALL` or `IFCWALLSTANDARDCASE` in a STEP file.
fn wall_tags(step: &str) -> BTreeSet<String> {
    step.lines()
        .filter_map(|line| {
            let body = line.split_once('=')?.1.trim_start();
            let args = body
                .strip_prefix("IFCWALL(")
                .or_else(|| body.strip_prefix("IFCWALLSTANDARDCASE("))?;
            // GlobalId, OwnerHistory, Name, Description, ObjectType,
            // ObjectPlacement, Representation, Tag: the Tag is the 8th
            // argument, and a name may hold commas only inside quotes.
            let mut fields = Vec::new();
            let (mut current, mut quoted) = (String::new(), false);
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
            Some(fields.get(7)?.trim().trim_matches('\'').to_string())
        })
        .collect()
}

#[test]
fn walls_not_in_the_export_phase_are_left_out() {
    let Some(dir) =
        std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(|d| PathBuf::from(d).join("r2025"))
    else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    for (model, walls, left_out) in [("01_Walls_Phases", 5, 0), ("01_Walls_Phases_Demo", 4, 1)] {
        let rvt = dir.join(format!("{model}.rvt"));
        let reference_ifc = dir.join(format!("{model}.ifc"));
        if !rvt.exists() || !reference_ifc.exists() {
            eprintln!("skipping {model}: model or reference export absent");
            continue;
        }
        let theirs = wall_tags(&std::fs::read_to_string(&reference_ifc).expect("reference IFC"));
        assert_eq!(theirs.len(), walls, "{model}: walls in Revit's export");
        let mut rf = RevitFile::open(&rvt).expect("open");
        let result = RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .expect("export");
        let ours = wall_tags(&write_step(&result.model));
        assert_eq!(ours, theirs, "{model}: walls rvt-rs writes against Revit's");
        let counted: usize = result
            .diagnostics
            .skipped
            .iter()
            .filter(|item| item.reason == "element_record_not_in_export_phase")
            .map(|item| item.count)
            .sum();
        assert_eq!(
            counted, left_out,
            "{model}: walls counted as not in the export phase"
        );
    }
}
