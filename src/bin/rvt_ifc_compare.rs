//! `rvt-ifc-compare` — compare two IFC4 STEP files for export QA (M5-05).
//!
//! Typical use: compare an `rvt-ifc` export against a Revit (or other
//! reference) IFC of the same model. Emits a human summary on stdout and
//! optional JSON for tooling.
//!
//! Exit codes:
//! - `0` — comparison produced (even when divergences exist)
//! - `1` — I/O or parse failure
//! - `2` — `--fail-on-diff` set and any structural divergence was found

use clap::Parser;
use rvt::ifc::compare::{compare_summaries, format_human_report, summarize_ifc_step};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-ifc-compare",
    version,
    about = "Compare two IFC4 STEP files (entity counts, storeys, bbox, objects, materials, properties)"
)]
struct Cli {
    /// Left IFC path (typically the rvt-rs export).
    left: PathBuf,
    /// Right IFC path (typically a Revit reference export).
    right: PathBuf,
    /// Write the full comparison report as JSON.
    #[arg(long, value_name = "PATH")]
    json: Option<PathBuf>,
    /// Suppress the human summary on stdout (JSON-only mode when `--json` is set).
    #[arg(long)]
    quiet: bool,
    /// Exit 2 when entity counts, storeys, objects, materials, property keys,
    /// or bounding-box presence differ.
    #[arg(long)]
    fail_on_diff: bool,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    let left_text = std::fs::read_to_string(&cli.left)?;
    let right_text = std::fs::read_to_string(&cli.right)?;
    let left = summarize_ifc_step(&left_text);
    let right = summarize_ifc_step(&right_text);
    let left_label = cli
        .left
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cli.left.display().to_string());
    let right_label = cli
        .right
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| cli.right.display().to_string());

    let report = compare_summaries(left_label, left, right_label, right);

    if !cli.quiet {
        print!("{}", format_human_report(&report));
    }

    if let Some(path) = &cli.json {
        let json = serde_json::to_vec_pretty(&report)?;
        std::fs::write(path, &json)?;
        if !cli.quiet {
            eprintln!(
                "rvt-ifc-compare: wrote JSON report ({} bytes) to {}",
                json.len(),
                path.display()
            );
        }
    }

    if cli.fail_on_diff && has_structural_diff(&report) {
        return Ok(ExitCode::from(2));
    }
    Ok(ExitCode::SUCCESS)
}

fn has_structural_diff(report: &rvt::ifc::compare::IfcCompareReport) -> bool {
    !report.entity_count_deltas.is_empty()
        || !report.storeys_only_left.is_empty()
        || !report.storeys_only_right.is_empty()
        || !report.objects_only_left.is_empty()
        || !report.objects_only_right.is_empty()
        || !report.materials_only_left.is_empty()
        || !report.materials_only_right.is_empty()
        || !report.property_keys_only_left.is_empty()
        || !report.property_keys_only_right.is_empty()
        || report
            .bounding_box_delta
            .as_ref()
            .is_some_and(|d| d.left.is_some() != d.right.is_some())
}
