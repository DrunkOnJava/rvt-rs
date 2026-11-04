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

use rvt::ifc::step_writer::{StepOptions, write_step_with_options};
use rvt::ifc::{ExportOptions, run_export};
use rvt::{RevitFile, research::native_document};
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

fn project(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?).join(name);
    path.exists().then_some(path)
}

/// What an export of `rf` writes and reports, comparable across handles: its
/// STEP text at a fixed time, and its diagnostics.
fn exported(rf: &mut RevitFile, phase: Option<&str>) -> (String, serde_json::Value) {
    let options = ExportOptions::default().with_phase(phase.map(str::to_string));
    let run = run_export(rf, &options).expect("export");
    let step = write_step_with_options(&run.model, &StepOptions { timestamp: Some(0) });
    (
        step,
        serde_json::to_value(&run.diagnostics).expect("diagnostics"),
    )
}

/// #1339: the production walk is memoised on the handle, and with it what the
/// partition recovery notes for the diagnostics (Areas left unwritten, family
/// spaces without a body). A second export of the same handle, which reads that
/// walk back, must write and report what a fresh handle does. This holds for
/// any model; it can only fail where a model has such notes, and whether
/// Einhoven or RE1 Architecture has any is not known here.
#[test]
fn a_handle_exported_again_reports_what_a_fresh_one_reports() {
    let mut checked = 0;
    for name in ["Revit_IFC5_Einhoven.rvt", "RE1-Architecture.rvt"] {
        let Some(path) = project(name) else {
            eprintln!("skipping {name}: absent");
            continue;
        };
        let fresh = exported(&mut RevitFile::open(&path).expect("open"), None);
        let mut reused = RevitFile::open(&path).expect("open");
        assert_eq!(exported(&mut reused, None), fresh, "{name}: first export");
        assert_eq!(exported(&mut reused, None), fresh, "{name}: second export");
        checked += 1;
    }
    eprintln!("{checked} model(s) exported twice");
}

/// #1339: one handle exported at one phase and then another gives what a
/// handle opened for that phase gives, in the IFC and in the diagnostics. RE1
/// Architecture's phases are Existing (12589) and New Construction (86961,
/// the last), and its 11 rooms sit in the second.
#[test]
fn a_handle_exported_at_one_phase_and_then_another_gives_what_a_fresh_handle_gives() {
    let Some(path) = project("RE1-Architecture.rvt") else {
        eprintln!("skipping: RE1-Architecture.rvt absent");
        return;
    };
    let open = || RevitFile::open(&path).expect("open");
    let default = exported(&mut open(), None);
    let existing = exported(&mut open(), Some("Existing"));
    assert!(default.0.contains("=IFCSPACE("), "rooms by default");
    assert!(!existing.0.contains("=IFCSPACE("), "no rooms in Existing");

    let mut rf = open();
    assert_eq!(exported(&mut rf, None), default);
    assert_eq!(rf.export_phase(), None);
    assert_eq!(exported(&mut rf, Some("Existing")), existing);
    assert_eq!(rf.export_phase(), Some(12589));
    // The phase in effect again keeps what the handle read.
    assert_eq!(exported(&mut rf, Some("12589")), existing);
    rf.clear_export_phase();
    assert_eq!(rf.export_phase(), None);
    assert_eq!(exported(&mut rf, None), default);
    assert_eq!(exported(&mut rf, Some("New Construction")), default);
    assert_eq!(rf.export_phase(), Some(86961));
}

fn nordic_model(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("RVT_NORDIC_LCA_DIR")?).join(name);
    path.exists().then_some(path)
}

/// #1339, RE-312, #1262: the walk, and the notes it makes for the diagnostics,
/// are memoised on the handle, so a second export of the same handle must
/// report what the first did. The BIM4LCA terrain office has 94 "Parking
/// Space" family instances written as `IfcSpace` by their "Export to IFC As":
/// Revit's export has no space for the 92 that draw no form, and rvt-rs leaves
/// out 89 of them and counts them as `element_record_family_space_without_body`.
/// That note is made only when the walk runs, so an export that wiped it before
/// reading the memoised walk lost the count while the model still lacked the
/// spaces. Runs against `RVT_NORDIC_LCA_DIR` (CI's tier-two job fetches the
/// file; `research/bim4lca-models.tsv`) and skips without it.
#[test]
fn a_handle_exported_twice_keeps_the_count_of_family_spaces_it_leaves_out() {
    let Some(path) = nordic_model("ARK_NordicLCA_Office_Terrain_Concrete_BuildingPermit.rvt")
    else {
        eprintln!("skipping: the terrain office is absent");
        return;
    };
    let left_out = |diagnostics: &serde_json::Value| -> u64 {
        diagnostics["skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .filter(|item| item["reason"] == "element_record_family_space_without_body")
            .map(|item| item["count"].as_u64().expect("count"))
            .sum()
    };
    // Spaces named for a bicycle bay, the ones the rule leaves out.
    let bicycle_bays = |step: &str| {
        step.lines()
            .filter(|line| line.contains("=IFCSPACE(") && line.contains("Parking Space:PP:"))
            .count()
    };
    let spaces = |step: &str| step.matches("=IFCSPACE(").count();

    let fresh = exported(&mut RevitFile::open(&path).expect("open"), None);
    let counted = left_out(&fresh.1);
    assert!(
        counted > 0,
        "the terrain office leaves family spaces out (RE-312 counted 89), got {counted}"
    );
    assert!(
        spaces(&fresh.0) < 94,
        "of the 94 parking instances, the model keeps fewer than all as spaces"
    );
    assert_eq!(bicycle_bays(&fresh.0), 0, "no bicycle bay is written");

    let mut reused = RevitFile::open(&path).expect("open");
    for export in ["first", "second"] {
        let run = exported(&mut reused, None);
        assert_eq!(
            left_out(&run.1),
            counted,
            "the {export} export counts the family spaces left out"
        );
        assert_eq!(bicycle_bays(&run.0), 0, "the {export} export writes no bay");
        assert_eq!(run, fresh, "the {export} export is a fresh handle's");
    }
}
