//! `research/witness-registry.json` must stay internally consistent and in
//! sync with the project-count manifests that carry the golden hashes
//! (docs/verification-protocol.md). Structural checks only — the registry
//! records evidence, it never asserts a capability.

use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_json(rel: &str) -> Value {
    let path = root().join(rel);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn ids(list: &Value, key: &str) -> BTreeSet<String> {
    let arr = list
        .as_array()
        .unwrap_or_else(|| panic!("`{key}` must be an array"));
    let mut out = BTreeSet::new();
    for item in arr {
        let id = item["id"]
            .as_str()
            .unwrap_or_else(|| panic!("{key}: every entry needs a string id"))
            .to_string();
        assert!(out.insert(id.clone()), "{key}: duplicate id {id}");
    }
    out
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

/// The OctetProof §9.4 controlled vocabulary. A registry `covers` entry and an
/// observation's `semantic_surface_covered` both draw from exactly this list;
/// witness identifiers are never members of it.
const COVERAGE_VOCABULARY: &[&str] = &[
    "entity_counts",
    "relations",
    "storeys",
    "layer_topology",
    "linework",
    "bounding_boxes",
    "xdata_fields",
    "text_content",
    "3d_solids",
    "meshes",
    "bim_parameters",
];

/// `covers` as declared by one witness, or `None` when the entry omits it.
fn declared_covers(entry: &Value) -> Option<BTreeSet<String>> {
    let id = entry["id"].as_str().unwrap();
    let raw = entry.get("covers")?;
    let arr = raw
        .as_array()
        .unwrap_or_else(|| panic!("witness {id}: `covers` must be an array"));
    assert!(!arr.is_empty(), "witness {id}: `covers` must not be empty");
    let mut out = BTreeSet::new();
    for item in arr {
        let class = item
            .as_str()
            .unwrap_or_else(|| panic!("witness {id}: `covers` entries must be strings"))
            .to_string();
        assert!(
            COVERAGE_VOCABULARY.contains(&class.as_str()),
            "witness {id}: `covers` entry {class} is not in the §9.4 vocabulary"
        );
        assert!(
            out.insert(class.clone()),
            "witness {id}: duplicate `covers` entry {class}"
        );
    }
    Some(out)
}

#[test]
fn registry_is_internally_consistent() {
    let reg = read_json("research/witness-registry.json");
    assert_eq!(reg["schema_version"], 1);
    assert!(
        root()
            .join(reg["protocol"].as_str().expect("protocol path"))
            .is_file(),
        "protocol doc must exist"
    );

    let nodes = ids(&reg["nodes"], "nodes");
    let witnesses = ids(&reg["witnesses"], "witnesses");
    let artifacts = ids(&reg["artifacts"], "artifacts");
    let edges = ids(&reg["edges"], "edges");
    ids(&reg["agreements"], "agreements");

    for w in reg["witnesses"].as_array().unwrap() {
        let node = w["node"].as_str().unwrap();
        assert!(
            nodes.contains(node),
            "witness {} names unknown node {node}",
            w["id"]
        );
        let status = w["status"].as_str().unwrap();
        assert!(
            ["adopted", "candidate", "rejected"].contains(&status),
            "witness {} bad status",
            w["id"]
        );
        if status == "candidate" {
            assert!(
                w.get("priority").is_some(),
                "candidate witness {} needs a priority",
                w["id"]
            );
        }

        // OctetProof §9.4: the coverage declaration is a claim about what a
        // witness *reads*. An authoring witness emits the artifact itself and
        // never an observation, so it declares nothing; an adopted reader must.
        let covers = declared_covers(w);
        let kind = w["kind"].as_str().unwrap();
        if kind == "author" {
            assert!(
                covers.is_none(),
                "authoring witness {} must not declare `covers` (§9.4 is a reading claim)",
                w["id"]
            );
        }
        if kind == "reader" && status == "adopted" {
            assert!(
                covers.is_some(),
                "adopted reader {} must declare `covers` (§9.4 coverage declaration)",
                w["id"]
            );
        }
    }

    let mut seen_hashes = BTreeSet::new();
    for a in reg["artifacts"].as_array().unwrap() {
        let id = a["id"].as_str().unwrap();
        assert!(
            nodes.contains(a["node"].as_str().unwrap()),
            "artifact {id} names unknown node"
        );
        let sha = a["sha256"].as_str().unwrap();
        assert!(
            is_sha256(sha),
            "artifact {id}: sha256 must be 64 lowercase hex chars"
        );
        assert!(
            seen_hashes.insert(sha.to_string()),
            "artifact {id}: duplicate sha256"
        );
        assert!(
            a["bytes"].as_u64().is_some_and(|b| b > 0),
            "artifact {id}: bytes"
        );
        if let Some(parent) = a.get("derived_from").and_then(Value::as_str) {
            assert!(
                artifacts.contains(parent),
                "artifact {id} derived from unknown artifact {parent}"
            );
            assert!(
                a.get("via")
                    .and_then(Value::as_str)
                    .is_some_and(|v| witnesses.contains(v)),
                "derived artifact {id} must name the authoring witness in `via`"
            );
        }
    }

    for e in reg["edges"].as_array().unwrap() {
        let id = e["id"].as_str().unwrap();
        assert!(
            nodes.contains(e["from"].as_str().unwrap())
                && nodes.contains(e["to"].as_str().unwrap()),
            "edge {id}: unknown node"
        );
        assert!(
            witnesses.contains(e["via"].as_str().unwrap()),
            "edge {id}: unknown witness in via"
        );
        let status = e["status"].as_str().unwrap();
        for key in ["source_artifact", "derived_artifact"] {
            match e.get(key).and_then(Value::as_str) {
                Some(art) => assert!(artifacts.contains(art), "edge {id}: unknown {key} {art}"),
                None => assert_eq!(
                    status, "pending",
                    "edge {id}: only pending edges may leave {key} null"
                ),
            }
        }
    }

    for g in reg["agreements"].as_array().unwrap() {
        let id = g["id"].as_str().unwrap();
        if let Some(edge) = g["edge"].as_str() {
            assert!(edges.contains(edge), "agreement {id}: unknown edge {edge}");
        }
        let ws: Vec<&str> = g["witnesses"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(
            ws.len() >= 2,
            "agreement {id}: needs at least two witnesses"
        );
        for w in &ws {
            assert!(
                witnesses.contains(*w),
                "agreement {id}: unknown witness {w}"
            );
            let entry = reg["witnesses"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["id"] == *w)
                .unwrap();
            assert_eq!(
                entry["status"], "adopted",
                "agreement {id}: witness {w} is not adopted"
            );
        }
        let gate = g["gate"].as_str().unwrap();
        assert!(
            root().join(gate).exists(),
            "agreement {id}: gate {gate} does not exist in the tree"
        );
        if g["status"] == "gated" {
            assert!(
                g.get("ci").and_then(Value::as_str).is_some(),
                "agreement {id}: gated agreements name their CI job"
            );
        }
    }
}

/// Every golden hash the project-count manifests carry must be registered,
/// and vice versa for manifests-backed artifacts — one source of truth.
#[test]
fn registry_and_project_count_manifests_agree_on_hashes() {
    let reg = read_json("research/witness-registry.json");
    let registered: BTreeSet<String> = reg["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["sha256"].as_str().unwrap().to_string())
        .collect();

    let dir = root().join("tests/fixtures/project-counts");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).unwrap().flatten() {
        let path: PathBuf = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let m: Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let source = &m["source"];
        for key in ["rvt_sha256", "reference_ifc_sha256"] {
            if let Some(sha) = source.get(key).and_then(Value::as_str) {
                assert!(
                    registered.contains(sha),
                    "{}: {key} {sha} is not in research/witness-registry.json",
                    Path::new(&path).file_name().unwrap().to_string_lossy()
                );
                checked += 1;
            }
        }
    }
    assert!(
        checked >= 6,
        "expected to cross-check at least six manifest hashes, saw {checked}"
    );
}

/// The IfcOpenShell gate script the registry names must exist, be
/// executable-shaped, and reference the manifest it is wired to in CI.
#[test]
fn ifcopenshell_gate_script_is_wired() {
    let script = std::fs::read_to_string(root().join("tools/ci/witness-ifcopenshell.py"))
        .expect("gate script");
    assert!(script.starts_with("#!/usr/bin/env python3"));
    assert!(
        script.contains("reference_ifc_sha256"),
        "script must verify the artifact hash"
    );
    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci.contains("tools/ci/witness-ifcopenshell.py"),
        "ci.yml must run the witness gate"
    );
    assert!(
        ci.contains("tests/fixtures/project-counts/2024-core-interior.json"),
        "gate must be wired to the Core Interior manifest"
    );
}

/// IFClite is the third implementation lineage on the RVT → IFC edge. Its
/// exact version pin (OctetProof §9.6) has to agree in three places — the
/// Cargo dependency, the constant the binary stamps into every observation,
/// and the registry entry — or a silent witness upgrade could move a verdict
/// without anyone noticing. The gate must also stay out of the `rvt`
/// workspace: `ifc-lite-core` is MPL-2.0 and is only ever run as a separate
/// process (docs/verification-protocol.md).
#[test]
fn ifc_lite_gate_is_wired_and_version_pinned() {
    let crate_dir = root().join("tools/ci/witness-ifc-lite");
    let manifest =
        std::fs::read_to_string(crate_dir.join("Cargo.toml")).expect("witness crate manifest");
    assert!(
        manifest.contains("[workspace]"),
        "the witness crate must declare its own workspace root so the \
         MPL-2.0 reader is never linked into the Apache-2.0 tree"
    );
    assert!(
        crate_dir.join("Cargo.lock").is_file(),
        "a committed lockfile is what makes `cargo run --locked` pin the \
         witness and its transitive deps"
    );
    let root_manifest = std::fs::read_to_string(root().join("Cargo.toml")).unwrap();
    assert!(
        !root_manifest.contains("witness-ifc-lite"),
        "the witness must not be a member of the rvt workspace"
    );

    let pin = manifest
        .lines()
        .find_map(|line| line.trim().strip_prefix("ifc-lite-core = \"="))
        .and_then(|rest| rest.split('"').next())
        .expect("Cargo.toml must pin `ifc-lite-core = \"=X.Y.Z\"` exactly (§9.6)");

    let source = std::fs::read_to_string(crate_dir.join("src/main.rs")).expect("witness source");
    assert!(
        source.contains(&format!("WITNESS_VERSION: &str = \"{pin}\"")),
        "WITNESS_VERSION must equal the pinned ifc-lite-core version {pin}"
    );
    assert!(
        source.contains("reference_ifc_sha256"),
        "gate must verify the artifact hash"
    );

    let reg = read_json("research/witness-registry.json");
    let entry = reg["witnesses"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["id"] == "ifc-lite")
        .expect("registry must carry the ifc-lite witness");
    assert_eq!(entry["status"], "adopted", "ifc-lite must be adopted");
    assert_eq!(
        entry["version"].as_str(),
        Some(pin),
        "registry version must equal the Cargo pin {pin}"
    );

    let ci = std::fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    assert!(
        ci.contains("tools/ci/witness-ifc-lite/Cargo.toml"),
        "ci.yml must run the IFClite gate"
    );
    assert!(
        ci.contains("/tmp/witness/observations/ifc-lite.json"),
        "the IFClite observation must land in the directory the verdict reads"
    );

    let verdict = read_json("research/witness/magnetar-2024-core-interior/verdict.json");
    let lineages: BTreeSet<&str> = verdict["independence"]["lineages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(
        lineages.len() >= 3 && lineages.contains("ifc-lite"),
        "the Core Interior verdict must span three lineages including ifc-lite, saw {lineages:?}"
    );
}

/// OctetProof §9.4: a witness is compared only on what the registry says it
/// covers. Every committed observation therefore has to claim a surface the
/// registry already declares — an observation that widened
/// `semantic_surface_covered` without a registry PR would be a witness
/// silently enlarging its own claim, which §9.4 calls a registration
/// violation.
#[test]
fn committed_observations_stay_inside_their_registered_coverage() {
    let reg = read_json("research/witness-registry.json");
    let entries = reg["witnesses"].as_array().unwrap();

    let base = root().join("research/witness");
    let mut artifact_dirs: Vec<PathBuf> = std::fs::read_dir(&base)
        .unwrap_or_else(|e| panic!("read {}: {e}", base.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.join("observations").is_dir())
        .collect();
    artifact_dirs.sort();
    assert!(
        !artifact_dirs.is_empty(),
        "no committed observations under research/witness"
    );

    let mut checked = 0;
    for dir in artifact_dirs {
        let obs_dir = dir.join("observations");
        let mut files: Vec<PathBuf> = std::fs::read_dir(&obs_dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", obs_dir.display()))
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        files.sort();

        for path in files {
            let rel = path.strip_prefix(root()).unwrap_or(&path).to_owned();
            let obs: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let witness_id = obs["witness_id"]
                .as_str()
                .unwrap_or_else(|| panic!("{}: observation needs a witness_id", rel.display()));
            let entry = entries
                .iter()
                .find(|w| w["id"] == witness_id)
                .unwrap_or_else(|| {
                    panic!(
                        "{}: witness {witness_id} is not in the registry",
                        rel.display()
                    )
                });
            let covers = declared_covers(entry).unwrap_or_else(|| {
                panic!(
                    "{}: witness {witness_id} emits observations but declares no `covers`",
                    rel.display()
                )
            });

            let claimed: BTreeSet<String> = obs["semantic_surface_covered"]
                .as_array()
                .unwrap_or_else(|| {
                    panic!(
                        "{}: semantic_surface_covered must be an array",
                        rel.display()
                    )
                })
                .iter()
                .map(|v| {
                    v.as_str()
                        .unwrap_or_else(|| {
                            panic!(
                                "{}: semantic_surface_covered entries must be strings",
                                rel.display()
                            )
                        })
                        .to_string()
                })
                .collect();
            assert!(
                !claimed.is_empty(),
                "{}: semantic_surface_covered must not be empty",
                rel.display()
            );
            for class in &claimed {
                assert!(
                    COVERAGE_VOCABULARY.contains(&class.as_str()),
                    "{}: {class} is not in the §9.4 vocabulary",
                    rel.display()
                );
            }
            let undeclared: Vec<&String> = claimed.difference(&covers).collect();
            assert!(
                undeclared.is_empty(),
                "{}: witness {witness_id} claims {undeclared:?} which its registry \
                 `covers` {covers:?} does not declare (§9.4)",
                rel.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 6,
        "expected at least six committed observations, saw {checked}"
    );
}
