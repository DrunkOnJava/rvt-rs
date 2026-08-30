//! `rvt-ifc` — convert a Revit file to IFC4.
//!
//! This is the first end-to-end `rvt → ifc` pipeline. Current scope:
//! document-level metadata export (IfcProject, framework entities,
//! units, geometric context). Element-level geometry is pending walker
//! expansion; this command's output is spec-valid IFC4 that readers
//! (IfcOpenShell, BlenderBIM) accept — just sparse until more of the
//! schema is walked.

use clap::{Parser, ValueEnum};
use rvt::{
    RevitFile,
    ifc::{
        DiagnosticRvtDocExporter, ExportDiagnostics, ExportQualityMode, PlaceholderExporter,
        RvtDocExporter, write_step,
    },
    walker::WalkerLimits,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum CliExportQualityMode {
    Scaffold,
    TypedNoGeometry,
    Geometry,
    Strict,
}

impl From<CliExportQualityMode> for ExportQualityMode {
    fn from(value: CliExportQualityMode) -> Self {
        match value {
            CliExportQualityMode::Scaffold => Self::Scaffold,
            CliExportQualityMode::TypedNoGeometry => Self::TypedNoGeometry,
            CliExportQualityMode::Geometry => Self::Geometry,
            CliExportQualityMode::Strict => Self::Strict,
        }
    }
}

#[derive(Parser)]
#[command(
    version,
    about = "Convert a Revit file to IFC4 (document-level export)"
)]
struct Args {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    input: PathBuf,
    /// Output path. If omitted, writes `<input>.ifc` next to the input.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Use the placeholder exporter (empty project body) instead of the
    /// document exporter. Mostly useful for testing the STEP writer.
    /// Kept as `--null` for backward compatibility with earlier versions.
    #[arg(long, alias = "null", conflicts_with = "diagnostic_proxies")]
    placeholder: bool,
    /// Require a minimum export quality before writing IFC.
    ///
    /// `scaffold` preserves historical behavior and accepts a valid IFC4
    /// framework even when real elements are missing. `typed-no-geometry`
    /// requires validated typed IFC elements. `geometry` requires at least
    /// one exported element with geometry. `strict` also requires recovered
    /// project metadata, units, levels, and zero export warnings.
    #[arg(long, value_enum, default_value = "scaffold")]
    mode: CliExportQualityMode,
    /// Include low-confidence schema-scan candidates as
    /// IFCBUILDINGELEMENTPROXY entities with diagnostic provenance.
    ///
    /// Default export suppresses these candidates because they are not
    /// validated typed model elements.
    #[arg(long, conflicts_with = "placeholder")]
    diagnostic_proxies: bool,
    /// Write a JSON diagnostics sidecar for bug reports and support.
    ///
    /// The sidecar includes input metadata, decoded/exported element counts,
    /// skipped low-confidence candidates, unsupported features, warnings, and
    /// an export confidence summary.
    #[arg(long, value_name = "PATH")]
    diagnostics: Option<PathBuf>,
    /// Witness mode: write an OctetProof observation of this input.
    ///
    /// The observation (docs/octetproof-spec-draft.md §6.2) records the
    /// SHA-256 of the input bytes, the exported entity counts by IFC type,
    /// and a canonical hash of that payload so an independent replay can
    /// prove this witness saw the same thing. Compared against other
    /// witnesses by tools/ci/witness-verdict.py.
    #[arg(long, value_name = "PATH")]
    observation: Option<PathBuf>,
    /// Golden-artifact id to stamp into the observation (registry id).
    #[arg(long, value_name = "ID", requires = "observation")]
    artifact_id: Option<String>,
    /// Maximum decompressed Global/Latest bytes scanned by the walker.
    #[arg(long)]
    max_walker_scan_bytes: Option<usize>,
    /// Maximum schema-scan candidates retained by the walker.
    #[arg(long)]
    max_walker_candidates: Option<usize>,
    /// Maximum trial decodes attempted by the walker.
    #[arg(long)]
    max_walker_trial_offsets: Option<usize>,
    /// Maximum bytes inspected while decoding one walker candidate.
    #[arg(long)]
    max_walker_record_decode_bytes: Option<usize>,
    /// Maximum records accepted in walker reference containers.
    #[arg(long)]
    max_walker_container_records: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rf = RevitFile::open(&args.input)?;
    let quality_mode = ExportQualityMode::from(args.mode);
    let walker_limits = walker_limits_from_args(&args);

    let result = if args.placeholder {
        PlaceholderExporter.export_with_diagnostics(&mut rf)?
    } else if args.diagnostic_proxies {
        DiagnosticRvtDocExporter.export_with_diagnostics_and_limits(&mut rf, walker_limits)?
    } else {
        RvtDocExporter.export_with_diagnostics_mode_and_limits(
            &mut rf,
            quality_mode,
            walker_limits,
        )?
    };
    let model = result.model;
    let diagnostics = result.diagnostics;

    let out_path = args.output.clone().unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("ifc");
        p
    });

    if let Err(err) = quality_mode.validate(&diagnostics) {
        if let Some(diagnostics_path) = &args.diagnostics {
            write_diagnostics_sidecar(diagnostics_path, &diagnostics)?;
        }
        anyhow::bail!("{err}");
    }

    warn_about_export_quality(&diagnostics);

    let step = write_step(&model);
    std::fs::write(&out_path, step.as_bytes())?;
    eprintln!(
        "rvt-ifc: wrote {} bytes to {}",
        step.len(),
        out_path.display()
    );
    if let Some(diagnostics_path) = &args.diagnostics {
        write_diagnostics_sidecar(diagnostics_path, &diagnostics)?;
    }
    if let Some(observation_path) = &args.observation {
        write_observation(
            observation_path,
            &args.input,
            args.artifact_id.as_deref(),
            &diagnostics,
            &step,
        )?;
    }
    if model.project_name.is_none() {
        eprintln!(
            "note: no project name extracted; output IFC uses \"Untitled\". \
             Title typically comes from PartAtom — this file may not have one."
        );
    }
    Ok(())
}

fn walker_limits_from_args(args: &Args) -> WalkerLimits {
    let defaults = WalkerLimits::default();
    WalkerLimits {
        max_scan_bytes: args
            .max_walker_scan_bytes
            .unwrap_or(defaults.max_scan_bytes),
        max_candidates: args
            .max_walker_candidates
            .unwrap_or(defaults.max_candidates),
        max_trial_offsets: args
            .max_walker_trial_offsets
            .unwrap_or(defaults.max_trial_offsets),
        max_per_record_decode_bytes: args
            .max_walker_record_decode_bytes
            .unwrap_or(defaults.max_per_record_decode_bytes),
        max_container_records: args
            .max_walker_container_records
            .unwrap_or(defaults.max_container_records),
    }
}

fn write_diagnostics_sidecar(
    path: &std::path::Path,
    diagnostics: &ExportDiagnostics,
) -> anyhow::Result<()> {
    let json = serde_json::to_vec_pretty(diagnostics)?;
    std::fs::write(path, &json)?;
    eprintln!(
        "rvt-ifc: wrote diagnostics {} bytes to {}",
        json.len(),
        path.display()
    );
    Ok(())
}

/// OctetProof observation (§6.2): canonical JSON is serde_json's default
/// output — keys sorted (no `preserve_order`), no whitespace — which is
/// byte-identical to the Python witnesses' `sort_keys` + compact separators,
/// so the payload hash is comparable across languages.
fn write_observation(
    path: &std::path::Path,
    input: &std::path::Path,
    artifact_id: Option<&str>,
    diagnostics: &ExportDiagnostics,
    step: &str,
) -> anyhow::Result<()> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(input)?;
    let input_hash = format!("{:x}", Sha256::digest(&bytes));
    // entity_counts is what an IFC reader would see in the STEP this run
    // wrote — every `#n=IFCTYPE(` constructor, not just the building-element
    // histogram in the diagnostics (which omits units, contexts, rels).
    let mut entity_counts = std::collections::BTreeMap::<String, usize>::new();
    for line in step.lines() {
        let ifc_type = line
            .strip_prefix('#')
            .and_then(|rest| rest.split_once('='))
            .and_then(|(_, after_eq)| after_eq.trim_start().split_once('('))
            .map(|(ifc_type, _)| ifc_type);
        if let Some(ifc_type) = ifc_type {
            *entity_counts.entry(ifc_type.to_string()).or_default() += 1;
        }
    }
    let payload = serde_json::json!({
        "entity_counts": entity_counts,
        "exported_building_elements": diagnostics.exported.by_ifc_type,
        "storey_count": diagnostics.exported.storey_count,
        "material_count": diagnostics.exported.material_count,
        "building_elements_with_geometry": diagnostics.exported.building_elements_with_geometry,
    });
    let canonical = serde_json::to_string(&payload)?;
    let observation = serde_json::json!({
        "schema_version": "1.0.0",
        "witness_id": "rvt-rs",
        "witness_version": env!("CARGO_PKG_VERSION"),
        "artifact_id": artifact_id,
        "input_role": "source",
        "input_file": input.file_name().and_then(|n| n.to_str()),
        "input_hash_sha256": input_hash,
        "deterministic": true,
        "semantic_surface_covered": ["entity_counts"],
        "observation": payload,
        "observation_hash_sha256": format!("{:x}", Sha256::digest(canonical.as_bytes())),
        "unsupported_entities": diagnostics.unsupported_features,
        "warnings": diagnostics.warnings,
    });
    let mut json = serde_json::to_vec_pretty(&observation)?;
    json.push(b'\n');
    std::fs::write(path, &json)?;
    eprintln!(
        "rvt-ifc: wrote observation ({} bytes) to {}",
        json.len(),
        path.display()
    );
    Ok(())
}

fn warn_about_export_quality(diagnostics: &ExportDiagnostics) {
    if diagnostics.confidence.level == "scaffold" {
        eprintln!(
            "warning: export confidence is scaffold-only; no validated building elements were exported. \
             Re-run with `--diagnostics <path>` for a shareable readiness report."
        );
    }
}
