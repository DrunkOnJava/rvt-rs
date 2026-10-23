//! The output formats are written deterministically (`RELEASE.md`, "What 1.0
//! keeps stable"): each shipped writer, run twice on the same file, writes the
//! same bytes. `rvt-ifc` runs with `SOURCE_DATE_EPOCH` set, because the time
//! in the IFC header and owner history is the one thing that changes between
//! two exports without it, and the export must carry that time. Runs on the
//! tier-1 synthetic project and, where `RVT_PROJECT_CORPUS_DIR` holds it, on
//! `Revit_IFC5_Einhoven.rvt`.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const EPOCH: &str = "1700000000";

fn inputs() -> Vec<PathBuf> {
    let tier1 = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus/tier1/architectural-2024/architectural-2024.rvt");
    // A configured project corpus must hold Einhoven; an unset one is a skip.
    let einhoven = std::env::var_os("RVT_PROJECT_CORPUS_DIR")
        .map(|dir| PathBuf::from(dir).join("Revit_IFC5_Einhoven.rvt"));
    if let Some(path) = &einhoven {
        assert!(
            path.exists(),
            "RVT_PROJECT_CORPUS_DIR is set but {} is absent",
            path.display()
        );
    }
    [Some(tier1), einhoven]
        .into_iter()
        .flatten()
        .filter(|path| path.exists())
        .collect()
}

fn run(bin: &str, args: &[&OsStr]) -> Output {
    let out = Command::new(bin)
        .args(args)
        .env("SOURCE_DATE_EPOCH", EPOCH)
        .output()
        .expect("spawn CLI");
    assert!(
        out.status.success(),
        "{bin} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}

/// Runs `bin input [extra] -o <dir>/<run>.<ext>` and returns what it wrote.
fn write(bin: &str, input: &Path, extra: &[&str], dir: &Path, run_tag: &str, ext: &str) -> Vec<u8> {
    let out = dir.join(format!("{run_tag}.{ext}"));
    let mut args: Vec<&OsStr> = vec![input.as_os_str()];
    args.extend(extra.iter().map(OsStr::new));
    args.extend([OsStr::new("-o"), out.as_os_str()]);
    run(bin, &args);
    std::fs::read(&out).unwrap_or_else(|e| panic!("{bin} wrote no {}: {e}", out.display()))
}

#[test]
fn every_writer_writes_the_same_bytes_twice() {
    let writers: [(&str, &[&str], &str); 5] = [
        (env!("CARGO_BIN_EXE_rvt-ifc"), &[], "ifc"),
        (env!("CARGO_BIN_EXE_rvt-gltf"), &[], "glb"),
        (env!("CARGO_BIN_EXE_rvt-sheet"), &[], "svg"),
        (env!("CARGO_BIN_EXE_rvt-schedule"), &[], "csv"),
        (
            env!("CARGO_BIN_EXE_rvt-schedule"),
            &["--schedule", "rooms"],
            "csv",
        ),
    ];
    let inputs = inputs();
    assert!(!inputs.is_empty(), "the tier-1 fixture is committed");
    for input in inputs {
        let dir = std::env::temp_dir().join(format!(
            "rvt-reproducible-{}-{}",
            input.file_stem().unwrap().to_string_lossy(),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // The two runs straddle a clock second, so a writer that takes the
        // wall clock's time cannot write the same bytes twice.
        let first: Vec<Vec<u8>> = writers
            .iter()
            .enumerate()
            .map(|(index, (bin, extra, ext))| {
                write(bin, &input, extra, &dir, &format!("{index}a"), ext)
            })
            .collect();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        for (index, ((bin, extra, ext), first)) in writers.iter().zip(&first).enumerate() {
            let second = write(bin, &input, extra, &dir, &format!("{index}b"), ext);
            assert!(!first.is_empty(), "{bin} {extra:?} wrote nothing");
            let differs = first.iter().zip(&second).position(|(a, b)| a != b);
            assert!(
                *first == second,
                "{bin} {extra:?} on {}: {} and {} bytes, first difference at {:?}",
                input.display(),
                first.len(),
                second.len(),
                differs
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn source_date_epoch_is_the_time_the_ifc_records() {
    let input = &inputs()[0];
    let dir = std::env::temp_dir().join(format!("rvt-epoch-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let ifc = write(
        env!("CARGO_BIN_EXE_rvt-ifc"),
        input,
        &[],
        &dir,
        "epoch",
        "ifc",
    );
    let step = String::from_utf8(ifc).expect("STEP is text");
    let file_name = step
        .lines()
        .find(|line| line.starts_with("FILE_NAME("))
        .expect("a FILE_NAME header");
    assert!(file_name.contains("'2023-11-14T22:13:20'"), "{file_name}");
    let history = step
        .lines()
        .find(|line| line.contains("IFCOWNERHISTORY("))
        .expect("an owner history");
    assert!(history.ends_with(&format!(",{EPOCH});")), "{history}");

    let bad = Command::new(env!("CARGO_BIN_EXE_rvt-ifc"))
        .arg(input)
        .arg("-o")
        .arg(dir.join("bad.ifc"))
        .env("SOURCE_DATE_EPOCH", "yesterday")
        .output()
        .expect("spawn rvt-ifc");
    assert!(
        !bad.status.success(),
        "a malformed SOURCE_DATE_EPOCH is refused"
    );
    assert!(
        String::from_utf8_lossy(&bad.stderr).contains("SOURCE_DATE_EPOCH"),
        "{}",
        String::from_utf8_lossy(&bad.stderr)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
