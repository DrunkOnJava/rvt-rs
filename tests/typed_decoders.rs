//! Lane Five — typed decoder corpus tests against `corpus/tier1/`.
//!
//! Synthetic `gen-fixture` CFBs are scaffold-oriented: they carry
//! schema classes and Global/Latest instance payloads with a small
//! field set (`m_flag`, `m_id`, `m_guid`, optional `m_height`), not
//! full Revit element records. These tests assert honest behavior:
//!
//! 1. MVP schema-driven decoders are registered and reject wrong schema.
//! 2. Matching schema decode over fixture instance bytes succeeds and
//!    projects typed views without inventing semantic fields.
//! 3. Known inventory counts from fixture recipes still hold.
//! 4. ArcWall partition decode finds **no** standard candidates on
//!    tier1 (no false positives); wrong-schema rejection is unit-tested
//!    in `elements::arc_wall`.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rvt::elements::{
    self, MVP_TYPED_CLASSES, floor::Floor, level::Level, openings::Door, openings::Window,
    styling::Material, wall::Wall, zones::Room,
};
use rvt::formats::ClassEntry;
use rvt::partition_scanner::{self, ScanOptions};
use rvt::walker::HandleIndex;
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

/// Payload size matching `gen_fixture::synthesize_fields` / tier1 health.
fn synth_payload_size(class_name: &str) -> usize {
    let base = 1 + 4 + 16;
    let extra = match class_name {
        "Wall" | "Level" | "Column" | "Beam" | "Slab" => 8,
        "Project" => 8,
        _ => 0,
    };
    base + extra
}

struct InstanceRef {
    class_name: String,
    payload: Vec<u8>,
}

fn collect_instances(decomp: &[u8], classes: &[String]) -> Vec<InstanceRef> {
    let mut out = Vec::new();
    if decomp.len() < 0x20 {
        return out;
    }
    let mut cursor = 0x20usize;
    let end = decomp.len().saturating_sub(64);
    while cursor + 8 <= end {
        let class_tag =
            u32::from_le_bytes(decomp[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        if class_tag >= classes.len() {
            break;
        }
        let class_name = classes[class_tag].clone();
        let payload_len = synth_payload_size(&class_name);
        if cursor + 8 + payload_len > decomp.len() {
            break;
        }
        let payload = decomp[cursor + 8..cursor + 8 + payload_len].to_vec();
        cursor += 8 + payload_len;
        out.push(InstanceRef {
            class_name,
            payload,
        });
    }
    out
}

fn category_key(class_name: &str) -> Option<&'static str> {
    match class_name {
        "Level" => Some("levels"),
        "Wall" => Some("walls"),
        "Floor" | "Slab" => Some("floors"),
        "Door" => Some("doors"),
        "Window" => Some("windows"),
        _ => None,
    }
}

#[test]
fn mvp_decoders_registered_and_reject_cross_schema() {
    for class in MVP_TYPED_CLASSES {
        let decoder = elements::decoder_for_class(class)
            .unwrap_or_else(|| panic!("missing decoder for {class}"));
        assert_eq!(decoder.class_name(), *class);

        // Feed every *other* MVP class name as the schema — must Err.
        for other in MVP_TYPED_CLASSES {
            if other == class {
                continue;
            }
            let wrong = ClassEntry {
                name: (*other).into(),
                offset: 0,
                fields: vec![],
                tag: None,
                parent: None,
                declared_field_count: None,
                was_parent_only: false,
                ancestor_tag: None,
            };
            let err = decoder
                .decode(&[], &wrong, &HandleIndex::new())
                .expect_err("wrong-schema must reject");
            assert!(
                err.to_string().contains("wrong schema"),
                "{class} vs {other}: {err}"
            );
        }
    }

    // ArcWall is partition-only — not in the schema-driven registry.
    assert!(elements::decoder_for_class("ArcWall").is_none());
}

#[test]
fn tier1_typed_decode_matches_inventory_and_rejects_mismatches() {
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
        let year = recipe["year"].as_u64().expect("year") as u32;
        let classes: Vec<String> = recipe["classes"]
            .as_array()
            .expect("classes")
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        let expected = recipe["expected_counts"].as_object().expect("counts");

        let mut rf = RevitFile::open(&rvt).unwrap_or_else(|e| {
            panic!("open {}: {e}", rvt.display());
        });
        let schema = rf.schema().expect("schema");
        let class_by_name: BTreeMap<&str, &ClassEntry> = schema
            .classes
            .iter()
            .map(|c| (c.name.as_str(), c))
            .collect();

        let raw = rf
            .read_stream(streams::GLOBAL_LATEST)
            .expect("Global/Latest");
        let decomp = compression::inflate_at(&raw, 8).expect("inflate");
        let instances = collect_instances(&decomp, &classes);
        assert_eq!(
            instances.len(),
            recipe["element_count"].as_u64().unwrap() as usize,
            "{name}: instance inventory drift"
        );

        let mut inventory: BTreeMap<&str, usize> = BTreeMap::new();
        for inst in &instances {
            if let Some(key) = category_key(&inst.class_name) {
                *inventory.entry(key).or_default() += 1;
            }

            let Some(entry) = class_by_name.get(inst.class_name.as_str()) else {
                // Class listed in recipe but absent from Formats — skip
                // typed decode; schema health is covered elsewhere.
                continue;
            };

            let Some(decoder) = elements::decoder_for_class(&inst.class_name) else {
                // Duct / Column / Beam etc. may lack MVP status — ok.
                continue;
            };

            let decoded = decoder
                .decode(&inst.payload, entry, &HandleIndex::new())
                .unwrap_or_else(|e| {
                    panic!(
                        "{name}: {} decoder failed on fixture payload: {e}",
                        inst.class_name
                    );
                });
            assert_eq!(decoded.class, inst.class_name);

            // Typed projection must not panic; scaffold fields are mostly
            // None except height on Level/Wall gen-fixture payloads.
            match inst.class_name.as_str() {
                "Level" => {
                    let level = Level::from_decoded(&decoded);
                    // gen-fixture Levels carry m_height = 10.0.
                    assert_eq!(
                        level.elevation_feet,
                        Some(10.0),
                        "{name}: Level m_height should project as elevation"
                    );
                }
                "Wall" => {
                    let wall = Wall::from_decoded(&decoded);
                    assert_eq!(wall.unconnected_height_feet, Some(10.0));
                }
                "Floor" => {
                    let _ = Floor::from_decoded(&decoded);
                }
                "Door" => {
                    let _ = Door::from_decoded(&decoded);
                }
                "Window" => {
                    let _ = Window::from_decoded(&decoded);
                }
                "Room" => {
                    let _ = Room::from_decoded(&decoded);
                }
                "Material" => {
                    let _ = Material::from_decoded(&decoded);
                }
                _ => {}
            }

            // Wrong-schema: the matching decoder must refuse a ClassEntry
            // whose name has been rewritten to another MVP class.
            for other in MVP_TYPED_CLASSES {
                if *other == inst.class_name.as_str() {
                    continue;
                }
                let mut wrong_schema = (*entry).clone();
                wrong_schema.name = (*other).into();
                assert!(
                    decoder
                        .decode(&inst.payload, &wrong_schema, &HandleIndex::new())
                        .is_err(),
                    "{name}: {} decoder must reject schema name {other}",
                    inst.class_name
                );
            }
        }

        for key in ["levels", "walls", "floors", "doors", "windows"] {
            let want = expected[key].as_u64().unwrap_or(0) as usize;
            let got = inventory.get(key).copied().unwrap_or(0);
            assert_eq!(got, want, "{name}: {key} inventory {got} != {want}");
        }

        // ArcWall: tier1 synthetics have empty/unsupported partition
        // geometry — scanner must not invent ArcWall-standard hits.
        let scan = partition_scanner::scan_partitions(&mut rf, year, &ScanOptions::default())
            .expect("partition scan");
        let arcwall_hits = scan
            .candidates
            .iter()
            .filter(|c| {
                c.class_name.as_deref() == Some("ArcWall")
                    || c.class_tag == rvt::arc_wall_record::ARC_WALL_TAG
            })
            .filter(|c| c.confidence >= 0.85)
            .count();
        assert_eq!(
            arcwall_hits, 0,
            "{name}: tier1 must not produce ArcWall-standard candidates, got {arcwall_hits}"
        );

        eprintln!(
            "typed-decoders tier1 ok · {name} · year={year} · instances={}",
            instances.len()
        );
    }
}

#[test]
fn decode_typed_helper_dispatches_and_wall_rejects_floor_name() {
    let root = tier1_dir();
    let path = root.join("architectural-2024/architectural-2024.rvt");
    assert!(path.exists(), "missing {}", path.display());

    let mut rf = RevitFile::open(&path).unwrap();
    let schema = rf.schema().unwrap();
    let wall = schema
        .classes
        .iter()
        .find(|c| c.name == "Wall")
        .expect("Wall in architectural-2024 schema");

    let decoded = elements::decode_typed(&[], wall, &HandleIndex::new()).expect("dispatch");
    assert_eq!(decoded.class, "Wall");

    let mut floor_named = wall.clone();
    floor_named.name = "Floor".into();
    let wall_dec = elements::decoder_for_class("Wall").unwrap();
    assert!(
        wall_dec
            .decode(&[], &floor_named, &HandleIndex::new())
            .is_err(),
        "WallDecoder must reject Floor-named schema"
    );
}
