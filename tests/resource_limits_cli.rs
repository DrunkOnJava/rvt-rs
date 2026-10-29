//! `rvt-ifc` and `rvt-gltf` when a resource limit keeps part of the input
//! from being read (#1075): they name the stream, write no model and exit
//! with status 3 unless `--allow-partial`, and the diagnostics list it.
//!
//! The committed tier-1 fixture runs everywhere: its `BasicFileInfo` is 236
//! bytes, so `--max-stream-bytes 200` stops it. Core Interior, when
//! `RVT_PROJECT_CORPUS_DIR` holds it, is the shape of #1075 itself: its
//! `Partitions/46` is about 17 MB stored, over an 8 MiB stream limit.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const REASON: &str = "stream_not_read_resource_limit";

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/architectural-2024/architectural-2024.rvt")
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rvt-rs-resource-limits-{tag}-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn run(bin: &str, args: &[&std::ffi::OsStr]) -> Output {
    Command::new(bin).args(args).output().expect("run the CLI")
}

fn report(output: &Output) -> String {
    format!(
        "status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn diagnostics(path: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(path).expect("read the diagnostics"))
        .expect("parse the diagnostics")
}

/// The streams the diagnostics say a limit stopped, or `None` when the
/// skipped item is absent; the warnings must name each one.
fn unread_streams(json: &Value) -> Option<Vec<String>> {
    let item = json["skipped"]
        .as_array()?
        .iter()
        .find(|item| item["reason"] == REASON)?;
    let streams: Vec<String> = item["classes"]
        .as_object()
        .expect("classes is an object")
        .keys()
        .cloned()
        .collect();
    assert_eq!(item["count"].as_u64(), Some(streams.len() as u64));
    let warnings: Vec<&str> = json["warnings"]
        .as_array()
        .expect("warnings is an array")
        .iter()
        .filter_map(Value::as_str)
        .filter(|warning| warning.starts_with("Not read whole: "))
        .collect();
    for stream in &streams {
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains(&format!("stream {stream} "))),
            "no warning names {stream}: {warnings:?}"
        );
    }
    Some(streams)
}

#[test]
fn rvt_ifc_refuses_a_partial_read_with_status_3() {
    let dir = scratch("refuse");
    let ifc = dir.join("out.ifc");
    let sidecar = dir.join("out.diagnostics.json");
    let output = run(
        env!("CARGO_BIN_EXE_rvt-ifc"),
        &[
            fixture().as_os_str(),
            "--max-stream-bytes".as_ref(),
            "200".as_ref(),
            "-o".as_ref(),
            ifc.as_os_str(),
            "--diagnostics".as_ref(),
            sidecar.as_os_str(),
        ],
    );
    assert_eq!(output.status.code(), Some(3), "{}", report(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("not read whole") && stderr.contains("BasicFileInfo"),
        "{}",
        report(&output)
    );
    assert!(!ifc.exists(), "no IFC is written without --allow-partial");
    let streams = unread_streams(&diagnostics(&sidecar)).expect("the skipped item is written");
    assert!(streams.iter().any(|s| s == "BasicFileInfo"), "{streams:?}");
}

#[test]
fn rvt_ifc_allow_partial_writes_the_incomplete_export() {
    let dir = scratch("allow");
    let ifc = dir.join("out.ifc");
    let sidecar = dir.join("out.diagnostics.json");
    let output = run(
        env!("CARGO_BIN_EXE_rvt-ifc"),
        &[
            fixture().as_os_str(),
            "--max-stream-bytes".as_ref(),
            "200".as_ref(),
            "--allow-partial".as_ref(),
            "-o".as_ref(),
            ifc.as_os_str(),
            "--diagnostics".as_ref(),
            sidecar.as_os_str(),
        ],
    );
    assert!(output.status.success(), "{}", report(&output));
    let step = std::fs::read_to_string(&ifc).expect("the IFC is written");
    assert!(
        step.starts_with("ISO-10303-21;"),
        "{}",
        &step[..step.len().min(80)]
    );
    let streams = unread_streams(&diagnostics(&sidecar)).expect("the skipped item is written");
    assert!(streams.iter().any(|s| s == "BasicFileInfo"), "{streams:?}");
}

#[test]
fn rvt_ifc_reads_the_fixture_whole_at_the_default_limits() {
    let dir = scratch("default");
    let ifc = dir.join("out.ifc");
    let sidecar = dir.join("out.diagnostics.json");
    let output = run(
        env!("CARGO_BIN_EXE_rvt-ifc"),
        &[
            fixture().as_os_str(),
            "-o".as_ref(),
            ifc.as_os_str(),
            "--diagnostics".as_ref(),
            sidecar.as_os_str(),
        ],
    );
    assert!(output.status.success(), "{}", report(&output));
    assert!(ifc.exists());
    assert_eq!(unread_streams(&diagnostics(&sidecar)), None);
    assert!(!String::from_utf8_lossy(&output.stderr).contains("not read whole"));
}

#[test]
fn rvt_gltf_refuses_a_partial_read_with_status_3() {
    let dir = scratch("gltf");
    let glb = dir.join("out.glb");
    let refused = run(
        env!("CARGO_BIN_EXE_rvt-gltf"),
        &[
            fixture().as_os_str(),
            "--max-stream-bytes".as_ref(),
            "200".as_ref(),
            "-o".as_ref(),
            glb.as_os_str(),
        ],
    );
    assert_eq!(refused.status.code(), Some(3), "{}", report(&refused));
    assert!(!glb.exists(), "no glb is written without --allow-partial");
    let allowed = run(
        env!("CARGO_BIN_EXE_rvt-gltf"),
        &[
            fixture().as_os_str(),
            "--max-stream-bytes".as_ref(),
            "200".as_ref(),
            "--allow-partial".as_ref(),
            "-o".as_ref(),
            glb.as_os_str(),
        ],
    );
    assert!(allowed.status.success(), "{}", report(&allowed));
    assert!(glb.exists());
}

#[test]
fn core_interior_partition_over_the_stream_limit_is_named() {
    let Some(corpus) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is unset");
        return;
    };
    let model = PathBuf::from(corpus).join("2024_Core_Interior.rvt");
    if !model.exists() {
        eprintln!("skipping: {} is missing", model.display());
        return;
    }
    let dir = scratch("core-interior");
    let ifc = dir.join("out.ifc");
    let sidecar = dir.join("out.diagnostics.json");
    let output = run(
        env!("CARGO_BIN_EXE_rvt-ifc"),
        &[
            model.as_os_str(),
            "--max-stream-bytes".as_ref(),
            "8388608".as_ref(),
            "--allow-partial".as_ref(),
            "-o".as_ref(),
            ifc.as_os_str(),
            "--diagnostics".as_ref(),
            sidecar.as_os_str(),
        ],
    );
    assert!(output.status.success(), "{}", report(&output));
    let json = diagnostics(&sidecar);
    let streams = unread_streams(&json).expect("the skipped item is written");
    assert!(
        streams.iter().any(|s| s == "Partitions/46"),
        "Partitions/46 is about 17 MB stored: {streams:?}"
    );
    let exported = json["exported"]["building_elements"]
        .as_u64()
        .expect("building_elements");
    assert!(
        exported < 970,
        "the 970 elements the full read exports include what Partitions/46 holds: {exported} exported"
    );
}
