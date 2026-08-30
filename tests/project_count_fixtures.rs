//! Known-count manifest checks for curated project corpus files.
//!
//! The manifests under `tests/fixtures/project-counts/` separate
//! authoritative counts, explicit unknowns, and current decoder baselines so
//! corpus gaps cannot be skipped accidentally.
//!
//! Tier-one (`tier: 1` / `tier1-*`) manifests resolve against the in-repo
//! `corpus/tier1/` tree (override with `RVT_CORPUS_TIER1_DIR`) and may use
//! `fixture_metric: "class_instances.<Class>"` for synthetic gen-fixture
//! inventories. Tier-two manifests resolve against `RVT_PROJECT_CORPUS_DIR`.
//! Setting that variable is an explicit request: a missing directory or zero
//! matching tier-two manifests then fails loudly instead of skipping, so a
//! mistyped corpus path cannot silently drop tier-two coverage.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use rvt::RevitFile;
use rvt::compression;
use rvt::ifc::{RvtDocExporter, write_step};
use rvt::streams;

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

fn tier2_project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

fn tier2_corpus_dir_is_explicit() -> bool {
    std::env::var_os("RVT_PROJECT_CORPUS_DIR").is_some()
}

fn tier1_project_dir() -> PathBuf {
    std::env::var("RVT_CORPUS_TIER1_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/tier1"))
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/project-counts")
}

fn manifest_tier(manifest: &Value) -> u64 {
    if let Some(t) = manifest.get("tier").and_then(Value::as_u64) {
        return t;
    }
    let id = manifest.get("id").and_then(Value::as_str).unwrap_or("");
    if id.starts_with("tier1-") { 1 } else { 2 }
}

fn corpus_dir_for_manifest(manifest: &Value) -> PathBuf {
    if manifest_tier(manifest) == 1 {
        tier1_project_dir()
    } else {
        tier2_project_dir()
    }
}

/// Payload size matching `gen_fixture::synthesize_fields`.
fn synth_payload_size(class_name: &str) -> usize {
    let base = 1 + 4 + 16;
    let extra = match class_name {
        "Wall" | "Level" | "Column" | "Beam" | "Slab" => 8,
        "Project" => 8,
        _ => 0,
    };
    base + extra
}

fn count_class_instances_from_fixture(
    rf: &mut RevitFile,
    classes: &[String],
) -> Result<BTreeMap<String, usize>, Box<dyn std::error::Error>> {
    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let decomp = compression::inflate_at(&raw, 8)?;
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for c in classes {
        counts.insert(c.clone(), 0);
    }
    if decomp.len() < 0x20 {
        return Ok(counts);
    }
    let mut cursor = 0x20usize;
    let end = decomp.len().saturating_sub(64);
    while cursor + 8 <= end {
        let class_tag =
            u32::from_le_bytes(decomp[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        if class_tag >= classes.len() {
            break;
        }
        let class_name = &classes[class_tag];
        let payload = synth_payload_size(class_name);
        if cursor + 8 + payload > decomp.len() {
            break;
        }
        cursor += 8 + payload;
        *counts.entry(class_name.clone()).or_insert(0) += 1;
    }
    Ok(counts)
}

fn load_fixture_classes(project_path: &Path) -> Option<Vec<String>> {
    let stem = project_path.file_stem()?.to_str()?;
    let recipe = project_path.parent()?.join(format!("{stem}.fixture.json"));
    if !recipe.exists() {
        return None;
    }
    let text = std::fs::read_to_string(recipe).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    let classes = value.get("classes")?.as_array()?;
    Some(
        classes
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
    )
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
        "diagnostics.exported.building_elements_with_geometry" => {
            diagnostics.exported.building_elements_with_geometry
        }
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

        // `relations` (#222): the OctetProof 1.1.0 relation-pair-set
        // field class. Optional per manifest, but every entry must
        // name its relation type and both sides of the comparison.
        if let Some(relations) = manifest.get("relations") {
            for (category, spec) in obj(relations, &format!("{context}.relations")) {
                let relation_context = format!("{context}.relations.{category}");
                str_field(spec, "relation_ifc_type", &relation_context);
                int_field(spec, "expected_pairs", &relation_context);
                str_field(spec, "source", &relation_context);
                str_field(spec, "decoder_metric", &relation_context);
                int_field(spec, "decoder_expected_pairs", &relation_context);
                let status = str_field(spec, "status", &relation_context);
                match status {
                    "known" => {}
                    "known_gap" | "unsupported" => {
                        str_field(spec, "unsupported_feature", &relation_context);
                        int_field(spec, "tracking_issue", &relation_context);
                    }
                    "decoder_baseline" => {
                        int_field(spec, "tracking_issue", &relation_context);
                    }
                    _ => panic!("{relation_context}.status has unsupported value {status}"),
                }
            }
        }

        // `storeys` (#218): the OctetProof 1.1.0 storey-set field
        // class. Optional per manifest, but every entry must name its
        // spatial type and both sides of the comparison.
        if let Some(storeys) = manifest.get("storeys") {
            for (category, spec) in obj(storeys, &format!("{context}.storeys")) {
                let storey_context = format!("{context}.storeys.{category}");
                str_field(spec, "storey_ifc_type", &storey_context);
                int_field(spec, "expected_storeys", &storey_context);
                str_field(spec, "source", &storey_context);
                str_field(spec, "decoder_metric", &storey_context);
                int_field(spec, "decoder_expected_storeys", &storey_context);
                let status = str_field(spec, "status", &storey_context);
                match status {
                    "known" => {}
                    "known_gap" | "unsupported" => {
                        str_field(spec, "unsupported_feature", &storey_context);
                        int_field(spec, "tracking_issue", &storey_context);
                    }
                    "decoder_baseline" => {
                        int_field(spec, "tracking_issue", &storey_context);
                    }
                    _ => panic!("{storey_context}.status has unsupported value {status}"),
                }
            }
        }
    }
}

#[test]
fn project_count_manifests_match_available_corpus() -> Result<(), Box<dyn std::error::Error>> {
    let mut exercised = 0usize;
    let mut skipped_missing_corpus = 0usize;
    let explicit_tier2 = tier2_corpus_dir_is_explicit();
    let mut tier2_exercised = 0usize;

    for path in manifest_paths() {
        let manifest = read_json(&path);
        let id = str_field(&manifest, "id", &path.display().to_string()).to_string();
        let tier = manifest_tier(&manifest);
        let corpus_dir = corpus_dir_for_manifest(&manifest);
        if !corpus_dir.exists() {
            if tier == 2 && explicit_tier2 {
                panic!(
                    "RVT_PROJECT_CORPUS_DIR is set to {} but that directory does not exist (manifest {id}); fix the path or unset the variable to skip tier-two checks",
                    corpus_dir.display()
                );
            }
            skipped_missing_corpus += 1;
            eprintln!(
                "skipping project-count manifest {id}: corpus dir missing at {}",
                corpus_dir.display()
            );
            continue;
        }

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
        if tier == 2 {
            tier2_exercised += 1;
        }

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
        let fixture_classes = load_fixture_classes(&project_path);
        let class_inventory = match fixture_classes.as_ref() {
            Some(classes) => Some(count_class_instances_from_fixture(&mut rf, classes)?),
            None => None,
        };

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

            if let Some(metric) = opt_str_field(count, "fixture_metric") {
                let expected = int_field(count, "expected", &format!("{id}.{category}"));
                let tolerance = int_field(count, "tolerance", &format!("{id}.{category}"));
                let class_name = metric.strip_prefix("class_instances.").unwrap_or_else(|| {
                    panic!("{id}.{category}: unsupported fixture_metric {metric}")
                });
                let inventory = class_inventory.as_ref().unwrap_or_else(|| {
                    panic!(
                        "{id}.{category}: fixture_metric requires a sibling *.fixture.json recipe"
                    )
                });
                let actual = inventory.get(class_name).copied().unwrap_or(0);
                assert_with_tolerance(
                    &format!("{id}.{category} fixture {metric}"),
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

        // Relation pair sets (#222). Both sides are scored as counts
        // here; the exact `[host Tag, filling Tag]` set equality is
        // the OctetProof gate (`relations.<TYPE>` in the verdict's
        // claimed surface) plus the corpus gate in
        // tests/iter_elements_typed.rs.
        let no_relations = serde_json::Map::new();
        let relations = manifest
            .get("relations")
            .map(|relations| obj(relations, &format!("{id}.relations")))
            .unwrap_or(&no_relations);
        for (category, spec) in relations {
            let relation_context = format!("{id}.relations.{category}");
            let relation_type = str_field(spec, "relation_ifc_type", &relation_context);
            if let Some(reference) = reference_ifc.as_ref() {
                assert_with_tolerance(
                    &format!("{relation_context} source {relation_type}"),
                    count_step_constructor(reference, relation_type),
                    int_field(spec, "expected_pairs", &relation_context),
                    0,
                );
            }
            let metric = str_field(spec, "decoder_metric", &relation_context);
            assert_with_tolerance(
                &format!("{relation_context} decoder {metric}"),
                metric_actual(metric, &result.diagnostics, &step),
                int_field(spec, "decoder_expected_pairs", &relation_context),
                0,
            );
        }

        // Storey sets (#218). Both sides are scored as counts here;
        // the exact `[name, elevation]` set equality is the
        // OctetProof gate (`storeys.<TYPE>` in the verdict's claimed
        // surface).
        let no_storeys = serde_json::Map::new();
        let storeys = manifest
            .get("storeys")
            .map(|storeys| obj(storeys, &format!("{id}.storeys")))
            .unwrap_or(&no_storeys);
        for (category, spec) in storeys {
            let storey_context = format!("{id}.storeys.{category}");
            let storey_type = str_field(spec, "storey_ifc_type", &storey_context);
            if let Some(reference) = reference_ifc.as_ref() {
                assert_with_tolerance(
                    &format!("{storey_context} source {storey_type}"),
                    count_step_constructor(reference, storey_type),
                    int_field(spec, "expected_storeys", &storey_context),
                    0,
                );
            }
            let metric = str_field(spec, "decoder_metric", &storey_context);
            assert_with_tolerance(
                &format!("{storey_context} decoder {metric}"),
                metric_actual(metric, &result.diagnostics, &step),
                int_field(spec, "decoder_expected_storeys", &storey_context),
                0,
            );
        }
    }

    // Tier-one manifests are always in-repo; failing to exercise any of them
    // means the corpus checkout is broken.
    let tier1_dir = tier1_project_dir();
    if tier1_dir.exists() {
        assert!(
            exercised > 0,
            "tier1 corpus at {} exists but no project-count manifests matched",
            tier1_dir.display()
        );
    } else if skipped_missing_corpus > 0 {
        eprintln!("no corpus directories available; skipped {skipped_missing_corpus} manifest(s)");
    }
    if explicit_tier2 {
        assert!(
            tier2_exercised > 0,
            "RVT_PROJECT_CORPUS_DIR is set to {} but no tier-two project-count manifest matched a project file there; fix the path or unset the variable to skip tier-two checks",
            tier2_project_dir().display()
        );
    }
    Ok(())
}
