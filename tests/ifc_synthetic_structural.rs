//! IFC-44: second synthetic integration test covering structural
//! framing + representation-map (shared geometry) + richer solid
//! shapes. Complements `ifc_synthetic_project.rs` (which focuses
//! on architectural walls / doors / windows / slabs / stairs).
//!
//! Built elements:
//!
//! | Element | IFC entity | Exercises |
//! |---|---|---|
//! | 1 IfcColumn | IFCCOLUMN | IFC-24 I-shape profile |
//! | 1 IfcColumn | IFCCOLUMN | IFC-24 circle profile |
//! | 2 IfcBeam | IFCBEAM | IFC-24 wide-flange + material profile set |
//! | 3 IfcDoor | IFCDOOR | IFC-21 shared RepresentationMap + 3 IfcMappedItem |
//! | 1 IfcFurniture | IFCFURNITURE | IFC-20 faceted-brep geometry |
//!
//! If the writer regresses — loses a representation map, miscounts
//! mapped items, changes the profile emission — this test catches
//! it with a specific failure message per entity category.
//!
//! Output optionally dumped to `tests/fixtures/synthetic-structural.ifc`
//! when `DUMP_IFC_STRUCTURAL=1` is set (matches the `DUMP_IFC`
//! pattern in `ifc_synthetic_project.rs`).

use rvt::ifc::entities::{
    BrepTriangle, Extrusion, IfcEntity, MaterialProfile, MaterialProfileSet, RepresentationMap,
    SolidShape,
};
use rvt::ifc::{IfcModel, MaterialInfo, Storey, write_step};

fn pick_owner_history_placeholder(s: &str) -> bool {
    s.contains("IFCOWNERHISTORY(")
}

fn count(s: &str, pat: &str) -> usize {
    s.matches(pat).count()
}

#[test]
fn synthetic_structural_ifc_has_expected_entity_counts() {
    // Build the model: 1 storey, 2 columns (I-shape + circle), 2 beams
    // with profile sets, 3 doors sharing one RepresentationMap, and 1
    // piece of furniture with faceted-brep geometry.

    let a992_steel = MaterialInfo {
        name: "A992 Steel".into(),
        color_packed: None,
        transparency: None,
    };
    let oak = MaterialInfo {
        name: "Wood - Oak".into(),
        color_packed: None,
        transparency: None,
    };

    // IFC-30: a shared MaterialProfileSet for the two beams.
    let w12x26_profile_set = MaterialProfileSet {
        name: "W12x26".into(),
        description: Some("AISC W12x26 wide-flange".into()),
        profiles: vec![MaterialProfile {
            material_index: 0,
            profile_name: "W12x26".into(),
            description: None,
        }],
    };

    // IFC-21: one RepresentationMap shared across 3 door instances.
    let door_map = RepresentationMap {
        name: Some("Standard Interior Door".into()),
        shape: SolidShape::ExtrudedArea(Extrusion::rectangle(
            3.0,        // 3 ft wide
            8.0 / 12.0, // 8" thick
            7.0,        // 7 ft tall
        )),
        origin_feet: [0.0, 0.0, 0.0],
    };

    // IFC-20: a tetrahedral furniture piece (minimal faceted brep).
    let tetrahedron = SolidShape::FacetedBrep {
        vertices_feet: vec![
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [0.0, 0.0, 2.0],
        ],
        triangles: vec![
            BrepTriangle(0, 1, 2),
            BrepTriangle(0, 1, 3),
            BrepTriangle(1, 2, 3),
            BrepTriangle(0, 2, 3),
        ],
    };

    let model = IfcModel {
        project_name: Some("Synthetic Structural Project (IFC-44)".into()),
        description: Some(
            "Second synthetic fixture — columns / beams / shared doors / \
             faceted-brep furniture — for regression-proofing the IFC-17 / \
             IFC-20 / IFC-21 / IFC-24 / IFC-30 emission paths."
                .into(),
        ),
        entities: vec![
            // I-shape column (approx AISC W12x26).
            IfcEntity::BuildingElement {
                ifc_type: "IFCCOLUMN".into(),
                name: "C1 (W12x26)".into(),
                type_guid: Some("C-W12x26-1".into()),
                storey_index: Some(0),
                material_index: Some(0),
                property_set: None,
                location_feet: Some([0.0, 0.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: Some(Extrusion::i_shape(
                    6.49 / 12.0,
                    12.2 / 12.0,
                    0.23 / 12.0,
                    0.38 / 12.0,
                    10.0,
                )),
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            // Round column (1-ft-diameter concrete pier).
            IfcEntity::BuildingElement {
                ifc_type: "IFCCOLUMN".into(),
                name: "C2 (Round)".into(),
                type_guid: Some("C-Round-1".into()),
                storey_index: Some(0),
                material_index: Some(0),
                property_set: None,
                location_feet: Some([10.0, 0.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: Some(Extrusion::circle(0.5, 10.0)),
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            },
            // Beam with IFC-30 MaterialProfileSet (index 0).
            IfcEntity::BuildingElement {
                ifc_type: "IFCBEAM".into(),
                name: "B1 (W12x26, bay 1)".into(),
                type_guid: Some("B-W12x26-1".into()),
                storey_index: Some(0),
                material_index: None,
                property_set: None,
                location_feet: Some([0.0, 0.0, 10.0]),
                rotation_radians: Some(0.0),
                extrusion: Some(Extrusion::i_shape(
                    6.49 / 12.0,
                    12.2 / 12.0,
                    0.23 / 12.0,
                    0.38 / 12.0,
                    20.0,
                )),
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: Some(0),
                solid_shape: None,
                representation_map_index: None,
            },
            IfcEntity::BuildingElement {
                ifc_type: "IFCBEAM".into(),
                name: "B2 (W12x26, bay 2)".into(),
                type_guid: Some("B-W12x26-2".into()),
                storey_index: Some(0),
                material_index: None,
                property_set: None,
                location_feet: Some([0.0, 10.0, 10.0]),
                rotation_radians: Some(0.0),
                extrusion: Some(Extrusion::i_shape(
                    6.49 / 12.0,
                    12.2 / 12.0,
                    0.23 / 12.0,
                    0.38 / 12.0,
                    20.0,
                )),
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: Some(0),
                solid_shape: None,
                representation_map_index: None,
            },
            // Three doors, all referencing RepresentationMap 0.
            IfcEntity::BuildingElement {
                ifc_type: "IFCDOOR".into(),
                name: "D1".into(),
                type_guid: Some("D-Standard".into()),
                storey_index: Some(0),
                material_index: None,
                property_set: None,
                location_feet: Some([2.0, 2.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: Some(0),
            },
            IfcEntity::BuildingElement {
                ifc_type: "IFCDOOR".into(),
                name: "D2".into(),
                type_guid: Some("D-Standard".into()),
                storey_index: Some(0),
                material_index: None,
                property_set: None,
                location_feet: Some([7.0, 2.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: Some(0),
            },
            IfcEntity::BuildingElement {
                ifc_type: "IFCDOOR".into(),
                name: "D3".into(),
                type_guid: Some("D-Standard".into()),
                storey_index: Some(0),
                material_index: None,
                property_set: None,
                location_feet: Some([12.0, 2.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: Some(0),
            },
            // Furniture with faceted-brep geometry.
            IfcEntity::BuildingElement {
                ifc_type: "IFCFURNITURE".into(),
                name: "F1 (Custom chair)".into(),
                type_guid: None,
                storey_index: Some(0),
                material_index: Some(1),
                property_set: None,
                location_feet: Some([5.0, 5.0, 0.0]),
                rotation_radians: Some(0.0),
                extrusion: None,
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: Some(tetrahedron),
                representation_map_index: None,
            },
        ],
        classifications: Vec::new(),
        units: Vec::new(),
        building_storeys: vec![Storey {
            name: "Ground Floor".into(),
            elevation_feet: 0.0,
        }],
        materials: vec![a992_steel, oak],
        material_layer_sets: Vec::new(),
        material_profile_sets: vec![w12x26_profile_set],
        representation_maps: vec![door_map],
    };

    let step = write_step(&model);

    // --- Structural / profile counts ---
    // Two columns, both with distinct profile emission.
    assert_eq!(
        count(&step, "IFCCOLUMN("),
        2,
        "expected 2 IFCCOLUMN entities"
    );
    // Two beams.
    assert_eq!(count(&step, "IFCBEAM("), 2, "expected 2 IFCBEAM entities");
    // IFC-24: I-shape profile emitted (W12x26 columns + beams =
    // 3 I-shape emissions). Round column emits CIRCLEPROFILEDEF.
    assert_eq!(
        count(&step, "IFCISHAPEPROFILEDEF("),
        3,
        "expected 3 IFCISHAPEPROFILEDEF entities (C1 + B1 + B2)"
    );
    assert_eq!(
        count(&step, "IFCCIRCLEPROFILEDEF("),
        1,
        "expected 1 IFCCIRCLEPROFILEDEF entity (C2)"
    );

    // --- IFC-30: material profile set ---
    assert_eq!(
        count(&step, "IFCMATERIALPROFILESET("),
        1,
        "expected 1 shared IFCMATERIALPROFILESET"
    );
    assert!(
        step.contains("IFCMATERIALPROFILESETUSAGE("),
        "beams with material_profile_set_index must emit usage"
    );

    // --- IFC-21: shared representation map ---
    assert_eq!(
        count(&step, "IFCREPRESENTATIONMAP("),
        1,
        "expected exactly 1 shared RepresentationMap (3 instances share it)"
    );
    assert_eq!(
        count(&step, "IFCMAPPEDITEM("),
        3,
        "expected 3 IFCMAPPEDITEM entities (one per door instance)"
    );
    assert_eq!(
        count(&step, "'Body','MappedRepresentation'"),
        3,
        "expected 3 IfcShapeRepresentation with 'MappedRepresentation' type"
    );

    // --- IFC-20: faceted brep ---
    assert_eq!(
        count(&step, "IFCFACETEDBREP("),
        1,
        "expected 1 IFCFACETEDBREP (the tetrahedron)"
    );
    assert_eq!(
        count(&step, "IFCPOLYLOOP("),
        4,
        "tetrahedron has 4 faces → 4 IFCPOLYLOOP entities"
    );
    assert!(step.contains("IFCCLOSEDSHELL("));

    // --- Framework baseline ---
    assert_eq!(count(&step, "IFCPROJECT("), 1);
    assert_eq!(count(&step, "IFCBUILDINGSTOREY("), 1);
    assert_eq!(count(&step, "IFCFURNITURE("), 1);
    assert_eq!(count(&step, "IFCDOOR("), 3);
    assert!(pick_owner_history_placeholder(&step));

    // Schema header.
    assert!(step.contains("IFC4"));
    assert!(step.starts_with("ISO-10303-21;"));

    // --- Optional: dump fixture for manual inspection ---
    if std::env::var("DUMP_IFC_STRUCTURAL").is_ok() {
        let out_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        std::fs::create_dir_all(&out_dir).expect("fixtures dir");
        let out_path = out_dir.join("synthetic-structural.ifc");
        std::fs::write(&out_path, &step).expect("write ifc");
        eprintln!("wrote {}", out_path.display());
    }
}
