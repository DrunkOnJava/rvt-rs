//! Tier-two corpus health — optional external project corpus.
//!
//! When `RVT_PROJECT_CORPUS_DIR` points at a directory of real `.rvt`
//! project files, runs a lightweight open/schema/summary sweep and
//! confirms at least one file exercises the pipeline. Skips cleanly
//! when the env var is unset or the directory is empty — Tier two is
//! never required for a green local `cargo test`.
//!
//! CI's `corpus-tier2` job sets `RVT_PROJECT_CORPUS_DIR` after cloning
//! `magnetar-io/revit-test-datasets`. Do not commit those files here
//! (see SECURITY.md / corpus/tier2/README.md).

use rvt::{RevitFile, compression, formats, streams};
use std::path::PathBuf;

fn project_dir() -> Option<PathBuf> {
    let dir = std::env::var_os("RVT_PROJECT_CORPUS_DIR").map(PathBuf::from)?;
    if dir.is_dir() { Some(dir) } else { None }
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

fn exercise(path: &PathBuf) -> Result<(), String> {
    let mut rf = RevitFile::open(path).map_err(|e| format!("open: {e}"))?;
    rf.summarize_strict()
        .map_err(|e| format!("summarize_strict: {e}"))?;
    let raw = rf
        .read_stream(streams::FORMATS_LATEST)
        .map_err(|e| format!("read Formats/Latest: {e}"))?;
    let (_, decomp) =
        compression::inflate_at_auto(&raw).map_err(|e| format!("inflate Formats/Latest: {e}"))?;
    let schema = formats::parse_schema(&decomp).map_err(|e| format!("parse_schema: {e}"))?;
    if schema.classes.is_empty() {
        return Err("parse_schema returned no classes".into());
    }
    Ok(())
}

#[test]
fn tier2_project_corpus_health() {
    let Some(dir) = project_dir() else {
        eprintln!(
            "skipping tier2 corpus health: RVT_PROJECT_CORPUS_DIR unset or missing \
             (see corpus/tier2/README.md)"
        );
        return;
    };
    let files = discover_rvts(&dir);
    if files.is_empty() {
        eprintln!(
            "skipping tier2 corpus health: no .rvt files under {}",
            dir.display()
        );
        return;
    }

    let mut passed = 0usize;
    let mut failed: Vec<(PathBuf, String)> = Vec::new();
    // Cap the always-on sweep so PR CI stays bounded; full coverage is
    // project_corpus_smoke's job when intentionally run.
    let limit = std::env::var("RVT_CORPUS_TIER2_LIMIT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8usize);
    for path in files.iter().take(limit) {
        match exercise(path) {
            Ok(()) => passed += 1,
            Err(e) => failed.push((path.clone(), e)),
        }
    }

    eprintln!(
        "tier2 health · scanned {}/{} · {} passed · {} failed",
        passed + failed.len(),
        files.len(),
        passed,
        failed.len()
    );
    for (path, err) in &failed {
        eprintln!("  FAIL {}: {err}", path.display());
    }
    assert!(
        failed.is_empty(),
        "tier2 health failed on {} file(s)",
        failed.len()
    );
    assert!(passed > 0, "tier2 corpus produced no successful opens");
}
