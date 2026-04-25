mod common;

use common::{sample_for_year, samples_dir};
use serde_json::Value;
use std::process::Command;

fn corpus_available() -> bool {
    sample_for_year(2024).exists()
}

#[test]
fn rvt_inspect_json_reports_file_health_and_export_readiness() {
    if !corpus_available() {
        eprintln!(
            "skipping rvt-inspect JSON assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-inspect"))
        .arg(sample_for_year(2024))
        .arg("--json")
        .output()
        .expect("run rvt-inspect --json");
    assert!(
        output.status.success(),
        "rvt-inspect --json failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: Value = serde_json::from_slice(&output.stdout).expect("parse inspect JSON");
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["file"]["file_opened"], true);
    assert_eq!(json["file"]["supported_revit_version"], true);
    assert_eq!(json["file"]["revit_version"], 2024);
    assert!(
        json["file"]["schema_parsed"].as_bool().unwrap_or(false),
        "2024 sample schema should parse"
    );
    assert!(json["decoded"]["class_name_count"].as_u64().unwrap_or(0) > 0);
    assert!(
        json["export"]["summary"]
            .as_str()
            .unwrap_or("")
            .contains("IFC")
    );
    assert!(json["export_diagnostics"].is_object());
    assert!(json["next_steps"].is_array());
}

#[test]
fn rvt_inspect_text_is_nontechnical_and_actionable() {
    if !corpus_available() {
        eprintln!(
            "skipping rvt-inspect text assertion: family corpus missing at {}",
            samples_dir().display()
        );
        return;
    }

    let output = Command::new(env!("CARGO_BIN_EXE_rvt-inspect"))
        .arg(sample_for_year(2024))
        .output()
        .expect("run rvt-inspect");
    assert!(
        output.status.success(),
        "rvt-inspect failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("File health"));
    assert!(text.contains("Decoded coverage"));
    assert!(text.contains("IFC export readiness"));
    assert!(text.contains("Warnings"));
    assert!(text.contains("Next steps"));
}
