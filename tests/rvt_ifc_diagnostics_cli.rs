mod common;

use common::{sample_for_year, samples_dir};
use serde_json::Value;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn corpus_available() -> bool {
    sample_for_year(2024).exists()
}

fn temp_diagnostics_dir(prefix: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nonce}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp diagnostics dir");
    dir
}

#[test]
fn rvt_ifc_cli_writes_diagnostics_sidecar() {
    if !corpus_available() {
        eprintln!(
            "skipping rvt-ifc diagnostics CLI assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }

    let dir = temp_diagnostics_dir("rvt-rs-ifc-diagnostics");
    let ifc_path = dir.join("sample.ifc");
    let diagnostics_path = dir.join("sample.diagnostics.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(sample_for_year(2024))
        .arg("-o")
        .arg(&ifc_path)
        .arg("--diagnostics")
        .arg(&diagnostics_path)
        .output()
        .expect("run rvt-ifc");
    assert!(
        output.status.success(),
        "rvt-ifc failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(ifc_path.exists(), "IFC output should exist");
    assert!(
        diagnostics_path.exists(),
        "diagnostics sidecar should exist"
    );
    let json: Value = serde_json::from_slice(
        &std::fs::read(&diagnostics_path).expect("read diagnostics sidecar"),
    )
    .expect("parse diagnostics JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["mode"], "default");
    assert!(
        json["input"]["has_formats_latest"]
            .as_bool()
            .unwrap_or(false)
    );
    assert!(
        json["input"]["has_global_latest"]
            .as_bool()
            .unwrap_or(false)
    );
    assert!(json["exported"]["total_entities"].as_u64().unwrap_or(0) >= 1);
    assert!(json["confidence"]["level"].is_string());

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rvt_ifc_cli_strict_mode_fails_before_writing_ifc_but_keeps_diagnostics() {
    if !corpus_available() {
        eprintln!(
            "skipping rvt-ifc strict mode assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }

    let dir = temp_diagnostics_dir("rvt-rs-ifc-strict");
    let ifc_path = dir.join("strict.ifc");
    let diagnostics_path = dir.join("strict.diagnostics.json");

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(sample_for_year(2024))
        .arg("-o")
        .arg(&ifc_path)
        .arg("--mode")
        .arg("strict")
        .arg("--diagnostics")
        .arg(&diagnostics_path)
        .output()
        .expect("run rvt-ifc strict");

    assert!(
        !output.status.success(),
        "strict mode should reject an incomplete export\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("IFC export mode `strict`"),
        "strict failure should explain the rejected mode\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !ifc_path.exists(),
        "strict mode must not write an IFC that failed the quality gate"
    );
    assert!(
        diagnostics_path.exists(),
        "strict mode should still write requested diagnostics"
    );

    let json: Value = serde_json::from_slice(
        &std::fs::read(&diagnostics_path).expect("read strict diagnostics sidecar"),
    )
    .expect("parse strict diagnostics JSON");
    assert_eq!(json["schema_version"], 1);
    // Strict rejects this export because it carries no element geometry.
    // Which below-geometry level it reports moves as decoders improve:
    // `scaffold` when this test was written, `typed_no_geometry` once the
    // sample family's typed elements began to export. The test pinned the
    // old value for months unseen because no CI job ran this file.
    let level = json["confidence"]["level"].as_str().unwrap_or_default();
    assert!(
        [
            "scaffold",
            "typed_no_geometry",
            "diagnostic_partial",
            "proxy_only"
        ]
        .contains(&level),
        "strict rejected an export at confidence level {level:?}; expected a below-geometry level"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Every name `--anonymize` must hide: the file stem, the project name, the
/// levels, the material sample and the skipped elements' sample names, as
/// the plain diagnostics give them. Names that already read as placeholders
/// (`Level 1`) are left out.
fn project_names(diagnostics: &Value) -> Vec<String> {
    let mut names = Vec::new();
    for key in ["original_path", "project_name"] {
        if let Some(name) = diagnostics["input"][key].as_str() {
            names.push(
                name.rsplit_once('.')
                    .map_or(name, |(stem, _)| stem)
                    .to_string(),
            );
        }
    }
    for key in ["storey_names", "material_names_sample"] {
        for name in diagnostics["exported"][key]
            .as_array()
            .into_iter()
            .flatten()
        {
            names.extend(name.as_str().map(str::to_string));
        }
    }
    for item in diagnostics["skipped"].as_array().into_iter().flatten() {
        for name in item["sample_names"].as_array().into_iter().flatten() {
            names.extend(name.as_str().map(str::to_string));
        }
    }
    let placeholder = |name: &str| {
        ["Level ", "Material ", "element-"].iter().any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| rest.chars().all(|c| c.is_ascii_digit()))
        })
    };
    names.retain(|name| name.chars().count() >= 3 && !placeholder(name));
    names
}

/// Every string value in `value`, keys left out.
fn string_values(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(text) => out.push(text.clone()),
        Value::Array(items) => items.iter().for_each(|item| string_values(item, out)),
        Value::Object(map) => map.values().for_each(|item| string_values(item, out)),
        _ => {}
    }
}

/// The first of `names` that a string value of `value` contains.
fn leaked<'a>(value: &Value, names: &'a [String]) -> Option<&'a String> {
    let mut texts = Vec::new();
    string_values(value, &mut texts);
    names
        .iter()
        .find(|name| texts.iter().any(|text| text.contains(name.as_str())))
}

#[test]
fn anonymized_diagnostics_and_inspect_report_hide_every_project_name() {
    if !corpus_available() {
        eprintln!(
            "skipping rvt-ifc --anonymize assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }
    let sample = sample_for_year(2024);
    let stem = sample
        .file_stem()
        .and_then(|stem| stem.to_str())
        .expect("sample file stem")
        .to_string();
    let dir = temp_diagnostics_dir("rvt-rs-anonymize");
    let run_ifc = |name: &str, anonymize: bool| {
        let path = dir.join(name);
        let mut command = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"));
        command
            .arg(&sample)
            .arg("-o")
            .arg(dir.join(format!("{name}.ifc")))
            .arg("--diagnostics")
            .arg(&path);
        if anonymize {
            command.arg("--anonymize");
        }
        let output = command.output().expect("run rvt-ifc");
        assert!(
            output.status.success(),
            "rvt-ifc failed\nstderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        std::fs::read_to_string(&path).expect("read diagnostics")
    };
    let plain: Value = serde_json::from_str(&run_ifc("plain.json", false)).expect("parse");
    let anonymized: Value = serde_json::from_str(&run_ifc("anonymized.json", true)).expect("parse");
    let mut names = project_names(&plain);
    names.push(stem);
    assert_eq!(
        leaked(&anonymized, &names),
        None,
        "anonymized diagnostics still name the project"
    );
    assert_eq!(
        plain["exported"]["by_ifc_type"],
        anonymized["exported"]["by_ifc_type"]
    );
    assert_eq!(
        plain["exported"]["material_names_sample"]
            .as_array()
            .map(Vec::len),
        anonymized["exported"]["material_names_sample"]
            .as_array()
            .map(Vec::len)
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-inspect"))
        .arg(&sample)
        .arg("--json")
        .arg("--anonymize")
        .output()
        .expect("run rvt-inspect");
    assert!(output.status.success(), "rvt-inspect --anonymize failed");
    let report: Value = serde_json::from_slice(&output.stdout).expect("parse inspect report");
    assert_eq!(
        leaked(&report, &names),
        None,
        "anonymized inspect report still names the project"
    );
    assert_eq!(report["file"]["input_path"], "<redacted>.rfa");

    let _ = std::fs::remove_dir_all(&dir);
}

/// rvt-ifc's diagnostics for `model`, with or without `--anonymize`.
fn diagnostics_of(
    model: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    anonymize: bool,
) -> Value {
    let path = dir.join(name);
    let mut command = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"));
    command
        .arg(model)
        .arg("-o")
        .arg(dir.join(format!("{name}.ifc")))
        .arg("--diagnostics")
        .arg(&path);
    if anonymize {
        command.arg("--anonymize");
    }
    let output = command.output().expect("run rvt-ifc");
    assert!(
        output.status.success(),
        "rvt-ifc failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_str(&std::fs::read_to_string(&path).expect("read diagnostics"))
        .expect("parse diagnostics")
}

/// The family test above on a project: Core Interior names its project,
/// fifteen Levels, materials and skipped elements, none of which the
/// anonymized diagnostics or inspect report may carry.
#[test]
fn anonymized_project_diagnostics_and_inspect_report_hide_every_project_name() {
    let Some(corpus) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!("skipping project --anonymize assertion: RVT_PROJECT_CORPUS_DIR is unset");
        return;
    };
    let model = std::path::PathBuf::from(corpus).join("2024_Core_Interior.rvt");
    if !model.exists() {
        eprintln!(
            "skipping project --anonymize assertion: {} is missing",
            model.display()
        );
        return;
    }
    let dir = temp_diagnostics_dir("rvt-rs-anonymize-project");
    let plain = diagnostics_of(&model, &dir, "plain.json", false);
    let anonymized = diagnostics_of(&model, &dir, "anonymized.json", true);
    let mut names = project_names(&plain);
    names.push("2024_Core_Interior".into());
    assert!(
        names.len() > 10,
        "the plain diagnostics should name the project's Levels and materials: {names:?}"
    );
    assert_eq!(
        leaked(&anonymized, &names),
        None,
        "anonymized project diagnostics still name the project"
    );
    assert_eq!(
        plain["exported"]["by_ifc_type"],
        anonymized["exported"]["by_ifc_type"]
    );

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-inspect"))
        .arg(&model)
        .arg("--json")
        .arg("--anonymize")
        .output()
        .expect("run rvt-inspect");
    assert!(output.status.success(), "rvt-inspect --anonymize failed");
    let report: Value = serde_json::from_slice(&output.stdout).expect("parse inspect report");
    assert_eq!(
        leaked(&report, &names),
        None,
        "anonymized project inspect report still names the project"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
