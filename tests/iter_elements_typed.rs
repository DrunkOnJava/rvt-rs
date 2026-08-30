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
    // Typed Wall / Door / Window come from the partition element-record
    // carrier (#211), never from the opening index or a schema-field
    // decode — RE-19's negative stands and is asserted by provenance
    // below. The counts are the exact ElementId sets Revit's own full
    // export tags: 360 IFCWALL, 132 IFCDOOR, 6 IFCWINDOW.
    let decoded: Vec<_> =
        walker::iter_elements_with_limits(&mut rf, walker::PRODUCTION_ELEMENT_MIN_SCORE, limits)
            .expect("iter_elements")
            .collect();
    for (class, expected) in [("Wall", 360usize), ("Door", 132), ("Window", 6)] {
        let hits: Vec<_> = decoded.iter().filter(|e| e.class == class).collect();
        assert_eq!(
            hits.len(),
            expected,
            "{class}: expected the exported instance count on 2024 Core Interior"
        );
        for hit in &hits {
            assert_eq!(
                hit.provenance.decoder.as_deref(),
                Some("partition_schema_mvp::element_category_record"),
                "RE-19: {class} must come from the partition element record, not a \
                 schema-field or opening-index decode"
            );
        }
    }
    assert!(
        decoded
            .iter()
            .filter(|e| e.class == "ArcWallRectOpening")
            .count()
            >= 50,
        "RE-19: openings must remain class ArcWallRectOpening in production iter_elements"
    );
    let mut elem_confirmed = 0usize;
    for opening in &mvp.rect_openings {
        let a_ok = opening.fields.iter().any(|(n, v)| {
            matches!(
                (n.as_str(), v),
                (
                    "m_related_id_a_in_elem_table",
                    rvt::walker::InstanceField::Bool(true)
                )
            )
        });
        let b_ok = opening.fields.iter().any(|(n, v)| {
            matches!(
                (n.as_str(), v),
                (
                    "m_related_id_b_in_elem_table",
                    rvt::walker::InstanceField::Bool(true)
                )
            )
        });
        if a_ok && b_ok {
            elem_confirmed += 1;
        }
    }
    assert!(
        elem_confirmed >= 50,
        "expected ≥50 openings with both related ids in ElemTable, got {elem_confirmed}"
    );
    // Materials / rooms may still surface from strings even when ArcWall
    // standard decode is version-gated off.
    assert!(
        mvp.materials.len() >= 5,
        "expected material name recovers on 2024, got {}",
        mvp.materials.len()
    );

    eprintln!(
        "2024 Core Interior MVP ok · openings={} · elem_confirmed={} · materials={} · levels={} · floors={} · rooms={}",
        mvp.rect_openings.len(),
        elem_confirmed,
        mvp.materials.len(),
        mvp.levels.len(),
        mvp.floors.len(),
        mvp.rooms.len()
    );
}

/// #212 / RE-22: slabs come from `OST_Floors` + `OST_BuildingPad`
/// element records under the #211 instance rule, the plan-loop floors
/// stand down when they do, and the twenty per-element "IFC Export As"
/// overrides are exactly the ids Revit's own export emits as
/// `IfcShadingDevice`.
#[test]
fn core_interior_2024_slab_instances_and_export_overrides() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }

    // The twenty `IFCSHADINGDEVICE` `Tag` values in
    // `IFC Exports/2024_Core_Interior_slim.ifc`.
    const SHADING_DEVICE_IDS: &[u32] = &[
        20953, 64160, 64227, 64292, 64358, 64423, 64488, 64553, 64618, 64683, 70366, 71171, 71231,
        71291, 71351, 71411, 71471, 71531, 71591, 71651,
    ];
    // `Pad:Site Pad:21975` — the one exported slab that is an
    // `OST_BuildingPad`, not an `OST_Floors`.
    const SITE_PAD_ID: u32 = 21975;

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    assert_eq!(version, 2024);
    let limits = walker::WalkerLimits {
        max_candidates: 2_000,
        ..walker::WalkerLimits::default()
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(&mut rf, version, limits)
        .expect("mvp");

    assert_eq!(
        mvp.slabs.len(),
        100,
        "expected the 100 exported OST_Floors / OST_BuildingPad instances"
    );
    assert!(
        mvp.floors.is_empty(),
        "plan-loop floors must stand down when record-backed slabs decode: \
         emitting both double-counts the same plates"
    );

    let pads: Vec<_> = mvp
        .slabs
        .iter()
        .filter(|e| e.class == "BuildingPad")
        .collect();
    assert_eq!(pads.len(), 1, "one OST_BuildingPad instance");
    assert_eq!(pads[0].id, Some(SITE_PAD_ID));

    let mut overridden: Vec<u32> = mvp
        .slabs
        .iter()
        .filter(|e| {
            e.fields.iter().any(|(name, value)| {
                matches!(
                    (name.as_str(), value),
                    ("m_ifc_export_as", rvt::walker::InstanceField::String(text))
                        if text == "IfcShadingDevice"
                )
            })
        })
        .filter_map(|e| e.id)
        .collect();
    overridden.sort_unstable();
    assert_eq!(
        overridden, SHADING_DEVICE_IDS,
        "the IfcShadingDevice overrides must be exactly the export's IFCSHADINGDEVICE Tag set"
    );

    for slab in &mvp.slabs {
        assert_eq!(
            slab.provenance.decoder.as_deref(),
            Some("partition_schema_mvp::element_category_record"),
            "slabs come from the element-record carrier, not a plan-loop scan"
        );
        assert!(slab.id.is_some(), "every record-backed slab carries an id");
    }
}

/// #31 / RE-25: every recovered slab carries the plan profile its
/// `OST_SketchLines` records close, and the plates whose sketch does
/// not close carry none.
///
/// The join is by ElementId only — the last slot of the second
/// counted reference list at `+0x88` — so this asserts an exact
/// partition of the 100 record-backed plates: the 80 the reference
/// export writes as `IfcSlab` resolve a profile, the 20 it writes as
/// `IfcShadingDevice` (rotated plates, whose sketch-line boxes are
/// axis-aligned envelopes of diagonal segments) resolve none.
#[test]
fn core_interior_2024_slab_plan_profiles() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    // The twenty `IFCSHADINGDEVICE` `Tag` values — the plates whose
    // sketch does not close from bounding boxes alone.
    const SHADING_DEVICE_IDS: &[u32] = &[
        20953, 64160, 64227, 64292, 64358, 64423, 64488, 64553, 64618, 64683, 70366, 71171, 71231,
        71291, 71351, 71411, 71471, 71531, 71591, 71651,
    ];
    const SITE_PAD_ID: u32 = 21975;

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    let limits = walker::WalkerLimits {
        max_candidates: 2_000,
        ..walker::WalkerLimits::default()
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(&mut rf, version, limits)
        .expect("mvp");
    assert_eq!(mvp.slabs.len(), 100);

    let mut without: Vec<u32> = Vec::new();
    let mut ring = 0usize;
    let mut rectangle = 0usize;
    for slab in &mvp.slabs {
        let id = slab.id.expect("record-backed slab carries an id");
        let Some(profile) =
            rvt::element_record_plan_profiles::plan_profile_from_fields(&slab.fields)
        else {
            without.push(id);
            continue;
        };
        // The profile's plan bounds are the record's own plan box:
        // every vertex is a corner of a recorded bounding box.
        let bounds = profile.plan_bounds_feet().expect("bounds");
        let mut record_box = [f64::NAN; 4];
        for (name, value) in &slab.fields {
            if let rvt::walker::InstanceField::Float { value, .. } = value {
                match name.as_str() {
                    "m_locationX" => record_box[0] = *value,
                    "m_locationY" => record_box[1] = *value,
                    "m_bboxWidth" => record_box[2] = *value,
                    "m_bboxDepth" => record_box[3] = *value,
                    _ => {}
                }
            }
        }
        let width = bounds[2] - bounds[0];
        let depth = bounds[3] - bounds[1];
        assert!(
            (width - record_box[2]).abs() < 1e-6 && (depth - record_box[3]).abs() < 1e-6,
            "slab {id}: profile bounds {width} x {depth} disagree with the record box"
        );
        match (profile.outer_xy.len(), profile.inner_xy.len()) {
            (26, 1) => ring += 1,
            (4, 0) => rectangle += 1,
            other => panic!("slab {id}: unexpected profile shape {other:?}"),
        }
        if id == SITE_PAD_ID {
            let mut xs: Vec<f64> = profile.outer_xy.iter().map(|p| p.0).collect();
            let mut ys: Vec<f64> = profile.outer_xy.iter().map(|p| p.1).collect();
            xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
            ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for (got, want) in xs.iter().zip([20.0, 20.0, 167.0, 167.0]) {
                assert!((got - want).abs() < 1e-6, "Site Pad x {got} != {want}");
            }
            for (got, want) in ys.iter().zip([25.0, 25.0, 114.0, 114.0]) {
                assert!((got - want).abs() < 1e-6, "Site Pad y {got} != {want}");
            }
        }
    }
    without.sort_unstable();
    assert_eq!(
        without, SHADING_DEVICE_IDS,
        "the plates without a closed sketch must be exactly the export's \
         IFCSHADINGDEVICE Tag set"
    );
    assert_eq!(
        ring, 42,
        "42 perimeter plates: 26-vertex outer loop + 1 void"
    );
    assert_eq!(rectangle, 38, "38 rectangular plates: 4-vertex outer loop");
}

/// Split a STEP attribute list on top-level commas (quotes and nested
/// lists shield their contents).
fn step_attributes(args: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut start = 0usize;
    for (index, ch) in args.char_indices() {
        match ch {
            '\'' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => depth = depth.saturating_sub(1),
            ',' if !in_string && depth == 0 => {
                out.push(args[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(args[start..].trim());
    out
}

/// `(host Tag, filling Tag)` pairs read out of a Revit-authored IFC by
/// following `IfcRelVoidsElement` into `IfcRelFillsElement`.
///
/// The reference side of the #222 / RE-23 gate: the set the decoder
/// must reproduce exactly.
fn reference_fill_pairs(step: &str) -> Vec<(String, String)> {
    use std::collections::BTreeMap;
    let mut instances: BTreeMap<u32, (String, Vec<String>)> = BTreeMap::new();
    for line in step.lines() {
        let line = line.trim();
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id_text, after_eq)) = rest.split_once('=') else {
            continue;
        };
        let Ok(id) = id_text.trim().parse::<u32>() else {
            continue;
        };
        let Some((ifc_type, args)) = after_eq.trim_start().split_once('(') else {
            continue;
        };
        let Some(args) = args
            .trim_end()
            .strip_suffix(';')
            .map(str::trim_end)
            .and_then(|a| a.strip_suffix(')'))
        else {
            continue;
        };
        instances.insert(
            id,
            (
                ifc_type.trim().to_ascii_uppercase(),
                step_attributes(args)
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
            ),
        );
    }
    let entity_ref = |attribute: &str| -> Option<u32> {
        attribute.trim().strip_prefix('#')?.trim().parse().ok()
    };
    // `Tag` is IfcElement's one attribute past IfcProduct's seven.
    let tag_of = |id: Option<u32>| -> String {
        id.and_then(|id| instances.get(&id))
            .and_then(|(_, attributes)| attributes.get(7))
            .and_then(|attribute| {
                attribute
                    .trim()
                    .strip_prefix('\'')
                    .and_then(|t| t.strip_suffix('\''))
            })
            .unwrap_or_default()
            .to_string()
    };
    let mut voided_by: BTreeMap<u32, u32> = BTreeMap::new();
    for (ifc_type, attributes) in instances.values() {
        if ifc_type != "IFCRELVOIDSELEMENT" {
            continue;
        }
        if let (Some(host), Some(opening)) = (
            attributes.get(4).and_then(|a| entity_ref(a)),
            attributes.get(5).and_then(|a| entity_ref(a)),
        ) {
            voided_by.insert(opening, host);
        }
    }
    let mut pairs = Vec::new();
    for (ifc_type, attributes) in instances.values() {
        if ifc_type != "IFCRELFILLSELEMENT" {
            continue;
        }
        let opening = attributes.get(4).and_then(|a| entity_ref(a));
        let filling = attributes.get(5).and_then(|a| entity_ref(a));
        let host = opening.and_then(|id| voided_by.get(&id).copied());
        pairs.push((tag_of(host), tag_of(filling)));
    }
    pairs.sort();
    pairs
}

/// #222 / RE-23: every recovered door and window binds to the wall
/// Revit's own exporter voids for it — exact pair set, no tolerance.
#[test]
fn core_interior_2024_door_window_host_wall_binding() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    let reference = project_dir.join("../IFC Exports/2024_Core_Interior_slim.ifc");
    if !path.exists() || !reference.exists() {
        eprintln!("skipping: {} or the slim export is missing", path.display());
        return;
    }

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    assert_eq!(version, 2024);
    let limits = walker::WalkerLimits {
        max_candidates: 2_000,
        ..walker::WalkerLimits::default()
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(&mut rf, version, limits)
        .expect("mvp");

    let wall_ids: std::collections::BTreeSet<u32> =
        mvp.walls.iter().filter_map(|wall| wall.id).collect();
    assert_eq!(wall_ids.len(), 360, "the 360 exported wall instances");

    let mut recovered: Vec<(String, String)> = Vec::new();
    let mut unbound: Vec<u32> = Vec::new();
    for opening in mvp.doors.iter().chain(mvp.windows.iter()) {
        let id = opening.id.expect("record-backed opening carries an id");
        let host = opening
            .fields
            .iter()
            .find_map(|(name, value)| match (name.as_str(), value) {
                (
                    rvt::partition_schema_mvp::OPENING_HOST_FIELD,
                    walker::InstanceField::ElementId { id, .. },
                ) => Some(*id),
                _ => None,
            });
        match host {
            Some(host) => {
                assert!(
                    wall_ids.contains(&host),
                    "recovered host {host} for {id} is not an exported wall"
                );
                recovered.push((host.to_string(), id.to_string()));
            }
            None => unbound.push(id),
        }
    }
    recovered.sort();

    let expected = reference_fill_pairs(&std::fs::read_to_string(&reference).expect("read export"));
    assert_eq!(
        expected.len(),
        138,
        "the export voids 132 doors and 6 windows out of their host walls"
    );
    assert!(unbound.is_empty(), "unbound doors/windows: {unbound:?}");
    assert_eq!(
        recovered, expected,
        "recovered (host wall, opening) pairs must equal Revit's own IfcRelVoidsElement \
         + IfcRelFillsElement chain exactly"
    );

    // The binding is what the reference list says, not a proximity
    // guess: every bound opening records its carrier.
    for opening in mvp.doors.iter().chain(mvp.windows.iter()) {
        assert!(
            opening.fields.iter().any(|(name, value)| matches!(
                (name.as_str(), value),
                (
                    rvt::partition_schema_mvp::OPENING_HOST_PROVENANCE_FIELD,
                    walker::InstanceField::String(text),
                ) if text == rvt::partition_schema_mvp::OPENING_HOST_PROVENANCE
            )),
            "opening {:?} lacks host provenance",
            opening.id
        );
    }
}

/// RE-26 (#215): every recovered column names its family/type symbol,
/// and the section that symbol carries is the one the instance
/// envelope already had.
///
/// The join is the last slot before the record's own ElementId in the
/// counted reference list at `+0x88` that is itself an `OST_Columns`
/// type-symbol record. On `2024_Core_Interior.rvt` that is `5755`
/// (`Column_Sqaure : 24" x 24"`) on all 256 exported columns, which
/// is the `IfcColumnType.Tag` Revit's own export writes for every one
/// of them.
#[test]
fn core_interior_2024_column_type_symbol_join() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    const COLUMN_TYPE_SYMBOL: u32 = 5755;

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    let columns =
        rvt::partition_schema_mvp::columns_from_partition_category_records(&mut rf, version)
            .expect("columns");
    assert_eq!(
        columns.len(),
        256,
        "the #211 instance rule still selects 256"
    );

    let mut joined = 0usize;
    for column in &columns {
        let mut symbol = None;
        let mut section = (None, None);
        let mut width = None;
        let mut depth = None;
        for (name, value) in &column.fields {
            match (name.as_str(), value) {
                (
                    rvt::partition_schema_mvp::TYPE_SYMBOL_FIELD,
                    walker::InstanceField::ElementId { id, .. },
                ) => symbol = Some(*id),
                (
                    rvt::partition_schema_mvp::TYPE_PROFILE_WIDTH_FIELD,
                    walker::InstanceField::Float { value, .. },
                ) => section.0 = Some(*value),
                (
                    rvt::partition_schema_mvp::TYPE_PROFILE_DEPTH_FIELD,
                    walker::InstanceField::Float { value, .. },
                ) => section.1 = Some(*value),
                ("m_bboxWidth", walker::InstanceField::Float { value, .. }) => {
                    width = Some(*value);
                }
                ("m_bboxDepth", walker::InstanceField::Float { value, .. }) => {
                    depth = Some(*value);
                }
                _ => {}
            }
        }
        let Some(symbol) = symbol else { continue };
        assert_eq!(
            symbol, COLUMN_TYPE_SYMBOL,
            "column {:?} joined to an unexpected type symbol",
            column.id
        );
        let (section_width, section_depth) = (
            section.0.expect("width travels with the join"),
            section.1.expect("depth travels with the join"),
        );
        // The guard the join ships with: the type's section and the
        // instance's plan envelope must agree, or the join is dropped.
        assert!(
            (section_width - width.expect("bbox width")).abs() < 1e-6
                && (section_depth - depth.expect("bbox depth")).abs() < 1e-6,
            "column {:?}: section {section_width} x {section_depth} disagrees with its envelope",
            column.id
        );
        assert!(
            (section_width - 2.0).abs() < 1e-6 && (section_depth - 2.0).abs() < 1e-6,
            "the 24\" x 24\" symbol is a 2 ft square"
        );
        joined += 1;
    }
    assert_eq!(joined, 256, "every recovered column names its type symbol");
}

/// RE-26: a wall's record box is the untrimmed prism and the joins cut
/// it back by half the thickness of the wall each end lands on.
///
/// The measured claim on `2024_Core_Interior.rvt`: all 360 walls
/// resolve (none declines), 329 of them take a trim at one end or
/// both, and the thin plan extent the solver reads as the thickness is
/// one of the file's three wall thicknesses everywhere. Wall 20800 is
/// walked explicitly because it is the arity gate's pinned wall.
#[test]
fn core_interior_2024_wall_join_trimmed_bodies() {
    let Some(project_dir) = project_dir() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR unset");
        return;
    };
    let path = project_dir.join("2024_Core_Interior.rvt");
    if !path.exists() {
        eprintln!("skipping: {} missing", path.display());
        return;
    }
    use rvt::element_record_wall_joins as joins;

    let mut rf = RevitFile::open(&path).expect("open 2024");
    let version = rf.basic_file_info().unwrap().version;
    let walls = rvt::partition_schema_mvp::walls_from_partition_category_records(&mut rf, version)
        .expect("walls");
    assert_eq!(walls.len(), 360, "the #211 instance rule still selects 360");

    let mut resolved = 0usize;
    let mut trimmed = 0usize;
    for wall in &walls {
        let mut source = None;
        let mut thickness = None;
        let mut trim = (None, None);
        let mut plan = (None, None, None, None);
        for (name, value) in &wall.fields {
            match (name.as_str(), value) {
                (joins::WALL_BODY_SOURCE_FIELD, walker::InstanceField::String(text)) => {
                    source = Some(text.clone());
                }
                (joins::WALL_THICKNESS_FIELD, walker::InstanceField::Float { value, .. }) => {
                    thickness = Some(*value);
                }
                (joins::WALL_TRIM_START_FIELD, walker::InstanceField::Float { value, .. }) => {
                    trim.0 = Some(*value);
                }
                (joins::WALL_TRIM_END_FIELD, walker::InstanceField::Float { value, .. }) => {
                    trim.1 = Some(*value);
                }
                ("m_locationX", walker::InstanceField::Float { value, .. }) => {
                    plan.0 = Some(*value)
                }
                ("m_locationY", walker::InstanceField::Float { value, .. }) => {
                    plan.1 = Some(*value)
                }
                ("m_bboxWidth", walker::InstanceField::Float { value, .. }) => {
                    plan.2 = Some(*value)
                }
                ("m_bboxDepth", walker::InstanceField::Float { value, .. }) => {
                    plan.3 = Some(*value)
                }
                _ => {}
            }
        }
        let Some(source) = source else { continue };
        assert_eq!(source, joins::WALL_BODY_JOIN_TRIMMED);
        resolved += 1;
        let thickness = thickness.expect("thickness travels with the trim");
        assert!(
            [0.5, 2.0 / 3.0, 1.5]
                .iter()
                .any(|nominal| (thickness - nominal).abs() < 1e-4),
            "wall {:?}: thickness {thickness} is not a 6\", 8\" or 18\" wall",
            wall.id
        );
        let (start, end) = (trim.0.expect("start"), trim.1.expect("end"));
        assert!(start >= 0.0 && end >= 0.0, "a join never lengthens a wall");
        if start > 0.0 || end > 0.0 {
            trimmed += 1;
        }
        if wall.id == Some(20800) {
            // Record box x 85.5 -> 137.25 ft, both ends cut by half of
            // an 8" wall. Revit's own export puts this wall's body at
            // 85.8333 -> 136.9167 ft.
            assert!((start - 1.0 / 3.0).abs() < 1e-4, "20800 start trim");
            assert!((end - 1.0 / 3.0).abs() < 1e-4, "20800 end trim");
            let centre = plan.0.expect("x");
            let width = plan.2.expect("width");
            assert!(
                (centre - width / 2.0 - 85.833_333).abs() < 1e-4
                    && (centre + width / 2.0 - 136.916_667).abs() < 1e-4,
                "20800 emits {} +- {}",
                centre,
                width / 2.0
            );
        }
    }
    assert_eq!(resolved, 360, "no wall declines its joins on this file");
    assert_eq!(trimmed, 329, "329 walls are cut at one end or both");
}
