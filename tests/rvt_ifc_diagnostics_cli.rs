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
    assert_eq!(json["confidence"]["level"], "scaffold");

    let _ = std::fs::remove_dir_all(&dir);
}
