//! `rvt-schedule`: CSV element and room schedules. The tier-1 synthetic
//! fixture proves the file shape everywhere; the 2024 magnetar project
//! (skipped unless `RVT_PROJECT_CORPUS_DIR` holds it) pins the counts the
//! element-record work measured against Revit's own export.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn tier1() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("corpus")
        .join("tier1")
        .join("architectural-2024")
        .join("architectural-2024.rvt");
    path.exists().then_some(path)
}

fn core_interior() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR")?;
    let path = PathBuf::from(dir).join("2024_Core_Interior.rvt");
    path.exists().then_some(path)
}

fn schedule(args: &[&std::ffi::OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_rvt-schedule"))
        .args(args)
        .output()
        .expect("run rvt-schedule")
}

fn csv_rows(csv: &str) -> Vec<Vec<String>> {
    // The schedules never embed CR/LF inside a cell for these inputs, so a
    // line split is enough; quoted commas are handled.
    csv.trim_start_matches('\u{FEFF}')
        .split("\r\n")
        .filter(|l| !l.is_empty())
        .map(|line| {
            let mut cells = Vec::new();
            let mut cell = String::new();
            let mut quoted = false;
            let mut chars = line.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '"' if quoted && chars.peek() == Some(&'"') => {
                        cell.push('"');
                        chars.next();
                    }
                    '"' => quoted = !quoted,
                    ',' if !quoted => cells.push(std::mem::take(&mut cell)),
                    other => cell.push(other),
                }
            }
            cells.push(cell);
            cells
        })
        .collect()
}

#[test]
fn writes_next_to_the_input_and_to_stdout() {
    let Some(fixture) = tier1() else { return };
    let dir = std::env::temp_dir().join(format!("rvt-schedule-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("model.rvt");
    std::fs::copy(&fixture, &input).unwrap();

    let out = schedule(&[input.as_os_str()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let csv = std::fs::read_to_string(dir.join("model.elements.csv")).expect("elements csv");
    let rows = csv_rows(&csv);
    assert_eq!(rows[0][0], "revit_element_id");
    assert_eq!(rows[0].len(), 17);
    assert!(rows.iter().all(|r| r.len() == 17), "ragged rows");

    let out = schedule(&[
        input.as_os_str(),
        "--schedule".as_ref(),
        "rooms".as_ref(),
        "--metric".as_ref(),
        "--excel".as_ref(),
        "-o".as_ref(),
        "-".as_ref(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let csv = String::from_utf8(out.stdout).unwrap();
    assert!(csv.starts_with('\u{FEFF}'), "--excel writes a BOM");
    assert_eq!(
        csv_rows(&csv)[0],
        [
            "revit_element_id",
            "number",
            "name",
            "level",
            "level_elevation_m",
            "body_source"
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_missing_input_is_one_named_error() {
    let out = schedule(&["no/such/model.rvt".as_ref()]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with("error: "), "{err}");
    assert_eq!(err.matches("no/such/model.rvt").count(), 1, "{err}");
}

/// Counts measured against Revit's own full-project export (RE-21 walls,
/// doors, windows, columns; RE-22 slabs; RE-29 rooms): 970 building
/// elements of which 116 are rooms, and every door and window hosted.
#[test]
fn core_interior_schedules_match_the_measured_counts() {
    let Some(model) = core_interior() else {
        eprintln!("skipping: 2024_Core_Interior.rvt not under RVT_PROJECT_CORPUS_DIR");
        return;
    };
    let dir = std::env::temp_dir().join(format!("rvt-schedule-core-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let elements_path = dir.join("core.elements.csv");
    let out = schedule(&[model.as_os_str(), "-o".as_ref(), elements_path.as_os_str()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rows = csv_rows(&std::fs::read_to_string(&elements_path).unwrap());
    let body = &rows[1..];
    let count = |ifc: &str| body.iter().filter(|r| r[1] == ifc).count();
    assert_eq!(body.len(), 970);
    assert_eq!(count("IFCWALL"), 360);
    assert_eq!(count("IFCDOOR"), 132);
    assert_eq!(count("IFCWINDOW"), 6);
    assert_eq!(count("IFCCOLUMN"), 256);
    assert_eq!(count("IFCSPACE"), 116);
    for opening in body
        .iter()
        .filter(|r| r[1] == "IFCDOOR" || r[1] == "IFCWINDOW")
    {
        assert!(!opening[14].is_empty(), "unhosted opening {opening:?}");
    }

    let rooms_path = dir.join("core.rooms.csv");
    let out = schedule(&[
        model.as_os_str(),
        "--schedule".as_ref(),
        "rooms".as_ref(),
        "-o".as_ref(),
        rooms_path.as_os_str(),
    ]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rooms = csv_rows(&std::fs::read_to_string(&rooms_path).unwrap());
    assert_eq!(rooms.len() - 1, 116);
    assert!(
        rooms[1..]
            .iter()
            .all(|r| !r[1].is_empty() && !r[2].is_empty())
    );
    let _ = std::fs::remove_dir_all(&dir);
}
