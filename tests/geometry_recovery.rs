//! Lane Six — geometry recovery over `corpus/tier1/` synthetics.
//!
//! Tier1 fixtures are scaffold-oriented (`m_flag` / `m_id` / `m_guid` /
//! optional `m_height`). These tests assert API shape and honest
//! empty/partial recovery — not invented wall curves or floor loops.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rvt::elements::{
    self, floor::Floor, level::Level, openings::Door, openings::Window, wall::Wall,
};
use rvt::formats::ClassEntry;
use rvt::geometry::{
    recover_door_host, recover_floor_boundary, recover_floor_boundary_from_floor,
    recover_level_elevation, recover_wall_location_curve_from_wall, recover_window_host,
};
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

#[test]
fn tier1_geometry_recovery_is_honest() {
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

        let mut wall_curves_recovered = 0usize;
        let mut floor_loops_recovered = 0usize;
        let mut level_elevations_recovered = 0usize;
        let mut door_hosts_recovered = 0usize;
        let mut window_hosts_recovered = 0usize;

        for inst in &instances {
            let Some(entry) = class_by_name.get(inst.class_name.as_str()) else {
                continue;
            };
            let Some(decoder) = elements::decoder_for_class(&inst.class_name) else {
                continue;
            };
            let decoded = decoder
                .decode(&inst.payload, entry, &HandleIndex::new())
                .expect("typed decode");

            match inst.class_name.as_str() {
                "Wall" => {
                    let wall = Wall::from_decoded(&decoded);
                    // Height is present on gen-fixture; location curve is not.
                    assert_eq!(wall.unconnected_height_feet, Some(10.0));
                    assert!(wall.location_start.is_none());
                    assert!(wall.location_curve_id.is_none());
                    let outcome = recover_wall_location_curve_from_wall(&wall, &decoded);
                    if outcome.is_recovered() {
                        wall_curves_recovered += 1;
                    } else {
                        assert_eq!(
                            outcome.diagnostic().map(|d| d.code),
                            Some("wall_location_curve_missing")
                        );
                    }
                }
                "Floor" => {
                    let floor = Floor::from_decoded(&decoded);
                    let outcome = recover_floor_boundary_from_floor(&floor, &decoded);
                    if outcome.is_recovered() {
                        floor_loops_recovered += 1;
                    } else {
                        assert_eq!(
                            outcome.diagnostic().map(|d| d.code),
                            Some("floor_boundary_missing")
                        );
                    }
                    let _ = recover_floor_boundary(&decoded);
                }
                "Level" => {
                    let level = Level::from_decoded(&decoded);
                    let outcome = recover_level_elevation(&level);
                    assert!(
                        outcome.is_recovered(),
                        "{name}: Level m_height must recover as elevation"
                    );
                    assert_eq!(
                        outcome.as_recovered().map(|e| e.elevation_feet),
                        Some(10.0)
                    );
                    level_elevations_recovered += 1;
                }
                "Door" => {
                    let door = Door::from_decoded(&decoded);
                    if recover_door_host(&door).is_recovered() {
                        door_hosts_recovered += 1;
                    }
                }
                "Window" => {
                    let window = Window::from_decoded(&decoded);
                    if recover_window_host(&window).is_recovered() {
                        window_hosts_recovered += 1;
                    }
                }
                _ => {}
            }
        }

        assert_eq!(
            wall_curves_recovered, 0,
            "{name}: tier1 must not invent wall location curves"
        );
        assert_eq!(
            floor_loops_recovered, 0,
            "{name}: tier1 must not invent floor boundaries"
        );
        assert_eq!(
            door_hosts_recovered, 0,
            "{name}: tier1 must not invent door hosts"
        );
        assert_eq!(
            window_hosts_recovered, 0,
            "{name}: tier1 must not invent window hosts"
        );
        if classes.iter().any(|c| c == "Level") {
            assert!(
                level_elevations_recovered > 0,
                "{name}: expected Level elevation recovery"
            );
        }

        // No ArcWall-standard false positives → no partition wall curves.
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
            "{name}: tier1 must not produce ArcWall candidates for geometry"
        );

        eprintln!(
            "geometry-recovery tier1 ok · {name} · year={year} · levels_elev={level_elevations_recovered}"
        );
    }
}
