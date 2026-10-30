//! Q01-02 — batch-validate every .rvt/.rfa file under a corpus root.
//!
//! Walks recursively, exercises the same 5-stage pipeline as
//! `tests/project_corpus_smoke.rs`:
//!
//!   1. RevitFile::open
//!   2. summarize_strict
//!   3. parse_schema on Formats/Latest
//!   4. elem_table::parse_header
//!   5. walker::read_adocument_lossy
//!   6. ifc::RvtDocExporter::export
//!
//! For each file, records which stage failed (if any) and the error
//! string. Emits a summary table at the end.

use rvt::{
    RevitFile, compression, elem_table, formats,
    ifc::{Exporter, RvtDocExporter, write_step},
    streams, walker,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn walk_recursive(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
            // skip .git to keep the sweep fast
            if p.file_name().and_then(|n| n.to_str()) == Some(".git") {
                continue;
            }
            walk_recursive(&p, out);
        } else if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
            if ext == "rvt" || ext == "rfa" {
                out.push(p);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
enum Stage {
    Open,
    Summarize,
    ParseSchema,
    ElemTable,
    ReadAdocument,
    IfcExport,
    AllPassed,
}

impl Stage {
    fn label(&self) -> &'static str {
        match self {
            Stage::Open => "open",
            Stage::Summarize => "summarize",
            Stage::ParseSchema => "parse_schema",
            Stage::ElemTable => "elem_table",
            Stage::ReadAdocument => "read_adocument",
            Stage::IfcExport => "ifc_export",
            Stage::AllPassed => "ALL_PASSED",
        }
    }
}

fn exercise(path: &Path) -> (Stage, String) {
    let mut rf = match RevitFile::open(path) {
        Ok(r) => r,
        Err(e) => return (Stage::Open, format!("{e}")),
    };

    if let Err(e) = rf.summarize_strict() {
        return (Stage::Summarize, format!("{e}"));
    }

    // parse_schema on Formats/Latest — only attempt if stream exists.
    if rf.stream_names().iter().any(|s| s == "Formats/Latest") {
        match rf.read_stream(streams::FORMATS_LATEST) {
            Ok(raw) => match compression::inflate_at(&raw, 0) {
                Ok(d) => {
                    if let Err(e) = formats::parse_schema(&d) {
                        return (Stage::ParseSchema, format!("{e}"));
                    }
                }
                Err(e) => return (Stage::ParseSchema, format!("inflate: {e}")),
            },
            Err(e) => return (Stage::ParseSchema, format!("read: {e}")),
        }
    }

    // elem_table — only if stream exists.
    if rf.stream_names().iter().any(|s| s == "Global/ElemTable") {
        if let Err(e) = elem_table::parse_header(&mut rf) {
            return (Stage::ElemTable, format!("{e}"));
        }
    }

    if let Err(e) = walker::read_adocument_lossy(&mut rf) {
        return (Stage::ReadAdocument, format!("{e}"));
    }

    let model = match RvtDocExporter.export(&mut rf) {
        Ok(m) => m,
        Err(e) => return (Stage::IfcExport, format!("{e}")),
    };
    let step = write_step(&model);
    if step.is_empty() {
        return (Stage::IfcExport, "empty STEP output".into());
    }

    (Stage::AllPassed, String::new())
}

fn main() {
    let root = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "_corpus_candidates".to_string());
    let root_path = PathBuf::from(&root);
    if !root_path.exists() {
        eprintln!("corpus root does not exist: {root}");
        std::process::exit(1);
    }

    let mut files = Vec::new();
    walk_recursive(&root_path, &mut files);
    files.sort();
    println!("Scanning {} files under {root}", files.len());

    let mut by_stage: BTreeMap<Stage, Vec<(PathBuf, String, u64)>> = BTreeMap::new();
    let mut by_repo: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // (pass, fail)

    for path in &files {
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let (stage, err) = exercise(path);
        by_stage
            .entry(stage)
            .or_default()
            .push((path.clone(), err, size));

        // repo = first dir under root
        let repo = path
            .strip_prefix(&root_path)
            .ok()
            .and_then(|p| p.components().next())
            .and_then(|c| c.as_os_str().to_str())
            .unwrap_or("?")
            .to_string();
        let entry = by_repo.entry(repo).or_insert((0, 0));
        if stage == Stage::AllPassed {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
    }

    println!("\n=== Results by stage ===");
    for (stage, items) in &by_stage {
        println!("  {:<16} {:>4}", stage.label(), items.len());
    }

    println!("\n=== Results by repo ===");
    for (repo, (pass, fail)) in &by_repo {
        let total = pass + fail;
        let pct = 100.0 * (*pass as f64) / (total as f64);
        println!("  {:<24} {pass:>3}/{total:<3} ({pct:>5.1}% pass)", repo);
    }

    println!("\n=== Sample failures per stage (first 3) ===");
    for (stage, items) in &by_stage {
        if *stage == Stage::AllPassed {
            continue;
        }
        println!("\n  stage = {}:", stage.label());
        for (path, err, size) in items.iter().take(3) {
            println!(
                "    {size:>10} B  {}  :: {err}",
                path.strip_prefix(&root_path).unwrap_or(path).display()
            );
        }
        if items.len() > 3 {
            println!("    (and {} more)", items.len() - 3);
        }
    }

    // Summary for Q01 reporting
    let all_pass = by_stage
        .get(&Stage::AllPassed)
        .map(|v| v.len())
        .unwrap_or(0);
    let total = files.len();
    println!(
        "\n=== Summary: {all_pass}/{total} files pass the full 5-stage pipeline ({:.1}%) ===",
        100.0 * (all_pass as f64) / (total as f64)
    );
}
