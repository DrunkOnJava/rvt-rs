//! B46 (#328): an element's Phase Created and Phase Demolished on Revit 2025.
//!
//! An element's phases are its native fields `m_createdPhaseId` and
//! `m_demolishedPhaseId` (RE-160), which `native_document::extract` reads
//! from each element's own record. On the four RE1 models (Revit 2025) the
//! extraction must complete, and every family instance Revit's export holds
//! (matched by its `Tag`) must have been created in one of the file's
//! `ProjectPhase` elements and demolished in none (-1), as nothing in these
//! models is demolished.
//!
//! Runs against `RVT_PROJECT_CORPUS_DIR` (`RE1-*.rvt` with Revit's
//! `RE1-*.ifc`). Skips what is absent.

use rvt::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// The `Tag`s of the elements in a Revit IFC export.
fn exported_tags(step: &str) -> BTreeSet<u64> {
    step.lines()
        .filter(|line| line.starts_with('#'))
        .filter_map(|line| line.rsplit('\'').nth(1)?.parse().ok())
        .collect()
}

#[test]
fn re1_family_instances_carry_their_phases() {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return;
    };
    let mut checked = 0;
    for model in ["Architecture", "Mechanical", "Plumbing", "Electrical"] {
        let rvt = dir.join(format!("RE1-{model}.rvt"));
        let reference = dir.join(format!("RE1-{model}.ifc"));
        if !rvt.exists() || !reference.exists() {
            eprintln!("skipping {}: model or reference absent", rvt.display());
            continue;
        }
        let exported = exported_tags(&std::fs::read_to_string(&reference).expect("reference"));
        let mut rf = RevitFile::open(&rvt).expect("open");
        let mut phases_of_file = BTreeSet::new();
        let mut instances: BTreeMap<u64, (Option<i64>, Option<i64>)> = BTreeMap::new();
        native_document::extract(&mut rf, &native_document::Options::default(), |record| {
            let id = record.identity.element_id;
            match record.class_name.as_deref() {
                Some("ProjectPhase") => {
                    phases_of_file.insert(id as i64);
                }
                Some("FamilyInstance") if exported.contains(&id) => {
                    let roots = record
                        .saved_metadata
                        .as_ref()
                        .map(|metadata| &metadata.root_references);
                    instances.insert(
                        id,
                        (
                            roots.and_then(|r| r.get("m_createdPhaseId").copied()),
                            roots.and_then(|r| r.get("m_demolishedPhaseId").copied()),
                        ),
                    );
                }
                _ => {}
            }
            Ok(())
        })
        .unwrap_or_else(|error| panic!("RE1 {model}: extract: {error:#}"));
        assert!(
            !instances.is_empty(),
            "RE1 {model}: no exported family instance was read"
        );
        let wrong: Vec<_> = instances
            .iter()
            .filter(|(_, (created, demolished))| {
                !created.is_some_and(|phase| phases_of_file.contains(&phase))
                    || *demolished != Some(-1)
            })
            .take(10)
            .collect();
        assert!(
            wrong.is_empty(),
            "RE1 {model}: of {} exported family instances these lack a created phase among the \
             file's phases {phases_of_file:?}, or have a demolished one: {wrong:?}",
            instances.len()
        );
        eprintln!(
            "RE1 {model}: {} exported family instances, phases {phases_of_file:?}",
            instances.len()
        );
        checked += 1;
    }
    assert!(
        checked > 0 || !dir.join("RE1-Plumbing.rvt").exists(),
        "no RE1 model was checked"
    );
}
