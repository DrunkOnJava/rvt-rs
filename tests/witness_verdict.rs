//! The committed OctetProof observations and verdict under
//! `research/witness/` must be internally consistent: every observation's
//! canonical payload hash recomputes, every input hash is a registered
//! artifact, and the verdict is `PASS` on the claimed surface with the
//! independence set satisfied (docs/verification-protocol.md,
//! docs/octetproof-spec.md §6, §9.3).

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(path: &Path) -> Value {
    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

/// serde_json without `preserve_order` serializes objects with sorted keys
/// and `to_string` emits no whitespace — the same bytes the Python witnesses
/// hash (`sort_keys=True, separators=(",", ":")`).
fn canonical_hash(value: &Value) -> String {
    let canonical = serde_json::to_string(value).expect("serialize");
    format!("{:x}", Sha256::digest(canonical.as_bytes()))
}

fn artifact_dirs() -> Vec<PathBuf> {
    let base = root().join("research/witness");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("verdict.json").is_file())
        .collect();
    dirs.sort();
    assert!(
        !dirs.is_empty(),
        "no committed artifacts under research/witness"
    );
    dirs
}

#[test]
fn committed_observations_rehash_and_name_registered_inputs() {
    let registry = read_json(&root().join("research/witness-registry.json"));
    let registered: BTreeSet<&str> = registry["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["sha256"].as_str().unwrap())
        .collect();
    let witnesses: BTreeSet<&str> = registry["witnesses"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|w| w["status"] == "adopted")
        .map(|w| w["id"].as_str().unwrap())
        .collect();

    for dir in artifact_dirs() {
        let obs_dir = dir.join("observations");
        let mut seen = 0;
        for entry in std::fs::read_dir(&obs_dir).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let obs = read_json(&path);
            let ctx = path.display().to_string();
            // OctetProof 1.1.0 added the `relations` field class; a 1.0.0
            // observation stays valid, so both envelope versions are
            // accepted (spec §16.1, §20).
            let schema_version = obs["schema_version"].as_str().unwrap_or_default();
            assert!(
                ["1.0.0", "1.1.0"].contains(&schema_version),
                "{ctx}: schema_version {schema_version}"
            );
            let wid = obs["witness_id"].as_str().unwrap();
            assert!(
                witnesses.contains(wid),
                "{ctx}: witness {wid} not adopted in registry"
            );
            assert_eq!(
                path.file_stem().unwrap().to_str().unwrap(),
                wid,
                "{ctx}: file name must be the witness id"
            );
            assert_eq!(obs["deterministic"], true, "{ctx}: deterministic");
            let input = obs["input_hash_sha256"].as_str().unwrap();
            assert!(
                registered.contains(input),
                "{ctx}: input {input} is not a registered artifact"
            );
            assert!(
                ["source", "bridge"].contains(&obs["input_role"].as_str().unwrap()),
                "{ctx}: input_role"
            );
            assert_eq!(
                obs["observation_hash_sha256"].as_str().unwrap(),
                canonical_hash(&obs["observation"]),
                "{ctx}: observation_hash_sha256 does not recompute from the canonical payload"
            );
            assert!(
                obs["observation"]["entity_counts"].is_object(),
                "{ctx}: entity_counts payload"
            );
            if schema_version == "1.1.0" {
                let relations = &obs["observation"]["relations"];
                assert!(relations.is_object(), "{ctx}: relations payload");
                for (relation, pairs) in relations.as_object().unwrap() {
                    let pairs = pairs
                        .as_array()
                        .unwrap_or_else(|| panic!("{ctx}: relations.{relation} must be an array"));
                    let mut previous: Option<Vec<&str>> = None;
                    for pair in pairs {
                        let pair: Vec<&str> = pair
                            .as_array()
                            .unwrap_or_else(|| {
                                panic!("{ctx}: relations.{relation} entries must be arrays")
                            })
                            .iter()
                            .map(|tag| {
                                tag.as_str().unwrap_or_else(|| {
                                    panic!("{ctx}: relations.{relation} tags must be strings")
                                })
                            })
                            .collect();
                        assert_eq!(pair.len(), 2, "{ctx}: relations.{relation} pair arity");
                        if let Some(previous) = &previous {
                            assert!(
                                previous.as_slice() <= pair.as_slice(),
                                "{ctx}: relations.{relation} must be canonically sorted"
                            );
                        }
                        previous = Some(pair);
                    }
                }
                // Storey sets (#218): `[name, elevation-in-feet]`, sorted,
                // the elevation always a six-decimal string so the canonical
                // form stays integer/string only (spec §7.2, §7.3).
                let storeys = &obs["observation"]["storeys"];
                assert!(storeys.is_object(), "{ctx}: storeys payload");
                for (storey_type, entries) in storeys.as_object().unwrap() {
                    let entries = entries
                        .as_array()
                        .unwrap_or_else(|| panic!("{ctx}: storeys.{storey_type} must be an array"));
                    let mut previous: Option<Vec<&str>> = None;
                    for entry in entries {
                        let entry: Vec<&str> = entry
                            .as_array()
                            .unwrap_or_else(|| {
                                panic!("{ctx}: storeys.{storey_type} entries must be arrays")
                            })
                            .iter()
                            .map(|field| {
                                field.as_str().unwrap_or_else(|| {
                                    panic!("{ctx}: storeys.{storey_type} fields must be strings")
                                })
                            })
                            .collect();
                        assert_eq!(entry.len(), 2, "{ctx}: storeys.{storey_type} pair arity");
                        let elevation = entry[1];
                        assert!(
                            elevation
                                .split_once('.')
                                .is_some_and(|(_, frac)| frac.len() == 6),
                            "{ctx}: storeys.{storey_type} elevation {elevation} must carry six decimals"
                        );
                        assert!(
                            elevation.parse::<f64>().is_ok(),
                            "{ctx}: storeys.{storey_type} elevation {elevation} must parse"
                        );
                        assert_ne!(
                            elevation, "-0.000000",
                            "{ctx}: storeys.{storey_type} must normalise negative zero"
                        );
                        if let Some(previous) = &previous {
                            assert!(
                                previous.as_slice() <= entry.as_slice(),
                                "{ctx}: storeys.{storey_type} must be canonically sorted"
                            );
                        }
                        previous = Some(entry);
                    }
                }
            }
            seen += 1;
        }
        assert!(seen >= 2, "{}: fewer than two observations", dir.display());
    }
}

#[test]
fn committed_verdicts_pass_with_independence() {
    for dir in artifact_dirs() {
        let verdict = read_json(&dir.join("verdict.json"));
        let ctx = dir.display().to_string();
        assert_eq!(verdict["status"], "PASS", "{ctx}: verdict status");
        assert_eq!(verdict["insufficient_witnesses"], false, "{ctx}");
        assert_eq!(
            verdict["diffs"].as_array().unwrap().len(),
            0,
            "{ctx}: diffs"
        );
        assert!(
            verdict["witnesses_compared"].as_array().unwrap().len() >= 2,
            "{ctx}: witnesses_compared"
        );
        assert_eq!(
            verdict["independence"]["satisfied"], true,
            "{ctx}: independence set"
        );
        let roles: BTreeSet<&str> = verdict["independence"]["roles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.as_str().unwrap())
            .collect();
        assert!(
            roles.contains("source") && roles.contains("bridge"),
            "{ctx}: needs one source-format and one bridge-format witness"
        );
        assert!(
            !verdict["semantic_surface"].as_array().unwrap().is_empty(),
            "{ctx}: empty semantic surface"
        );
        // Excluded fields are first-class: each names its reason.
        for ex in verdict["excluded"].as_array().unwrap() {
            assert!(
                ex["reason"].is_string(),
                "{ctx}: excluded entry without reason"
            );
        }
        // The verdict hash covers everything except the timestamp.
        let mut body = verdict.clone();
        let obj = body.as_object_mut().unwrap();
        obj.remove("timestamp");
        obj.remove("verdict_hash_sha256");
        assert_eq!(
            verdict["verdict_hash_sha256"].as_str().unwrap(),
            canonical_hash(&body),
            "{ctx}: verdict_hash_sha256 does not recompute"
        );
    }
}
