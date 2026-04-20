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

use rvt::elements::wall::{LocationLine, StructuralUsage, Wall, WallType};
use rvt::ifc::MaterialInfo;
use rvt::ifc::from_decoded::{
    slab_extrusion, wall_extrusion, wall_property_set, window_property_set,
};
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

    // Build typed Wall views for the two walls we'll attach property
    // sets to. In a real pipeline this comes from `Wall::from_decoded`;
    // here we construct directly so the test isn't coupled to
    // schema-fixture bytes.
    let wall_north = Wall {
        base_offset_feet: Some(0.0),
        top_offset_feet: Some(0.0),
        unconnected_height_feet: Some(10.0),
        structural_usage: Some(StructuralUsage::Bearing),
        location_line: Some(LocationLine::WallCenterline),
        ..Default::default()
    };
    // 8" thick walls (0.667 ft). Passed to wall_extrusion below
    // along with the per-wall length in feet.
    let wall_type = WallType {
        width_feet: Some(8.0 / 12.0),
        ..Default::default()
    };
    let win_sample = rvt::elements::openings::Window {
        sill_height_feet: Some(2.5),
        rotation_radians: Some(0.0),
        ..Default::default()
    };

    let inputs = vec![
        ElementInput {
            decoded: &north_wall,
            display_name: "North Wall".into(),
            guid: Some("W-N-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: Some(wall_property_set(&wall_north)),
            // Building footprint is 20 ft × 10 ft. Walls sit on the
            // edges; their placement origins are the near-corner of
            // each wall segment.
            location_feet: Some([0.0, 10.0, 0.0]),
            rotation_radians: Some(0.0),
            // North/South walls run 20 ft east-west, 8" thick, 10 ft high.
            extrusion: Some(wall_extrusion(&wall_north, Some(&wall_type), 20.0)),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &south_wall,
            display_name: "South Wall".into(),
            guid: Some("W-S-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: None,
            location_feet: Some([0.0, 0.0, 0.0]),
            rotation_radians: Some(0.0),
            extrusion: Some(wall_extrusion(&wall_north, Some(&wall_type), 20.0)),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &east_wall,
            display_name: "East Wall".into(),
            guid: Some("W-E-001".into()),
            material_index: Some(0),
            storey_index: Some(1),
            property_set: None,
            // East wall on the second floor — 10 ft up, 20 ft east.
            // Rotated 90° so it runs +Y.
            location_feet: Some([20.0, 0.0, 10.0]),
            rotation_radians: Some(std::f64::consts::FRAC_PI_2),
            // East/West walls run 10 ft in profile-local X (which is
            // world-Y after the 90° rotation).
            extrusion: Some(wall_extrusion(&wall_north, Some(&wall_type), 10.0)),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &west_wall,
            display_name: "West Wall".into(),
            guid: Some("W-W-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: None,
            location_feet: Some([0.0, 0.0, 0.0]),
            rotation_radians: Some(std::f64::consts::FRAC_PI_2),
            extrusion: Some(wall_extrusion(&wall_north, Some(&wall_type), 10.0)),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &floor,
            display_name: "Ground Floor Slab".into(),
            guid: Some("SLAB-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: None,
            // Slab footprint: 20 ft × 10 ft × 1 ft thick, lying flat
            // at z = -1 ft (top of slab coincides with storey origin).
            location_feet: Some([0.0, 0.0, -1.0]),
            rotation_radians: Some(0.0),
            extrusion: Some(slab_extrusion(20.0, 10.0, None)),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &front_door,
            display_name: "Front Entry Door".into(),
            guid: Some("DOOR-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: None,
            // Door sits mid-way along the south wall (10 ft east).
            // 3 ft wide × 8" thick × 7 ft tall.
            location_feet: Some([10.0, 0.0, 0.0]),
            rotation_radians: Some(0.0),
            extrusion: Some(rvt::ifc::entities::Extrusion {
                width_feet: 3.0,
                depth_feet: 8.0 / 12.0,
                height_feet: 7.0,
                profile_override: None,
            }),
            // South wall is at inputs[1]. Writer emits
            // IfcOpeningElement + IfcRelVoidsElement(wall → opening)
            // + IfcRelFillsElement(opening → door) so the wall shows
            // an actual hole where the door goes.
            host_element_index: Some(1),
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &north_window,
            display_name: "North Window".into(),
            guid: Some("WIN-N-001".into()),
            material_index: Some(1),
            storey_index: Some(0),
            property_set: Some(window_property_set(&win_sample)),
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &south_window,
            display_name: "South Window".into(),
            guid: Some("WIN-S-001".into()),
            material_index: Some(1),
            storey_index: Some(0),
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &stair,
            display_name: "Main Stair".into(),
            guid: Some("STAIR-001".into()),
            material_index: Some(0),
            storey_index: Some(0),
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
        },
        ElementInput {
            decoded: &unknown,
            display_name: "Mystery Element".into(),
            guid: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: None,
            rotation_radians: None,
            extrusion: None,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
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
        materials: vec![
            // material_index 0: walls + slab
            MaterialInfo {
                name: "Concrete".into(),
                color_packed: Some(0x00AAAAAA), // light grey
                transparency: Some(0.0),
            },
            // material_index 1: windows
            MaterialInfo {
                name: "Glass - Tinted".into(),
                color_packed: Some(0x00DDAA88), // blue-ish (B=0x88, G=0xAA, R=0xDD)
                transparency: Some(0.6),
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

    // --- Containment rels group elements by storey ---
    // 9 elements on Ground (storey 0, incl. the None → default),
    // 1 element on Second Floor (storey 1, the East Wall),
    // 0 on Roof Deck → 2 IfcRelContainedInSpatialStructure
    // entities. If you move an element between storeys, update
    // this count.
    assert_eq!(
        step.matches("IFCRELCONTAINEDINSPATIALSTRUCTURE(").count(),
        2,
        "expect 2 containment rels (Ground + Second)"
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

    // --- Materials emit IfcMaterial + IfcSurfaceStyle + RelAssociates ---
    // 2 materials (Concrete + Glass - Tinted) → 2 IFCMATERIAL + 2
    // IFCSURFACESTYLE (both have color). 2 IFCRELASSOCIATESMATERIAL
    // bundling concrete to 7 elements (4 walls + slab + door + stair)
    // and glass to 2 elements (N/S windows). Mystery Element has no
    // material — not counted.
    assert_eq!(
        step.matches("IFCMATERIAL(").count(),
        2,
        "expect 2 materials"
    );
    assert_eq!(
        step.matches("IFCSURFACESTYLE(").count(),
        2,
        "each material with color gets a surface style"
    );
    assert_eq!(
        step.matches("IFCRELASSOCIATESMATERIAL(").count(),
        2,
        "one rel per material (concrete + glass)"
    );
    assert!(step.contains("Concrete"), "concrete material name");
    assert!(step.contains("Glass - Tinted"), "glass material name");

    // --- Property sets attached to wall + window ---
    // North Wall carries Pset_WallCommon (BaseOffset, TopOffset,
    // UnconnectedHeight, StructuralUsage, LocationLine = 5 properties).
    // North Window carries Pset_WindowCommon (SillHeight, Rotation
    // = 2 properties). Each produces one IFCPROPERTYSET +
    // IFCRELDEFINESBYPROPERTIES.
    assert!(
        step.contains("'Pset_WallCommon'"),
        "missing Pset_WallCommon"
    );
    assert!(
        step.contains("'Pset_WindowCommon'"),
        "missing Pset_WindowCommon"
    );
    assert!(step.contains("'BaseOffset'"), "BaseOffset property");
    assert!(step.contains("'SillHeight'"), "SillHeight property");
    assert_eq!(
        step.matches("IFCPROPERTYSET(").count(),
        2,
        "expect 2 property sets"
    );
    assert_eq!(
        step.matches("IFCRELDEFINESBYPROPERTIES(").count(),
        2,
        "one rel per property set"
    );
    // SillHeight = 2.5 ft → 0.762 m, IfcLengthMeasure.
    assert!(
        step.contains("IFCLENGTHMEASURE(0.762000)"),
        "2.5ft → 0.762m"
    );

    // --- Per-element placements (IFC-25) ---
    // Four walls carry location_feet. North wall at (0, 10 ft, 0)
    // → (0, 3.048, 0) metres. East wall at (20 ft, 0, 10 ft) →
    // (6.096, 0, 3.048) metres.
    assert!(
        step.contains("IFCCARTESIANPOINT((0.000000,3.048000,0.000000))"),
        "North Wall placement point missing"
    );
    assert!(
        step.contains("IFCCARTESIANPOINT((6.096000,0.000000,3.048000))"),
        "East Wall placement point missing"
    );
    // East + West walls rotated π/2 → X-axis IfcDirection (0,1,0).
    assert!(
        step.contains("IFCDIRECTION((0.000000,1.000000,0.))"),
        "rotated X axis direction missing"
    );

    // --- Extrusion geometry (IFC-16) ---
    // 4 walls + 1 slab + 1 door = 6 elements with Extrusion.
    // The door ALSO triggers an opening-element chain (same shape
    // used as the subtraction volume), so the chain-entity counts
    // are 7 each (6 elements + 1 opening clone of the door shape).
    assert_eq!(
        step.matches("IFCRECTANGLEPROFILEDEF(").count(),
        7,
        "expect 7 rectangle profiles (6 elements + 1 opening)"
    );
    assert_eq!(
        step.matches("IFCEXTRUDEDAREASOLID(").count(),
        7,
        "expect 7 extruded solids"
    );
    assert_eq!(
        step.matches("IFCSHAPEREPRESENTATION(").count(),
        7,
        "expect 7 shape representations"
    );
    assert_eq!(
        step.matches("IFCPRODUCTDEFINITIONSHAPE(").count(),
        7,
        "expect 7 product-definition shapes"
    );
    // Slab profile: 20 ft × 10 ft = 6.096 m × 3.048 m; 1 ft thick.
    assert!(
        step.contains(",6.096000,3.048000)"),
        "slab profile dims missing (20' × 10')"
    );

    // --- Opening voids (IFC-37 / IFC-38) ---
    // Front door has host_element_index = 1 (South Wall). Writer
    // emits IfcOpeningElement + IfcRelVoidsElement + IfcRelFillsElement.
    assert_eq!(
        step.matches("IFCOPENINGELEMENT(").count(),
        1,
        "expect 1 opening element for the front door"
    );
    assert_eq!(
        step.matches("IFCRELVOIDSELEMENT(").count(),
        1,
        "expect 1 IfcRelVoidsElement(wall, opening)"
    );
    assert_eq!(
        step.matches("IFCRELFILLSELEMENT(").count(),
        1,
        "expect 1 IfcRelFillsElement(opening, door)"
    );
    assert!(
        step.contains("Opening for Front Entry Door"),
        "opening should carry the host element's derived name"
    );
    // N/S walls = 20 ft long = 6.096 m profile width; 8" = 0.2032 m
    // thick; 10 ft = 3.048 m high. Floats round to 6 decimals.
    assert!(
        step.contains("IFCRECTANGLEPROFILEDEF(.AREA.,$,") && step.contains(",6.096000,0.203200)"),
        "N/S wall profile dims missing (20' × 8\")"
    );
    // E/W walls = 10 ft long = 3.048 m profile width.
    assert!(
        step.contains(",3.048000,0.203200)"),
        "E/W wall profile dims missing (10' × 8\")"
    );

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
        storey_index: None,
        material_index: None,
        property_set: None,
        location_feet: None,
        rotation_radians: None,
        extrusion: None,
        host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
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
