//! RE-187: a door's `OverallWidth` and `OverallHeight` are its type's Width
//! and Height, where no opening gives them (#759). RE-215: before either,
//! a door's or window's are the extents of the outline the symbol that
//! draws it cuts in its host (#844).
//!
//! Revit's own IFC exports of `RE1-Architecture.rvt` (Revit 2025, MIT) and
//! `2024_Core_Interior.rvt` (Revit 2024, MIT) write every door's type Width
//! and Height as its `OverallWidth` / `OverallHeight`; the values below are
//! Revit's, in metres. Core Interior's six windows are of a type whose size
//! is 1010 mm square and are drawn by a symbol of their own, whose cut is
//! Revit's 1500 x 2700 mm; before RE-215 they had `$`. RE1's curtain-wall
//! door has no cut and its type no size, and keeps `$`. Corpus-gated
//! through `RVT_PROJECT_CORPUS_DIR`.

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// 0.001 ft, in metres, as `tools/re/door_window_sizes_vs_ifc.py` compares.
const TOLERANCE_M: f64 = 0.001 * 0.3048;

/// Tag -> (entity, OverallHeight, OverallWidth), metres; `None` for `$`.
type Sizes = BTreeMap<String, (String, Option<f64>, Option<f64>)>;

/// The [`Sizes`] of every door and window the export of `name` writes.
/// Skips when the model is absent.
fn overall_sizes(name: &str) -> Option<Sizes> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from) else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR is not set");
        return None;
    };
    let path = dir.join(name);
    if !path.exists() {
        eprintln!("skipping: no {name}");
        return None;
    }
    let mut rf = RevitFile::open(&path).expect("open");
    let result = RvtDocExporter
        .export_with_diagnostics(&mut rf)
        .expect("export");
    let step = write_step(&result.model);
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some((_, body)) = line.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = ["IFCDOOR(", "IFCWINDOW("]
            .iter()
            .find_map(|prefix| Some((prefix.trim_end_matches('('), body.strip_prefix(prefix)?)))
        else {
            continue;
        };
        // Attributes 8 to 10: Tag, OverallHeight, OverallWidth. Names and
        // types are quoted and may hold commas, so count from the end.
        let args: Vec<&str> = args.trim_end_matches(");").rsplitn(6, ',').collect();
        let [_, _, _, width, height, rest] = args[..] else {
            continue;
        };
        let tag = rest
            .rsplit(',')
            .next()
            .unwrap_or_default()
            .trim_matches('\'');
        let length = |text: &str| text.parse::<f64>().ok();
        out.insert(
            tag.to_string(),
            (entity.to_string(), length(height), length(width)),
        );
    }
    Some(out)
}

#[test]
fn re1_doors_carry_their_types_size() {
    let Some(sizes) = overall_sizes("RE1-Architecture.rvt") else {
        return;
    };
    // (Tag, OverallHeight, OverallWidth) from Revit's RE1-Architecture.ifc.
    let revit = [
        ("416336", 2.4, 2.15),
        ("416703", 2.4, 1.5),
        ("416909", 2.4, 0.75),
        ("417097", 2.4, 0.75),
    ];
    for (tag, height, width) in revit {
        let (_, h, w) = sizes
            .get(tag)
            .unwrap_or_else(|| panic!("door {tag} not written"));
        let (h, w) = (h.expect("OverallHeight"), w.expect("OverallWidth"));
        assert!(
            (h - height).abs() <= TOLERANCE_M && (w - width).abs() <= TOLERANCE_M,
            "{tag}: {h} x {w}"
        );
    }
    // A curtain-wall door: no cut, and its type holds no size. It fails closed.
    assert_eq!(sizes.get("445975").map(|s| (s.1, s.2)), Some((None, None)));
}

#[test]
fn core_interior_doors_and_windows_carry_revits_size() {
    let Some(sizes) = overall_sizes("2024_Core_Interior.rvt") else {
        return;
    };
    let mut doors: BTreeMap<String, usize> = BTreeMap::new();
    let mut windows_sized = 0;
    for (entity, height, width) in sizes.values() {
        match entity.as_str() {
            "IFCDOOR" => {
                let size = match (height, width) {
                    (Some(h), Some(w))
                        if (h - 2.4384).abs() <= TOLERANCE_M
                            && (w - 0.9144).abs() <= TOLERANCE_M =>
                    {
                        "8 x 3 ft"
                    }
                    (Some(h), Some(w))
                        if (h - 2.134).abs() <= TOLERANCE_M && (w - 0.915).abs() <= TOLERANCE_M =>
                    {
                        "2134 x 915 mm"
                    }
                    _ => "other",
                };
                *doors.entry(size.to_string()).or_default() += 1;
            }
            _ => {
                windows_sized += usize::from(matches!(
                    (height, width),
                    (Some(h), Some(w)) if (h - 2.7).abs() <= TOLERANCE_M && (w - 1.5).abs() <= TOLERANCE_M
                ))
            }
        }
    }
    // Revit's 2024_Core_Interior_slim.ifc: 120 doors 8 x 3 ft, 12 doors 2134 x 915 mm.
    assert_eq!(
        doors,
        BTreeMap::from([
            ("8 x 3 ft".to_string(), 120),
            ("2134 x 915 mm".to_string(), 12)
        ])
    );
    // Revit's: all six windows 2700 x 1500 mm.
    assert_eq!(windows_sized, 6, "windows sized as Revit's export");
}
