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

/// Real saved-file contract for the default CLI path. The rectangular files
/// and curved files are in the pinned family-revolutions fixture package.
/// A configured but incomplete fixture folder is an error, never a skip.
#[test]
fn standalone_revolutions_reach_the_default_ifc_export() {
    let mut checked = 0;
    for (variable, prefix) in [
        ("RVT_RECTANGULAR_REVOLUTIONS_DIR", "REV"),
        ("RVT_CURVED_REVOLUTIONS_DIR", "ARC"),
    ] {
        let Some(folder) = std::env::var_os(variable).map(std::path::PathBuf::from) else {
            eprintln!("skipping {prefix} export fixtures: {variable} unset");
            continue;
        };
        for suffix in ["A", "B", "BT"] {
            let source = folder.join(format!("{prefix}-{suffix}.rfa"));
            assert!(source.is_file(), "missing fixture {}", source.display());
            let original = std::fs::read(&source).unwrap();
            let dir = temp_diagnostics_dir("rvt-native-revolution");
            let output = dir.join("body.ifc");
            let diagnostics = dir.join("body.json");
            let run = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
                .args([
                    source.as_os_str(),
                    "-o".as_ref(),
                    output.as_os_str(),
                    "--diagnostics".as_ref(),
                    diagnostics.as_os_str(),
                ])
                .env("SOURCE_DATE_EPOCH", "0")
                .output()
                .unwrap();
            assert!(
                run.status.success(),
                "{}",
                String::from_utf8_lossy(&run.stderr)
            );
            let step = std::fs::read_to_string(&output).unwrap();
            if prefix == "ARC" && suffix != "A" {
                let report: Value =
                    serde_json::from_slice(&std::fs::read(&diagnostics).unwrap()).unwrap();
                assert!(!step.contains("IFCREVOLVEDAREASOLID"));
                assert_eq!(report["exported"]["building_elements_with_geometry"], 0);
                assert_eq!(report["confidence"]["has_geometry"], false);
                assert!(String::from_utf8_lossy(&run.stderr).contains(
                    "partial revolution with an arc profile lacks native export confirmation"
                ));
                assert_eq!(std::fs::read(&source).unwrap(), original, "source modified");
                std::fs::remove_dir_all(dir).unwrap();
                checked += 1;
                continue;
            }
            assert_eq!(step.matches("=IFCREVOLVEDAREASOLID(").count(), 1);
            assert!(!step.contains("IFCTRIANGULATEDFACESET"));
            assert!(step.len() < 100_000, "analytic solid unexpectedly large");
            if prefix == "ARC" {
                assert_eq!(step.matches("IFCARCINDEX(").count(), 2);
            }
            let report: Value =
                serde_json::from_slice(&std::fs::read(&diagnostics).unwrap()).unwrap();
            assert_eq!(report["exported"]["building_elements_with_geometry"], 1);
            assert_eq!(report["exported"]["body_sources"]["family_revolution"], 1);
            assert_eq!(report["confidence"]["has_geometry"], true);
            assert_eq!(report["confidence"]["has_typed_elements"], false);
            assert!(
                report["warnings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|warning| {
                        warning != "No building elements were exported; output is scaffold-only."
                    })
            );
            // The geometry-only finding must not bypass mapped-type quality
            // modes, nor add bodies to an explicitly requested placeholder.
            for mode in ["typed-no-geometry", "geometry", "strict"] {
                let path = dir.join(format!("{mode}.ifc"));
                let run = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
                    .arg(&source)
                    .arg("-o")
                    .arg(&path)
                    .args(["--mode", mode])
                    .output()
                    .unwrap();
                assert!(!run.status.success(), "unclassified form passed {mode}");
                assert!(!path.exists(), "quality refusal wrote an IFC");
            }
            let placeholder = dir.join("placeholder.ifc");
            let run = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
                .arg(&source)
                .arg("-o")
                .arg(&placeholder)
                .arg("--placeholder")
                .output()
                .unwrap();
            assert!(run.status.success());
            assert!(
                !std::fs::read_to_string(placeholder)
                    .unwrap()
                    .contains("IFCREVOLVEDAREASOLID")
            );
            let mut file = rvt::RevitFile::open(&source).unwrap();
            let export =
                rvt::ifc::run_export(&mut file, &rvt::ifc::ExportOptions::default()).unwrap();
            let shape = export
                .model
                .entities
                .iter()
                .find_map(|entity| match entity {
                    rvt::ifc::entities::IfcEntity::BuildingElement {
                        solid_shape: Some(shape),
                        ..
                    } => Some(shape),
                    _ => None,
                })
                .unwrap();
            let display = rvt::ifc::body_geometry::solid_mesh(shape)
                .expect("analytic solid needs a display mesh");
            assert!(!display.triangles.is_empty());
            assert!(display.vertices.iter().flatten().all(|v| v.is_finite()));
            assert_eq!(std::fs::read(source).unwrap(), original, "source modified");
            std::fs::remove_dir_all(dir).unwrap();
            checked += 1;
        }
    }
    eprintln!("checked {checked} native revolution exports and their mode refusals");
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
    diagnostics_with(model, dir, name, anonymize, &[])
}

/// [`diagnostics_of`] with more `rvt-ifc` arguments.
fn diagnostics_with(
    model: &std::path::Path,
    dir: &std::path::Path,
    name: &str,
    anonymize: bool,
    extra: &[&str],
) -> Value {
    let path = dir.join(name);
    let mut command = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"));
    command
        .arg(model)
        .arg("-o")
        .arg(dir.join(format!("{name}.ifc")))
        .arg("--diagnostics")
        .arg(&path)
        .args(extra);
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

/// The entries of the diagnostics' log of decisions that are of `reason`, or,
/// with `matching` false, those that are not.
fn log_entries(diagnostics: &Value, reason: &str, matching: bool) -> Vec<Value> {
    diagnostics["export_decisions"]["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|entry| (entry["reason"] == reason) == matching)
        .cloned()
        .collect()
}

/// #1326: `--anonymize` scrubs the log of decisions as it scrubs `skipped`.
/// RE1 Architecture has two phases, Existing and New Construction (the last);
/// exporting the first leaves its 11 rooms out, and the plain log names the
/// second as the detail of each of their entries. The anonymized log keeps
/// every entry and every count, and none of the phase names, project names,
/// level and material names or element names that the plain diagnostics hold.
#[test]
fn anonymized_decision_log_hides_the_phases_and_every_name() {
    let Some(corpus) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!(
            "skipping the decision log --anonymize assertion: RVT_PROJECT_CORPUS_DIR is unset"
        );
        return;
    };
    let model = std::path::PathBuf::from(corpus).join("RE1-Architecture.rvt");
    if !model.exists() {
        eprintln!(
            "skipping the decision log --anonymize assertion: {} is missing",
            model.display()
        );
        return;
    }
    let dir = temp_diagnostics_dir("rvt-rs-anonymize-log");
    let default_args = ["--export-decisions"];
    let first_phase_args = ["--phase", "Existing", "--export-decisions"];
    let plain_default = diagnostics_with(&model, &dir, "plain-default.json", false, &default_args);
    let plain_phase = diagnostics_with(&model, &dir, "plain-phase.json", false, &first_phase_args);

    // What the plain diagnostics name, so the check below has something to find.
    let rooms = log_entries(&plain_phase, "room_not_in_export_phase", true);
    assert_eq!(rooms.len(), 11, "the first phase leaves the 11 rooms out");
    assert!(
        rooms
            .iter()
            .all(|entry| entry["detail"] == "New Construction"),
        "the plain log names the room's phase: {rooms:?}"
    );
    let sample_names: usize = plain_default["skipped"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|item| item["sample_names"].as_array().map_or(0, Vec::len))
        .sum();
    assert!(sample_names > 0, "the plain diagnostics name no element");
    let mut names = project_names(&plain_default);
    names.extend(project_names(&plain_phase));
    names.extend(["New Construction".to_string(), "Existing".to_string()]);
    names.push("RE1-Architecture".to_string());

    for (tag, plain, args) in [
        ("default", &plain_default, &default_args[..]),
        ("phase", &plain_phase, &first_phase_args[..]),
    ] {
        let anonymized =
            diagnostics_with(&model, &dir, &format!("anonymized-{tag}.json"), true, args);
        for section in ["export_decisions", "skipped"] {
            assert_eq!(
                leaked(&anonymized[section], &names),
                None,
                "{tag}: the anonymized {section} still name the project"
            );
        }
        assert_eq!(
            plain["export_decisions"]["reasons"], anonymized["export_decisions"]["reasons"],
            "{tag}: the counts by reason"
        );
        assert_eq!(
            log_entries(plain, "room_not_in_export_phase", false),
            log_entries(&anonymized, "room_not_in_export_phase", false),
            "{tag}: the entries that name nothing are unchanged"
        );
        let plain_rooms = log_entries(plain, "room_not_in_export_phase", true);
        let scrubbed = log_entries(&anonymized, "room_not_in_export_phase", true);
        assert_eq!(scrubbed.len(), plain_rooms.len(), "{tag}: the room entries");
        assert!(
            scrubbed.iter().all(|entry| entry["detail"] == "<redacted>"),
            "{tag}: {scrubbed:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}
