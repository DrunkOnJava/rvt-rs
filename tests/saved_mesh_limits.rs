//! Public Autodesk 2023 sample projects listed in
//! research/autodesk-sample-projects.tsv. Put the RVTs in RVT_SAMPLE_PROJECTS_DIR
//! with their year-prefixed names.

use rvt::control::{CancelToken, Stage, WalkerControl};
use rvt::ifc::saved_meshes::{
    SavedMeshLimits, SavedMeshReport, SavedMeshScope, attach_scoped_with_control,
    attach_scoped_with_limits_and_control,
};
use rvt::ifc::{
    ExportNote, ExportOptions, ExportQualityMode, IfcModel, SavedMeshChoice, run_export,
};
use rvt::{Error, RevitFile};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, process::Command};

fn sample_path(name: &str) -> Option<PathBuf> {
    let Some(directory) = std::env::var_os("RVT_SAMPLE_PROJECTS_DIR") else {
        eprintln!("skipping: RVT_SAMPLE_PROJECTS_DIR is unset");
        return None;
    };
    let path = PathBuf::from(directory).join(name);
    if !path.is_file() {
        eprintln!("skipping: public sample {name} is unavailable");
        return None;
    }
    Some(path)
}

fn sample(name: &str) -> Option<(RevitFile, IfcModel)> {
    let mut file = RevitFile::open(sample_path(name)?).expect("open public sample");
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
    // The sample's largest Saved graph holds 20,292 values: one under it leaves
    // that graph's bodies to the normal refusal and keeps the others; at it,
    // the attach is the default one. What is held is how the counts relate,
    // not the counts, which move with the saved-mesh decoder.
    let incomplete = attach(&mut file, &body, scope, limit(20_291));
    let complete = attach(&mut file, &body, scope, limit(20_292));
    assert!(
        incomplete.1.replaced > 0,
        "a graph under the limit still attaches"
    );
    assert!(
        incomplete.1.replaced < complete.1.replaced,
        "a graph over the limit is refused"
    );
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
    for scope in [SavedMeshScope::All, SavedMeshScope::DoorsAndWindows] {
        let refused = attach(&mut file, &body, scope, low);
        assert_eq!(refused.1.replaced, 0);
        assert!(
            serde_json::to_string(&refused.0)
                .unwrap()
                .contains("native field value budget exceeded"),
            "expected the ordinary Saved refusal"
        );
        let legacy = legacy(&mut file, &body, scope);
        assert!(legacy.1.replaced > 0, "the default attach replaces bodies");
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
    assert_same_attach(&retried, &legacy(&mut file, &body, scope));
}

#[test]
fn public_structural_whole_export_applies_the_saved_graph_limit() {
    let Some(path) = sample_path("2023-rstbasicsampleproject.rvt") else {
        return;
    };
    let export = |options: &ExportOptions| {
        let mut file = RevitFile::open(&path).expect("open public sample");
        let run = run_export(&mut file, options).expect("export with Saved graph limits");
        assert!(run.unread.is_empty(), "public sample export is incomplete");
        run
    };
    let default_options = ExportOptions::default();
    let limits =
        |max| SavedMeshLimits::default().with_max_graph_values(NonZeroUsize::new(max).unwrap());
    assert_eq!(default_options.saved_mesh_limits, limits(1_000_000));
    let with_limit = |max| default_options.clone().with_saved_mesh_limits(limits(max));
    let default = export(&default_options);
    let replaced = |run: &rvt::ifc::ExportRun| {
        run.notes.iter().find_map(|note| match note {
            ExportNote::SavedMeshes { replaced, .. } => Some(*replaced),
            _ => None,
        })
    };
    let under_limit = export(&with_limit(20_291));
    let at_limit = export(&with_limit(20_292));
    let incomplete = replaced(&under_limit).expect("Saved attach report at the lower limit");
    let complete = replaced(&at_limit).expect("Saved attach report at the boundary");
    assert!(incomplete > 0 && incomplete < complete);
    assert_eq!(Some(complete), replaced(&default));
    assert!(
        serde_json::to_vec(&at_limit.model).unwrap() == serde_json::to_vec(&default.model).unwrap(),
        "the explicit admitting limit must reproduce the default model"
    );
    let disabled = export(&with_limit(20_292).with_saved_meshes(SavedMeshChoice::Off));
    assert_eq!(replaced(&disabled), None);
}

#[test]
fn public_structural_ifc_cli_uses_the_default_and_explicit_saved_limits() {
    let Some(path) = sample_path("2023-rstbasicsampleproject.rvt") else {
        return;
    };
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let folder = std::env::temp_dir().join(format!(
        "rvt-saved-graph-cli-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&folder).expect("create the Saved graph CLI output directory");
    let export = |label: &str, limit: Option<&str>| {
        let ifc = folder.join(format!("{label}.ifc"));
        let diagnostics = folder.join(format!("{label}.json"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"));
        command
            .arg(&path)
            .arg("-o")
            .arg(&ifc)
            .arg("--diagnostics")
            .arg(&diagnostics)
            .env("SOURCE_DATE_EPOCH", "0");
        if let Some(limit) = limit {
            command.args(["--saved-mesh-graph-values", limit]);
        }
        let output = command.output().expect("run rvt-ifc");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "{stderr}");
        (fs::read(ifc).unwrap(), fs::read(diagnostics).unwrap())
    };
    let default = export("default", None);
    let explicit_default = export("explicit-default", Some("1000000"));
    let below_boundary = export("below-boundary", Some("20291"));
    let at_boundary = export("at-boundary", Some("20292"));
    fs::remove_dir_all(folder).expect("remove the Saved graph CLI test outputs");
    let saved_count = |bytes: &[u8]| {
        let diagnostics: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        diagnostics
            .pointer("/exported/body_sources/saved_graphics_mesh")
            .and_then(serde_json::Value::as_u64)
            .expect("count saved graphics in diagnostic export")
    };
    let incomplete = saved_count(&below_boundary.1);
    assert!(incomplete > 0 && incomplete < saved_count(&at_boundary.1));

    assert!(
        default == explicit_default,
        "the documented default changes the export"
    );
    assert!(
        default == at_boundary,
        "the admitting limit changes the export"
    );
    assert!(
        default.0 != below_boundary.0,
        "the graph refusal must change the IFC"
    );
    assert!(
        default.1 != below_boundary.1,
        "the graph refusal must change diagnostics"
    );
}

#[test]
fn ifc_cli_rejects_an_invalid_saved_graph_limit_before_opening_input() {
    for (value, conflicting_flag) in [
        ("0", None),
        ("1", Some("--no-saved-meshes")),
        ("1", Some("--placeholder")),
    ] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"));
        command.args(["not-opened.rvt", "--saved-mesh-graph-values", value]);
        if let Some(flag) = conflicting_flag {
            command.arg(flag);
        }
        let result = command.output().expect("parse rvt-ifc arguments");
        assert_eq!(result.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&result.stderr).contains("--saved-mesh-graph-values"));
    }
}
