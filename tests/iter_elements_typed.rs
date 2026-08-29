//! Production `iter_elements` typed MVP + partition ArcWall wiring.
//!
//! - Tier1 synthetics: fail-closed honesty (no invented ArcWalls;
//!   HostObjAttr never on production path).
//! - Optional magnetar project corpus: ArcWalls appear as typed
//!   `DecodedElement`s when `RVT_PROJECT_CORPUS_DIR` is set.

use std::path::{Path, PathBuf};

use rvt::elements::MVP_TYPED_CLASSES;
use rvt::elements::typed_json::is_mvp_typed_class;
use rvt::geometry::{
    recover_level_elevation, recover_wall_location_curve, recover_wall_location_curve_from_arc_wall,
};
use rvt::walker::{self, PRODUCTION_ELEMENT_MIN_SCORE};
use rvt::{RevitFile, elements};

fn tier1_dir() -> PathBuf {
    std::env::var("RVT_CORPUS_TIER1_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("corpus/tier1"))
}

fn project_dir() -> Option<PathBuf> {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .ok()
        .map(PathBuf::from)
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
fn production_iter_elements_tier1_no_hostobjattr_no_fake_arcwall() {
    let root = tier1_dir();
    assert!(root.is_dir(), "missing {}", root.display());
    for dir in discover_fixture_dirs(&root) {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let path = dir.join(format!("{name}.rvt"));
        let mut rf = RevitFile::open(&path).expect("open tier1");
        let elements: Vec<_> = walker::iter_elements(&mut rf)
            .expect("iter_elements")
            .collect();
        assert!(
            elements.iter().all(|e| e.class != "HostObjAttr"),
            "{name}: production must not yield HostObjAttr"
        );
        assert!(
            elements.iter().all(|e| e.class != "ArcWall"),
            "{name}: tier1 must not invent ArcWall typed hits"
        );
        // Any MVP class that did surface must round-trip through the
        // typed view helpers without panic.
        for el in &elements {
            if is_mvp_typed_class(&el.class) {
                let _ = elements::typed_json::mvp_typed_view(el);
            }
        }
        eprintln!("iter_elements tier1 ok · {name} · count={}", elements.len());
    }
}

#[test]
fn decode_instance_prefer_typed_fails_closed_for_mvp_wrong_schema() {
    // Build a minimal Wall-named schema, then ask prefer_typed with a
    // *Floor* ClassEntry name while feeding Wall decoder expectations
    // via decode_typed reject path.
    let floor_schema = rvt::formats::ClassEntry {
        name: "Floor".into(),
        offset: 0,
        fields: vec![],
        tag: None,
        parent: None,
        declared_field_count: None,
        was_parent_only: false,
        ancestor_tag: None,
    };
    // Floor is MVP → typed path. Empty schema decode succeeds (generic
    // walk of zero fields). Wrong-schema is enforced when the
    // *decoder* name disagrees with schema.name:
    let wall = elements::decoder_for_class("Wall").unwrap();
    assert!(
        wall.decode(&[], &floor_schema, &rvt::walker::HandleIndex::new())
            .is_err(),
        "WallDecoder must reject Floor schema"
    );
    // Unregistered class → decode_typed fails closed.
    let bogus = rvt::formats::ClassEntry {
        name: "NoSuchClass".into(),
        ..floor_schema
    };
    assert!(elements::decode_typed(&[], &bogus, &rvt::walker::HandleIndex::new()).is_err());
    assert!(MVP_TYPED_CLASSES.contains(&"Wall"));
}

#[test]
fn einhoven_iter_elements_yields_typed_arcwalls_with_location_curves() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open Einhoven");
    let elements: Vec<_> = walker::iter_elements(&mut rf)
        .expect("iter_elements")
        .collect();

    assert!(
        elements.iter().all(|e| e.class != "HostObjAttr"),
        "production must filter HostObjAttr: {:?}",
        elements.iter().map(|e| &e.class).collect::<Vec<_>>()
    );

    let arcwalls: Vec<_> = elements.iter().filter(|e| e.class == "ArcWall").collect();
    assert!(
        arcwalls.len() >= 20,
        "expected ≥20 ArcWall DecodedElements from production iter_elements, got {}",
        arcwalls.len()
    );

    let mut curves = 0usize;
    for el in &arcwalls {
        // Prefer geometry recovery over the DecodedElement field view.
        if recover_wall_location_curve(el).is_recovered() {
            curves += 1;
            continue;
        }
        // Fail closed path: reconstruct typed ArcWall from partition
        // via element id is not required here — field endpoints alone.
    }
    assert!(
        curves >= 20,
        "expected ≥20 recovered wall location curves from ArcWall fields, got {curves}"
    );

    // Diagnostic path may still expose HostObjAttr.
    let mut diag_rf = RevitFile::open(&path).unwrap();
    let diagnostic: Vec<_> =
        walker::iter_elements_with_options(&mut diag_rf, walker::DIAGNOSTIC_ELEMENT_MIN_SCORE)
            .unwrap()
            .collect();
    assert!(
        diagnostic.iter().any(|e| e.class == "HostObjAttr"),
        "diagnostic scan should still find HostObjAttr"
    );

    eprintln!(
        "iter_elements Einhoven ok · total={} · arcwalls={} · curves={} · production_min_score={PRODUCTION_ELEMENT_MIN_SCORE}",
        elements.len(),
        arcwalls.len(),
        curves
    );
}

#[test]
fn einhoven_geometry_p0_levels_and_arcwall_partition_path() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("Revit_IFC5_Einhoven.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open");
    let version = rf.basic_file_info().unwrap().version;
    assert_eq!(version, 2023);

    // Wall location curves via typed ArcWall partition decoder.
    let scan = rvt::partition_arc_walls::scan_partition_arc_walls(&mut rf, version).unwrap();
    assert!(scan.walls.len() >= 20);
    let mut recovered_curves = 0usize;
    let mut with_elevation = 0usize;
    for wall in &scan.walls {
        let typed = elements::arc_wall::from_partition_arc_wall(wall);
        let loc = recover_wall_location_curve_from_arc_wall(&typed);
        assert!(loc.line_length_feet().unwrap_or(0.0) > 0.0);
        recovered_curves += 1;
        if typed.base_elevation_feet.is_some() {
            with_elevation += 1;
        }
    }
    assert_eq!(recovered_curves, scan.walls.len());
    assert!(
        with_elevation >= 15,
        "expected most ArcWall trailers to carry base elevation, got {with_elevation}"
    );

    // Level elevations: partition Level-like names + ArcWall storey recovery.
    let level_names = {
        let records = rvt::object_graph::string_records_from_partitions(&mut rf).unwrap();
        rvt::partition_name_candidates::building_storey_name_candidates(
            records.iter().map(|r| r.value.as_str()),
        )
    };
    assert!(
        level_names.iter().any(|n| n.contains("Level")),
        "expected Level-like partition names: {level_names:?}"
    );
    let storeys =
        rvt::partition_arc_walls::recover_storeys_from_arc_walls(&scan.walls, &level_names);
    assert!(
        !storeys.storeys.is_empty(),
        "expected elevation-derived storeys"
    );

    // Floor / door / window hosts: honest Absent on scaffold-less
    // partition soup unless a schema-driven hit appears in
    // iter_elements (Global/Latest). Do not invent.
    let mut rf2 = RevitFile::open(&path).unwrap();
    let decoded: Vec<_> = walker::iter_elements(&mut rf2).unwrap().collect();
    let floors: Vec<_> = decoded.iter().filter(|e| e.class == "Floor").collect();
    let doors: Vec<_> = decoded.iter().filter(|e| e.class == "Door").collect();
    let windows: Vec<_> = decoded.iter().filter(|e| e.class == "Window").collect();
    let levels: Vec<_> = decoded.iter().filter(|e| e.class == "Level").collect();

    for floor in &floors {
        let outcome = rvt::geometry::recover_floor_boundary(floor);
        // Real project Floor schema hits may or may not carry sketch
        // vectors yet — Absent is OK; Recovered must have ≥3 verts.
        if let Some(loop_) = outcome.as_recovered() {
            assert!(loop_.vertices_xy.len() >= 3);
        }
    }
    for door in &doors {
        let d = elements::openings::Door::from_decoded(door);
        let _ = rvt::geometry::recover_door_host(&d);
    }
    for window in &windows {
        let w = elements::openings::Window::from_decoded(window);
        let _ = rvt::geometry::recover_window_host(&w);
    }
    for level in &levels {
        let l = elements::level::Level::from_decoded(level);
        let outcome = recover_level_elevation(&l);
        if outcome.is_recovered() {
            assert!(outcome.as_recovered().unwrap().elevation_feet.is_finite());
        }
    }

    eprintln!(
        "geometry P0 Einhoven ok · arcwalls={} · curves={} · elev={} · storeys={} · floors={} · doors={} · windows={} · levels={}",
        scan.walls.len(),
        recovered_curves,
        with_elevation,
        storeys.storeys.len(),
        floors.len(),
        doors.len(),
        windows.len(),
        levels.len()
    );
}
