//! `rvt-inspect` — plain-language file health and export readiness.

use clap::Parser;
use rvt::{
    RevitFile,
    ifc::{ExportDiagnostics, RvtDocExporter},
};
use serde::Serialize;
use std::path::PathBuf;
use std::process::ExitCode;

const INSPECT_SCHEMA_VERSION: u32 = 1;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-inspect",
    version,
    about = "Summarize Revit file health, decoded model coverage, and IFC export readiness"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,

    /// Emit a stable JSON report instead of the human summary.
    #[arg(long)]
    json: bool,

    /// Keep original paths in the report. By default, paths are redacted
    /// so support output is safe to share.
    #[arg(long)]
    no_redact: bool,
}

#[derive(Debug, Serialize)]
struct InspectReport {
    schema_version: u32,
    file: FileHealth,
    decoded: DecodedHealth,
    export: ExportReadiness,
    warnings: Vec<String>,
    next_steps: Vec<String>,
    export_diagnostics: ExportDiagnostics,
}

#[derive(Debug, Serialize)]
struct FileHealth {
    input_path: String,
    file_size_bytes: u64,
    file_opened: bool,
    supported_revit_version: bool,
    revit_version: Option<u32>,
    build: Option<String>,
    original_path: Option<String>,
    stream_count: usize,
    schema_parsed: bool,
}

#[derive(Debug, Serialize)]
struct DecodedHealth {
    class_name_count: usize,
    production_elements: usize,
    diagnostic_candidates: usize,
    arcwall_records: usize,
    geometry_elements: usize,
}

#[derive(Debug, Serialize)]
struct ExportReadiness {
    level: String,
    score: f32,
    summary: String,
    can_write_ifc: bool,
    building_elements: usize,
    building_elements_with_geometry: usize,
    unsupported_features: Vec<String>,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let report = build_report(&cli.file, !cli.no_redact)?;
    if cli.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_report(&report);
    }
    Ok(())
}

fn build_report(path: &std::path::Path, redact: bool) -> anyhow::Result<InspectReport> {
    let file_size_bytes = std::fs::metadata(path)?.len();
    let mut rf = RevitFile::open(path)?;
    let mut summary = rf.summarize_lossy()?.value;
    if redact {
        redact_summary(&mut summary);
    }
    let schema_parsed = rf.schema().is_ok();
    let export_result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let diagnostics = export_result.diagnostics;
    let warnings = diagnostics.warnings.clone();
    let input_path = display_path(path, redact);
    let supported_revit_version = matches!(summary.version, 2016..=2026);
    let decoded = DecodedHealth {
        class_name_count: summary.class_name_count,
        production_elements: diagnostics.decoded.production_walker_elements,
        diagnostic_candidates: diagnostics.decoded.diagnostic_proxy_candidates,
        arcwall_records: diagnostics.decoded.arcwall_records,
        geometry_elements: diagnostics.exported.building_elements_with_geometry,
    };
    let export = export_readiness(&diagnostics);
    let next_steps = next_steps(&diagnostics, schema_parsed);

    Ok(InspectReport {
        schema_version: INSPECT_SCHEMA_VERSION,
        file: FileHealth {
            input_path,
            file_size_bytes,
            file_opened: true,
            supported_revit_version,
            revit_version: Some(summary.version),
            build: summary.build,
            original_path: summary.original_path,
            stream_count: summary.streams.len(),
            schema_parsed,
        },
        decoded,
        export,
        warnings,
        next_steps,
        export_diagnostics: diagnostics,
    })
}

fn redact_summary(summary: &mut rvt::reader::Summary) {
    if let Some(path) = &summary.original_path {
        summary.original_path = Some(rvt::redact::redact_sensitive(path));
    }
}

fn display_path(path: &std::path::Path, redact: bool) -> String {
    let raw = path.display().to_string();
    if redact {
        rvt::redact::redact_sensitive(&raw)
    } else {
        raw
    }
}

fn export_readiness(diagnostics: &ExportDiagnostics) -> ExportReadiness {
    let level = diagnostics.confidence.level.clone();
    let summary = match level.as_str() {
        "geometry" => "IFC export includes decoded element geometry.",
        "typed_no_geometry" => "IFC export includes typed elements, but no decoded geometry.",
        "diagnostic_partial" => {
            "IFC export includes diagnostic proxy candidates; treat them as support evidence."
        }
        "proxy_only" => "IFC export contains proxy elements only.",
        "scaffold" => "IFC export is scaffold-only; no validated building elements were decoded.",
        _ => "IFC export readiness is unknown.",
    }
    .to_string();

    ExportReadiness {
        level,
        score: diagnostics.confidence.score,
        summary,
        can_write_ifc: true,
        building_elements: diagnostics.exported.building_elements,
        building_elements_with_geometry: diagnostics.exported.building_elements_with_geometry,
        unsupported_features: diagnostics.unsupported_features.clone(),
    }
}

fn next_steps(diagnostics: &ExportDiagnostics, schema_parsed: bool) -> Vec<String> {
    let mut steps = Vec::new();
    if !schema_parsed {
        steps.push("Attach the diagnostics JSON; the schema stream could not be parsed.".into());
    }
    if diagnostics.exported.building_elements == 0 {
        steps.push(
            "This file currently exports as an IFC scaffold. Share diagnostics before relying on element counts."
                .into(),
        );
    }
    if diagnostics.exported.building_elements_with_geometry == 0 {
        steps.push(
            "No real-file element geometry was decoded. Use the output for metadata/status, not geometry handoff."
                .into(),
        );
    }
    if diagnostics.confidence.warning_count > 0 {
        steps
            .push("Review warnings before sending this file through an automated workflow.".into());
    }
    if steps.is_empty() {
        steps.push("This file meets the currently decoded export profile.".into());
    }
    steps
}

fn print_human_report(report: &InspectReport) {
    println!("File health");
    println!(
        "  Opened: yes · Revit {}{} · {} streams · {}",
        report
            .file
            .revit_version
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown".into()),
        if report.file.supported_revit_version {
            " (supported)"
        } else {
            " (outside verified range)"
        },
        report.file.stream_count,
        if report.file.schema_parsed {
            "schema parsed"
        } else {
            "schema not parsed"
        }
    );
    if let Some(build) = &report.file.build {
        println!("  Build: {build}");
    }
    if let Some(original_path) = &report.file.original_path {
        println!("  Original path: {original_path}");
    }

    println!("\nDecoded coverage");
    println!("  Class names: {}", report.decoded.class_name_count);
    println!(
        "  Validated elements: {}",
        report.decoded.production_elements
    );
    println!(
        "  Diagnostic candidates: {}",
        report.decoded.diagnostic_candidates
    );
    println!("  ArcWall records: {}", report.decoded.arcwall_records);
    println!(
        "  Elements with geometry: {}",
        report.decoded.geometry_elements
    );

    println!("\nIFC export readiness");
    println!(
        "  {} · {}%",
        report.export.level,
        (report.export.score * 100.0).round() as u32
    );
    println!("  {}", report.export.summary);
    if !report.export.unsupported_features.is_empty() {
        println!("  Unsupported features:");
        for feature in &report.export.unsupported_features {
            println!("    - {feature}");
        }
    }

    println!("\nWarnings");
    if report.warnings.is_empty() {
        println!("  none");
    } else {
        for warning in &report.warnings {
            println!("  - {warning}");
        }
    }

    println!("\nNext steps");
    for step in &report.next_steps {
        println!("  - {step}");
    }
}
