use super::Scene;
use anyhow::{Result, ensure};
use serde_json::json;

pub fn encode(scene: &Scene) -> Result<Vec<u8>> {
    let (mut nodes, mut meshes, mut views, mut accessors) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut bin = Vec::new();
    for element in &scene.elements {
        let extras = json!({"native_element_id":element.id,"native_identity":element.identity,"native_class":element.class,"status":element.status,"diagnostics":element.diagnostics,"source":element.source});
        if element.meshes.is_empty() {
            nodes.push(json!({"name":format!("{} {}",element.class,element.id),"extras":extras}));
            continue;
        }
        for mesh in &element.meshes {
            ensure!(
                !mesh.vertices.is_empty()
                    && !mesh.triangles.is_empty()
                    && mesh
                        .triangles
                        .iter()
                        .flatten()
                        .all(|&i| (i as usize) < mesh.vertices.len()),
                "invalid GLB mesh indices or empty mesh"
            );
            let position_offset = bin.len();
            let mut low = [f32::INFINITY; 3];
            let mut high = [f32::NEG_INFINITY; 3];
            for p in &mesh.vertices {
                let converted = [p[0] * 0.3048, p[2] * 0.3048, -p[1] * 0.3048];
                for j in 0..3 {
                    let value = converted[j] as f32;
                    ensure!(value.is_finite(), "GLB position exceeds finite f32 range");
                    low[j] = low[j].min(value);
                    high[j] = high[j].max(value);
                    bin.extend(value.to_le_bytes());
                }
            }
            let pv = views.len();
            views.push(json!({"buffer":0,"byteOffset":position_offset,"byteLength":bin.len()-position_offset,"target":34962}));
            let pa = accessors.len();
            accessors.push(json!({"bufferView":pv,"componentType":5126,"count":mesh.vertices.len(),"type":"VEC3","min":low,"max":high}));
            let index_offset = bin.len();
            for t in &mesh.triangles {
                for i in t {
                    bin.extend(i.to_le_bytes());
                }
            }
            let iv = views.len();
            views.push(json!({"buffer":0,"byteOffset":index_offset,"byteLength":bin.len()-index_offset,"target":34963}));
            let ia = accessors.len();
            accessors.push(json!({"bufferView":iv,"componentType":5125,"count":mesh.triangles.len()*3,"type":"SCALAR"}));
            let mi = meshes.len();
            meshes.push(json!({"name":format!("{} {}",element.class,element.id),"primitives":[{"attributes":{"POSITION":pa},"indices":ia,"mode":4,"material":0}]}));
            nodes.push(
                json!({"name":format!("{} {}",element.class,element.id),"mesh":mi,"extras":extras}),
            );
        }
    }
    let mut doc = json!({"asset":{"version":"2.0","generator":"rvt-rs native scene"},"scene":0,"scenes":[{"nodes":(0..nodes.len()).collect::<Vec<_>>()}],"nodes":nodes,"extras":{"complete_document_geometry":false,"status":scene.status,"diagnostics":scene.diagnostics,"schema_sha256":scene.schema_sha256,"source_sha256":scene.source_sha256,"coverage":scene.coverage,"coordinate_conversion":"native feet/Z-up to meters/Y-up: (x,z,-y)*0.3048"}});
    if !meshes.is_empty() {
        doc["meshes"] = json!(meshes);
        doc["bufferViews"] = json!(views);
        doc["accessors"] = json!(accessors);
        doc["buffers"] = json!([{"byteLength":bin.len()}]);
        doc["materials"] = json!([{"pbrMetallicRoughness":{"baseColorFactor":[0.65,0.72,0.8,1.0],"metallicFactor":0.0,"roughnessFactor":0.8}}]);
    }
    let mut text = serde_json::to_vec(&doc)?;
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }
    let total = 12usize
        .checked_add(8 + text.len())
        .and_then(|n| n.checked_add(if bin.is_empty() { 0 } else { 8 + bin.len() }))
        .ok_or_else(|| anyhow::anyhow!("GLB length overflow"))?;
    ensure!(
        total <= u32::MAX as usize,
        "GLB exceeds format length budget"
    );
    let mut output = Vec::with_capacity(total);
    output.extend(0x46546c67u32.to_le_bytes());
    output.extend(2u32.to_le_bytes());
    output.extend((total as u32).to_le_bytes());
    output.extend((text.len() as u32).to_le_bytes());
    output.extend(0x4e4f534au32.to_le_bytes());
    output.extend(text);
    if !bin.is_empty() {
        output.extend((bin.len() as u32).to_le_bytes());
        output.extend(0x004e4942u32.to_le_bytes());
        output.extend(bin);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_scene::{Element, Mesh, Source};
    #[test]
    fn glb_positions_convert_feet_z_up_to_meters_y_up() {
        let source = Source {
            stream: "Partitions/0".into(),
            group_marker_offset: 0,
            group_record_offset: 0,
            group_segment_count: 1,
            body_sha256: String::new(),
            stored_revision: 0,
            other_revision: 0,
        };
        let mut scene = Scene::default();
        scene.elements.push(Element {
            id: 1,
            identity: crate::native_index::Identity {
                element_id: 1,
                original_id_suffix: 19,
                creation_episode: 0,
                stored_revision: 0,
                other_revision: 0,
                row_offset: 6,
                raw_fields: vec![],
                owning_element_id: -1,
                partition_id: 0,
                unique_id: "test-00000013".into(),
            },
            class: "Wall".into(),
            status: "test".into(),
            diagnostics: vec![],
            source,
            curves: vec![],
            geometry: None,
            meshes: vec![Mesh {
                vertices: vec![[1., 2., 3.], [2., 2., 3.], [1., 3., 3.]],
                triangles: vec![[0, 1, 2]],
            }],
        });
        let glb = encode(&scene).unwrap();
        let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let data = 20 + length + 8;
        let got: [f32; 3] = std::array::from_fn(|i| {
            f32::from_le_bytes(glb[data + 4 * i..data + 4 * i + 4].try_into().unwrap())
        });
        assert_eq!(got, [0.3048, 0.9144, -0.6096]);
        let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + length]).unwrap();
        assert_eq!(json["extras"]["complete_document_geometry"], false);
        assert_eq!(json["nodes"][0]["extras"]["native_element_id"], 1);
        assert_eq!(
            json["nodes"][0]["extras"]["native_identity"]["unique_id"],
            "test-00000013"
        );
    }
    #[test]
    fn unsupported_scene_has_no_placeholder_mesh() {
        let scene = Scene {
            status: "unsupported_schema_profile".into(),
            ..Scene::default()
        };
        let glb = encode(&scene).unwrap();
        let length = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let json: serde_json::Value = serde_json::from_slice(&glb[20..20 + length]).unwrap();
        assert!(json.get("meshes").is_none());
        assert!(json.get("buffers").is_none());
    }
}
