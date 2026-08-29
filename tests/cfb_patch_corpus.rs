//! Always-on stream-level patch corpus (Lane Twelve / M9-01).
//!
//! Complements the optional Autodesk-family / project-corpus coverage in
//! [`cfb_roundtrip_delta`] with fixtures that never skip in Cloud / CI:
//!
//! - **Project**: synthetic CFB from `gen-fixture` (license-free).
//! - **Family**: committed MIT `tests/fixtures/families/empty.rfa`
//!   (`DynamoDS/RevitTestFramework`).
//!
//! Cases: identity, grow, shrink, multi-stream, and missing-stream
//! (`Error::StreamNotFound`, no output file).

use rvt::streams::{BASIC_FILE_INFO, GLOBAL_LATEST, PART_ATOM};
use rvt::writer::{
    guid_preserved, history_entries_preserved, write_with_patches, StreamFraming, StreamPatch,
};
use rvt::{Error, Result, RevitFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch_dir(tag: &str) -> PathBuf {
    let dir = workspace_root()
        .join("target")
        .join("cfb-patch-corpus")
        .join(tag);
    fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

/// Build a synthetic project-shaped `.rvt` via `gen-fixture`.
fn synthetic_project_fixture(tag: &str) -> PathBuf {
    let out = scratch_dir("project").join(format!("{tag}.rvt"));
    let status = Command::new(env!("CARGO"))
        .args(["run", "--quiet", "--bin", "gen-fixture", "--"])
        .arg(format!("patch-corpus-{tag}"))
        .arg("--output")
        .arg(&out)
        .args([
            "--seed",
            "12",
            "--year",
            "2024",
            "--classes",
            "Wall,Level,Project,Column,Door",
            "--element-count",
            "25",
        ])
        .current_dir(workspace_root())
        .status()
        .expect("cargo run gen-fixture");
    assert!(
        status.success(),
        "gen-fixture failed for project fixture {tag}: {status:?}"
    );
    assert!(out.exists(), "gen-fixture did not create {}", out.display());
    out
}

/// Redistributable MIT family fixture committed under tests/fixtures.
fn redistributable_family_fixture() -> PathBuf {
    let path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("families")
        .join("empty.rfa");
    assert!(
        path.exists(),
        "missing redistributable family fixture at {} \
         (expected tests/fixtures/families/empty.rfa + sibling .license.json)",
        path.display()
    );
    path
}

fn snapshot_streams(path: &Path) -> Result<Vec<(String, Vec<u8>)>> {
    let mut rf = RevitFile::open(path)?;
    let mut streams = Vec::new();
    for name in rf.stream_names() {
        let bytes = rf.read_stream(&name)?;
        streams.push((name, bytes));
    }
    Ok(streams)
}

/// Prefer verbatim-friendly streams that are large enough to grow and
/// shrink. Skip BasicFileInfo / Global/Latest so GUID + history checks
/// remain meaningful.
fn mutable_targets(streams: &[(String, Vec<u8>)]) -> Vec<(String, Vec<u8>)> {
    let mut candidates: Vec<(String, Vec<u8>)> = streams
        .iter()
        .filter(|(name, bytes)| {
            name != BASIC_FILE_INFO && name != GLOBAL_LATEST && bytes.len() >= 64
        })
        .cloned()
        .collect();
    candidates.sort_by_key(|(name, _)| match name.as_str() {
        PART_ATOM => 0,
        "Contents" => 1,
        "RevitPreview4.0" => 2,
        "TransmissionData" => 3,
        _ => 4,
    });
    candidates
}

fn assert_patch_roundtrip(
    label: &str,
    src: &Path,
    dst: &Path,
    snapshots: &[(String, Vec<u8>)],
    patches: &[StreamPatch],
) -> Result<()> {
    write_with_patches(src, dst, patches)?;

    let mut written = RevitFile::open(dst)?;
    for patch in patches {
        let actual = written.read_stream(&patch.stream_name)?;
        assert_eq!(
            actual, patch.new_decompressed,
            "{label}: {} patched stream did not round-trip",
            patch.stream_name
        );
    }

    for (name, original) in snapshots {
        if patches.iter().any(|patch| patch.stream_name == *name) {
            continue;
        }
        let actual = written.read_stream(name)?;
        assert_eq!(
            &actual,
            original,
            "{label}: unpatched stream {name} changed (src={} B, dst={} B)",
            original.len(),
            actual.len()
        );
    }

    assert!(
        guid_preserved(src, dst)?,
        "{label}: document GUID changed after stream patch"
    );
    assert!(
        history_entries_preserved(src, dst)?,
        "{label}: document history changed after stream patch"
    );

    Ok(())
}

fn assert_missing_stream_errors(label: &str, src: &Path, dst: &Path) {
    if dst.exists() {
        let _ = fs::remove_file(dst);
    }
    let missing_name = "Does/Not/Exist";
    let patch = StreamPatch {
        stream_name: missing_name.into(),
        new_decompressed: b"must-not-write".to_vec(),
        framing: StreamFraming::Verbatim,
    };
    let err = write_with_patches(src, dst, &[patch])
        .expect_err(&format!("{label}: missing stream must error"));
    assert!(
        matches!(&err, Error::StreamNotFound(name) if name == missing_name),
        "{label}: expected StreamNotFound({missing_name:?}), got {err:?}"
    );
    assert!(
        !dst.exists(),
        "{label}: writer created {} even though patch validation failed",
        dst.display()
    );
}

fn run_patch_modes(label: &str, src: &Path, out_dir: &Path) -> Result<()> {
    let snapshots = snapshot_streams(src)?;
    let targets = mutable_targets(&snapshots);
    assert!(
        targets.len() >= 2,
        "{label}: need at least two mutable streams on {}; got {:?}",
        src.display(),
        targets
            .iter()
            .map(|(n, b)| format!("{n}={}", b.len()))
            .collect::<Vec<_>>()
    );

    let (target_a, bytes_a) = &targets[0];
    let (target_b, bytes_b) = &targets[1];

    let identity = [StreamPatch {
        stream_name: target_a.clone(),
        new_decompressed: bytes_a.clone(),
        framing: StreamFraming::Verbatim,
    }];
    assert_patch_roundtrip(label, src, &out_dir.join("identity"), &snapshots, &identity)?;

    let mut grown = bytes_a.clone();
    grown.extend(std::iter::repeat_n(0xA5u8, 4096));
    let grow = [StreamPatch {
        stream_name: target_a.clone(),
        new_decompressed: grown,
        framing: StreamFraming::Verbatim,
    }];
    assert_patch_roundtrip(label, src, &out_dir.join("grow"), &snapshots, &grow)?;

    let shrink_len = (bytes_b.len() / 2).max(1);
    let shrunk = bytes_b[..shrink_len].to_vec();
    let shrink = [StreamPatch {
        stream_name: target_b.clone(),
        new_decompressed: shrunk,
        framing: StreamFraming::Verbatim,
    }];
    assert_patch_roundtrip(label, src, &out_dir.join("shrink"), &snapshots, &shrink)?;

    let mut multi_a = bytes_a.clone();
    multi_a.extend(std::iter::repeat_n(0x11u8, 1024));
    let multi_b = bytes_b[..shrink_len].to_vec();
    let multi = [
        StreamPatch {
            stream_name: target_a.clone(),
            new_decompressed: multi_a,
            framing: StreamFraming::Verbatim,
        },
        StreamPatch {
            stream_name: target_b.clone(),
            new_decompressed: multi_b,
            framing: StreamFraming::Verbatim,
        },
    ];
    assert_patch_roundtrip(label, src, &out_dir.join("multi"), &snapshots, &multi)?;

    assert_missing_stream_errors(label, src, &out_dir.join("missing-stream"));
    Ok(())
}

#[test]
fn project_gen_fixture_patch_grow_shrink_multi_and_missing() -> Result<()> {
    let src = synthetic_project_fixture("project");
    let out = scratch_dir("out-project");
    run_patch_modes("project/gen-fixture", &src, &out)
}

#[test]
fn family_mit_fixture_patch_grow_shrink_multi_and_missing() -> Result<()> {
    let src = redistributable_family_fixture();
    let out = scratch_dir("out-family");
    run_patch_modes("family/empty.rfa", &src, &out)
}

#[test]
fn missing_stream_error_message_is_actionable_on_both_fixtures() -> Result<()> {
    let project = synthetic_project_fixture("missing-msg");
    let family = redistributable_family_fixture();
    let out = scratch_dir("out-missing-msg");

    for (label, src) in [("project", project.as_path()), ("family", family.as_path())] {
        let dst = out.join(format!("{label}-missing.out"));
        let patch = StreamPatch {
            stream_name: "Typo/Stream/Name".into(),
            new_decompressed: vec![1, 2, 3],
            framing: StreamFraming::Verbatim,
        };
        let err = write_with_patches(src, &dst, &[patch]).expect_err("must error");
        let rendered = err.to_string();
        assert!(
            matches!(&err, Error::StreamNotFound(name) if name == "Typo/Stream/Name"),
            "{label}: expected StreamNotFound, got {err:?}"
        );
        assert!(
            rendered.contains("Typo/Stream/Name"),
            "{label}: error string should name the missing stream; got {rendered:?}"
        );
        assert!(
            !dst.exists(),
            "{label}: must not create output on validation failure"
        );
    }
    Ok(())
}
