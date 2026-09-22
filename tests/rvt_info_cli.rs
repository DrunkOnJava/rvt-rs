//! `rvt-info` inventory mode: several files or a folder produce one row per
//! file, unreadable files become error rows, and the exit code says whether
//! every file was read. Uses the tier-1 synthetic fixtures only.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tier1(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("tier1")
        .join(name)
        .join(format!("{name}.rvt"))
}

fn rvt_info(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rvt-info"))
        .args(args)
        .output()
        .expect("run rvt-info")
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).expect("utf-8 stdout")
}

/// A scratch folder holding: two tier-1 fixtures (one as a Revit backup
/// copy, one in a subfolder), a mis-named non-Revit file, and a text file
/// the folder walk must ignore.
fn scratch_folder(tag: &str) -> Option<PathBuf> {
    let arch = tier1("architectural-2024");
    let mep = tier1("mep-2024");
    if !arch.exists() || !mep.exists() {
        eprintln!("skipping: tier-1 fixtures missing");
        return None;
    }
    let dir = std::env::temp_dir().join(format!("rvt-info-cli-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sub")).unwrap();
    std::fs::copy(&arch, dir.join("Tower.rvt")).unwrap();
    std::fs::copy(&mep, dir.join("sub").join("Tower.0003.rvt")).unwrap();
    std::fs::write(dir.join("broken.rvt"), b"not a revit file").unwrap();
    std::fs::write(dir.join("notes.txt"), b"ignored").unwrap();
    Some(dir)
}

#[test]
fn folder_inventory_lists_every_revit_file_and_flags_failures() {
    let Some(dir) = scratch_folder("table") else {
        return;
    };
    let out = rvt_info(&[dir.as_os_str()]);
    let text = stdout(&out);
    assert_eq!(
        out.status.code(),
        Some(1),
        "one unreadable file → exit 1\n{text}"
    );
    assert!(text.starts_with("VERSION"), "{text}");
    assert!(text.contains("Tower.rvt"), "{text}");
    assert!(text.contains("Tower.0003.rvt"), "{text}");
    assert!(text.contains("broken.rvt"), "{text}");
    assert!(text.contains("Not a Revit file"), "{text}");
    assert!(!text.contains("notes.txt"), "{text}");
    assert!(text.contains("3 files"), "{text}");
    assert!(text.contains("unreadable: 1"), "{text}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn no_recurse_stays_at_the_top_level() {
    let Some(dir) = scratch_folder("norecurse") else {
        return;
    };
    let out = rvt_info(&[
        dir.as_os_str(),
        "--no-recurse".as_ref(),
        "--format".as_ref(),
        "jsonl".as_ref(),
    ]);
    let rows: Vec<serde_json::Value> = stdout(&out)
        .lines()
        .map(|l| serde_json::from_str(l).expect("one JSON object per line"))
        .collect();
    assert_eq!(rows.len(), 2, "{rows:?}");
    assert!(
        rows.iter()
            .all(|r| !r["path"].as_str().unwrap().contains("sub"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn csv_inventory_has_a_stable_header_and_marks_backups() {
    let Some(dir) = scratch_folder("csv") else {
        return;
    };
    let out = rvt_info(&[dir.as_os_str(), "-f".as_ref(), "csv".as_ref()]);
    let text = stdout(&out);
    let mut lines = text.lines();
    assert_eq!(
        lines.next(),
        Some(
            "path,file_type,backup,size_bytes,error,revit_version,build,title,last_saved,\
             worksharing,workshared,username,central_model_path,last_save_path,document_guid,\
             document_increments,locale,single_user_cloud_model,properties"
        )
    );
    let rows: Vec<&str> = lines.collect();
    assert_eq!(rows.len(), 3, "{text}");
    let backup = rows
        .iter()
        .find(|r| r.contains("Tower.0003.rvt"))
        .expect("backup row");
    assert!(backup.contains(",project,true,"), "{backup}");
    let broken = rows
        .iter()
        .find(|r| r.contains("broken.rvt"))
        .expect("error row");
    assert!(broken.contains("Not a Revit file"), "{broken}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn several_files_give_a_json_array_in_argument_order() {
    let (arch, mep) = (tier1("architectural-2024"), tier1("mep-2024"));
    if !arch.exists() || !mep.exists() {
        return;
    }
    let out = rvt_info(&[mep.as_os_str(), arch.as_os_str(), "--json".as_ref()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows: serde_json::Value = serde_json::from_str(&stdout(&out)).unwrap();
    let paths: Vec<&str> = rows
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["path"].as_str().unwrap())
        .collect();
    assert_eq!(paths.len(), 2);
    assert!(paths[0].ends_with("mep-2024.rvt"), "{paths:?}");
    assert!(paths[1].ends_with("architectural-2024.rvt"), "{paths:?}");
    assert!(rows[0]["revit_version"].as_u64().is_some());
}

#[test]
fn a_missing_file_is_named_in_the_error() {
    let out = rvt_info(&["no/such/model.rvt".as_ref()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no/such/model.rvt"), "{err}");
}

#[test]
fn preview_extraction_refuses_a_folder() {
    let Some(dir) = scratch_folder("preview") else {
        return;
    };
    let out = rvt_info(&[
        dir.as_os_str(),
        "--extract-preview".as_ref(),
        dir.join("p.png").as_os_str(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("exactly one file"));
    let _ = std::fs::remove_dir_all(&dir);
}
