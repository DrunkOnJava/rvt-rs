//! Full-pipeline smoke test against the project-file corpus.
//!
//! For every `.rvt` file found under `RVT_PROJECT_CORPUS_DIR` (defaults
//! to `/private/tmp/rvt-corpus-probe/magnetar/Revit`), exercises the
//! complete public reader API:
//!
//!   - `RevitFile::open`
//!   - `summarize_strict`
//!   - `parse_schema` on `Formats/Latest`
//!   - `elem_table::parse_header` + `parse_records`
//!   - `walker::read_adocument_lossy`
//!   - `ifc::RvtDocExporter::export` (IFC4 STEP scaffold emission)
//!
//! Asserts no panics, no errors on the first four, and a non-empty
//! output on the IFC emission. Skips entirely when the corpus
//! directory is absent so the test passes green on machines without
//! the files.
//!
//! Complementary to `elem_table_corpus.rs` (specific invariants) —
//! this is the "everything still works" sweep that L5B-59 calls for.

use rvt::{
    RevitFile, compression, elem_table, formats,
    ifc::{Exporter, RvtDocExporter, write_step},
    streams, walker,
};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

fn discover_rvts(dir: &PathBuf) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) == Some("rvt") {
            out.push(p);
        }
    }
    out.sort();
    out
}

#[test]
fn full_pipeline_works_on_every_project_rvt_in_corpus() {
    let dir = project_dir();
    let files = discover_rvts(&dir);
    if files.is_empty() {
        eprintln!(
            "skipping: no .rvt files found under {} — set RVT_PROJECT_CORPUS_DIR",
            dir.display()
        );
        return;
    }

    let mut passed = 0usize;
    let mut failed: Vec<(PathBuf, String)> = Vec::new();

    for path in &files {
        match exercise_full_pipeline(path) {
            Ok(()) => passed += 1,
            Err(e) => failed.push((path.clone(), e)),
        }
    }

    eprintln!(
        "full-pipeline smoke · {} files · {} passed · {} failed",
        files.len(),
        passed,
        failed.len()
    );
    for (path, err) in &failed {
        eprintln!("  FAIL {}: {err}", path.display());
    }

    assert!(
        failed.is_empty(),
        "full-pipeline smoke failed on {}/{} corpus file(s); see stderr for details",
        failed.len(),
        files.len()
    );
    assert!(passed > 0, "corpus contained no .rvt files to exercise");
}

fn exercise_full_pipeline(path: &PathBuf) -> Result<(), String> {
    let mut rf = RevitFile::open(path).map_err(|e| format!("open: {e}"))?;

    // Summarize — BasicFileInfo + PartAtom + schema enumerate.
    rf.summarize_strict()
        .map_err(|e| format!("summarize_strict: {e}"))?;

    // Schema parse on Formats/Latest.
    let raw = rf
        .read_stream(streams::FORMATS_LATEST)
        .map_err(|e| format!("read Formats/Latest: {e}"))?;
    let (_, decomp) = compression::inflate_at_auto(&raw)
        .map_err(|e| format!("inflate Formats/Latest: {e}"))?;
    let schema = formats::parse_schema(&decomp).map_err(|e| format!("parse_schema: {e}"))?;
    if schema.classes.is_empty() {
        return Err("parse_schema returned no classes".into());
    }

    // ElemTable header + records.
    let header = elem_table::parse_header(&mut rf)
        .map_err(|e| format!("elem_table::parse_header: {e}"))?;
    let records = elem_table::parse_records(&mut rf)
        .map_err(|e| format!("elem_table::parse_records: {e}"))?;
    if records.is_empty() {
        return Err("parse_records returned empty vec".into());
    }
    if records.len() as u16 > header.record_count {
        return Err(format!(
            "parsed {} records > header record_count {}",
            records.len(),
            header.record_count
        ));
    }

    // ADocument walker — lossy so diagnostics accumulate instead of erroring.
    let doc = walker::read_adocument_lossy(&mut rf)
        .map_err(|e| format!("read_adocument_lossy: {e}"))?;
    if doc.value.fields.is_empty() {
        return Err("read_adocument_lossy returned empty fields".into());
    }

    // IFC4 STEP emission — scaffold-level. Must produce non-empty STEP.
    let model = RvtDocExporter
        .export(&mut rf)
        .map_err(|e| format!("RvtDocExporter::export: {e}"))?;
    let step = write_step(&model);
    if !step.starts_with("ISO-10303-21") {
        return Err("IFC output does not start with ISO-10303-21 header".into());
    }
    if !step.contains("FILE_SCHEMA(('IFC4'))") {
        return Err("IFC output missing FILE_SCHEMA(('IFC4'))".into());
    }

    Ok(())
}
