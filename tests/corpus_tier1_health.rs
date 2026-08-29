//! Tier-one corpus health — always-on redistributable synthetic fixtures.
//!
//! Exercises every `.rvt` under `corpus/tier1/` (override with
//! `RVT_CORPUS_TIER1_DIR`):
//!
//!   - license + fixture recipe sidecars present
//!   - SHA256 matches the license sidecar
//!   - open / BasicFileInfo year / schema classes
//!   - Global/Latest class-instance inventory matches known counts
//!     for levels, walls, floors, doors, and windows
//!
//! These fixtures are license-free (`gen-fixture`); they must never be
//! replaced with Autodesk-owned samples (see SECURITY.md).

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rvt::{RevitFile, compression, streams};

fn tier1_dir() -> PathBuf {
    std::env::var("RVT_CORPUS_TIER1_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/tier1"))
}

fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| {
        panic!("read {}: {e}", path.display());
    });
    serde_json::from_str(&text).unwrap_or_else(|e| {
        panic!("parse {}: {e}", path.display());
    })
}

/// Payload byte size matching `gen_fixture::synthesize_fields`.
fn synth_payload_size(class_name: &str) -> usize {
    let base = 1 + 4 + 16; // m_flag + m_id + m_guid
    let extra = match class_name {
        "Wall" | "Level" | "Column" | "Beam" | "Slab" => 8, // m_height f64
        "Project" => 8,                                     // m_versionStamp i64
        _ => 0,
    };
    base + extra
}

/// Inventory class instances written by gen-fixture into Global/Latest.
fn count_class_instances(decomp: &[u8], classes: &[String]) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for c in classes {
        counts.insert(c.clone(), 0);
    }
    if decomp.len() < 0x20 {
        return counts;
    }
    let mut cursor = 0x20usize;
    // Trailing 64-byte pad from gen-fixture.
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
    counts
}

fn category_key(class_name: &str) -> Option<&'static str> {
    match class_name {
        "Level" => Some("levels"),
        "Wall" => Some("walls"),
        "Floor" | "Slab" => Some("floors"),
        "Door" => Some("doors"),
        "Window" => Some("windows"),
        "Column" => Some("columns"),
        "Beam" => Some("beams"),
        "Duct" | "Pipe" | "CableTray" => Some("mep"),
        _ => None,
    }
}

fn discover_fixture_dirs(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut dirs = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.join(format!("{name}.rvt")).exists() {
                dirs.push(p);
            }
        }
    }
    dirs.sort();
    dirs
}

#[test]
fn tier1_corpus_is_present_and_licensed() {
    let root = tier1_dir();
    assert!(
        root.is_dir(),
        "Tier-one corpus missing at {} — expected in-repo corpus/tier1",
        root.display()
    );
    let dirs = discover_fixture_dirs(&root);
    assert!(
        dirs.len() >= 3,
        "expected at least 3 tier1 fixtures under {}, found {}",
        root.display(),
        dirs.len()
    );

    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy();
        let rvt = dir.join(format!("{name}.rvt"));
        let license = dir.join(format!("{name}.license.json"));
        let fixture = dir.join(format!("{name}.fixture.json"));
        assert!(rvt.is_file(), "missing {}", rvt.display());
        assert!(
            license.is_file(),
            "missing license sidecar {}",
            license.display()
        );
        assert!(
            fixture.is_file(),
            "missing fixture recipe {}",
            fixture.display()
        );

        let lic = read_json(&license);
        let expected_sha = lic
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("{}.license.json missing sha256", name));
        let bytes = std::fs::read(&rvt).expect("read rvt");
        let actual_sha = hex::encode(Sha256::digest(&bytes));
        assert_eq!(
            actual_sha, expected_sha,
            "{name}: SHA256 drift — regenerate license sidecar or fixture"
        );
        let license_spdx = lic.get("license").and_then(Value::as_str).unwrap_or("");
        assert!(
            !license_spdx.is_empty(),
            "{name}: license sidecar must record an SPDX id"
        );
    }
}

#[test]
fn tier1_fixtures_open_and_match_known_counts() {
    let root = tier1_dir();
    let dirs = discover_fixture_dirs(&root);
    assert!(
        !dirs.is_empty(),
        "no tier1 fixtures under {}",
        root.display()
    );

    for dir in &dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let rvt = dir.join(format!("{name}.rvt"));
        let recipe = read_json(&dir.join(format!("{name}.fixture.json")));
        let year = recipe
            .get("year")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("{name}.fixture.json missing year"))
            as u32;
        let element_count = recipe
            .get("element_count")
            .and_then(Value::as_u64)
            .unwrap_or_else(|| panic!("{name}.fixture.json missing element_count"))
            as usize;
        let classes: Vec<String> = recipe
            .get("classes")
            .and_then(Value::as_array)
            .expect("classes array")
            .iter()
            .map(|v| v.as_str().expect("class string").to_string())
            .collect();
        let expected = recipe
            .get("expected_counts")
            .and_then(Value::as_object)
            .expect("expected_counts object");

        let mut rf = RevitFile::open(&rvt).unwrap_or_else(|e| {
            panic!("open {}: {e}", rvt.display());
        });
        assert!(
            rf.missing_required_streams().is_empty(),
            "{name}: missing required streams {:?}",
            rf.missing_required_streams()
        );
        let bfi = rf.basic_file_info().expect("BasicFileInfo");
        assert_eq!(bfi.version, year, "{name}: BasicFileInfo year mismatch");

        let schema = rf.schema().expect("schema");
        let schema_names: Vec<&str> = schema.classes.iter().map(|c| c.name.as_str()).collect();
        for class in &classes {
            assert!(
                schema_names.contains(&class.as_str()),
                "{name}: schema missing class {class}; got {schema_names:?}"
            );
        }

        let raw = rf
            .read_stream(streams::GLOBAL_LATEST)
            .expect("read Global/Latest");
        let decomp = compression::inflate_at(&raw, 8).expect("inflate Global/Latest");
        let inventory = count_class_instances(&decomp, &classes);
        let total: usize = inventory.values().sum();
        assert_eq!(
            total, element_count,
            "{name}: inventoried {total} instances, recipe element_count={element_count}; inventory={inventory:?}"
        );

        for key in ["levels", "walls", "floors", "doors", "windows"] {
            let want = expected
                .get(key)
                .and_then(Value::as_u64)
                .unwrap_or_else(|| panic!("{name}.expected_counts missing {key}"))
                as usize;
            let mut got = 0usize;
            for (class, n) in &inventory {
                if category_key(class) == Some(key) {
                    got += n;
                }
            }
            assert_eq!(
                got, want,
                "{name}: category {key} got {got}, expected {want} (inventory={inventory:?})"
            );
        }

        eprintln!("tier1 ok · {name} · year={year} · elements={element_count} · {inventory:?}");
    }
}
