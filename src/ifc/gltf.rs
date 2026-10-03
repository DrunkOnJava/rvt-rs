//! glTF 2.0 exporter (VW1-04) — dep-free binary GLB emission.
//!
//! Produces a `.glb` file ready to load into Three.js, BabylonJS,
//! Blender, or any glTF 2.0 viewer. Each `BuildingElement` is drawn as the
//! body the STEP writer gives it ([`super::body_geometry`]), at its
//! `location_feet` turned by its `rotation_radians`. A plain rectangular
//! extrusion shares one unit cube, scaled by its node's matrix; every
//! other body (a sketched profile, a steel section, a swept or revolved
//! solid, a brep) has its own mesh. An element with no body gets a node
//! with no mesh. Each element gets a material drawn from
//! `PbrMaterial::from_material_info` (VW1-06).
//!
//! Element nodes are in the model's frame, feet with +Z up. glTF is
//! metres with +Y up, so they hang under one root node whose matrix
//! ([`Z_UP_FEET_TO_Y_UP_METRES`]) converts; without it every model lay on
//! its side, ten times too large, in any glTF viewer.
//!
//! GLB file layout (per the glTF 2.0 spec):
//!
//! ```text
//! [u32 magic=0x46546C67 "glTF"] [u32 version=2] [u32 total_length]
//! [u32 json_chunk_length] [u32 chunk_type=0x4E4F534A "JSON"]
//! [json_chunk_length bytes UTF-8 JSON, padded to 4-byte boundary with 0x20]
//! [u32 bin_chunk_length] [u32 chunk_type=0x004E4942 "BIN\0"]
//! [bin_chunk_length bytes binary buffer, padded to 4-byte with 0x00]
//! ```

use super::IfcModel;
use super::body_geometry::{self, Body, Placement, body_mesh, element_body};
use super::entities::IfcEntity;
use super::pbr::PbrMaterial;
use serde::{Deserialize, Serialize};

/// Top-level glTF 2.0 JSON document (VW1-04).
///
/// Matches the glTF spec field names exactly (camelCase) so the
/// serialized JSON is a valid glTF file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GltfDocument {
    pub asset: Asset,
    pub scene: Option<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scenes: Vec<Scene>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nodes: Vec<Node>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub meshes: Vec<Mesh>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materials: Vec<Material>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub buffers: Vec<Buffer>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[serde(rename = "bufferViews")]
    pub buffer_views: Vec<BufferView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accessors: Vec<Accessor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generator: Option<String>,
}

impl Default for Asset {
    fn default() -> Self {
        Self {
            version: "2.0".into(),
            generator: Some("rvt-rs VW1-04".into()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<usize>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Node {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mesh: Option<usize>,
    /// 16-element column-major transformation matrix. `None` =
    /// identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matrix: Option<[f32; 16]>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<usize>,
    /// Application-specific payload. glTF loaders surface this as
    /// the object's `userData`, which is how the viewer maps a
    /// picked mesh back to its entity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extras: Option<NodeExtras>,
}

/// Identity the viewer needs on every drawable node: the
/// `model.entities` index behind it and its IFC type. The viewer
/// reads these as `userData.entityIndex` / `userData.ifcType` to
/// drive picking, selection highlight, and category visibility —
/// all three of which were inert while the nodes carried no
/// extras at all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeExtras {
    #[serde(rename = "entityIndex")]
    pub entity_index: usize,
    #[serde(rename = "ifcType")]
    pub ifc_type: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Mesh {
    pub primitives: Vec<Primitive>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Primitive {
    pub attributes: std::collections::BTreeMap<String, usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indices: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<usize>,
    /// 4 = triangle list (the default per the glTF spec).
    #[serde(default)]
    pub mode: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Material {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(rename = "pbrMetallicRoughness")]
    pub pbr_metallic_roughness: PbrMetallicRoughness,
    #[serde(default, rename = "doubleSided")]
    pub double_sided: bool,
    #[serde(default, rename = "alphaMode", skip_serializing_if = "Option::is_none")]
    pub alpha_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PbrMetallicRoughness {
    /// `[r, g, b, a]` linear 0-1.
    #[serde(rename = "baseColorFactor")]
    pub base_color_factor: [f32; 4],
    #[serde(rename = "metallicFactor")]
    pub metallic_factor: f32,
    #[serde(rename = "roughnessFactor")]
    pub roughness_factor: f32,
}

impl Default for PbrMetallicRoughness {
    fn default() -> Self {
        Self {
            base_color_factor: [0.75, 0.75, 0.75, 1.0],
            metallic_factor: 0.0,
            roughness_factor: 0.6,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Buffer {
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    /// `None` = embedded in the GLB binary chunk (the common case
    /// for single-file .glb output).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferView {
    pub buffer: usize,
    #[serde(rename = "byteOffset", default)]
    pub byte_offset: usize,
    #[serde(rename = "byteLength")]
    pub byte_length: usize,
    /// glTF target: 34962 = ARRAY_BUFFER (vertex attrs),
    /// 34963 = ELEMENT_ARRAY_BUFFER (indices). `None` for
    /// unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Accessor {
    #[serde(rename = "bufferView")]
    pub buffer_view: usize,
    #[serde(rename = "byteOffset", default)]
    pub byte_offset: usize,
    /// 5120..5126 per glTF component-type enum:
    /// 5120=i8, 5121=u8, 5122=i16, 5123=u16, 5125=u32, 5126=f32.
    #[serde(rename = "componentType")]
    pub component_type: u32,
    pub count: usize,
    /// `"SCALAR"`, `"VEC2"`, `"VEC3"`, `"VEC4"`, `"MAT4"`, etc.
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub max: Vec<f32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub min: Vec<f32>,
}

/// Unit-cube vertex positions ([-0.5, 0.5]³, 24 verts — 4 per
/// face for correct per-face normals). Shared across every
/// element in the first-pass VW1-04 export.
fn unit_cube_vertices() -> [f32; 24 * 3] {
    // 6 faces × 4 verts × 3 coords.
    let h = 0.5_f32;
    [
        // +X face
        h, -h, -h, h, h, -h, h, h, h, h, -h, h, // -X face
        -h, -h, h, -h, h, h, -h, h, -h, -h, -h, -h, // +Y face
        -h, h, -h, -h, h, h, h, h, h, h, h, -h, // -Y face
        h, -h, -h, h, -h, h, -h, -h, h, -h, -h, -h, // +Z face
        -h, -h, h, h, -h, h, h, h, h, -h, h, h, // -Z face
        -h, -h, -h, -h, h, -h, h, h, -h, h, -h, -h,
    ]
}

fn unit_cube_indices() -> [u16; 36] {
    // 2 triangles per face.
    [
        0, 1, 2, 0, 2, 3, // +X
        4, 5, 6, 4, 6, 7, // -X
        8, 9, 10, 8, 10, 11, // +Y
        12, 13, 14, 12, 14, 15, // -Y
        16, 17, 18, 16, 18, 19, // +Z
        20, 21, 22, 20, 22, 23, // -Z
    ]
}

/// Column-major matrix taking the model's frame (feet, +Z up) to glTF's
/// (metres, +Y up): model `(x, y, z)` feet is glTF `(x, z, -y)` metres.
pub const Z_UP_FEET_TO_Y_UP_METRES: [f32; 16] = [
    0.3048, 0.0, 0.0, 0.0, // model X stays X
    0.0, 0.0, -0.3048, 0.0, // model Y is glTF -Z
    0.0, 0.3048, 0.0, 0.0, // model Z (up) is glTF Y (up)
    0.0, 0.0, 0.0, 1.0,
];

/// Build a glTF document + binary buffer from an `IfcModel`
/// (VW1-04). Returns `(document, binary_buffer)` — wire into
/// [`write_glb`] for a single-file `.glb` output.
///
/// Every `BuildingElement` becomes a node carrying its entity index and
/// IFC type, a child of the root node that converts the model's frame to
/// glTF's. Its mesh is its body in element-local feet, and its matrix
/// its placement: the shared unit cube scaled, turned and moved for a
/// plain rectangular extrusion, a per-element mesh turned and moved for
/// any other body, and no mesh for an element with none.
pub fn build_gltf(model: &IfcModel) -> (GltfDocument, Vec<u8>) {
    let mut bin = Vec::<u8>::new();
    let mut doc = GltfDocument::default();

    // Binary layout:
    //   [positions: 24 * VEC3 f32 = 288 bytes]
    //   [indices: 36 * u16 = 72 bytes → pad to 4-aligned = 72]
    let positions = unit_cube_vertices();
    let pos_bytes = bytemuck_cast_f32(&positions);
    let pos_offset = 0;
    let pos_len = pos_bytes.len();
    bin.extend_from_slice(&pos_bytes);
    pad_to_4(&mut bin);

    let indices = unit_cube_indices();
    let idx_bytes = bytemuck_cast_u16(&indices);
    let idx_offset = bin.len();
    let idx_len = idx_bytes.len();
    bin.extend_from_slice(&idx_bytes);
    pad_to_4(&mut bin);

    // BufferView 0: positions.
    doc.buffer_views.push(BufferView {
        buffer: 0,
        byte_offset: pos_offset,
        byte_length: pos_len,
        target: Some(34962), // ARRAY_BUFFER
    });
    // BufferView 1: indices.
    doc.buffer_views.push(BufferView {
        buffer: 0,
        byte_offset: idx_offset,
        byte_length: idx_len,
        target: Some(34963), // ELEMENT_ARRAY_BUFFER
    });

    // Accessor 0: POSITION (VEC3 f32, 24 verts).
    doc.accessors.push(Accessor {
        buffer_view: 0,
        byte_offset: 0,
        component_type: 5126, // f32
        count: 24,
        type_: "VEC3".into(),
        min: vec![-0.5, -0.5, -0.5],
        max: vec![0.5, 0.5, 0.5],
    });
    // Accessor 1: indices (SCALAR u16, 36 indices).
    doc.accessors.push(Accessor {
        buffer_view: 1,
        byte_offset: 0,
        component_type: 5123, // u16
        count: 36,
        type_: "SCALAR".into(),
        ..Accessor {
            buffer_view: 0,
            byte_offset: 0,
            component_type: 0,
            count: 0,
            type_: String::new(),
            max: Vec::new(),
            min: Vec::new(),
        }
    });

    // Materials — one per model.materials entry via PbrMaterial.
    for m in &model.materials {
        let pbr = PbrMaterial::from_material_info(m);
        let mat = Material {
            name: Some(m.name.clone()),
            pbr_metallic_roughness: PbrMetallicRoughness {
                base_color_factor: [
                    pbr.base_color_rgb[0],
                    pbr.base_color_rgb[1],
                    pbr.base_color_rgb[2],
                    pbr.alpha,
                ],
                metallic_factor: pbr.metallic,
                roughness_factor: pbr.roughness,
            },
            double_sided: pbr.double_sided,
            alpha_mode: if pbr.alpha < 1.0 {
                Some("BLEND".into())
            } else {
                None
            },
        };
        doc.materials.push(mat);
    }

    // An element with no material of its own takes its category's colour,
    // one material per category, after the model's own.
    let mut category_materials: std::collections::BTreeMap<String, usize> = Default::default();
    // Layer colours (RE-53), one material per colour and alpha.
    let mut layer_materials: std::collections::BTreeMap<(u32, u32), usize> = Default::default();
    // A family instance whose type draws in one material (RE-82) takes that
    // material's colour, when the colour was read (RE-53). One drawn in
    // several keeps its category's: a single body cannot show which part is
    // which.
    let single_material: std::collections::BTreeMap<usize, usize> = model
        .material_constituent_sets
        .iter()
        .filter_map(|set| match set.material_indices.as_slice() {
            [material]
                if model
                    .materials
                    .get(*material)
                    .is_some_and(|info| info.color_packed.is_some()) =>
            {
                Some(
                    set.elements
                        .iter()
                        .map(move |&element| (element, *material)),
                )
            }
            _ => None,
        })
        .flatten()
        // A member written with a material profile set of one material
        // (RE-105) takes it the same way.
        .chain(
            model
                .entities
                .iter()
                .enumerate()
                .filter_map(|(index, entity)| match entity {
                    IfcEntity::BuildingElement {
                        material_profile_set_index: Some(set),
                        ..
                    } => match model.material_profile_sets.get(*set)?.profiles.as_slice() {
                        [profile]
                            if model
                                .materials
                                .get(profile.material_index)
                                .is_some_and(|info| info.color_packed.is_some()) =>
                        {
                            Some((index, profile.material_index))
                        }
                        _ => None,
                    },
                    _ => None,
                }),
        )
        .collect();

    // Per-element: a node carrying the entity's identity, with the
    // shared cube or its own mesh.
    let mut scene_nodes: Vec<usize> = Vec::new();
    for (entity_index, ent) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            material_index,
            location_feet,
            rotation_radians,
            ..
        } = ent
        else {
            continue;
        };
        // An opening is a void in its host, not a solid (RE-84).
        if ifc_type == "IFCOPENINGELEMENT" {
            continue;
        }
        let placement = Placement::new(*location_feet, *rotation_radians);
        let body = element_body(ent, &model.representation_maps);
        let (c, s) = (placement.cos as f32, placement.sin as f32);
        let [tx, ty, tz] = placement.origin.map(|v| v as f32);
        let body_matrix = [
            c,
            s,
            0.0,
            0.0,
            if s == 0.0 { 0.0 } else { -s },
            c,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            tx,
            ty,
            tz,
            1.0,
        ];
        // RE-53: a layered element is one node per layer, each in its
        // material's colour, all carrying the element's identity.
        let layered = type_guid
            .as_deref()
            .and_then(|tag| tag.parse::<u32>().ok())
            .and_then(|id| model.element_layers.get(&id));
        if let (Some(layers), Some(Body::Extrusion(extrusion))) = (layered, body) {
            let [nx, ny] = layers.exterior_normal;
            let local = [
                nx * placement.cos + ny * placement.sin,
                -nx * placement.sin + ny * placement.cos,
            ];
            let widths: Vec<f64> = layers.layers.iter().map(|l| l.width_feet).collect();
            let meshes = if layers.stacked {
                body_geometry::stacked_extrusion_meshes(extrusion, &widths)
            } else {
                body_geometry::layered_extrusion_meshes(extrusion, local, &widths)
            };
            if let Some(meshes) = meshes {
                for (mesh, band) in meshes.iter().zip(&layers.layers) {
                    let Some((position, indices)) = push_mesh(&mut doc, &mut bin, mesh) else {
                        continue;
                    };
                    let material = match band.color_packed {
                        Some(packed) => {
                            let rgb = ((packed & 0xff) << 16)
                                | (packed & 0xff00)
                                | ((packed >> 16) & 0xff);
                            let alpha = (1.0 - band.transparency.clamp(0.0, 1.0)) as f32;
                            *layer_materials
                                .entry((rgb, alpha.to_bits()))
                                .or_insert_with(|| {
                                    doc.materials.push(colour_material(
                                        format!("Layer {rgb:06x}"),
                                        rgb,
                                        alpha,
                                    ));
                                    doc.materials.len() - 1
                                })
                        }
                        None => *category_materials
                            .entry(ifc_type.clone())
                            .or_insert_with(|| {
                                doc.materials.push(category_material(ifc_type));
                                doc.materials.len() - 1
                            }),
                    };
                    let mut attributes = std::collections::BTreeMap::new();
                    attributes.insert("POSITION".into(), position);
                    let mesh_idx = doc.meshes.len();
                    doc.meshes.push(Mesh {
                        primitives: vec![Primitive {
                            attributes,
                            indices: Some(indices),
                            material: Some(material),
                            mode: 4,
                        }],
                        name: Some(name.clone()),
                    });
                    let node_idx = doc.nodes.len();
                    doc.nodes.push(Node {
                        name: Some(name.clone()),
                        mesh: Some(mesh_idx),
                        matrix: Some(body_matrix),
                        children: Vec::new(),
                        extras: Some(NodeExtras {
                            entity_index,
                            ifc_type: ifc_type.clone(),
                        }),
                    });
                    scene_nodes.push(node_idx);
                }
                continue;
            }
        }
        // (accessor pair, matrix)
        let drawn: Option<((usize, usize), [f32; 16])> = match body {
            Some(Body::Extrusion(ex))
                if ex.profile_override.is_none()
                    && [ex.width_feet, ex.depth_feet, ex.height_feet]
                        .iter()
                        .all(|v| v.is_finite() && *v > 0.0) =>
            {
                let (sx, sy, sz) = (
                    ex.width_feet as f32,
                    ex.depth_feet as f32,
                    ex.height_feet as f32,
                );
                // Translate · rotate about Z · scale the unit cube, whose
                // base sits half its height below its centre.
                Some((
                    (0, 1),
                    [
                        c * sx,
                        s * sx,
                        0.0,
                        0.0,
                        if s == 0.0 { 0.0 } else { -s * sy },
                        c * sy,
                        0.0,
                        0.0,
                        0.0,
                        0.0,
                        sz,
                        0.0,
                        tx,
                        ty,
                        tz + sz * 0.5,
                        1.0,
                    ],
                ))
            }
            Some(body) => body_mesh(body).and_then(|mesh| {
                let accessors = push_mesh(&mut doc, &mut bin, &mesh)?;
                Some((accessors, body_matrix))
            }),
            None => None,
        };
        let (mesh, matrix) = match drawn {
            Some(((position, indices), matrix)) => {
                let material = material_index
                    .or_else(|| single_material.get(&entity_index).copied())
                    .or_else(|| {
                        Some(
                            *category_materials
                                .entry(ifc_type.clone())
                                .or_insert_with(|| {
                                    doc.materials.push(category_material(ifc_type));
                                    doc.materials.len() - 1
                                }),
                        )
                    });
                let mut attributes = std::collections::BTreeMap::new();
                attributes.insert("POSITION".into(), position);
                let mesh_idx = doc.meshes.len();
                doc.meshes.push(Mesh {
                    primitives: vec![Primitive {
                        attributes,
                        indices: Some(indices),
                        material,
                        mode: 4, // TRIANGLES
                    }],
                    name: Some(name.clone()),
                });
                (Some(mesh_idx), Some(matrix))
            }
            None => (None, None),
        };
        let node_idx = doc.nodes.len();
        doc.nodes.push(Node {
            name: Some(name.clone()),
            mesh,
            matrix,
            children: Vec::new(),
            extras: Some(NodeExtras {
                entity_index,
                ifc_type: ifc_type.clone(),
            }),
        });
        scene_nodes.push(node_idx);
    }

    // Buffer (singular — GLB embeds the whole bin chunk as buffer 0).
    doc.buffers.push(Buffer {
        byte_length: bin.len(),
        uri: None,
    });
    let root = doc.nodes.len();
    doc.nodes.push(Node {
        name: Some("model".into()),
        mesh: None,
        matrix: Some(Z_UP_FEET_TO_Y_UP_METRES),
        children: scene_nodes,
        extras: None,
    });
    doc.scenes.push(Scene { nodes: vec![root] });
    doc.scene = Some(0);

    (doc, bin)
}

/// The sRGB colour, and the alpha, an element with no material of its own
/// is drawn in: its category's, from the viewer design system's gray, blue,
/// red, green and amber ramps, in the hues of the plan's palette
/// ([`super::sheet`]). Blue-6 is left to the viewer's selection highlight.
/// Glass-like categories are translucent, and spaces faint enough not to
/// hide the building they fill.
pub fn category_colour(ifc_type: &str) -> (u32, f32) {
    match ifc_type {
        "IFCWALL" | "IFCWALLSTANDARDCASE" | "IFCCURTAINWALL" => (0xCDCED0, 1.0),
        "IFCSLAB" | "IFCROOF" | "IFCCOVERING" => (0x909398, 1.0),
        "IFCDOOR" => (0xF59E0B, 1.0),
        "IFCWINDOW" | "IFCPLATE" => (0x8FC1FF, 0.45),
        "IFCCOLUMN" => (0xDC2626, 1.0),
        "IFCBEAM" | "IFCMEMBER" => (0xF87171, 1.0),
        "IFCSTAIR" | "IFCSTAIRFLIGHT" | "IFCRAILING" | "IFCRAMP" | "IFCRAMPFLIGHT" => {
            (0x92400E, 1.0)
        }
        "IFCFURNITURE" | "IFCFURNISHINGELEMENT" => (0x16A34A, 1.0),
        "IFCSPACE" => (0x3D94FF, 0.12),
        _ => (0xB2B4B8, 1.0),
    }
}

/// The material a category's colour makes, named for the category.
fn category_material(ifc_type: &str) -> Material {
    let (rgb, alpha) = category_colour(ifc_type);
    colour_material(format!("Category {ifc_type}"), rgb, alpha)
}

/// A material of one sRGB colour (`0xRRGGBB`) and alpha. glTF base colours
/// are linear, so the colour is linearised.
fn colour_material(name: String, rgb: u32, alpha: f32) -> Material {
    let linear = |shift: u32| {
        let c = ((rgb >> shift) & 0xff) as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Material {
        name: Some(name),
        pbr_metallic_roughness: PbrMetallicRoughness {
            base_color_factor: [linear(16), linear(8), linear(0), alpha],
            metallic_factor: 0.0,
            roughness_factor: 0.8,
        },
        double_sided: alpha < 1.0,
        alpha_mode: (alpha < 1.0).then(|| "BLEND".into()),
    }
}

/// Append `mesh` to the binary buffer as f32 positions and u32 indices,
/// with a buffer view and an accessor for each. Returns the
/// `(POSITION, indices)` accessor indices, or `None` for an empty mesh or
/// one past `u32` indexing.
fn push_mesh(
    doc: &mut GltfDocument,
    bin: &mut Vec<u8>,
    mesh: &body_geometry::Mesh,
) -> Option<(usize, usize)> {
    if mesh.is_empty() || u32::try_from(mesh.vertices.len()).is_err() {
        return None;
    }
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    let positions: Vec<f32> = mesh
        .vertices
        .iter()
        .flat_map(|v| {
            let p = v.map(|c| c as f32);
            for axis in 0..3 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
            p
        })
        .collect();
    if !positions.iter().all(|v| v.is_finite()) {
        return None;
    }
    let indices: Vec<u8> = mesh
        .triangles
        .iter()
        .flatten()
        .flat_map(|i| i.to_le_bytes())
        .collect();

    let position_offset = bin.len();
    bin.extend_from_slice(&bytemuck_cast_f32(&positions));
    let position_view = doc.buffer_views.len();
    doc.buffer_views.push(BufferView {
        buffer: 0,
        byte_offset: position_offset,
        byte_length: bin.len() - position_offset,
        target: Some(34962), // ARRAY_BUFFER
    });
    pad_to_4(bin);
    let index_offset = bin.len();
    bin.extend_from_slice(&indices);
    let index_view = doc.buffer_views.len();
    doc.buffer_views.push(BufferView {
        buffer: 0,
        byte_offset: index_offset,
        byte_length: indices.len(),
        target: Some(34963), // ELEMENT_ARRAY_BUFFER
    });
    pad_to_4(bin);

    let position = doc.accessors.len();
    doc.accessors.push(Accessor {
        buffer_view: position_view,
        byte_offset: 0,
        component_type: 5126, // f32
        count: mesh.vertices.len(),
        type_: "VEC3".into(),
        min: min.to_vec(),
        max: max.to_vec(),
    });
    let index = doc.accessors.len();
    doc.accessors.push(Accessor {
        buffer_view: index_view,
        byte_offset: 0,
        component_type: 5125, // u32
        count: mesh.triangles.len() * 3,
        type_: "SCALAR".into(),
        min: Vec::new(),
        max: Vec::new(),
    });
    Some((position, index))
}

/// Write a GLB (binary glTF) file to `out` given a JSON document
/// + binary buffer (VW1-04).
///
/// Spec reference: <https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#glb-file-format-specification>
pub fn write_glb(doc: &GltfDocument, bin: &[u8], out: &mut Vec<u8>) {
    let json_text = serde_json::to_string(doc).expect("serialize glTF doc");
    let mut json_bytes = json_text.into_bytes();
    // Pad JSON chunk to 4-byte boundary with ASCII space (per spec).
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let mut bin_padded = bin.to_vec();
    while !bin_padded.len().is_multiple_of(4) {
        bin_padded.push(0);
    }

    // Header: 12 bytes (magic + version + total length).
    let total_len = 12 + 8 + json_bytes.len() + 8 + bin_padded.len();
    out.extend_from_slice(&0x4654_6C67_u32.to_le_bytes()); // "glTF"
    out.extend_from_slice(&2_u32.to_le_bytes()); // version
    out.extend_from_slice(&(total_len as u32).to_le_bytes());

    // JSON chunk.
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x4E4F_534A_u32.to_le_bytes()); // "JSON"
    out.extend_from_slice(&json_bytes);

    // BIN chunk.
    out.extend_from_slice(&(bin_padded.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004E_4942_u32.to_le_bytes()); // "BIN\0"
    out.extend_from_slice(&bin_padded);
}

/// One-call convenience: `IfcModel` → GLB bytes.
pub fn model_to_glb(model: &IfcModel) -> Vec<u8> {
    let (doc, bin) = build_gltf(model);
    let mut out = Vec::new();
    write_glb(&doc, &bin, &mut out);
    out
}

// ---- Internal: dep-free f32/u16 slice-to-bytes casts ----

fn bytemuck_cast_f32(src: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 4);
    for &v in src {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn bytemuck_cast_u16(src: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(src.len() * 2);
    for &v in src {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

fn pad_to_4(buf: &mut Vec<u8>) {
    while !buf.len().is_multiple_of(4) {
        buf.push(0);
    }
}
