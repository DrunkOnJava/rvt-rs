//! glTF export of native saved meshes, retaining face and material provenance.
use crate::{
    native_saved_mesh::Primitive,
    native_saved_scene::{SavedElement, SavedScene},
};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::collections::BTreeMap;
/// The material callback must return a qualified glTF material or None. Unresolved
/// materials remain explicitly counted and are never represented by guessed IDs.
pub fn encode(
    scene: &SavedScene,
    resolve: &dyn Fn(&SavedElement, usize, &Primitive) -> Option<Value>,
) -> Result<Vec<u8>> {
    let (mut nodes, mut meshes, mut views, mut accessors, mut materials) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut material_keys = BTreeMap::new();
    let mut bin = Vec::new();
    let mut unresolved = 0;
    for element in &scene.elements {
        let mut groups = BTreeMap::<Option<usize>, Vec<&Primitive>>::new();
        for (primitive_index, p) in element.meshes.primitives.iter().enumerate() {
            let material = match resolve(element, primitive_index, p) {
                None => {
                    unresolved += 1;
                    None
                }
                Some(m) => {
                    let key = serde_json::to_string(&m)?;
                    Some(*material_keys.entry(key).or_insert_with(|| {
                        let i = materials.len();
                        materials.push(m);
                        i
                    }))
                }
            };
            groups.entry(material).or_default().push(p)
        }
        let mut primitives = Vec::new();
        for (mat, parts) in groups {
            let mut vertices = Vec::new();
            let mut normals = Vec::new();
            let mut triangles = Vec::new();
            let mut ranges = Vec::new();
            for p in parts {
                ensure!(!p.vertices.is_empty(), "empty primitive vertices");
                ensure!(!p.triangles.is_empty(), "empty primitive triangles");
                ensure!(
                    p.normals.len() == p.vertices.len(),
                    "normal/vertex binding mismatch"
                );
                let base = u32::try_from(vertices.len())?;
                let start = triangles.len() * 3;
                for v in &p.vertices {
                    vertices.push([v[0] * 0.3048, v[2] * 0.3048, -v[1] * 0.3048])
                }
                for n in &p.normals {
                    let norm_squared = n.iter().map(|x| x * x).sum::<f64>();
                    ensure!(
                        n.iter().all(|x| x.is_finite()) && (norm_squared - 1.).abs() <= 1e-6,
                        "invalid primitive normal"
                    );
                    normals.push([n[0], n[2], -n[1]])
                }
                for t in &p.triangles {
                    ensure!(
                        t.iter().all(|i| (*i as usize) < p.vertices.len()),
                        "triangle index out of bounds"
                    );
                    triangles.push([
                        base.checked_add(t[0])
                            .ok_or_else(|| anyhow::anyhow!("index overflow"))?,
                        base.checked_add(t[1])
                            .ok_or_else(|| anyhow::anyhow!("index overflow"))?,
                        base.checked_add(t[2])
                            .ok_or_else(|| anyhow::anyhow!("index overflow"))?,
                    ]);
                }
                ranges.push(json!({"first_index":start,"index_count":p.triangles.len()*3,"source_owner_id":p.source_owner_id,"graphics_object_index":p.object_index,"face_tag":p.face_tag,"render_style_id":p.render_style_id,"explicit_material_id":p.material_id}));
            }
            let mut attrs = serde_json::Map::new();
            for (name, values) in [("POSITION", &vertices), ("NORMAL", &normals)] {
                let offset = bin.len();
                let mut low = [f32::INFINITY; 3];
                let mut high = [f32::NEG_INFINITY; 3];
                for p in values {
                    for i in 0..3 {
                        let x = p[i] as f32;
                        ensure!(x.is_finite(), "nonfinite GLB vector");
                        low[i] = low[i].min(x);
                        high[i] = high[i].max(x);
                        bin.extend(x.to_le_bytes());
                    }
                }
                let vi = views.len();
                views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34962}));
                let ai = accessors.len();
                let mut a = json!({"bufferView":vi,"componentType":5126,"count":values.len(),"type":"VEC3"});
                if name == "POSITION" {
                    a["min"] = json!(low);
                    a["max"] = json!(high)
                }
                accessors.push(a);
                attrs.insert(name.into(), json!(ai));
            }
            let offset = bin.len();
            for t in &triangles {
                for i in t {
                    bin.extend(i.to_le_bytes())
                }
            }
            let vi = views.len();
            views.push(json!({"buffer":0,"byteOffset":offset,"byteLength":bin.len()-offset,"target":34963}));
            let ai = accessors.len();
            accessors.push(json!({"bufferView":vi,"componentType":5125,"count":triangles.len()*3,"type":"SCALAR"}));
            let mut primitive = json!({"attributes":attrs,"indices":ai,"mode":4,"extras":{"face_ranges":ranges,"material_resolved":mat.is_some()}});
            if let Some(m) = mat {
                primitive["material"] = json!(m)
            }
            primitives.push(primitive);
        }
        let mut node = json!({"name":element.id.to_string(),"extras":{"native_identity":element.identity,"source":element.source,"referenced_graph_sources":element.referenced_graph_sources,"status":element.status,"diagnostics":element.meshes.diagnostics,"unbounded_faces":element.meshes.unbounded_faces,"rejected_filters":element.meshes.rejected_filters,"excluded_visibility_branches":element.meshes.excluded_visibility_branches,"excluded_non_surface_branches":element.meshes.excluded_non_surface_branches}});
        if !primitives.is_empty() {
            node["mesh"] = json!(meshes.len());
            meshes.push(json!({"name":element.id.to_string(),"primitives":primitives}))
        }
        nodes.push(node);
    }
    let mut document = json!({"asset":{"version":"2.0","generator":"rvt-rs current saved graphics"},"scene":0,"scenes":[{"nodes":(0..nodes.len()).collect::<Vec<_>>()}],"nodes":nodes,"extras":{"complete_document_geometry":false,"source_sha256":scene.source_sha256,"graphics_profile":scene.graphics_profile,"detail_level":scene.detail_level,"unresolved_material_face_batches":unresolved,"coordinate_conversion":"native document feet/Z-up to glTF meters/Y-up: (x,z,-y)*0.3048"}});
    if !meshes.is_empty() {
        document["meshes"] = json!(meshes);
        document["bufferViews"] = json!(views);
        document["accessors"] = json!(accessors);
        document["buffers"] = json!([{"byteLength":bin.len()}]);
    }
    if !materials.is_empty() {
        document["materials"] = json!(materials)
    }
    let mut text = serde_json::to_vec(&document)?;
    while text.len() % 4 != 0 {
        text.push(b' ')
    }
    while bin.len() % 4 != 0 {
        bin.push(0)
    }
    let len = 12 + 8 + text.len() + if bin.is_empty() { 0 } else { 8 + bin.len() };
    let mut bytes = Vec::with_capacity(len);
    bytes.extend(0x46546c67u32.to_le_bytes());
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(u32::try_from(len)?.to_le_bytes());
    bytes.extend(u32::try_from(text.len())?.to_le_bytes());
    bytes.extend(0x4e4f534au32.to_le_bytes());
    bytes.extend(text);
    if !bin.is_empty() {
        bytes.extend(u32::try_from(bin.len())?.to_le_bytes());
        bytes.extend(0x004e4942u32.to_le_bytes());
        bytes.extend(bin)
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{native_saved_mesh::GraphicsMeshes, native_saved_scene::SavedElement};
    fn scene() -> SavedScene {
        let identity=serde_json::from_value(json!({"element_id":1,"original_id_suffix":1,"creation_episode":0,"stored_revision":0,"other_revision":0,"row_offset":0,"raw_fields":[],"owning_element_id":-1,"partition_id":0,"unique_id":"test-00000001"})).unwrap();
        SavedScene {
            source_sha256: Some("source-proof".into()),
            format: "test",
            units: "feet",
            coordinate_frame: "document Z-up",
            detail_level: 3,
            graphics_profile: "test",
            complete_document_geometry: false,
            summaries: vec![],
            elements: vec![SavedElement {
                id: 1,
                identity,
                source: json!({}),
                referenced_graph_sources: BTreeMap::new(),
                status: "test".into(),
                render_materials: BTreeMap::new(),
                unresolved_material_primitives: vec![0],
                meshes: GraphicsMeshes {
                    primitives: vec![Primitive {
                        source_owner_id: None,
                        object_index: 1,
                        face_tag: 2,
                        render_style_id: 3,
                        material_id: None,
                        vertices: vec![[1., 2., 3.], [2., 2., 3.], [1., 3., 3.]],
                        normals: vec![[0., 0., 1.]; 3],
                        triangles: vec![[0, 1, 2]],
                    }],
                    ..Default::default()
                },
            }],
        }
    }
    #[test]
    fn glb_coordinate_conversion_normals_and_source_provenance() {
        let b = encode(&scene(), &|_, _, _| None).unwrap();
        let n = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let j: Value = serde_json::from_slice(&b[20..20 + n]).unwrap();
        let data = 28 + n;
        let p: Vec<_> = (0..3)
            .map(|i| f32::from_le_bytes(b[data + 4 * i..data + 4 * i + 4].try_into().unwrap()))
            .collect();
        assert_eq!(p, vec![0.3048, 0.9144, -0.6096]);
        assert_eq!(j["extras"]["source_sha256"], "source-proof");
        assert_eq!(j["extras"]["unresolved_material_face_batches"], 1);
        assert!(j["meshes"][0]["primitives"][0].get("material").is_none());
        let ni = j["meshes"][0]["primitives"][0]["attributes"]["NORMAL"]
            .as_u64()
            .unwrap() as usize;
        let vi = j["accessors"][ni]["bufferView"].as_u64().unwrap() as usize;
        let offset = j["bufferViews"][vi]["byteOffset"].as_u64().unwrap() as usize;
        let normal: Vec<_> = (0..3)
            .map(|i| {
                f32::from_le_bytes(
                    b[data + offset + 4 * i..data + offset + 4 * i + 4]
                        .try_into()
                        .unwrap(),
                )
            })
            .collect();
        assert_eq!(normal, vec![0., 1., -0.]);
    }
    #[test]
    fn same_material_batches_faces_and_retains_ranges() {
        let mut s = scene();
        let p = s.elements[0].meshes.primitives[0].clone();
        s.elements[0].meshes.primitives.push(p);
        let b = encode(&s, &|_, _, _| {
            Some(json!({"name":"native","pbrMetallicRoughness":{"baseColorFactor":[1,0,0,1]}}))
        })
        .unwrap();
        let n = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let j: Value = serde_json::from_slice(&b[20..20 + n]).unwrap();
        assert_eq!(j["meshes"][0]["primitives"].as_array().unwrap().len(), 1);
        assert_eq!(
            j["meshes"][0]["primitives"][0]["extras"]["face_ranges"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(j["materials"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn glb_retains_all_mesh_coverage_counters() {
        let mut s = scene();
        s.elements[0].meshes.unbounded_faces = 2;
        s.elements[0].meshes.rejected_filters = 3;
        s.elements[0].meshes.excluded_visibility_branches = 4;
        s.elements[0].meshes.excluded_non_surface_branches = 5;
        let b = encode(&s, &|_, _, _| None).unwrap();
        let n = u32::from_le_bytes(b[12..16].try_into().unwrap()) as usize;
        let j: Value = serde_json::from_slice(&b[20..20 + n]).unwrap();
        let extras = &j["nodes"][0]["extras"];
        assert_eq!(extras["unbounded_faces"], 2);
        assert_eq!(extras["rejected_filters"], 3);
        assert_eq!(extras["excluded_visibility_branches"], 4);
        assert_eq!(extras["excluded_non_surface_branches"], 5);
    }

    #[test]
    fn glb_rejects_empty_or_zero_normal_primitives() {
        let mut empty = scene();
        empty.elements[0].meshes.primitives[0].vertices.clear();
        assert!(encode(&empty, &|_, _, _| None).is_err());

        let mut zero = scene();
        zero.elements[0].meshes.primitives[0].normals = vec![[0., 0., 0.]; 3];
        assert!(encode(&zero, &|_, _, _| None).is_err());

        let mut nonunit = scene();
        nonunit.elements[0].meshes.primitives[0].normals = vec![[2., 0., 0.]; 3];
        assert!(encode(&nonunit, &|_, _, _| None).is_err());

        let mut nonfinite = scene();
        nonfinite.elements[0].meshes.primitives[0].normals = vec![[f64::NAN, 0., 0.]; 3];
        assert!(encode(&nonfinite, &|_, _, _| None).is_err());
    }
}
