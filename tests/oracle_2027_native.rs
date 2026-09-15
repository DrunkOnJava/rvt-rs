//! Optional exact-native regression; full RVTs remain in the authorized local lane.
//! RVT_ORACLE_RUN_DIR=/path/to/walls-v3 cargo test --release --test oracle_2027_native
use rvt::{RevitFile, compression, partition_parameter_records as parameters};
use serde_json::Value;
#[path = "../examples/support/oracle_record_frames_2027.rs"]
mod record_frames;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

#[test]
fn native_api_string_multisets_match_without_oracle_assisted_decoding() {
    let Some(root) = std::env::var_os("RVT_ORACLE_RUN_DIR") else {
        eprintln!("skipping native 2027 corpus: RVT_ORACLE_RUN_DIR unset");
        return;
    };
    let root = PathBuf::from(root);
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for variant in manifest["variants"].as_array().unwrap() {
        let name = variant["variant"].as_str().unwrap();
        let model = root.join("models").join(format!("{name}.rvt"));
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(&model).unwrap())),
            variant["sha256"].as_str().unwrap()
        );
        let mut rf = RevitFile::open(&model).unwrap();
        let version = rf.basic_file_info().unwrap().version;
        assert_eq!(version, 2027);
        let mut actual = BTreeMap::new();
        for stream in rf
            .stream_names()
            .iter()
            .filter(|name| name.starts_with("Partitions/"))
        {
            let stored = rf.read_stream(stream).unwrap();
            let prepared = compression::prepare_stream_for_inflate(stream, &stored);
            for offset in compression::find_gzip_offsets(&prepared) {
                let member = compression::inflate_at(&prepared, offset).unwrap();
                for (_, body) in record_frames::record_bodies(&member, version) {
                    for id in [
                        parameters::INSTANCE_MARK_PARAMETER_ID,
                        parameters::TYPE_MARK_PARAMETER_ID,
                    ] {
                        let report =
                            parameters::scan_bounded_string_parameter_occurrences_2027_with_limit(
                                &member[body.clone()],
                                i64::from(id),
                                version,
                                8192, // explicit research budget; 4097 exceeds the production default
                            );
                        assert!(
                            report.issues.is_empty(),
                            "incomplete scan in {name}: {:?}",
                            report.issues
                        );
                        for item in report.occurrences {
                            *actual
                                .entry((item.parameter_id, item.value))
                                .or_insert(0usize) += 1;
                        }
                    }
                }
            }
        }
        // The oracle is loaded only after byte extraction is complete.
        let snapshot: Value = serde_json::from_slice(
            &fs::read(root.join("snapshots").join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        let mut expected = BTreeMap::new();
        for element in snapshot["elements"].as_array().unwrap() {
            for parameter in element["parameters"].as_array().unwrap() {
                let id = parameter["id"].as_i64().unwrap();
                if ![-1001203, -1001405].contains(&id) {
                    continue;
                }
                if let Some(value) = parameter["raw_value"].as_str() {
                    *expected.entry((id, value.to_owned())).or_insert(0usize) += 1;
                }
            }
        }
        // Ordinary-save history retains the seed record in an older partition.
        // This byte scanner deliberately has no live-revision selection contract.
        if manifest["target_wall_id"].is_number()
            && matches!(
                name,
                "save_b"
                    | "save_c"
                    | "save_noop_c"
                    | "delete_target"
                    | "save_noop_deleted"
                    | "append_d"
            )
        {
            *expected
                .entry((-1001203, "HISTORY_A001".to_owned()))
                .or_insert(0usize) += 1;
        }
        assert_eq!(actual, expected, "native variant {name}");
    }
}

#[test]
fn native_defaults_do_not_invent_rooms_floors_or_proxy_meshes() {
    let Some(root) = std::env::var_os("RVT_ORACLE_RUN_DIR") else {
        eprintln!("skipping native 2027 corpus: RVT_ORACLE_RUN_DIR unset");
        return;
    };
    let root = PathBuf::from(root);
    let manifest: Value =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for variant in manifest["variants"].as_array().unwrap() {
        let name = variant["variant"].as_str().unwrap();
        let snapshot: Value = serde_json::from_slice(
            &fs::read(root.join("snapshots").join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        let api = snapshot["elements"].as_array().unwrap();
        assert!(api.iter().any(|e| e["class"] == "Autodesk.Revit.DB.Wall"));
        let mut rf = RevitFile::open(root.join("models").join(format!("{name}.rvt"))).unwrap();
        let production: Vec<_> = rvt::walker::iter_elements(&mut rf).unwrap().collect();
        for element in &production {
            let id = element
                .id
                .expect("production entity must have a native identity");
            assert!(
                api.iter().any(|e| e["id"].as_u64() == Some(u64::from(id))
                    && e["class"].as_str().unwrap().rsplit('.').next()
                        == Some(element.class.as_str())),
                "invented identity/class in {name}: {id} {}",
                element.class
            );
        }
        assert!(
            production
                .iter()
                .all(|e| e.provenance.record_kind.as_deref() != Some("partition_schema_mvp")),
            "unbound name/loop entered production in {name}"
        );
        let exported = rvt::ifc::RvtDocExporter
            .export_with_diagnostics(&mut rf)
            .unwrap();
        let (gltf, _) = rvt::ifc::gltf::build_gltf(&exported.model);
        assert!(
            gltf.meshes.len()
                <= exported
                    .diagnostics
                    .exported
                    .building_elements_with_geometry,
            "unverified proxy mesh in {name}"
        );
    }
}
