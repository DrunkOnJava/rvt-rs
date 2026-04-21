//! Multi-megabyte project-file benchmark (Q-07).
//!
//! Timing against real `.rvt` project files — not synthetic inputs
//! — to establish how rvt-rs scales with file size.
//!
//! Corpus path is resolved via `RVT_PROJECT_CORPUS_DIR` (defaults
//! to `/private/tmp/rvt-corpus-probe/magnetar/Revit`). If neither
//! project file is present, each sub-benchmark short-circuits with
//! an `eprintln!` so `cargo bench` stays green on machines without
//! the corpus. The files themselves are MIT-licensed via
//! magnetar-io/revit-test-datasets but not redistributed here (LFS).
//!
//! What we measure per file:
//!
//! * `open` — CFB parse + stream directory.
//! * `summarize_strict` — BasicFileInfo + schema + stream enumeration
//!   (the "tell me what this file is" path).
//! * `parse_schema` — Formats/Latest decode on the ~360 KB compressed
//!   schema.
//! * `elem_table::parse_records` — enumerate all declared ElementIds
//!   from Global/ElemTable.
//! * `read_adocument_lossy` — ADocument walker entry-point detection +
//!   13-field decode.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rvt::{RevitFile, compression, elem_table, formats, streams, walker};
use std::path::PathBuf;

fn project_dir() -> PathBuf {
    std::env::var("RVT_PROJECT_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/private/tmp/rvt-corpus-probe/magnetar/Revit"))
}

fn corpus_files() -> Vec<(&'static str, PathBuf)> {
    let dir = project_dir();
    vec![
        ("project-2023-913KB", dir.join("Revit_IFC5_Einhoven.rvt")),
        ("project-2024-34MB", dir.join("2024_Core_Interior.rvt")),
    ]
}

fn bench_open(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_file::open");
    for (label, path) in corpus_files() {
        if !path.exists() {
            eprintln!("skipping {label}: {} not present", path.display());
            continue;
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, p| {
            b.iter(|| RevitFile::open(p).unwrap())
        });
    }
    group.finish();
}

fn bench_summarize_strict(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_file::summarize_strict");
    for (label, path) in corpus_files() {
        if !path.exists() {
            continue;
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, p| {
            b.iter(|| {
                let mut rf = RevitFile::open(p).unwrap();
                let _ = rf.summarize_strict().unwrap();
            })
        });
    }
    group.finish();
}

fn bench_parse_schema(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_file::parse_schema");
    for (label, path) in corpus_files() {
        if !path.exists() {
            continue;
        }
        // Pre-inflate the Formats/Latest bytes; we want to measure
        // parse throughput on the decompressed payload, not the
        // inflate step (which is benched separately in
        // `compression.rs`).
        let mut rf = RevitFile::open(&path).unwrap();
        let raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
        let (_, decomp) = compression::inflate_at_auto(&raw).unwrap();
        group.bench_with_input(BenchmarkId::from_parameter(label), &decomp, |b, d| {
            b.iter(|| formats::parse_schema(d).unwrap())
        });
    }
    group.finish();
}

fn bench_elem_table_records(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_file::elem_table_records");
    for (label, path) in corpus_files() {
        if !path.exists() {
            continue;
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, p| {
            b.iter(|| {
                let mut rf = RevitFile::open(p).unwrap();
                let _ = elem_table::parse_records(&mut rf).unwrap();
            })
        });
    }
    group.finish();
}

fn bench_read_adocument(c: &mut Criterion) {
    let mut group = c.benchmark_group("project_file::read_adocument_lossy");
    for (label, path) in corpus_files() {
        if !path.exists() {
            continue;
        }
        group.bench_with_input(BenchmarkId::from_parameter(label), &path, |b, p| {
            b.iter(|| {
                let mut rf = RevitFile::open(p).unwrap();
                let _ = walker::read_adocument_lossy(&mut rf);
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_open,
    bench_summarize_strict,
    bench_parse_schema,
    bench_elem_table_records,
    bench_read_adocument
);
criterion_main!(benches);
