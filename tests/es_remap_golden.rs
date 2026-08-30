//! Contract for committed Revit-oracle runs under `research/es-remap/golden/`.
//!
//! Each run directory is what the pyRevit runner wrote (`bundle.json`,
//! `observations.json`, `truth-*.json`, the saved `.rvt` files). This test
//! keeps a committed run honest: every referenced file is present with the
//! recorded SHA-256, every observation matches the observation schema's
//! required shape, transition ids are ones the manifest names, and the Revit
//! build is recorded. It does not — and cannot — assert that rvt-rs decodes
//! any of it; `oracle_agrees` stays `null` until a decoder exists.
//!
//! With no committed runs the test reports that and passes, so CI stays
//! green while Phase 2 waits on a Revit-hosted run.

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const OBSERVATION_KINDS: &[&str] = &[
    "scalar",
    "copy",
    "remap_candidate",
    "null_baseline",
    "noop_baseline",
    "localization_attempt",
];
const TIERS: &[&str] = &["E0", "E1", "E2", "E3", "E4", "E5"];
const PATH_KINDS: &[&str] = &["field", "index", "map_key", "opaque"];
const REQUIRED: &[&str] = &[
    "schema_version",
    "observation_id",
    "fixture_id",
    "kind",
    "evidence_tier",
    "document_key",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn sha256_hex(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    format!("{:x}", hasher.finalize())
}

/// Transition ids the manifest names under `phase2_families:`.
fn manifest_transitions() -> Vec<String> {
    let text =
        std::fs::read_to_string(root().join("research/es-remap/manifest.yaml")).expect("manifest");
    let mut ids = Vec::new();
    let mut in_block = false;
    for line in text.lines() {
        if line.starts_with("phase2_families:") {
            in_block = true;
            continue;
        }
        if in_block {
            if let Some(id) = line.trim_start().strip_prefix("- ") {
                ids.push(id.trim().to_string());
            } else if !line.trim().is_empty() {
                break;
            }
        }
    }
    assert!(!ids.is_empty(), "manifest lists no phase2_families");
    ids
}

fn committed_runs() -> Vec<PathBuf> {
    let golden = root().join("research/es-remap/golden");
    let Ok(entries) = std::fs::read_dir(&golden) else {
        return Vec::new();
    };
    let mut runs: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("bundle.json").is_file())
        .collect();
    runs.sort();
    runs
}

fn field_str<'a>(v: &'a Value, key: &str, ctx: &str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("{ctx}: `{key}` must be a string"))
}

#[test]
fn committed_oracle_runs_are_complete_hashed_and_schema_shaped() {
    let runs = committed_runs();
    if runs.is_empty() {
        eprintln!(
            "no committed ES-remap oracle runs under research/es-remap/golden/ — Phase 2 needs a Revit-hosted run (tools/oracle/runner/pyrevit)"
        );
        return;
    }
    let transitions = manifest_transitions();

    for run in runs {
        let ctx = run.display().to_string();
        let bundle = read_json(&run.join("bundle.json"));
        assert_eq!(bundle["schema_version"], 1, "{ctx}: bundle schema_version");
        let fixture_id = field_str(&bundle, "fixture_id", &ctx).to_string();
        let revit_version = bundle["revit_version"]
            .as_u64()
            .unwrap_or_else(|| panic!("{ctx}: bundle revit_version must be an integer"));
        assert!(
            (2016..=2030).contains(&revit_version),
            "{ctx}: implausible revit_version {revit_version}"
        );
        assert!(
            bundle["revit_build"]
                .as_str()
                .is_some_and(|b| !b.is_empty()),
            "{ctx}: bundle must record revit_build"
        );

        // Every file the bundle references is committed next to it with the recorded hash.
        let files = bundle["files"]
            .as_object()
            .unwrap_or_else(|| panic!("{ctx}: bundle.files must be an object"));
        assert!(!files.is_empty(), "{ctx}: bundle lists no files");
        for (label, entry) in files {
            let recorded = field_str(entry, "sha256", &format!("{ctx}:{label}"));
            let path_field = field_str(entry, "path", &format!("{ctx}:{label}"));
            let basename = Path::new(path_field)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| panic!("{ctx}:{label}: bundle path has no file name"));
            let committed = run.join(&basename);
            assert!(
                committed.is_file(),
                "{ctx}: bundle file `{label}` ({basename}) is not committed alongside bundle.json"
            );
            assert_eq!(
                sha256_hex(&committed),
                recorded,
                "{ctx}: sha256 mismatch for {basename}"
            );
        }

        // Observations: schema-shaped, tied to this run, transitions from the manifest.
        let observations = read_json(&run.join("observations.json"));
        let list = observations
            .as_array()
            .unwrap_or_else(|| panic!("{ctx}: observations.json must be an array"));
        assert!(!list.is_empty(), "{ctx}: no observations");
        for (i, obs) in list.iter().enumerate() {
            let octx = format!("{ctx}: observation[{i}]");
            for key in REQUIRED {
                assert!(obs.get(*key).is_some(), "{octx}: missing required `{key}`");
            }
            assert_eq!(obs["schema_version"], 1, "{octx}: schema_version");
            assert_eq!(
                field_str(obs, "fixture_id", &octx),
                fixture_id,
                "{octx}: fixture_id"
            );
            let kind = field_str(obs, "kind", &octx);
            assert!(
                OBSERVATION_KINDS.contains(&kind),
                "{octx}: unknown kind {kind}"
            );
            let tier = field_str(obs, "evidence_tier", &octx);
            assert!(
                TIERS.contains(&tier),
                "{octx}: unknown evidence_tier {tier}"
            );
            assert!(
                field_str(obs, "document_key", &octx).starts_with(&fixture_id),
                "{octx}: document_key must be scoped to the fixture"
            );
            if let Some(t) = obs.get("transition_id").and_then(Value::as_str) {
                assert!(
                    transitions.contains(&t.to_string()),
                    "{octx}: transition {t} not in manifest.yaml"
                );
            }
            for key in ["before_element_id", "after_element_id"] {
                let v = &obs[key];
                assert!(
                    v.is_null() || v.as_u64().is_some(),
                    "{octx}: {key} must be null or a non-negative integer"
                );
            }
            if let Some(path) = obs.get("path").and_then(Value::as_array) {
                for seg in path {
                    let k = field_str(seg, "kind", &octx);
                    assert!(
                        PATH_KINDS.contains(&k),
                        "{octx}: unknown path segment kind {k}"
                    );
                }
            }
            let agrees = &obs["oracle_agrees"];
            assert!(
                agrees.is_null() || agrees.is_boolean(),
                "{octx}: oracle_agrees must be null or boolean"
            );
            if let Some(v) = obs.get("revit_version").and_then(Value::as_u64) {
                assert_eq!(
                    v, revit_version,
                    "{octx}: revit_version differs from bundle"
                );
            }
        }
        eprintln!(
            "{ctx}: {} files, {} observations validated",
            files.len(),
            list.len()
        );
    }
}

/// Owned synthetic fixtures are small; a run that balloons past this is a
/// sign something other than the seed got committed.
#[test]
fn committed_oracle_runs_stay_small() {
    for run in committed_runs() {
        let mut bytes = 0u64;
        for entry in std::fs::read_dir(&run).unwrap().flatten() {
            if let Ok(meta) = entry.metadata() {
                bytes += meta.len();
            }
        }
        assert!(
            bytes <= 64 * 1024 * 1024,
            "{}: {} MiB committed; keep oracle runs under 64 MiB (owned synthetics only)",
            run.display(),
            bytes / (1024 * 1024)
        );
    }
}
