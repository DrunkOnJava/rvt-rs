//! Public Autodesk 2023 sample projects listed in
//! research/autodesk-sample-projects.tsv. Put the RVTs in RVT_SAMPLE_PROJECTS_DIR
//! with their year-prefixed names.

use rvt::control::{CancelToken, Stage, WalkerControl};
use rvt::ifc::saved_meshes::{
    SavedMeshLimits, SavedMeshReport, SavedMeshScope, attach_scoped_with_control,
    attach_scoped_with_limits_and_control,
};
use rvt::ifc::{ExportOptions, ExportQualityMode, IfcModel, SavedMeshChoice, run_export};
use rvt::{Error, RevitFile};
use std::num::NonZeroUsize;
use std::path::PathBuf;

fn sample(name: &str) -> Option<(RevitFile, IfcModel)> {
    let Some(directory) = std::env::var_os("RVT_SAMPLE_PROJECTS_DIR") else {
        eprintln!("skipping: RVT_SAMPLE_PROJECTS_DIR is unset");
        return None;
    };
    let path = PathBuf::from(directory).join(name);
    if !path.is_file() {
        eprintln!("skipping: public sample {name} is unavailable");
        return None;
    }
    let mut file = RevitFile::open(path).expect("open public sample");
    let base = run_export(
        &mut file,
        &ExportOptions::default()
            .with_mode(ExportQualityMode::Scaffold)
            .with_saved_meshes(SavedMeshChoice::Off),
    )
    .expect("export without Saved attach");
    assert!(base.unread.is_empty(), "public sample export is incomplete");
    Some((file, base.model))
}

fn attach(
    file: &mut RevitFile,
    body: &IfcModel,
    scope: SavedMeshScope,
    limits: SavedMeshLimits,
) -> (IfcModel, SavedMeshReport) {
    let mut model = body.clone();
    let report = attach_scoped_with_limits_and_control(
        file,
        &mut model,
        scope,
        limits,
        &WalkerControl::default(),
    )
    .expect("attach with limits");
    (model, report)
}

fn assert_same_attach(
    actual: &(IfcModel, SavedMeshReport),
    expected: &(IfcModel, SavedMeshReport),
) {
    assert_eq!(actual.1, expected.1, "attach report");
    assert!(
        serde_json::to_vec(&actual.0).unwrap() == serde_json::to_vec(&expected.0).unwrap(),
        "complete IFC model differs"
    );
}

fn legacy(
    file: &mut RevitFile,
    body: &IfcModel,
    scope: SavedMeshScope,
) -> (IfcModel, SavedMeshReport) {
    let mut model = body.clone();
    let report = attach_scoped_with_control(file, &mut model, scope, &WalkerControl::default())
        .expect("legacy attach");
    (model, report)
}

#[test]
fn public_structural_sample_obeys_the_graph_budget_boundary() {
    let Some((mut file, body)) = sample("2023-rstbasicsampleproject.rvt") else {
        return;
    };
    let scope = SavedMeshScope::All;
    let defaults = SavedMeshLimits::default();
    let limit = |values| defaults.with_max_graph_values(NonZeroUsize::new(values).unwrap());
    let incomplete = attach(&mut file, &body, scope, limit(20_291));
    assert_eq!(incomplete.1.replaced, 77);
    let complete = attach(&mut file, &body, scope, limit(20_292));
    assert_eq!(complete.1.replaced, 94);
    let legacy = legacy(&mut file, &body, scope);
    assert_same_attach(&complete, &legacy);
    assert_same_attach(&attach(&mut file, &body, scope, defaults), &legacy);
    assert_same_attach(&attach(&mut file, &body, scope, limit(20_291)), &incomplete);
}

#[test]
fn public_architectural_sample_replays_default_and_refused_requests() {
    let Some((mut file, body)) = sample("2023-racbasicsampleproject.rvt") else {
        return;
    };
    let defaults = SavedMeshLimits::default();
    let low = defaults.with_max_graph_values(NonZeroUsize::MIN);
    for (scope, replaced) in [
        (SavedMeshScope::All, 365),
        (SavedMeshScope::DoorsAndWindows, 33),
    ] {
        let refused = attach(&mut file, &body, scope, low);
        assert_eq!(refused.1.replaced, 0);
        assert!(
            serde_json::to_string(&refused.0)
                .unwrap()
                .contains("native field value budget exceeded"),
            "expected the ordinary Saved refusal"
        );
        let legacy = legacy(&mut file, &body, scope);
        assert_eq!(legacy.1.replaced, replaced);
        assert_same_attach(&attach(&mut file, &body, scope, defaults), &legacy);
        assert_same_attach(&attach(&mut file, &body, scope, low), &refused);
    }
}

#[test]
fn public_structural_sample_retries_after_saved_graphics_cancellation() {
    let Some((mut file, body)) = sample("2023-rstbasicsampleproject.rvt") else {
        return;
    };
    let scope = SavedMeshScope::All;
    let limits =
        SavedMeshLimits::default().with_max_graph_values(NonZeroUsize::new(20_292).unwrap());
    let token = CancelToken::new();
    let signal = token.clone();
    let control = WalkerControl::default()
        .with_cancel(token.clone())
        .with_progress(move |event| {
            if event.stage == Stage::SavedGraphics {
                signal.cancel();
            }
        });

    let error = attach_scoped_with_limits_and_control(
        &mut file,
        &mut body.clone(),
        scope,
        limits,
        &control,
    )
    .expect_err("cancelled attach");
    assert!(token.is_cancelled());
    assert!(matches!(
        error.downcast_ref::<Error>(),
        Some(Error::Cancelled)
    ));
    let retried = attach(&mut file, &body, scope, limits);
    assert_eq!(retried.1.replaced, 94);
    assert_same_attach(&retried, &legacy(&mut file, &body, scope));
}
