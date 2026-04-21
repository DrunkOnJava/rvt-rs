//! glTF 2.0 exporter (VW1-04) — dep-free binary GLB emission.
//!
//! Produces a `.glb` file ready to load into Three.js, BabylonJS,
//! Blender, or any glTF 2.0 viewer. The export strategy for the
//! first pass is intentionally simple: one unit-cube mesh shared
//! across all elements, placed with per-element transforms derived
//! from each `BuildingElement`'s `extrusion` + `location_feet`.
//! Each element gets a material drawn from `PbrMaterial::from_
//! material_info` (VW1-06).
//!
//! That's enough to show a recognizable massing model in a
//! browser — real profile + solid-shape geometry will layer on
//! top once the reader surfaces it (GEO-28..35 on the reader
//! side is the dependency).
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

/// Build a glTF document + binary buffer from an `IfcModel`
/// (VW1-04). Returns `(document, binary_buffer)` — wire into
/// [`write_glb`] for a single-file `.glb` output.
///
/// Each `BuildingElement` with an `extrusion` becomes a Node
/// with a cube mesh scaled to the extrusion's (width × depth ×
/// height) and translated to `location_feet`. Elements without
/// geometry are emitted as transform-less nodes for hierarchy.
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

    // Buffer (singular — GLB embeds the whole bin chunk as buffer 0).
    doc.buffers.push(Buffer {
        byte_length: bin.len(),
        uri: None,
    });

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

    // Per-element: one Mesh with one Primitive referencing the
    // shared accessors + the element's material index.
    let mut scene_nodes: Vec<usize> = Vec::new();
    for ent in &model.entities {
        if let IfcEntity::BuildingElement {
            name,
            material_index,
            location_feet,
            extrusion,
            ..
        } = ent
        {
            let mut primitive = Primitive {
                attributes: {
                    let mut m = std::collections::BTreeMap::new();
                    m.insert("POSITION".into(), 0usize);
                    m
                },
                indices: Some(1),
                material: *material_index,
                mode: 4, // TRIANGLES
            };
            // Stabilise the default Primitive.mode (0 = POINTS,
            // 4 = TRIANGLES); serde default would emit 0.
            primitive.mode = 4;
            let mesh = Mesh {
                primitives: vec![primitive],
                name: Some(name.clone()),
            };
            let mesh_idx = doc.meshes.len();
            doc.meshes.push(mesh);
            // Transform: translate to location, scale to extrusion
            // dims. Default unit cube = 1×1×1 centered at origin.
            let (sx, sy, sz) = extrusion
                .as_ref()
                .map(|e| {
                    (
                        e.width_feet as f32,
                        e.depth_feet as f32,
                        e.height_feet as f32,
                    )
                })
                .unwrap_or((1.0, 1.0, 1.0));
            let (tx, ty, tz) = location_feet
                .map(|l| (l[0] as f32, l[1] as f32, l[2] as f32 + sz * 0.5))
                .unwrap_or((0.0, 0.0, sz * 0.5));
            let matrix = [
                sx, 0.0, 0.0, 0.0, // col 0
                0.0, sy, 0.0, 0.0, // col 1
                0.0, 0.0, sz, 0.0, // col 2
                tx, ty, tz, 1.0, // col 3 (translation)
            ];
            let node = Node {
                name: Some(name.clone()),
                mesh: Some(mesh_idx),
                matrix: Some(matrix),
                children: Vec::new(),
            };
            let node_idx = doc.nodes.len();
            doc.nodes.push(node);
            scene_nodes.push(node_idx);
        }
    }

    doc.scenes.push(Scene { nodes: scene_nodes });
    doc.scene = Some(0);

    (doc, bin)
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
        assert_eq!(doc.meshes.len(), 2);
        assert_eq!(doc.nodes.len(), 2);
        assert_eq!(doc.scenes[0].nodes.len(), 2);
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
        assert_eq!(doc.scenes[0].nodes.len(), 3);
    }
}
