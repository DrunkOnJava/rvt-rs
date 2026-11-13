//! First-run ergonomics shared by the shipped CLIs: positional input with a
//! default output next to it, the older `--src` / `--dst` spelling, errors
//! that name the file, `--json` as shorthand for `--format json`, and a quiet
//! exit when stdout is closed early. Tier-1 synthetic fixtures only, except
//! the closed-pipe test, which needs output larger than a pipe buffer.

mod common;

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn fixture() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("tier1")
        .join("architectural-2024")
        .join("architectural-2024.rvt");
    if path.exists() {
        Some(path)
    } else {
        eprintln!("skipping: tier-1 fixture missing at {}", path.display());
        None
    }
}

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rvt-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(bin: &str, args: &[&std::ffi::OsStr]) -> Output {
    Command::new(bin).args(args).output().expect("spawn CLI")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn gltf_and_sheet_write_next_to_a_positional_input() {
    let Some(source) = fixture() else { return };
    let dir = scratch("positional");
    let input = dir.join("model.rvt");
    std::fs::copy(&source, &input).unwrap();

    let out = run(env!("CARGO_BIN_EXE_rvt-gltf"), &[input.as_os_str()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let glb = std::fs::read(dir.join("model.glb")).expect("model.glb written");
    assert_eq!(&glb[..4], b"glTF");
    assert_eq!(
        u32::from_le_bytes([glb[4], glb[5], glb[6], glb[7]]),
        2,
        "a glTF 2.0 binary container"
    );

    let out = run(env!("CARGO_BIN_EXE_rvt-sheet"), &[input.as_os_str()]);
    assert!(out.status.success(), "{}", stderr(&out));
    let svg = std::fs::read_to_string(dir.join("model.svg")).expect("model.svg written");
    assert!(svg.contains("<svg"), "{svg}");

    let custom = dir.join("plan.svg");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-sheet"),
        &[input.as_os_str(), "-o".as_ref(), custom.as_os_str()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(custom.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn legacy_src_dst_flags_still_work() {
    let Some(source) = fixture() else { return };
    let dir = scratch("legacy");
    let glb = dir.join("legacy.glb");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-gltf"),
        &[
            "--src".as_ref(),
            source.as_os_str(),
            "--dst".as_ref(),
            glb.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(glb.exists());

    let svg = dir.join("legacy.svg");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-sheet"),
        &[
            "-s".as_ref(),
            source.as_os_str(),
            "-d".as_ref(),
            svg.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(svg.exists());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn refuses_to_overwrite_the_input() {
    let Some(source) = fixture() else { return };
    let dir = scratch("overwrite");
    let input = dir.join("model.rvt");
    std::fs::copy(&source, &input).unwrap();
    let before = std::fs::read(&input).unwrap();
    let out = run(
        env!("CARGO_BIN_EXE_rvt-gltf"),
        &[input.as_os_str(), "-o".as_ref(), input.as_os_str()],
    );
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("refusing to overwrite"),
        "{}",
        stderr(&out)
    );
    assert_eq!(std::fs::read(&input).unwrap(), before);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A manifest with one patch that gives BasicFileInfo a new payload, framed as
/// `framing`; the tier-1 fixture stores that stream as it stands, not as gzip.
fn manifest(dir: &Path, patch: serde_json::Value) -> PathBuf {
    let path = dir.join("patches.json");
    std::fs::write(&path, serde_json::to_vec(&patch).unwrap()).unwrap();
    path
}

#[test]
fn rvt_write_dry_run_needs_no_manifest_and_checks_what_the_write_would() {
    let Some(source) = fixture() else { return };
    let dir = scratch("write-dry");
    let dst = dir.join("out.rvt");
    let write = env!("CARGO_BIN_EXE_rvt-write");

    // No manifest: the source is read and the exit is 0 (#1885, item 1).
    let out = run(
        write,
        &[
            "--src".as_ref(),
            source.as_os_str(),
            "--dst".as_ref(),
            dst.as_os_str(),
            "--dry-run".as_ref(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));

    // A stream the file does not have, a framing it is not stored in: both
    // refused by the dry run, as the write would, and nothing is written (item 2
    // and the framing check of item 3).
    let missing = manifest(
        &dir,
        serde_json::json!({ "patches": [{
            "stream_name": "No/Such/Stream", "new_decompressed": [1, 2, 3], "framing": "Verbatim"
        }] }),
    );
    let wrong = dir.join("wrong-framing.json");
    std::fs::write(
        &wrong,
        serde_json::to_vec(&serde_json::json!({ "patches": [{
            "stream_name": "BasicFileInfo", "new_decompressed": [1, 2, 3], "framing": "RawGzipFromZero"
        }] }))
        .unwrap(),
    )
    .unwrap();
    for (manifest, words) in [(&missing, "No/Such/Stream"), (&wrong, "not gzip-framed")] {
        for dry in [true, false] {
            let mut args: Vec<&std::ffi::OsStr> = vec![
                "--src".as_ref(),
                source.as_os_str(),
                "--dst".as_ref(),
                dst.as_os_str(),
                "--patches".as_ref(),
                manifest.as_os_str(),
            ];
            if dry {
                args.push("--dry-run".as_ref());
            }
            let out = run(write, &args);
            assert_eq!(out.status.code(), Some(1), "dry={dry}: {}", stderr(&out));
            assert!(
                stderr(&out).contains(words),
                "dry={dry}, {words}: {}",
                stderr(&out)
            );
            assert!(!dst.exists(), "dry={dry}: a refused patch left a file");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rvt_write_refuses_a_base64_value_that_empties_a_stream() {
    let Some(source) = fixture() else { return };
    let dir = scratch("write-empty-base64");
    let dst = dir.join("out.rvt");
    for value in ["=", ""] {
        let manifest = manifest(
            &dir,
            serde_json::json!({ "patches": [{
                "stream_name": "BasicFileInfo", "new_decompressed_base64": value, "framing": "Verbatim"
            }] }),
        );
        let out = run(
            env!("CARGO_BIN_EXE_rvt-write"),
            &[
                "--src".as_ref(),
                source.as_os_str(),
                "--dst".as_ref(),
                dst.as_os_str(),
                "--patches".as_ref(),
                manifest.as_os_str(),
            ],
        );
        assert_eq!(out.status.code(), Some(1), "{value:?}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("decodes to no bytes"),
            "{value:?}: {}",
            stderr(&out)
        );
        assert!(!dst.exists(), "{value:?}: a refused patch left a file");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rvt_write_with_no_patch_copies_the_file_byte_for_byte_and_the_file_onto_itself_is_left() {
    let Some(source) = fixture() else { return };
    let dir = scratch("write-copy");
    let write = env!("CARGO_BIN_EXE_rvt-write");
    let empty = manifest(&dir, serde_json::json!({ "patches": [] }));
    let before = std::fs::read(&source).unwrap();

    // An empty manifest is a plain copy, not "0 stream(s) failed verification"
    // (item 5).
    let dst = dir.join("copy.rvt");
    let out = run(
        write,
        &[
            "--src".as_ref(),
            source.as_os_str(),
            "--dst".as_ref(),
            dst.as_os_str(),
            "--patches".as_ref(),
            empty.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(std::fs::read(&dst).unwrap(), before);

    // The copy onto itself, spelled two ways, leaves the bytes as they were and
    // no temporary beside it (items 6 and 7).
    for spelled in [dst.clone(), dir.join(".").join("copy.rvt")] {
        let out = run(
            write,
            &[
                "--src".as_ref(),
                dst.as_os_str(),
                "--dst".as_ref(),
                spelled.as_os_str(),
                "--patches".as_ref(),
                empty.as_os_str(),
            ],
        );
        assert!(out.status.success(), "{}", stderr(&out));
        assert_eq!(std::fs::read(&dst).unwrap(), before);
    }
    let leftovers: Vec<String> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(leftovers.is_empty(), "temporaries left: {leftovers:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The same file under other names is still the input (#1885): the paths as
/// typed differ, so comparing them let `./model.rvt` and a link through, and
/// the export replaced the file it had been read from.
#[test]
fn refuses_to_overwrite_the_input_however_it_is_spelled() {
    let Some(source) = fixture() else { return };
    let dir = scratch("overwrite-spelled");
    let input = dir.join("model.rvt");
    std::fs::copy(&source, &input).unwrap();
    let before = std::fs::read(&input).unwrap();
    #[allow(unused_mut)]
    let mut spellings = vec![dir.join(".").join("model.rvt")];
    #[cfg(unix)]
    {
        let link = dir.join("link.rvt");
        std::os::unix::fs::symlink(&input, &link).unwrap();
        spellings.push(link);
        let hard = dir.join("hard.rvt");
        std::fs::hard_link(&input, &hard).unwrap();
        spellings.push(hard);
    }
    for bin in [
        env!("CARGO_BIN_EXE_rvt-gltf"),
        env!("CARGO_BIN_EXE_rvt-sheet"),
        env!("CARGO_BIN_EXE_rvt-schedule"),
        env!("CARGO_BIN_EXE_rvt-ifc"),
    ] {
        for output in &spellings {
            let out = run(bin, &[input.as_os_str(), "-o".as_ref(), output.as_os_str()]);
            assert_eq!(
                out.status.code(),
                Some(1),
                "{bin} -o {}: {}",
                output.display(),
                stderr(&out)
            );
            assert!(
                stderr(&out).contains("refusing to overwrite"),
                "{bin} -o {}: {}",
                output.display(),
                stderr(&out)
            );
            assert_eq!(
                std::fs::read(&input).unwrap(),
                before,
                "{bin} -o {} changed the input",
                output.display()
            );
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_input_is_named_with_one_error_prefix() {
    for bin in [
        env!("CARGO_BIN_EXE_rvt-gltf"),
        env!("CARGO_BIN_EXE_rvt-sheet"),
        env!("CARGO_BIN_EXE_rvt-ifc"),
        env!("CARGO_BIN_EXE_rvt-elements"),
        env!("CARGO_BIN_EXE_rvt-doc"),
    ] {
        let out = run(bin, &["no/such/model.rvt".as_ref()]);
        let err = stderr(&out);
        assert_eq!(out.status.code(), Some(1), "{bin}: {err}");
        assert!(err.starts_with("error: "), "{bin}: {err}");
        assert_eq!(
            err.matches("no/such/model.rvt").count(),
            1,
            "{bin} should name the path exactly once: {err}"
        );
        assert!(!err.contains("panicked"), "{bin}: {err}");
    }
}

#[test]
fn json_is_shorthand_for_format_json() {
    let Some(source) = fixture() else { return };
    let long = run(
        env!("CARGO_BIN_EXE_rvt-schema"),
        &[source.as_os_str(), "--format".as_ref(), "json".as_ref()],
    );
    let short = run(
        env!("CARGO_BIN_EXE_rvt-schema"),
        &[source.as_os_str(), "--json".as_ref()],
    );
    assert!(short.status.success(), "{}", stderr(&short));
    assert_eq!(long.stdout, short.stdout);
    let parsed: serde_json::Value = serde_json::from_slice(&short.stdout).expect("JSON");
    assert!(parsed["classes"].is_array());

    let caps = run(env!("CARGO_BIN_EXE_rvt-capabilities"), &["--json".as_ref()]);
    assert!(caps.status.success(), "{}", stderr(&caps));
    serde_json::from_slice::<serde_json::Value>(&caps.stdout).expect("capabilities JSON");

    let clash = run(
        env!("CARGO_BIN_EXE_rvt-schema"),
        &[
            source.as_os_str(),
            "--json".as_ref(),
            "-f".as_ref(),
            "text".as_ref(),
        ],
    );
    assert_eq!(
        clash.status.code(),
        Some(2),
        "--json with --format is a usage error"
    );
}

#[test]
fn every_cli_help_shows_examples() {
    for bin in [
        env!("CARGO_BIN_EXE_rvt-analyze"),
        env!("CARGO_BIN_EXE_rvt-info"),
        env!("CARGO_BIN_EXE_rvt-inspect"),
        env!("CARGO_BIN_EXE_rvt-schema"),
        env!("CARGO_BIN_EXE_rvt-history"),
        env!("CARGO_BIN_EXE_rvt-diff"),
        env!("CARGO_BIN_EXE_rvt-corpus"),
        env!("CARGO_BIN_EXE_rvt-dump"),
        env!("CARGO_BIN_EXE_rvt-doc"),
        env!("CARGO_BIN_EXE_rvt-ifc"),
        env!("CARGO_BIN_EXE_rvt-ifc-compare"),
        env!("CARGO_BIN_EXE_gen-fixture"),
        env!("CARGO_BIN_EXE_rvt-write"),
        env!("CARGO_BIN_EXE_rvt-gltf"),
        env!("CARGO_BIN_EXE_rvt-sheet"),
        env!("CARGO_BIN_EXE_rvt-elem-table"),
        env!("CARGO_BIN_EXE_rvt-elements"),
        env!("CARGO_BIN_EXE_rvt-capabilities"),
        env!("CARGO_BIN_EXE_rvt-schedule"),
    ] {
        let out = run(bin, &["--help".as_ref()]);
        assert!(out.status.success(), "{bin}");
        let help = String::from_utf8_lossy(&out.stdout);
        assert!(
            help.contains("Examples:"),
            "{bin} --help has no examples:\n{help}"
        );
    }
}

/// `rvt-schema family.rfa --json | head` closes stdout long before the
/// ~1 MB of schema JSON is written; the CLI must exit quietly instead of
/// panicking. Needs a real family file for output larger than a pipe
/// buffer.
#[test]
fn a_closed_stdout_pipe_is_a_quiet_exit() {
    let sample = common::sample_for_year(2024);
    if !sample.exists() {
        eprintln!("skipping: {} not present", sample.display());
        return;
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_rvt-schema"))
        .args([sample.as_os_str(), "--json".as_ref()])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn rvt-schema");
    let mut first = [0u8; 16];
    child
        .stdout
        .as_mut()
        .expect("stdout")
        .read_exact(&mut first)
        .expect("first bytes");
    drop(child.stdout.take());
    let out = child.wait_with_output().expect("wait");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(!err.contains("panicked"), "{err}");
    assert!(!err.contains("Broken pipe"), "{err}");
    #[cfg(unix)]
    assert_eq!(out.status.code(), Some(141), "{err}");
}
