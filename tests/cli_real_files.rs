//! B53: the six shipped CLIs that only `--help` was tested for, run end to
//! end on real Revit files.
//!
//! Each test runs the built binary and checks what the tool's own
//! documentation promises, against facts the files state independently of
//! the tool (the release a family corpus file's name gives, the stream list,
//! an unchanged export):
//!
//! - `rvt-elem-table --json`: the parsed records number the table's
//!   `record_count`, their ids rise strictly, and every owner is a declared id;
//! - `rvt-history --json`: the releases named rise and end with the file's own;
//! - `rvt-analyze --json`: the identity's release is the file's;
//! - `rvt-diff`: a file against itself has every stream identical, and two
//!   releases of the same family differ in some stream;
//! - `rvt-ifc-compare --fail-on-diff`: an export against itself exits 0 with
//!   no entity count delta, and against Revit's own export exits 2;
//! - `rvt-write`: patching `Formats/Latest` or `Global/Latest` with its own
//!   decompressed bytes either writes a file whose patched stream rvt-rs reads
//!   back as the same bytes, the other streams byte-identical, or is refused
//!   with an error naming the stored pages; never exits 0 with a stream that
//!   does not decode.
//!
//! Runs against `RVT_SAMPLES_DIR` (the phi-ag family corpus) and
//! `RVT_PROJECT_CORPUS_DIR`. Skips what is absent.

mod common;

use common::{ALL_YEARS, sample_for_year};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Command, Output};

fn run(bin: &str, args: &[&std::ffi::OsStr]) -> Output {
    Command::new(bin).args(args).output().expect("run binary")
}

fn ok(bin: &str, output: &Output) -> String {
    assert!(
        output.status.success(),
        "{bin} failed ({:?})\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn families() -> Vec<(u32, PathBuf)> {
    ALL_YEARS
        .iter()
        .map(|&year| (year, sample_for_year(year)))
        .filter(|(_, path)| path.exists())
        .collect()
}

fn project(name: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?).join(name);
    path.exists().then_some(path)
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvt-rs-cli-real-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// Every four-digit release year (2000 to 2099) in `text`, in order.
fn years(text: &str) -> Vec<u32> {
    let bytes = text.as_bytes();
    (0..bytes.len().saturating_sub(3))
        .filter(|&i| {
            bytes[i] == b'2'
                && bytes[i + 1] == b'0'
                && bytes[i + 2..i + 4].iter().all(u8::is_ascii_digit)
                && (i == 0 || !bytes[i - 1].is_ascii_digit())
                && bytes.get(i + 4).is_none_or(|b| !b.is_ascii_digit())
        })
        .filter_map(|i| text[i..i + 4].parse().ok())
        .collect()
}

#[test]
fn elem_table_json_holds_the_tables_records() {
    let mut files: Vec<PathBuf> = families().into_iter().map(|(_, p)| p).collect();
    files.extend(project("Revit_IFC5_Einhoven.rvt"));
    files.extend(project("2024_Core_Interior.rvt"));
    if files.is_empty() {
        eprintln!("skipping: no corpus");
        return;
    }
    for file in &files {
        let out = ok(
            "rvt-elem-table",
            &run(
                env!("CARGO_BIN_EXE_rvt-elem-table"),
                &[file.as_os_str(), "--json".as_ref()],
            ),
        );
        let json: Value = serde_json::from_str(&out).expect("rvt-elem-table JSON");
        let name = file.file_name().unwrap_or_default().to_string_lossy();
        let records = json["records"].as_array().expect("records");
        assert_eq!(
            json["parsed_records"].as_u64(),
            json["record_count"].as_u64(),
            "{name}: parsed records against the table's record_count"
        );
        assert_eq!(
            records.len() as u64,
            json["parsed_records"].as_u64().unwrap_or(0),
            "{name}"
        );
        let ids: Vec<u64> = records
            .iter()
            .filter_map(|r| r["id_primary"].as_u64())
            .collect();
        assert!(
            ids.windows(2).all(|w| w[0] < w[1]),
            "{name}: record ids do not rise strictly"
        );
        let declared: std::collections::BTreeSet<u64> = ids.iter().copied().collect();
        let stray: Vec<u64> = records
            .iter()
            .filter_map(|r| r["owner_id"].as_u64())
            .filter(|owner| !declared.contains(owner))
            .take(5)
            .collect();
        assert!(stray.is_empty(), "{name}: owners not declared: {stray:?}");
    }
}

#[test]
fn history_ends_with_the_files_own_release() {
    let files = families();
    if files.is_empty() {
        eprintln!("skipping: family corpus missing");
        return;
    }
    for (year, file) in &files {
        let out = ok(
            "rvt-history",
            &run(
                env!("CARGO_BIN_EXE_rvt-history"),
                &[file.as_os_str(), "--json".as_ref()],
            ),
        );
        let json: Value = serde_json::from_str(&out).expect("rvt-history JSON");
        let entries: Vec<String> = json["entries"]
            .as_array()
            .expect("entries")
            .iter()
            .filter_map(|e| e.as_str().map(String::from))
            .collect();
        let releases: Vec<u32> = entries
            .iter()
            .filter_map(|e| years(e).first().copied())
            .collect();
        assert!(!releases.is_empty(), "{year}: no release in {entries:?}");
        assert!(
            releases.windows(2).all(|w| w[0] <= w[1]),
            "{year}: releases out of order: {releases:?}"
        );
        assert_eq!(releases.last(), Some(year), "{year}: history {entries:?}");
    }
}

#[test]
fn analyze_identity_is_the_files_release() {
    let files = families();
    if files.is_empty() {
        eprintln!("skipping: family corpus missing");
        return;
    }
    for (year, file) in &files {
        let out = ok(
            "rvt-analyze",
            &run(
                env!("CARGO_BIN_EXE_rvt-analyze"),
                &[file.as_os_str(), "--json".as_ref()],
            ),
        );
        let json: Value = serde_json::from_str(&out).expect("rvt-analyze JSON");
        assert_eq!(
            json["identity"]["revit_version"].as_u64(),
            Some(u64::from(*year)),
            "{year}: rvt-analyze identity {}",
            json["identity"]
        );
    }
}

#[test]
fn diff_finds_a_file_identical_to_itself_and_two_releases_different() {
    let files = families();
    let (Some((_, a)), Some((_, b))) = (files.first(), files.last()) else {
        eprintln!("skipping: family corpus missing");
        return;
    };
    if a == b {
        eprintln!("skipping: one family file");
        return;
    }
    let same = ok(
        "rvt-diff",
        &run(
            env!("CARGO_BIN_EXE_rvt-diff"),
            &[a.as_os_str(), a.as_os_str()],
        ),
    );
    let byte_level = same
        .split_once("=== byte-level diff")
        .map(|(_, rest)| rest)
        .expect("byte-level section");
    let lines: Vec<&str> = byte_level
        .lines()
        .skip(1)
        .filter(|l| l.starts_with("  ") && !l.starts_with("   "))
        .collect();
    assert!(!lines.is_empty(), "no stream compared:\n{same}");
    assert!(
        lines.iter().all(|l| l.trim_end().ends_with("IDENTICAL")),
        "a file differs from itself:\n{byte_level}"
    );
    let differ = ok(
        "rvt-diff",
        &run(
            env!("CARGO_BIN_EXE_rvt-diff"),
            &[a.as_os_str(), b.as_os_str()],
        ),
    );
    let byte_level = differ
        .split_once("=== byte-level diff")
        .map(|(_, rest)| rest)
        .expect("byte-level section");
    assert!(
        byte_level.lines().any(|l| l.contains("Δbytes=")),
        "two releases show no differing stream:\n{byte_level}"
    );
}

#[test]
fn ifc_compare_tells_an_export_from_itself_and_from_revits() {
    let Some(rvt) = project("RE1-Architecture.rvt") else {
        eprintln!("skipping: RE1-Architecture.rvt absent");
        return;
    };
    let revit_ifc = rvt.with_extension("ifc");
    let dir = scratch("ifc-compare");
    let ours = dir.join("ours.ifc");
    ok(
        "rvt-ifc",
        &run(
            env!("CARGO_BIN_EXE_rvt-ifc"),
            &[rvt.as_os_str(), "-o".as_ref(), ours.as_os_str()],
        ),
    );
    let report = dir.join("self.json");
    let itself = run(
        env!("CARGO_BIN_EXE_rvt-ifc-compare"),
        &[
            ours.as_os_str(),
            ours.as_os_str(),
            "--fail-on-diff".as_ref(),
            "--quiet".as_ref(),
            "--json".as_ref(),
            report.as_os_str(),
        ],
    );
    assert_eq!(itself.status.code(), Some(0), "an export against itself");
    let json: Value =
        serde_json::from_slice(&std::fs::read(&report).expect("report")).expect("report JSON");
    assert_eq!(
        json["entity_count_deltas"]
            .as_object()
            .map(|deltas| deltas.len()),
        Some(0),
        "entity count deltas against itself: {}",
        json["entity_count_deltas"]
    );
    if revit_ifc.exists() {
        let against_revit = run(
            env!("CARGO_BIN_EXE_rvt-ifc-compare"),
            &[
                ours.as_os_str(),
                revit_ifc.as_os_str(),
                "--fail-on-diff".as_ref(),
                "--quiet".as_ref(),
            ],
        );
        assert_eq!(
            against_revit.status.code(),
            Some(2),
            "rvt-rs's IFC4 export against Revit's IFC2X3 export differs structurally"
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn write_round_trips_or_refuses_a_streams_own_bytes() {
    let files = families();
    if files.is_empty() {
        eprintln!("skipping: family corpus missing");
        return;
    }
    let dir = scratch("write");
    let (mut written_ok, mut refused) = (0, 0);
    for (year, source) in &files {
        for (stream, framing) in [
            ("Formats/Latest", "RawGzipFromZero"),
            ("Global/Latest", "CustomPrefix8"),
        ] {
            let mut rf = rvt::RevitFile::open(source).expect("open source");
            let decoded =
                rvt::native_document::read_single(&mut rf, stream).expect("decode source stream");
            let manifest = dir.join("patches.json");
            let patches = serde_json::json!({
                "patches": [{ "stream_name": stream, "new_decompressed": decoded, "framing": framing }]
            });
            std::fs::write(&manifest, serde_json::to_vec(&patches).expect("manifest"))
                .expect("write manifest");
            let target = dir.join(format!("patched-{year}.rfa"));
            let _ = std::fs::remove_file(&target);
            let output = run(
                env!("CARGO_BIN_EXE_rvt-write"),
                &[
                    "--src".as_ref(),
                    source.as_os_str(),
                    "--dst".as_ref(),
                    target.as_os_str(),
                    "--patches".as_ref(),
                    manifest.as_os_str(),
                ],
            );
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(
                    stderr.contains("page"),
                    "{year} {stream}: rvt-write failed for another reason than the stored pages:\n{stderr}"
                );
                assert!(
                    !target.exists(),
                    "{year} {stream}: a refused patch left a file"
                );
                refused += 1;
                continue;
            }
            // rvt-write said it wrote the stream: rvt-rs's reader must read
            // back the bytes it was given, and nothing else may change.
            let mut written = rvt::RevitFile::open(&target).expect("open written file");
            let read_back =
                rvt::native_document::read_single(&mut written, stream).unwrap_or_else(|e| {
                    panic!(
                        "{year} {stream}: rvt-write exited 0 but the stream does not decode: {e:#}"
                    )
                });
            assert!(
                read_back == decoded,
                "{year} {stream}: decodes to other bytes after the round trip"
            );
            let names = rf.stream_names();
            assert_eq!(
                written.stream_names(),
                names,
                "{year}: the written file's streams"
            );
            for name in names.iter().filter(|n| n.as_str() != stream) {
                assert_eq!(
                    written.read_stream(name).expect("written stream"),
                    rf.read_stream(name).expect("source stream"),
                    "{year}: {name} changed though no patch named it"
                );
            }
            written_ok += 1;
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    eprintln!("rvt-write: {written_ok} patches round-tripped, {refused} refused");
    assert!(written_ok > 0, "no patch round-tripped");
}
