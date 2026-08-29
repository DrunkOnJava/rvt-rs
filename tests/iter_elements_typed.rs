//! Production `iter_elements` typed MVP + partition schema MVP wiring.
//!
//! - Tier1 synthetics: fail-closed honesty (no invented ArcWalls /
//!   Levels / Materials / Floors / openings; HostObjAttr never on
//!   production path).
//! - Optional magnetar project corpus: ArcWalls, Levels, Materials,
//!   Floor plan-loops, and (2024) ArcWallRectOpening rows when
//!   `RVT_PROJECT_CORPUS_DIR` is set.

use std::path::{Path, PathBuf};

use rvt::elements::MVP_TYPED_CLASSES;
use rvt::elements::typed_json::is_mvp_typed_class;
use rvt::geometry::{
    recover_floor_boundary, recover_level_elevation, recover_wall_location_curve,
    recover_wall_location_curve_from_arc_wall,
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
fn production_iter_elements_tier1_no_hostobjattr_no_fake_partition_mvp() {
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
        assert!(
            elements.iter().all(|e| e.class != "ArcWallRectOpening"),
            "{name}: tier1 must not invent opening-index hits"
        );
        // Partition MVP name/geometry recovers must not invent on
        // scaffold CFBs (empty / unsupported partitions).
        for class in ["Level", "Material", "Room", "Floor", "Door", "Window"] {
            let count = elements.iter().filter(|e| e.class == class).count();
            assert_eq!(
                count, 0,
                "{name}: tier1 must not invent partition {class} hits, got {count}"
            );
        }
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
    let wall = elements::decoder_for_class("Wall").unwrap();
    assert!(
        wall.decode(&[], &floor_schema, &rvt::walker::HandleIndex::new())
            .is_err(),
        "WallDecoder must reject Floor schema"
    );
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
        if recover_wall_location_curve(el).is_recovered() {
            curves += 1;
        }
    }
    assert!(
        curves >= 20,
        "expected ≥20 recovered wall location curves from ArcWall fields, got {curves}"
    );

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
fn einhoven_partition_schema_mvp_levels_materials_floors() {
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

    let mut rf2 = RevitFile::open(&path).unwrap();
    let decoded: Vec<_> = walker::iter_elements(&mut rf2).unwrap().collect();
    let floors: Vec<_> = decoded.iter().filter(|e| e.class == "Floor").collect();
    let doors: Vec<_> = decoded.iter().filter(|e| e.class == "Door").collect();
    let windows: Vec<_> = decoded.iter().filter(|e| e.class == "Window").collect();
    let levels: Vec<_> = decoded.iter().filter(|e| e.class == "Level").collect();
    let materials: Vec<_> = decoded.iter().filter(|e| e.class == "Material").collect();
    let rooms: Vec<_> = decoded.iter().filter(|e| e.class == "Room").collect();
    let openings: Vec<_> = decoded
        .iter()
        .filter(|e| e.class == "ArcWallRectOpening")
        .collect();

    // Levels: elevation-derived storeys merge into production iter_elements.
    assert!(
        levels.len() >= 2,
        "expected ≥2 Level DecodedElements from partition storeys, got {}",
        levels.len()
    );
    let mut level_elevations = 0usize;
    for level in &levels {
        let l = elements::level::Level::from_decoded(level);
        assert!(l.name.is_some(), "Level must carry a name");
        let outcome = recover_level_elevation(&l);
        if outcome.is_recovered() {
            assert!(outcome.as_recovered().unwrap().elevation_feet.is_finite());
            level_elevations += 1;
        }
    }
    assert!(
        level_elevations >= 2,
        "expected ≥2 Levels with recovered elevations, got {level_elevations}"
    );

    // Materials: strict partition display names.
    assert!(
        materials.len() >= 5,
        "expected ≥5 Material DecodedElements from partition names, got {}",
        materials.len()
    );
    for mat in &materials {
        let m = elements::styling::Material::from_decoded(mat);
        assert!(m.name.is_some());
    }

    // Floors: ArcWall-excluded plan loops with recoverable boundaries.
    assert!(
        !floors.is_empty(),
        "expected ≥1 Floor plan-loop DecodedElement on Einhoven"
    );
    let mut recovered_boundaries = 0usize;
    for floor in &floors {
        let outcome = recover_floor_boundary(floor);
        if let Some(loop_) = outcome.as_recovered() {
            assert!(loop_.vertices_xy.len() >= 3);
            assert!(loop_.area_sqft() > 0.0);
            recovered_boundaries += 1;
        }
    }
    assert!(
        recovered_boundaries >= 1,
        "expected ≥1 recovered floor boundary, got {recovered_boundaries}"
    );

    // Door/Window: fail closed — do not invent typed Door/Window from
    // 2023 Einhoven (no ArcWallRectOpening envelope on this file).
    assert_eq!(doors.len(), 0, "must not invent Door on Einhoven 2023");
    assert_eq!(windows.len(), 0, "must not invent Window on Einhoven 2023");
    assert_eq!(
        openings.len(),
        0,
        "2023 must not emit 2024-only ArcWallRectOpening rows"
    );

    // Rooms are optional (strict filter may yield 0 on Einhoven).
    eprintln!(
        "partition MVP Einhoven ok · arcwalls={} · curves={} · elev={} · levels={} · level_elev={} · materials={} · floors={} · floor_bounds={} · rooms={} · doors={} · windows={}",
        scan.walls.len(),
        recovered_curves,
        with_elevation,
        levels.len(),
        level_elevations,
        materials.len(),
        floors.len(),
        recovered_boundaries,
        rooms.len(),
        doors.len(),
        windows.len()
    );
}

#[test]
fn core_interior_2024_rect_openings_not_fake_doors() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    assert_eq!(version, 2024);

    // Cap candidates so the large 2024 partition scan stays tractable.
    let limits = walker::WalkerLimits {
        max_candidates: 2_000,
        ..walker::WalkerLimits::default()
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(&mut rf, version, limits)
        .expect("mvp");
    assert!(
        mvp.rect_openings.len() >= 50,
        "expected ≥50 ArcWallRectOpening index rows on 2024 Core Interior, got {}",
        mvp.rect_openings.len()
    );
    assert!(
        mvp.rect_openings
            .iter()
            .all(|e| e.class == "ArcWallRectOpening"),
        "opening index must not be relabeled as Door/Window"
    );
    // Materials / rooms may still surface from strings even when ArcWall
    // standard decode is version-gated off.
    assert!(
        mvp.materials.len() >= 5,
        "expected material name recovers on 2024, got {}",
        mvp.materials.len()
    );

    eprintln!(
        "2024 Core Interior MVP ok · openings={} · materials={} · levels={} · floors={} · rooms={}",
        mvp.rect_openings.len(),
        mvp.materials.len(),
        mvp.levels.len(),
        mvp.floors.len(),
        mvp.rooms.len()
    );
}
