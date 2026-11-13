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

/// A fixture made by `gen-fixture`: the streams `rvt-dump` reads, and a 1x1
/// preview PNG that ends with its IEND chunk.
fn generated_fixture(dir: &Path) -> PathBuf {
    let path = dir.join("generated.rvt");
    let out = run(
        env!("CARGO_BIN_EXE_gen-fixture"),
        &["demo".as_ref(), "--output".as_ref(), path.as_os_str()],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    path
}

/// A stream over `--max-stream-bytes` does not stop the dump (#1885): it is
/// listed as not read, the streams after it are written, the reader's report of
/// what a limit left unread goes to stderr and the exit status is 3 unless
/// `--allow-partial` says the partial dump is wanted.
#[test]
fn rvt_dump_lists_a_stream_over_the_limit_and_dumps_the_rest() {
    let dir = scratch("dump-limit");
    let model = generated_fixture(&dir);
    let whole = dir.join("whole");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-dump"),
        &[
            model.as_os_str(),
            "-o".as_ref(),
            whole.as_os_str(),
            "--raw".as_ref(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stderr(&out).is_empty(), "{}", stderr(&out));
    assert!(whole.join("BasicFileInfo.raw").exists());

    // BasicFileInfo is 236 bytes or more and the preview, 83.
    let limited = dir.join("limited");
    let args: [&std::ffi::OsStr; 6] = [
        model.as_os_str(),
        "-o".as_ref(),
        limited.as_os_str(),
        "--raw".as_ref(),
        "--max-stream-bytes".as_ref(),
        "100".as_ref(),
    ];
    let out = run(env!("CARGO_BIN_EXE_rvt-dump"), &args);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    let basic = stdout
        .lines()
        .find(|line| line.contains("BasicFileInfo"))
        .unwrap_or_else(|| panic!("no line for BasicFileInfo in {stdout}"));
    assert!(basic.contains("not read: over the stream limit"), "{basic}");
    let err = stderr(&out);
    assert!(err.contains("were not read whole"), "{err}");
    assert!(err.contains("BasicFileInfo"), "{err}");
    assert!(err.contains("--allow-partial"), "{err}");
    assert!(
        !limited.join("BasicFileInfo.raw").exists(),
        "a stream that was not read is not written"
    );
    assert!(
        limited.join("RevitPreview4.0.raw").exists(),
        "the stream after it, which fits the limit, was dumped"
    );

    let partial = dir.join("partial");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-dump"),
        &[
            model.as_os_str(),
            "-o".as_ref(),
            partial.as_os_str(),
            "--max-stream-bytes".as_ref(),
            "100".as_ref(),
            "--allow-partial".as_ref(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("were not read whole"),
        "{}",
        stderr(&out)
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// Replaces the preview stream of the compound file at `path`.
fn set_preview(path: &Path, bytes: &[u8]) {
    use std::io::{Seek, SeekFrom, Write};
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut compound = cfb::CompoundFile::open(file).unwrap();
    {
        let mut stream = compound.open_stream("/RevitPreview4.0").unwrap();
        stream.set_len(0).unwrap();
        stream.seek(SeekFrom::Start(0)).unwrap();
        stream.write_all(bytes).unwrap();
    }
    compound.flush().unwrap();
}

/// `rvt-info --extract-preview` writes a PNG that ends with its IEND chunk, and
/// refuses a preview whose stream is cut short (#1885): the library returns
/// what is there, and a file named `.png` that no viewer opens is not what was
/// asked for.
#[test]
fn extract_preview_writes_a_whole_png_and_refuses_one_cut_short() {
    const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
    let dir = scratch("preview");
    let whole = generated_fixture(&dir);
    let thumb = dir.join("thumb.png");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-info"),
        &[
            whole.as_os_str(),
            "--extract-preview".as_ref(),
            thumb.as_os_str(),
        ],
    );
    assert!(out.status.success(), "{}", stderr(&out));
    let png = std::fs::read(&thumb).unwrap();
    assert_eq!(png[..8], PNG_MAGIC);
    assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");

    // The same file with the PNG's stream ending inside its IHDR chunk.
    let cut = dir.join("cut.rvt");
    std::fs::copy(&whole, &cut).unwrap();
    let mut stream = vec![0u8; 16];
    stream.extend_from_slice(&PNG_MAGIC);
    stream.extend_from_slice(&[0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 1]);
    set_preview(&cut, &stream);
    let cut_png = dir.join("cut.png");
    let out = run(
        env!("CARGO_BIN_EXE_rvt-info"),
        &[
            cut.as_os_str(),
            "--extract-preview".as_ref(),
            cut_png.as_os_str(),
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("cut short"), "{}", stderr(&out));
    assert!(!cut_png.exists(), "nothing was written");
    let _ = std::fs::remove_dir_all(&dir);
}

/// `gen-fixture`'s name is a file name under tests/fixtures, so a name with a
/// path in it is refused rather than written somewhere else (#1885).
#[test]
fn gen_fixture_name_with_a_path_in_it_writes_nothing() {
    let dir = scratch("gen-name");
    let project = dir.join("project");
    std::fs::create_dir_all(&project).unwrap();
    for name in ["../escape", "a/b", "a\\b"] {
        let out = Command::new(env!("CARGO_BIN_EXE_gen-fixture"))
            .arg(name)
            .env("CARGO_MANIFEST_DIR", &project)
            .output()
            .expect("spawn gen-fixture");
        assert_eq!(out.status.code(), Some(1), "{name}: {}", stderr(&out));
        assert!(
            stderr(&out).contains("path separator"),
            "{name}: {}",
            stderr(&out)
        );
    }
    assert!(
        std::fs::read_dir(&project).unwrap().next().is_none(),
        "no folder was made under the project"
    );
    assert!(!dir.join("escape.rvt").exists());

    let out = Command::new(env!("CARGO_BIN_EXE_gen-fixture"))
        .arg("plain")
        .env("CARGO_MANIFEST_DIR", &project)
        .output()
        .expect("spawn gen-fixture");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        project
            .join("tests")
            .join("fixtures")
            .join("synthetic-plain.rvt")
            .exists()
    );
    let _ = std::fs::remove_dir_all(&dir);
}
