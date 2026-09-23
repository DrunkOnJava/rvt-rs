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

    // Per-element: a node carrying the entity's identity, with the
    // shared cube or its own mesh.
    let mut scene_nodes: Vec<usize> = Vec::new();
    for (entity_index, ent) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            material_index,
            location_feet,
            rotation_radians,
            ..
        } = ent
        else {
            continue;
        };
        let placement = Placement::new(*location_feet, *rotation_radians);
        let body = element_body(ent, &model.representation_maps);
        let (c, s) = (placement.cos as f32, placement.sin as f32);
        let [tx, ty, tz] = placement.origin.map(|v| v as f32);
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
                Some((
                    accessors,
                    [
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
                    ],
                ))
            }),
            None => None,
        };
        let (mesh, matrix) = match drawn {
            Some(((position, indices), matrix)) => {
                let mut attributes = std::collections::BTreeMap::new();
                attributes.insert("POSITION".into(), position);
                let mesh_idx = doc.meshes.len();
                doc.meshes.push(Mesh {
                    primitives: vec![Primitive {
                        attributes,
                        indices: Some(indices),
                        material: *material_index,
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
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    let mut bin_padded = bin.to_vec();
    while bin_padded.len() % 4 != 0 {
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
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

#[cfg(test)]
mod tests {
    use super::super::Storey;
    use super::super::entities::{Extrusion, IfcEntity};
    use super::*;

    fn mk_wall(name: &str, loc: Option<[f64; 3]>, extrusion: Option<Extrusion>) -> IfcEntity {
        IfcEntity::BuildingElement {
            ifc_type: "IFCWALL".into(),
            name: name.into(),
            type_guid: None,
            predefined_type: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: loc,
            rotation_radians: None,
            extrusion,
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }
    }

    #[test]
    fn every_element_node_carries_its_entity_index_and_ifc_type() {
        // The viewer maps a picked mesh back to its entity through
        // glTF node extras. Without them, picking, highlight and
        // category visibility have nothing to match on.
        let model = IfcModel {
            entities: vec![
                mk_wall("W1", Some([0.0, 0.0, 0.0]), None),
                IfcEntity::Project {
                    name: Some("P".into()),
                    description: None,
                    long_name: None,
                },
                mk_wall("W2", Some([5.0, 0.0, 0.0]), None),
            ],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        let extras: Vec<(usize, &str)> = element_nodes(&doc)
            .map(|n| {
                let e = n.extras.as_ref().expect("node carries extras");
                (e.entity_index, e.ifc_type.as_str())
            })
            .collect();
        // Indices are into model.entities, so the skipped Project
        // entity at index 1 leaves a gap rather than renumbering.
        assert_eq!(extras, vec![(0, "IFCWALL"), (2, "IFCWALL")]);
    }

    #[test]
    fn node_extras_serialize_as_the_camel_case_keys_the_viewer_reads() {
        let model = IfcModel {
            entities: vec![mk_wall("W1", None, None)],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("\"extras\""));
        assert!(json.contains("\"entityIndex\""));
        assert!(json.contains("\"ifcType\""));
    }

    #[test]
    fn empty_model_produces_valid_glb_framing() {
        let glb = model_to_glb(&IfcModel::default());
        // 12-byte header + 8-byte JSON chunk header + JSON body
        // padded to 4 + 8-byte BIN chunk header + BIN body
        // padded to 4. Minimum realistic size ~200 bytes.
        assert!(glb.len() >= 100);
        // Magic "glTF"
        assert_eq!(&glb[..4], b"glTF");
        // Version 2
        let ver = u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]);
        assert_eq!(ver, 2);
        // Total length matches
        let total = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(total, glb.len());
    }

    #[test]
    fn glb_has_json_and_bin_chunks() {
        let glb = model_to_glb(&IfcModel::default());
        // JSON chunk type at offset 16 = "JSON"
        assert_eq!(&glb[16..20], b"JSON");
        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        // BIN chunk type at 20 + json_len + 4
        let bin_type_offset = 20 + json_len + 4;
        assert_eq!(&glb[bin_type_offset..bin_type_offset + 4], b"BIN\0");
    }

    #[test]
    fn build_gltf_includes_asset_version_2() {
        let (doc, _) = build_gltf(&IfcModel::default());
        assert_eq!(doc.asset.version, "2.0");
        assert!(doc.asset.generator.is_some());
    }

    #[test]
    fn build_gltf_shares_mesh_accessors() {
        let (doc, _) = build_gltf(&IfcModel::default());
        // Shared cube: 1 buffer, 2 bufferViews, 2 accessors.
        assert_eq!(doc.buffers.len(), 1);
        assert_eq!(doc.buffer_views.len(), 2);
        assert_eq!(doc.accessors.len(), 2);
    }

    #[test]
    fn each_building_element_becomes_a_node_with_mesh() {
        let model = IfcModel {
            entities: vec![
                mk_wall(
                    "Wall-1",
                    Some([10.0, 5.0, 0.0]),
                    Some(Extrusion {
                        width_feet: 20.0,
                        depth_feet: 0.5,
                        height_feet: 10.0,
                        profile_override: None,
                    }),
                ),
                mk_wall("Wall-2", None, None),
            ],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        // Wall-2 has no body: it keeps its node, with no mesh, rather than
        // being drawn as a unit cube it does not have.
        assert_eq!(doc.meshes.len(), 1);
        assert_eq!(element_nodes(&doc).count(), 2);
        assert_eq!(doc.scenes[0].nodes.len(), 1, "the root");
        assert!(doc.nodes[0].mesh.is_some());
        assert!(doc.nodes[1].mesh.is_none() && doc.nodes[1].matrix.is_none());
    }

    fn with_rotation(mut entity: IfcEntity, rotation: f64) -> IfcEntity {
        if let IfcEntity::BuildingElement {
            rotation_radians, ..
        } = &mut entity
        {
            *rotation_radians = Some(rotation);
        }
        entity
    }

    #[test]
    fn a_rotated_element_is_drawn_turned() {
        let wall = with_rotation(
            mk_wall(
                "W",
                Some([3.0, 7.0, 1.0]),
                Some(Extrusion::rectangle(20.0, 0.5, 10.0)),
            ),
            std::f64::consts::FRAC_PI_2,
        );
        let model = IfcModel {
            entities: vec![wall],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        let m = doc.nodes[0].matrix.unwrap();
        // The cube's X (the wall's length) now runs along +Y.
        assert!(m[0].abs() < 1e-6 && (m[1] - 20.0).abs() < 1e-5);
        assert!((m[4] + 0.5).abs() < 1e-6 && m[5].abs() < 1e-6);
        assert_eq!((m[12], m[13], m[14]), (3.0, 7.0, 6.0));
    }

    /// Read back a mesh's world-space vertices from the GLB buffer.
    fn world_vertices(doc: &GltfDocument, bin: &[u8], node: usize) -> Vec<[f32; 3]> {
        let n = &doc.nodes[node];
        let mesh = &doc.meshes[n.mesh.expect("mesh")];
        let accessor = &doc.accessors[mesh.primitives[0].attributes["POSITION"]];
        let view = &doc.buffer_views[accessor.buffer_view];
        let m = n.matrix.expect("matrix");
        bin[view.byte_offset..view.byte_offset + view.byte_length]
            .chunks_exact(12)
            .map(|c| {
                let v: Vec<f32> = c
                    .chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                [
                    m[0] * v[0] + m[4] * v[1] + m[8] * v[2] + m[12],
                    m[1] * v[0] + m[5] * v[1] + m[9] * v[2] + m[13],
                    m[2] * v[0] + m[6] * v[1] + m[10] * v[2] + m[14],
                ]
            })
            .collect()
    }

    #[test]
    fn a_sketched_profile_is_drawn_as_its_own_mesh() {
        // An L-shaped slab, 0.5 ft thick, placed at (10, 0, 2).
        let slab = mk_wall(
            "S",
            Some([10.0, 0.0, 2.0]),
            Some(Extrusion::arbitrary_closed(
                vec![
                    (0.0, 0.0),
                    (4.0, 0.0),
                    (4.0, 2.0),
                    (2.0, 2.0),
                    (2.0, 4.0),
                    (0.0, 4.0),
                ],
                0.5,
            )),
        );
        let model = IfcModel {
            entities: vec![slab],
            ..Default::default()
        };
        let (doc, bin) = build_gltf(&model);
        assert_eq!(doc.buffers[0].byte_length, bin.len());
        let vertices = world_vertices(&doc, &bin, 0);
        let xs: Vec<f32> = vertices.iter().map(|v| v[0]).collect();
        let zs: Vec<f32> = vertices.iter().map(|v| v[2]).collect();
        assert_eq!(xs.iter().copied().fold(f32::INFINITY, f32::min), 10.0);
        assert_eq!(xs.iter().copied().fold(f32::NEG_INFINITY, f32::max), 14.0);
        assert_eq!(zs.iter().copied().fold(f32::INFINITY, f32::min), 2.0);
        assert_eq!(zs.iter().copied().fold(f32::NEG_INFINITY, f32::max), 2.5);
        // No vertex falls in the L's missing corner.
        assert!(
            !vertices
                .iter()
                .any(|v| v[0] > 12.0 + 1e-4 && v[1] > 2.0 + 1e-4)
        );
        let accessor = &doc.accessors[doc.meshes[0].primitives[0].attributes["POSITION"]];
        assert_eq!(accessor.min, vec![0.0, 0.0, 0.0]);
        assert_eq!(accessor.max, vec![4.0, 4.0, 0.5]);
    }

    #[test]
    fn a_swept_solid_is_drawn_along_its_directrix() {
        let mut beam = mk_wall("B", Some([0.0, 0.0, 0.0]), None);
        if let IfcEntity::BuildingElement {
            ifc_type,
            solid_shape,
            extrusion,
            ..
        } = &mut beam
        {
            *ifc_type = "IFCBEAM".into();
            // The record box the swept solid takes precedence over.
            *extrusion = Some(Extrusion::rectangle(1.0, 30.0, 2.0));
            *solid_shape = Some(super::super::entities::SolidShape::SweptPath {
                profile: super::super::entities::ProfileDef::Rectangle {
                    width_feet: 1.0,
                    depth_feet: 0.5,
                },
                directrix_points_feet: vec![[0.0, -10.0, 1.0], [0.0, 10.0, 1.5]],
                fixed_reference: [0.0, 0.0, 1.0],
            });
        }
        let model = IfcModel {
            entities: vec![beam],
            ..Default::default()
        };
        let (doc, bin) = build_gltf(&model);
        let vertices = world_vertices(&doc, &bin, 0);
        let span = |axis: usize| {
            let values: Vec<f32> = vertices.iter().map(|v| v[axis]).collect();
            values.iter().copied().fold(f32::NEG_INFINITY, f32::max)
                - values.iter().copied().fold(f32::INFINITY, f32::min)
        };
        assert!((span(0) - 0.5).abs() < 1e-4, "the section's width");
        assert!((span(1) - 20.0).abs() < 0.05, "the directrix, not the box");
    }

    #[test]
    fn node_transform_reflects_extrusion_dims() {
        let ext = Extrusion {
            width_feet: 20.0,
            depth_feet: 0.5,
            height_feet: 10.0,
            profile_override: None,
        };
        let model = IfcModel {
            entities: vec![mk_wall("W", Some([3.0, 7.0, 0.0]), Some(ext))],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        let m = doc.nodes[0].matrix.unwrap();
        assert_eq!(m[0], 20.0); // scale X
        assert_eq!(m[5], 0.5); // scale Y
        assert_eq!(m[10], 10.0); // scale Z
        assert_eq!(m[12], 3.0); // translate X
        assert_eq!(m[13], 7.0); // translate Y
        // translate Z = loc.z + height/2 = 0 + 5 = 5
        assert_eq!(m[14], 5.0);
    }

    #[test]
    fn materials_mirror_model_materials() {
        let model = IfcModel {
            materials: vec![
                super::super::MaterialInfo {
                    name: "Concrete".into(),
                    color_packed: Some(0x00808080),
                    transparency: None,
                },
                super::super::MaterialInfo {
                    name: "Glass".into(),
                    color_packed: None,
                    transparency: Some(0.6),
                },
            ],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        assert_eq!(doc.materials.len(), 2);
        assert_eq!(doc.materials[0].name.as_deref(), Some("Concrete"));
        assert_eq!(doc.materials[1].name.as_deref(), Some("Glass"));
        assert!(doc.materials[1].double_sided); // glass is double-sided
        assert_eq!(doc.materials[1].alpha_mode.as_deref(), Some("BLEND"));
    }

    #[test]
    fn json_chunk_is_valid_json() {
        let model = IfcModel {
            project_name: Some("Test".into()),
            entities: vec![mk_wall("W1", None, None)],
            ..Default::default()
        };
        let glb = model_to_glb(&model);
        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        let json_bytes = &glb[20..20 + json_len];
        let json_text = std::str::from_utf8(json_bytes)
            .unwrap()
            .trim_end_matches(' ');
        let _: serde_json::Value =
            serde_json::from_str(json_text).expect("emitted JSON must parse");
    }

    #[test]
    fn unit_cube_has_24_vertices_and_36_indices() {
        assert_eq!(unit_cube_vertices().len(), 24 * 3);
        assert_eq!(unit_cube_indices().len(), 36);
    }

    #[test]
    fn pad_to_4_rounds_up_only() {
        let mut v = vec![1u8, 2, 3];
        pad_to_4(&mut v);
        assert_eq!(v.len(), 4);
        pad_to_4(&mut v);
        assert_eq!(v.len(), 4); // already aligned
    }

    #[test]
    fn glb_json_and_bin_chunks_are_4_byte_aligned() {
        let model = IfcModel {
            entities: vec![
                mk_wall("A", None, None),
                mk_wall("B", None, None),
                mk_wall("C", None, None),
            ],
            ..Default::default()
        };
        let glb = model_to_glb(&model);
        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        assert_eq!(json_len % 4, 0, "JSON chunk length not 4-aligned");
        let bin_len_offset = 20 + json_len;
        let bin_len = u32::from_le_bytes([
            glb[bin_len_offset],
            glb[bin_len_offset + 1],
            glb[bin_len_offset + 2],
            glb[bin_len_offset + 3],
        ]) as usize;
        assert_eq!(bin_len % 4, 0, "BIN chunk length not 4-aligned");
    }

    #[test]
    fn scene_zero_references_all_nodes() {
        let model = IfcModel {
            entities: vec![
                mk_wall("a", None, None),
                mk_wall("b", None, None),
                mk_wall("c", None, None),
            ],
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            ..Default::default()
        };
        let (doc, _) = build_gltf(&model);
        assert_eq!(doc.scene, Some(0));
        let root = &doc.nodes[doc.scenes[0].nodes[0]];
        assert_eq!(root.children.len(), 3);
        assert_eq!(root.matrix, Some(Z_UP_FEET_TO_Y_UP_METRES));
    }

    /// The nodes of building elements, without the root.
    fn element_nodes(doc: &GltfDocument) -> impl Iterator<Item = &Node> {
        doc.nodes.iter().filter(|n| n.extras.is_some())
    }

    #[test]
    fn the_root_turns_z_up_feet_into_y_up_metres() {
        let m = Z_UP_FEET_TO_Y_UP_METRES;
        let apply = |p: [f32; 3]| {
            [
                m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
                m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
                m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
            ]
        };
        // 10 ft up is 3.048 m along glTF's +Y; 10 ft north is -Z.
        assert_eq!(apply([0.0, 0.0, 10.0]), [0.0, 3.048, 0.0]);
        assert_eq!(apply([0.0, 10.0, 0.0]), [0.0, 0.0, -3.048]);
        assert_eq!(apply([10.0, 0.0, 0.0]), [3.048, 0.0, 0.0]);
    }
}
