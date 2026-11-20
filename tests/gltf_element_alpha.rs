//! A body drawn whole keeps the opacity of the materials it is made of
//! (#1812). The glTF exporter has to choose one material for an element it
//! draws as a single box: it keeps the category colour, and must take the
//! **most transparent** material of the element's constituent set (the glass
//! of a glass door) rather than the category's opaque alpha. The same rule
//! covers a single material whose colour was not read, a single-profile set,
//! and a layer set whose layers the body is not drawn in.
//!
//! This is a contract test: it builds the `IfcModel` the exporter takes and
//! reads the glTF document it produces. It needs no Revit file.

use rvt::ifc::entities::{
    Extrusion, IfcEntity, MaterialConstituentSet, MaterialLayer, MaterialLayerSet, MaterialProfile,
    MaterialProfileSet,
};
use rvt::ifc::gltf::{GltfDocument, Material, SOURCE_RVT_RS, build_gltf};
use rvt::ifc::{IfcModel, MaterialInfo};

fn material(name: &str, color: Option<u32>, transparency: Option<f64>) -> MaterialInfo {
    MaterialInfo {
        name: name.into(),
        color_packed: color,
        transparency,
    }
}

fn element(
    ifc_type: &str,
    material_index: Option<usize>,
    layer_set: Option<usize>,
    profile_set: Option<usize>,
) -> IfcEntity {
    IfcEntity::BuildingElement {
        ifc_type: ifc_type.into(),
        name: format!("{ifc_type} 1"),
        type_guid: None,
        predefined_type: None,
        storey_index: None,
        material_index,
        property_set: None,
        location_feet: Some([0.0, 0.0, 0.0]),
        rotation_radians: Some(0.0),
        extrusion: Some(Extrusion::rectangle(3.0, 8.0 / 12.0, 7.0)),
        host_element_index: None,
        material_layer_set_index: layer_set,
        material_profile_set_index: profile_set,
        solid_shape: None,
        representation_map_index: None,
    }
}

fn model(materials: Vec<MaterialInfo>, entities: Vec<IfcEntity>) -> IfcModel {
    IfcModel {
        materials,
        entities,
        ..IfcModel::default()
    }
}

fn category<'a>(doc: &'a GltfDocument, ifc_type: &str) -> &'a Material {
    let name = format!("Category {ifc_type}");
    doc.materials
        .iter()
        .find(|material| material.name.as_deref() == Some(name.as_str()))
        .unwrap_or_else(|| panic!("no category material {name}"))
}

fn alpha(material: &Material) -> f32 {
    material.pbr_metallic_roughness.base_color_factor[3]
}

#[test]
fn a_multi_material_element_takes_its_most_transparent_alpha() {
    let mut model = model(
        vec![
            material("Glass", Some(0x8F_C1_FF), Some(0.9)),
            material("Frame", Some(0x33_33_33), Some(0.0)),
        ],
        vec![element("IFCDOOR", None, None, None)],
    );
    model.material_constituent_sets = vec![MaterialConstituentSet {
        material_indices: vec![0, 1],
        names: Vec::new(),
        elements: vec![0],
    }];

    let (doc, _) = build_gltf(&model);
    let tinted = category(&doc, "IFCDOOR");
    assert!(
        (alpha(tinted) - 0.1).abs() < 1e-5,
        "the glass (0.9 transparent) gives alpha 0.1, got {}",
        alpha(tinted)
    );
    assert_eq!(tinted.alpha_mode.as_deref(), Some("BLEND"));
    assert!(
        tinted.double_sided,
        "a translucent whole body is double-sided"
    );
    assert_eq!(
        tinted
            .extras
            .as_ref()
            .map(|extras| extras.alpha_source.as_str()),
        Some(SOURCE_RVT_RS),
        "the chosen alpha is rvt-rs's, not the file's"
    );
}

#[test]
fn all_opaque_or_unread_materials_keep_the_category_opaque() {
    let mut model = model(
        vec![
            material("Paint", Some(0x11_22_33), Some(0.0)),
            material("Unread", Some(0x44_55_66), None),
        ],
        vec![element("IFCDOOR", None, None, None)],
    );
    model.material_constituent_sets = vec![MaterialConstituentSet {
        material_indices: vec![0, 1],
        names: Vec::new(),
        elements: vec![0],
    }];

    let (doc, _) = build_gltf(&model);
    let opaque = category(&doc, "IFCDOOR");
    assert_eq!(alpha(opaque), 1.0, "no material was read as transparent");
    assert_eq!(opaque.alpha_mode, None, "no invented BLEND");
    assert!(!opaque.double_sided);
}

#[test]
fn a_single_unread_colour_material_still_contributes_its_alpha() {
    let mut model = model(
        vec![material("Frosted", None, Some(0.4))],
        vec![element("IFCDOOR", None, None, None)],
    );
    // The single material's colour was not read, so the exporter cannot use
    // it outright; the category colour stays, but its alpha is kept.
    model.material_constituent_sets = vec![MaterialConstituentSet {
        material_indices: vec![0],
        names: Vec::new(),
        elements: vec![0],
    }];

    let (doc, _) = build_gltf(&model);
    let tinted = category(&doc, "IFCDOOR");
    assert!((alpha(tinted) - 0.6).abs() < 1e-5, "{}", alpha(tinted));
    assert_eq!(tinted.alpha_mode.as_deref(), Some("BLEND"));
}

#[test]
fn a_single_profile_with_unread_colour_takes_its_alpha() {
    let mut model = model(
        vec![material("Frosted profile", None, Some(0.25))],
        vec![element("IFCBEAM", None, None, Some(0))],
    );
    model.material_profile_sets = vec![MaterialProfileSet {
        name: "Profile".into(),
        profiles: vec![MaterialProfile {
            material_index: 0,
            profile_name: "RECT".into(),
            description: None,
            profile: None,
        }],
        description: None,
    }];

    let (doc, _) = build_gltf(&model);
    let tinted = category(&doc, "IFCBEAM");
    assert!((alpha(tinted) - 0.75).abs() < 1e-5, "{}", alpha(tinted));
    assert_eq!(tinted.alpha_mode.as_deref(), Some("BLEND"));
}

#[test]
fn a_layer_set_contributes_its_most_transparent_alpha() {
    let mut model = model(
        vec![
            material("Glass", Some(0x8F_C1_FF), Some(0.9)),
            material("Frame", Some(0x33_33_33), Some(0.0)),
        ],
        vec![element("IFCWALL", None, Some(0), None)],
    );
    model.material_layer_sets = vec![MaterialLayerSet {
        name: "Glass wall".into(),
        layers: vec![
            MaterialLayer {
                material_index: Some(0),
                thickness_feet: 0.1,
                name: None,
            },
            MaterialLayer {
                material_index: Some(1),
                thickness_feet: 0.05,
                name: None,
            },
        ],
        description: None,
    }];

    let (doc, _) = build_gltf(&model);
    let tinted = category(&doc, "IFCWALL");
    assert!((alpha(tinted) - 0.1).abs() < 1e-5, "{}", alpha(tinted));
    assert_eq!(tinted.alpha_mode.as_deref(), Some("BLEND"));
}
