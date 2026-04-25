//! Known-count manifest checks for curated project corpus files.
//!
//! The manifests under `tests/fixtures/project-counts/` separate
//! authoritative counts, explicit unknowns, and current decoder baselines so
//! corpus gaps cannot be skipped accidentally.

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};

const REQUIRED_CATEGORIES: &[&str] = &[
    "levels",
    "walls",
    "floors",
    "roofs",
    "doors",
    "windows",
    "rooms_spaces",
    "columns",
    "beams",
    "mep",
    "materials",
    "units",
];

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project-counts")
}

fn manifest_paths() -> Vec<PathBuf> {
    let mut out: Vec<_> = std::fs::read_dir(fixture_dir())
        .expect("read project-count fixture dir")
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("json"))
        .collect();
    out.sort();
    out
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!("parse {}: {e}", path.display());
    })
}

fn obj<'a>(value: &'a Value, context: &str) -> &'a serde_json::Map<String, Value> {
    value
        .as_object()
        .unwrap_or_else(|| panic!("{context} must be a JSON object"))
}

fn str_field<'a>(value: &'a Value, key: &str, context: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{context}.{key} must be a string"))
}

fn opt_str_field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn int_field(value: &Value, key: &str, context: &str) -> i64 {
    value
        .get(key)
        .and_then(Value::as_i64)
        .unwrap_or_else(|| panic!("{context}.{key} must be an integer"))
}

fn assert_with_tolerance(context: &str, actual: usize, expected: i64, tolerance: i64) {
    let actual = actual as i64;
    let delta = (actual - expected).abs();
    assert!(
        delta <= tolerance,
        "{context}: actual {actual}, expected {expected} +/- {tolerance}"
    );
}

fn count_step_constructor(step: &str, ifc_type: &str) -> usize {
    step.matches(&format!("{ifc_type}(")).count()
}

fn metric_actual(metric: &str, diagnostics: &rvt::ifc::ExportDiagnostics, step: &str) -> usize {
    match metric {
        "diagnostics.exported.storey_count" => diagnostics.exported.storey_count,
        "diagnostics.exported.material_count" => diagnostics.exported.material_count,
        metric if metric.starts_with("diagnostics.exported.by_ifc_type.") => {
            let ifc_type = metric
                .strip_prefix("diagnostics.exported.by_ifc_type.")
                .expect("prefix checked");
            diagnostics
                .exported
                .by_ifc_type
                .get(ifc_type)
                .copied()
                .unwrap_or(0)
        }
        metric if metric.starts_with("step.") => {
            let ifc_type = metric.strip_prefix("step.").expect("prefix checked");
            count_step_constructor(step, ifc_type)
        }
        _ => panic!("unsupported decoder metric {metric}"),
    }
}

#[test]
fn project_count_manifests_are_complete_and_explicit() {
    let paths = manifest_paths();
    assert!(
        !paths.is_empty(),
        "tests/fixtures/project-counts/*.json must contain at least one manifest"
    );

    for path in paths {
        let manifest = read_json(&path);
        let context = path.display().to_string();
        assert_eq!(int_field(&manifest, "schema_version", &context), 1);
        let id = str_field(&manifest, "id", &context);
        assert!(!id.trim().is_empty(), "{context}.id must not be empty");
        str_field(&manifest, "project_file", &context);
        obj(
            manifest
                .get("source")
                .unwrap_or_else(|| panic!("{context}.source is required")),
            &format!("{context}.source"),
        );
        let counts = obj(
            manifest
                .get("counts")
                .unwrap_or_else(|| panic!("{context}.counts is required")),
            &format!("{context}.counts"),
        );

        for required in REQUIRED_CATEGORIES {
            assert!(
                counts.contains_key(*required),
                "{context}.counts must explicitly include {required}"
            );
        }

        for (category, count) in counts {
            let count_context = format!("{context}.counts.{category}");
            let status = str_field(count, "status", &count_context);
            match status {
                "known" => {
                    int_field(count, "expected", &count_context);
                    int_field(count, "tolerance", &count_context);
                    str_field(count, "source", &count_context);
                }
                "known_gap" => {
                    int_field(count, "expected", &count_context);
                    int_field(count, "tolerance", &count_context);
                    str_field(count, "source", &count_context);
                    int_field(count, "decoder_expected", &count_context);
                    int_field(count, "decoder_tolerance", &count_context);
                    int_field(count, "tracking_issue", &count_context);
                    str_field(count, "unsupported_feature", &count_context);
                }
                "decoder_baseline" => {
                    int_field(count, "expected", &count_context);
                    str_field(count, "source", &count_context);
                    str_field(count, "decoder_metric", &count_context);
                    int_field(count, "decoder_expected", &count_context);
                    int_field(count, "decoder_tolerance", &count_context);
                    int_field(count, "tracking_issue", &count_context);
                }
                "unknown" => {
                    let reason = str_field(count, "reason", &count_context);
                    assert!(
                        !reason.trim().is_empty(),
                        "{count_context}.reason must explain why the count is unknown"
                    );
                }
                _ => panic!("{count_context}.status has unsupported value {status}"),
            }
        }
    }
}

#[test]
fn project_count_manifests_match_available_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let corpus_dir = project_dir();
    if !corpus_dir.exists() {
        eprintln!(
            "skipping project-count fixture checks: corpus dir missing at {}",
            corpus_dir.display()
        );
        return Ok(());
    }

    let mut exercised = 0usize;
    for path in manifest_paths() {
        let manifest = read_json(&path);
        let id = str_field(&manifest, "id", &path.display().to_string()).to_string();
        let project_file = str_field(&manifest, "project_file", &id);
        let project_path = corpus_dir.join(project_file);
        if !project_path.exists() {
            eprintln!(
                "skipping project-count manifest {id}: project file missing at {}",
                project_path.display()
            );
            continue;
        }
        exercised += 1;

        let reference_ifc = match manifest.get("reference_ifc_file") {
            Some(Value::String(name)) => {
                let reference_path = corpus_dir.join(name);
                assert!(
                    reference_path.exists(),
                    "{id}: reference IFC missing at {}",
                    reference_path.display()
                );
                Some(std::fs::read_to_string(&reference_path)?)
            }
            Some(Value::Null) | None => None,
            _ => panic!("{id}.reference_ifc_file must be string or null"),
        };

        let mut rf = RevitFile::open(&project_path)?;
        let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
        let step = write_step(&result.model);
        let unsupported: BTreeSet<&str> = result
            .diagnostics
            .unsupported_features
            .iter()
            .map(String::as_str)
            .collect();

        let counts = obj(
            manifest.get("counts").expect("counts exists"),
            &format!("{id}.counts"),
        );
        for (category, count) in counts {
            let status = str_field(count, "status", &format!("{id}.{category}"));
            if status == "unknown" {
                continue;
            }

            if let (Some(reference), Some(ifc_type)) = (
                reference_ifc.as_ref(),
                opt_str_field(count, "source_ifc_type"),
            ) {
                let expected = int_field(count, "expected", &format!("{id}.{category}"));
                let tolerance = int_field(count, "tolerance", &format!("{id}.{category}"));
                let actual = count_step_constructor(reference, ifc_type);
                assert_with_tolerance(
                    &format!("{id}.{category} source {ifc_type}"),
                    actual,
                    expected,
                    tolerance,
                );
            }

            if let Some(metric) = opt_str_field(count, "decoder_metric") {
                let expected = int_field(count, "decoder_expected", &format!("{id}.{category}"));
                let tolerance = int_field(count, "decoder_tolerance", &format!("{id}.{category}"));
                let actual = metric_actual(metric, &result.diagnostics, &step);
                assert_with_tolerance(
                    &format!("{id}.{category} decoder {metric}"),
                    actual,
                    expected,
                    tolerance,
                );
            }

            if status == "known_gap" {
                let feature = str_field(count, "unsupported_feature", &format!("{id}.{category}"));
                assert!(
                    unsupported.contains(feature),
                    "{id}.{category}: expected diagnostics.unsupported_features to contain {feature}"
                );
            }
        }
    }

    assert!(
        exercised > 0,
        "project corpus dir {} existed but no project-count manifests matched available files",
        corpus_dir.display()
    );
    Ok(())
}
