//! `rvt-corpus` — analyze multiple Revit files as a corpus.
//!
//! Phase B of the attack plan. Feed in 3+ versions of the same content and
//! get a structural classification of every byte position per stream:
//! invariant (structural constants), low-variance (type tags), size-correlated
//! (length fields), or variable (payload data).
//!
//! This drives Phase D (object graph parsing) by generating 80% of the
//! structural hypotheses automatically.

use clap::{Args, Parser, Subcommand, ValueEnum};
use rvt::corpus::{Sample, analyze_corpus};
use rvt::ifc::RvtDocExporter;
use rvt::streams::{BASIC_FILE_INFO, FORMATS_LATEST, GLOBAL_LATEST};
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-corpus",
    version,
    about = "Corpus-wide delta analysis across multiple Revit files"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// 3+ Revit files for delta analysis (same logical content, different versions works best).
    #[arg(num_args = 0..)]
    files: Vec<PathBuf>,

    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "text", value_enum)]
    format: Format,

    /// Print up to N invariant runs per stream (each has offset/length/hex preview).
    #[arg(long = "runs", default_value_t = 5)]
    runs: usize,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Classify corpus files into issue-ready failure buckets.
    Doctor(DoctorArgs),
}

#[derive(Args, Debug)]
struct DoctorArgs {
    /// Revit files or directories to scan recursively.
    #[arg(required = true, num_args = 1..)]
    paths: Vec<PathBuf>,

    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "text", value_enum)]
    format: Format,
}

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    if let Some(command) = cli.command {
        return match command {
            Command::Doctor(args) => run_doctor(args),
        };
    }

    if cli.files.len() < 3 {
        anyhow::bail!(
            "delta analysis requires at least 3 Revit files; use `rvt-corpus doctor <path>...` for triage"
        );
    }

    let mut samples: Vec<Sample> = cli
        .files
        .iter()
        .map(|p| Sample::open(p).map_err(anyhow::Error::from))
        .collect::<Result<_, _>>()?;

    let report = analyze_corpus(&mut samples)?;

    match cli.format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Format::Text => {
            println!("=== Corpus delta analysis ===");
            println!("samples: {}", report.samples.len());
            for s in &report.samples {
                println!("  · {}", s);
            }
            println!(
                "partition-number → year mapping: {} entries",
                report.partition_mapping.len()
            );
            for (num, year) in &report.partition_mapping {
                let y = year.map(|n| n.to_string()).unwrap_or_else(|| "?".into());
                println!("  Partitions/{}  → Revit {}", num, y);
            }
            println!();

            println!(
                "{:<32} {:>5}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}",
                "stream", "smp", "raw sz", "decomp sz", "invar", "low-var", "var"
            );
            println!("{}", "-".repeat(95));

            for s in &report.streams {
                let raw = format!("{}-{}", s.raw_size_min, s.raw_size_max);
                let decomp = match (s.decomp_size_min, s.decomp_size_max) {
                    (Some(mn), Some(mx)) => format!("{}-{}", mn, mx),
                    _ => "—".into(),
                };
                let inv = s.counts.get("Invariant").copied().unwrap_or(0);
                let lowvar = s.counts.get("LowVariance").copied().unwrap_or(0);
                let var = s.counts.get("Variable").copied().unwrap_or(0);
                let marker = if s.used_decompressed { "D" } else { "R" };
                println!(
                    "{:<32} {:>5}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}   [{}]",
                    s.name, s.samples, raw, decomp, inv, lowvar, var, marker
                );
            }

            println!();
            println!("=== Invariant runs (length ≥ 8 bytes, suggests structural markers) ===");
            for s in &report.streams {
                if s.invariant_runs.is_empty() {
                    continue;
                }
                println!("\n{} ({} runs total)", s.name, s.invariant_runs.len());
                for r in s.invariant_runs.iter().take(cli.runs) {
                    println!(
                        "  offset=0x{:06x}  len={:>4}  {}",
                        r.offset, r.length, r.hex_preview
                    );
                }
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    schema_version: u32,
    files_scanned: usize,
    buckets: BTreeMap<String, DoctorBucket>,
    files: Vec<DoctorFileReport>,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorBucket {
    count: usize,
    suggested_labels: Vec<&'static str>,
    sample_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorFileReport {
    path: String,
    bucket: &'static str,
    ok: bool,
    revit_version: Option<u32>,
    failure: Option<String>,
    warnings: Vec<String>,
    unsupported_features: Vec<String>,
    suggested_labels: Vec<&'static str>,
}

fn run_doctor(args: DoctorArgs) -> anyhow::Result<()> {
    let files = collect_revit_files(&args.paths)?;
    let mut reports = Vec::with_capacity(files.len());
    for path in files {
        reports.push(doctor_file(&path));
    }
    let report = doctor_report(reports);

    match args.format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Format::Text => print_doctor_report(&report),
    }

    Ok(())
}

fn collect_revit_files(paths: &[PathBuf]) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_file() {
            out.push(path.to_path_buf());
        } else {
            collect_revit_files_inner(path, &mut out)?;
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

fn collect_revit_files_inner(path: &Path, out: &mut Vec<PathBuf>) -> anyhow::Result<()> {
    if path.is_file() {
        if has_revit_extension(path) {
            out.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !path.is_dir() {
        anyhow::bail!("path not found: {}", path.display());
    }

    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let child = entry.path();
        if child
            .components()
            .any(|component| component.as_os_str() == ".git")
        {
            continue;
        }
        if child.is_dir() {
            collect_revit_files_inner(&child, out)?;
        } else if has_revit_extension(&child) {
            out.push(child);
        }
    }
    Ok(())
}

fn has_revit_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "rvt" | "rfa" | "rte" | "rft"
            )
        })
        .unwrap_or(false)
}

fn doctor_file(path: &Path) -> DoctorFileReport {
    let display_path = path.display().to_string();
    if !has_cfb_magic(path) {
        return doctor_failure(
            display_path,
            "not_cfb",
            None,
            "file is not an OLE/CFB container",
        );
    }

    let mut rf = match rvt::RevitFile::open(path) {
        Ok(rf) => rf,
        Err(err) => {
            let bucket = if matches!(err, rvt::Error::NotACfbFile) {
                "not_cfb"
            } else {
                "cfb_open_error"
            };
            return doctor_failure(display_path, bucket, None, err.to_string());
        }
    };

    let streams = rf.stream_names();
    let missing: Vec<_> = [BASIC_FILE_INFO, FORMATS_LATEST, GLOBAL_LATEST]
        .into_iter()
        .filter(|required| !streams.iter().any(|stream| stream == required))
        .collect();
    if !missing.is_empty() {
        return doctor_failure(
            display_path,
            "missing_revit_streams",
            None,
            format!("missing required stream(s): {}", missing.join(", ")),
        );
    }

    let revit_version = rf.basic_file_info().ok().map(|bfi| bfi.version);
    if let Some(version) = revit_version {
        if !(2016..=2026).contains(&version) {
            return doctor_failure(
                display_path,
                "unsupported_version",
                Some(version),
                format!("Revit {version} is outside the verified 2016-2026 corpus range"),
            );
        }
    }

    match rf
        .read_stream(FORMATS_LATEST)
        .and_then(|bytes| rvt::compression::inflate_at(&bytes, 0).map(|_| ()))
    {
        Ok(()) => {}
        Err(err) => {
            return doctor_failure(display_path, "corrupt_gzip", revit_version, err.to_string());
        }
    }

    if let Err(err) = rf.schema() {
        return doctor_failure(
            display_path,
            "schema_parse_failure",
            revit_version,
            err.to_string(),
        );
    }

    let mut warnings = Vec::new();
    if let Ok(decoded) = rf.summarize_lossy() {
        if !decoded.is_clean() {
            warnings.push(decoded.diagnostics.to_string());
        }
    }

    let export = match RvtDocExporter.export_with_diagnostics(&mut rf) {
        Ok(export) => export,
        Err(err) => {
            return doctor_failure(
                display_path,
                "ifc_export_failure",
                revit_version,
                err.to_string(),
            );
        }
    };
    warnings.extend(export.diagnostics.warnings.clone());
    let unsupported_features = export.diagnostics.unsupported_features.clone();

    let bucket = if export.diagnostics.exported.building_elements == 0 {
        "empty_ifc_export"
    } else if !warnings.is_empty() {
        "partial_walker_decode"
    } else {
        "ok"
    };

    DoctorFileReport {
        path: display_path,
        bucket,
        ok: bucket == "ok",
        revit_version,
        failure: None,
        warnings,
        unsupported_features,
        suggested_labels: labels_for_bucket(bucket),
    }
}

fn has_cfb_magic(path: &Path) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return false,
    };
    let mut magic = [0u8; 8];
    file.read_exact(&mut magic).is_ok() && magic == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]
}

fn doctor_failure(
    path: String,
    bucket: &'static str,
    revit_version: Option<u32>,
    failure: impl Into<String>,
) -> DoctorFileReport {
    DoctorFileReport {
        path,
        bucket,
        ok: false,
        revit_version,
        failure: Some(failure.into()),
        warnings: Vec::new(),
        unsupported_features: Vec::new(),
        suggested_labels: labels_for_bucket(bucket),
    }
}

fn doctor_report(files: Vec<DoctorFileReport>) -> DoctorReport {
    let mut buckets = BTreeMap::<String, DoctorBucket>::new();
    for file in &files {
        let bucket = buckets
            .entry(file.bucket.to_string())
            .or_insert_with(|| DoctorBucket {
                count: 0,
                suggested_labels: labels_for_bucket(file.bucket),
                sample_paths: Vec::new(),
            });
        bucket.count += 1;
        if bucket.sample_paths.len() < 8 {
            bucket.sample_paths.push(file.path.clone());
        }
    }

    DoctorReport {
        schema_version: 1,
        files_scanned: files.len(),
        buckets,
        files,
    }
}

fn labels_for_bucket(bucket: &str) -> Vec<&'static str> {
    match bucket {
        "ok" => vec!["area:corpus"],
        "not_cfb" | "cfb_open_error" => vec!["type:bug", "area:reader", "area:corpus"],
        "missing_revit_streams" => vec!["type:bug", "area:reader", "area:corpus"],
        "corrupt_gzip" => vec!["type:bug", "area:reader"],
        "schema_parse_failure" => vec!["type:bug", "area:schema"],
        "unsupported_version" => vec!["type:feature", "area:reader"],
        "partial_walker_decode" => vec!["type:bug", "area:walker", "area:ifc"],
        "empty_ifc_export" => vec!["type:feature", "area:elements", "area:ifc"],
        "ifc_export_failure" => vec!["type:bug", "area:ifc"],
        _ => vec!["type:bug"],
    }
}

fn print_doctor_report(report: &DoctorReport) {
    println!("=== Corpus doctor ===");
    println!("files scanned: {}", report.files_scanned);
    println!();
    println!("{:<32} {:>6}  labels", "bucket", "count");
    println!("{}", "-".repeat(72));
    for (name, bucket) in &report.buckets {
        println!(
            "{:<32} {:>6}  {}",
            name,
            bucket.count,
            bucket.suggested_labels.join(", ")
        );
    }

    println!();
    for file in &report.files {
        println!("{}  {}", file.bucket, file.path);
        if let Some(version) = file.revit_version {
            println!("  Revit {version}");
        }
        if let Some(failure) = &file.failure {
            println!("  failure: {failure}");
        }
        for warning in file.warnings.iter().take(3) {
            println!("  warning: {warning}");
        }
        if !file.unsupported_features.is_empty() {
            println!(
                "  unsupported: {}",
                file.unsupported_features
                    .iter()
                    .take(8)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        println!("  labels: {}", file.suggested_labels.join(", "));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_labels_are_issue_ready() {
        assert_eq!(
            labels_for_bucket("empty_ifc_export"),
            vec!["type:feature", "area:elements", "area:ifc"]
        );
        assert_eq!(
            labels_for_bucket("schema_parse_failure"),
            vec!["type:bug", "area:schema"]
        );
    }

    #[test]
    fn doctor_report_groups_buckets_and_samples() {
        let files = vec![
            doctor_failure("a.rvt".into(), "not_cfb", None, "bad magic"),
            doctor_failure("b.rvt".into(), "not_cfb", None, "bad magic"),
            doctor_failure("c.rvt".into(), "corrupt_gzip", Some(2024), "inflate"),
        ];
        let report = doctor_report(files);

        assert_eq!(report.files_scanned, 3);
        assert_eq!(report.buckets["not_cfb"].count, 2);
        assert_eq!(report.buckets["corrupt_gzip"].count, 1);
        assert_eq!(
            report.buckets["not_cfb"].suggested_labels,
            vec!["type:bug", "area:reader", "area:corpus"]
        );
    }
}
