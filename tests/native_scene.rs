//! Optional actual-native tests. RVT_NATIVE_SCENE_RUN_DIR must name a downloaded
//! controlled run with RVTs and Revit API geometry snapshots.
use rvt::native_scene;
use serde_json::Value;
use std::{fs, path::PathBuf};
#[test]
fn native_scene_preserves_identities_and_withholds_known_unsupported_geometry() {
    let Some(root) = std::env::var_os("RVT_NATIVE_SCENE_RUN_DIR") else {
        eprintln!("native scene corpus not configured");
        return;
    };
    let root = PathBuf::from(root);
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for variant in manifest["variants"].as_array().unwrap() {
        if variant["outcome"] == "api_rejected" {
            continue;
        }
        let name = variant["variant"].as_str().unwrap();
        let model = root.join("models").join(format!("{name}.rvt"));
        let mut file = rvt::RevitFile::open(model).unwrap();
        let scene = native_scene::extract(&mut file).unwrap();
        assert_eq!(scene.status, "partial_native_geometry");
        assert!(!scene.complete_document_geometry);
        let oracle: Value = serde_json::from_slice(
            &fs::read(root.join("geometry").join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        let expected_ids: std::collections::BTreeSet<u64> = oracle["elements"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| {
                ["Autodesk.Revit.DB.Wall", "Autodesk.Revit.DB.Floor"]
                    .contains(&e["class"].as_str().unwrap())
            })
            .map(|e| e["id"].as_u64().unwrap())
            .collect();
        let actual_ids: std::collections::BTreeSet<u64> =
            scene.elements.iter().map(|e| e.id).collect();
        assert_eq!(
            actual_ids, expected_ids,
            "{name}: native owners missing or invented"
        );
        for e in &scene.elements {
            let expected = oracle["elements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|x| x["id"].as_u64() == Some(e.id))
                .unwrap();
            assert!(
                expected["class"]
                    .as_str()
                    .unwrap()
                    .ends_with(&format!(".{}", e.class))
            );
            assert_eq!(e.identity.element_id, e.id);
            assert_eq!(
                e.identity.unique_id,
                expected["unique_id"].as_str().unwrap(),
                "{name}: native geometry identity disagrees with API"
            );
            if name == "wall_rectangular_cut" && e.id == 2789 {
                assert_eq!(e.status, "decoded_wall_with_rectangular_cuts");
            }
            let should_reject = match name {
                "wall_t_join" | "wall_corner_join" => e.class == "Wall" && e.id != 2790,
                "floor_inner_sketch_loop" | "floor_slope" => e.class == "Floor",
                _ => false,
            };
            if should_reject {
                assert!(
                    e.meshes.is_empty(),
                    "{name}: unsupported geometry emitted for {}",
                    e.id
                );
                assert!(!e.diagnostics.is_empty());
            } else {
                assert_eq!(
                    e.meshes.len(),
                    1,
                    "{name}: missing geometry for {}: {:?}",
                    e.id,
                    e.diagnostics
                );
            }
        }
        let glb = scene.to_glb().unwrap();
        assert_eq!(&glb[..4], b"glTF");
        assert_eq!(
            u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize,
            glb.len()
        );
    }
}
