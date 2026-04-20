//! End-to-end integration test: decoded Revit elements →
//! `build_ifc_model` → `write_step` → valid IFC4 STEP file.
//!
//! This test doesn't require any corpus — it synthesizes
//! `DecodedElement` values matching the stable field shape that the
//! Layer-5b decoders produce, wires them through the bridge, and
//! validates the emitted STEP file against IFC4 structural
//! requirements.
//!
//! The output is optionally written to
//! `tests/fixtures/synthetic-project.ifc` when the `DUMP_IFC`
//! environment variable is set — useful for opening in BlenderBIM
//! / IfcOpenShell to verify the file renders as expected.
//!
//! # What gets emitted
//!
//! - IfcProject "Synthetic Test Project"
//! - IfcSite → IfcBuilding → IfcBuildingStorey × 3 (Ground, Second,
//!   Roof Deck) with real elevations 0ft / 10ft / 20ft
//! - IfcWall × 4 (north/south/east/west walls on ground floor)
//! - IfcSlab × 1 (ground floor slab)
//! - IfcDoor × 1 (front door)
//! - IfcWindow × 2 (north/south windows)
//! - IfcStair × 1 (ground-to-second connection)
//! - IfcBuildingElementProxy × 1 (unknown-class fallback sanity check)
//!
//! All elements wire to the first storey via
//! IfcRelContainedInSpatialStructure (per-level containment is a
//! follow-up per IFC-35).

use rvt::ifc::{BuilderOptions, ElementInput, Storey, build_ifc_model, write_step};
use rvt::walker::{DecodedElement, InstanceField};

fn mk_decoded(class: &str) -> DecodedElement {
    DecodedElement {
        id: None,
        class: class.to_string(),
        fields: vec![("name".to_string(), InstanceField::String(class.to_string()))],
        byte_range: 0..0,
    }
}

#[test]
fn synthetic_project_emits_valid_ifc4() {
    // Build the fake building.
    let north_wall = mk_decoded("Wall");
    let south_wall = mk_decoded("Wall");
    let east_wall = mk_decoded("Wall");
    let west_wall = mk_decoded("Wall");
    let floor = mk_decoded("Floor");
    let front_door = mk_decoded("Door");
    let north_window = mk_decoded("Window");
    let south_window = mk_decoded("Window");
    let stair = mk_decoded("Stair");
    let unknown = mk_decoded("AutodeskCustomThing");

    let inputs = vec![
        ElementInput {
            decoded: &north_wall,
            display_name: "North Wall".into(),
            guid: Some("W-N-001".into()),
        },
        ElementInput {
            decoded: &south_wall,
            display_name: "South Wall".into(),
            guid: Some("W-S-001".into()),
        },
        ElementInput {
            decoded: &east_wall,
            display_name: "East Wall".into(),
            guid: Some("W-E-001".into()),
        },
        ElementInput {
            decoded: &west_wall,
            display_name: "West Wall".into(),
            guid: Some("W-W-001".into()),
        },
        ElementInput {
            decoded: &floor,
            display_name: "Ground Floor Slab".into(),
            guid: Some("SLAB-001".into()),
        },
        ElementInput {
            decoded: &front_door,
            display_name: "Front Entry Door".into(),
            guid: Some("DOOR-001".into()),
        },
        ElementInput {
            decoded: &north_window,
            display_name: "North Window".into(),
            guid: Some("WIN-N-001".into()),
        },
        ElementInput {
            decoded: &south_window,
            display_name: "South Window".into(),
            guid: Some("WIN-S-001".into()),
        },
        ElementInput {
            decoded: &stair,
            display_name: "Main Stair".into(),
            guid: Some("STAIR-001".into()),
        },
        ElementInput {
            decoded: &unknown,
            display_name: "Mystery Element".into(),
            guid: None,
        },
    ];

    let opts = BuilderOptions {
        project_name: Some("Synthetic Test Project".into()),
        description: Some("End-to-end rvt-rs pipeline smoke test".into()),
        storeys: vec![
            Storey {
                name: "Ground Floor".into(),
                elevation_feet: 0.0,
            },
            Storey {
                name: "Second Floor".into(),
                elevation_feet: 10.0,
            },
            Storey {
                name: "Roof Deck".into(),
                elevation_feet: 20.0,
            },
        ],
        ..Default::default()
    };

    let model = build_ifc_model(&inputs, opts);
    let step = write_step(&model);

    // --- Structural validation (minimal IFC4 conformance) ---
    assert!(step.starts_with("ISO-10303-21;\n"), "missing header");
    assert!(step.ends_with("END-ISO-10303-21;\n"), "missing terminator");
    assert!(step.contains("FILE_SCHEMA(('IFC4'));"), "wrong schema");
    assert!(step.contains("IFCPROJECT("), "no project");

    // --- Spatial hierarchy ---
    assert_eq!(step.matches("IFCSITE(").count(), 1, "expect 1 site");
    assert_eq!(step.matches("IFCBUILDING(").count(), 1, "expect 1 building");
    assert_eq!(
        step.matches("IFCBUILDINGSTOREY(").count(),
        3,
        "expect 3 storeys"
    );

    // --- Per-element entities ---
    assert_eq!(step.matches("IFCWALL(").count(), 4, "expect 4 walls");
    assert_eq!(step.matches("IFCSLAB(").count(), 1);
    assert_eq!(step.matches("IFCDOOR(").count(), 1);
    assert_eq!(step.matches("IFCWINDOW(").count(), 2);
    assert_eq!(step.matches("IFCSTAIR(").count(), 1);
    assert_eq!(
        step.matches("IFCBUILDINGELEMENTPROXY(").count(),
        1,
        "unknown-class should fall back to proxy"
    );

    // --- Containment rel ties elements to storey ---
    assert_eq!(
        step.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(),
        1,
        "expect one bundled containment rel"
    );

    // --- Named entities round-trip ---
    for name in [
        "Synthetic Test Project",
        "Ground Floor",
        "Second Floor",
        "Roof Deck",
        "North Wall",
        "South Wall",
        "Front Entry Door",
        "Main Stair",
    ] {
        assert!(step.contains(name), "missing '{name}' in STEP output");
    }

    // --- GUIDs round-trip as element Tags ---
    for guid in ["W-N-001", "SLAB-001", "DOOR-001", "STAIR-001"] {
        assert!(step.contains(guid), "missing guid '{guid}' in output");
    }

    // --- Elevation conversion ft → m (10ft = 3.048m, 20ft = 6.096m) ---
    assert!(step.contains("3.048"), "2nd floor elevation missing");
    assert!(step.contains("6.096"), "roof elevation missing");

    // --- Optional: dump to a fixture file when asked ---
    if std::env::var("DUMP_IFC").is_ok() {
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        std::fs::create_dir_all(&out_dir).expect("fixtures dir");
        let out_path = out_dir.join("synthetic-project.ifc");
        std::fs::write(&out_path, &step).expect("write ifc");
        eprintln!("wrote {}", out_path.display());
    }
}

#[test]
fn synthetic_project_is_byte_stable_under_fixed_timestamp() {
    use rvt::ifc::step_writer::{StepOptions, write_step_with_options};
    let wall = mk_decoded("Wall");
    let inputs = vec![ElementInput {
        decoded: &wall,
        display_name: "Stable Wall".into(),
        guid: Some("W-1".into()),
    }];
    let opts = BuilderOptions {
        project_name: Some("Stable".into()),
        ..Default::default()
    };
    let model = build_ifc_model(&inputs, opts);
    let step_opts = StepOptions {
        timestamp: Some(1_700_000_000),
    };
    let a = write_step_with_options(&model, &step_opts);
    let b = write_step_with_options(&model, &step_opts);
    assert_eq!(a, b, "fixed-timestamp output must be byte-stable");
    assert_eq!(a.matches("IFCWALL(").count(), 1);
}
