//! `rvt-ifc` — convert a Revit file to IFC4.
//!
//! Writes the project, its storeys and the elements rvt-rs decodes as typed
//! IFC4 entities, each with its body where the geometry is recovered and its
//! bounding box where it is not (`docs/support-matrix.json` rates each
//! category). The same input gives the same bytes, apart from the time in
//! the header and the owner history, which `SOURCE_DATE_EPOCH` fixes.

use clap::{Parser, ValueEnum};
use rvt::{
    RevitFile,
    ifc::{
        ExportDiagnostics, ExportKind, ExportOptions, ExportQualityMode, SavedMeshChoice,
        run_export_with_limits,
        step_writer::{StepOptions, write_step_to, write_step_to_with_space_materials},
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
    about = "Convert a Revit file to IFC4",
    after_help = "Examples:\n  \
        rvt-ifc model.rvt                     writes model.ifc next to the input\n  \
        rvt-ifc model.rvt -o out.ifc --diagnostics out.diagnostics.json\n  \
        rvt-ifc model.rvt --diagnostics out.diagnostics.json --anonymize   to post publicly\n  \
        rvt-ifc model.rvt --diagnostics out.diagnostics.json --element-provenance   list where each element was read from\n  \
        rvt-ifc model.rvt --diagnostics out.diagnostics.json --export-decisions   list why each element was left out\n  \
        rvt-ifc model.rvt --mode geometry\n  \
        rvt-ifc model.rvt --phase \"Working Drawings\"   export that phase exactly, not the last\n\n\
        SOURCE_DATE_EPOCH (Unix seconds) sets the time the IFC header and owner\n\
        history record, so two exports of the same file are the same bytes.\n\n\
        Exit status 3: a resource limit kept part of the file from being read, so\n\
        no IFC was written (the diagnostics sidecar, if asked for, lists the\n\
        streams). Re-run with --large-model, or --allow-partial to write it anyway."
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
    /// Draw every element whose body would be its record's bounding box
    /// (furniture, fixtures, equipment and the rest) from the mesh Revit
    /// saved for display (#255), on any release. This is the default on Revit
    /// 2023 to 2027, so the switch changes only Revit 2015 to 2022 files
    /// (RE-342). The walls that embed a curtain wall are drawn so on Revit
    /// 2024 to 2027 where Revit raised their base (RE-340).
    #[arg(long, conflicts_with_all = ["placeholder", "no_saved_meshes"])]
    saved_meshes: bool,
    /// Keep every element as its record's bounding box, and the walls that
    /// embed a curtain wall as their record's body, instead of the mesh Revit
    /// saved for them. Reading every box's saved graphics takes 1.1 to 1.9 times
    /// the time and 1.4 to 4.4 times the peak memory of 0.17.0 on Autodesk's
    /// samples of Revit 2023 to 2027 (#1191, #1694).
    #[arg(long, conflicts_with = "placeholder")]
    no_saved_meshes: bool,
    /// Draw a family instance from its family's own extrusions (the sketch
    /// lines they were drawn from, swept between their offsets) instead of
    /// its saved mesh, where they reproduce the mesh; every other instance
    /// keeps it (#156). An arc in a profile is written as the arc, whatever
    /// its radius, in the form Revit 24.0.5's IFC4 export gives it (#1366).
    /// Off by default. Implies `--saved-meshes`.
    /// Each instance is checked against its own mesh. Revit 2024 to 2027 files
    /// (measured on 2024 and 2025). Slower on large files.
    #[arg(long, conflicts_with_all = ["placeholder", "no_saved_meshes"])]
    family_extrusions: bool,
    /// Also write Revit's own property sets, as Revit's exporter does with
    /// "Export Revit property sets": each element's named parameters in one
    /// set per parameter group, and its type's on its type object (RE-210).
    /// Revit 2023 to 2027 files. Slower on large files.
    #[arg(long, conflicts_with = "placeholder")]
    revit_property_sets: bool,
    /// Also write properties where Revit's own exporter writes them and
    /// IFC4's definitions do not: a standard set whose entity's name begins
    /// with the element's (`Pset_StairFlightCommon` on a stair,
    /// `Pset_SpaceHeaterTypeCommon` on a space), its `Reference` only, and a
    /// text parameter named `<set>.<property>` for a property the set's IFC4
    /// definition lacks (`Pset_SpaceCommon.FloorCovering`) (RE-235). By
    /// default every property is in the set buildingSMART's IFC4 definitions
    /// put it in. It also keeps the material Revit's export associates with
    /// an `IfcSpace`, which IFC4's `IfcRelAssociatesMaterial.AllowedElements`
    /// does not allow: with it the output breaks that rule, as Revit's own
    /// export does (#1430).
    #[arg(long, conflicts_with = "placeholder")]
    revit_set_placement: bool,
    /// Leave out the base quantities (the `Qto_` sets: lengths, areas and
    /// widths), as Revit's exporter does when its export setup's "Export
    /// base quantities" is off. Revit's built-in IFC4 setups turn it on, and
    /// so does rvt-ifc by default (RE-242).
    #[arg(long)]
    no_base_quantities: bool,
    /// Write a JSON diagnostics sidecar for bug reports and support.
    ///
    /// The sidecar includes input metadata, decoded/exported element counts,
    /// skipped low-confidence candidates, unsupported features, warnings, and
    /// an export confidence summary.
    #[arg(long, value_name = "PATH")]
    diagnostics: Option<PathBuf>,
    /// Replace every name the diagnostics sidecar takes from the project
    /// with a placeholder: the file and project names, levels, materials,
    /// sample element names and the phase a left-out room sits in, in the
    /// list of decisions too. Counts and ElementIds stay, so the sidecar can
    /// be posted publicly.
    #[arg(long, requires = "diagnostics")]
    anonymize: bool,
    /// Also list, in the diagnostics sidecar, where each decoded element was
    /// read from (#1325): the kind of record, its stream and byte offset in
    /// the inflated stream (what rvt-dump writes as Partitions_NN.decomp),
    /// where its ElementId is stored, its class and its native record. Off by
    /// default: on a large model the list is hundreds of times the rest of
    /// the sidecar (docs/export-diagnostics.md, "Provenance table size", gives
    /// the measured sizes). Never written into the IFC.
    #[arg(long, requires = "diagnostics", conflicts_with = "placeholder")]
    element_provenance: bool,
    /// Also list, in the diagnostics sidecar, each fail-closed decision per
    /// element (#1326): why an element was left out, a body is only its
    /// record's box, an opening was not cut or an outline was not read, as
    /// its ElementId, a stage and a reason code, with the stream and byte
    /// offset its record was read from where known. The `skipped` counts are
    /// made from the same decisions. Bounded at 5,000 entries a reason, with
    /// the rest counted as dropped. Off by default. Never written into the IFC.
    #[arg(long, requires = "diagnostics", conflicts_with = "placeholder")]
    export_decisions: bool,
    /// Witness mode: write an OctetProof observation of this input.
    ///
    /// The observation (docs/octetproof-spec.md §6.2) records the
    /// SHA-256 of the input bytes, the exported entity counts by IFC type,
    /// the `IfcRelFillsElement` host/filling `Tag` pairs, the
    /// `IfcBuildingStorey` name/elevation set, and a canonical hash of
    /// that payload so an independent replay can prove this witness saw
    /// the same thing. Compared against other witnesses by
    /// tools/ci/witness-verdict.py.
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
    /// Maximum bytes of one stream read from the file (default 256 MiB). A
    /// larger stream is not read, and nothing is written unless
    /// `--allow-partial` (#1075).
    #[arg(long, value_name = "BYTES")]
    max_stream_bytes: Option<u64>,
    /// Maximum inflated bytes of one compressed member (default 256 MiB). The
    /// members of one stream may inflate to this or, whichever is larger,
    /// 16 times their stored size, at least 1 GiB and at most 4 GiB (1 GiB
    /// on a 32-bit build).
    #[arg(long, value_name = "BYTES")]
    max_inflate_bytes: Option<usize>,
    /// Export this phase, by its name (exactly) or its ElementId, instead of
    /// the project's last. A name is matched first and an ElementId only
    /// when no phase has that name, so a name made of digits wins. Revit's
    /// export shows one phase: an element that does not exist in it, and a
    /// room whose own phase is not it, are left out. A phase named is exact,
    /// the last included: without --phase, a project whose last phase holds
    /// no room writes the rooms of the latest phase that holds any, and
    /// says so (#1359), where `--phase` with the last phase writes none. An
    /// unknown phase is an error that lists the file's phases (RE-326).
    #[arg(long, value_name = "NAME|ID", conflicts_with = "placeholder")]
    phase: Option<String>,
    /// Raise the stream, inflate and walker-scan limits for large models:
    /// streams up to 2 GiB, 8 GiB inflated. A model whose largest partition
    /// is 1 GiB stored needs several GB of memory.
    #[arg(long)]
    large_model: bool,
    /// Write the IFC even when a resource limit kept part of the file from
    /// being read. The diagnostics list the streams that were not read.
    #[arg(long)]
    allow_partial: bool,
}

/// A resource limit kept part of the input from being read, and
/// `--allow-partial` was not given (#1075): exit status 3.
#[derive(Debug)]
struct PartialRead;

impl std::fmt::Display for PartialRead {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no IFC written: part of the input was not read (see above)")
    }
}

impl std::error::Error for PartialRead {}

fn main() -> std::process::ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run(Args::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            if e.downcast_ref::<PartialRead>().is_some() {
                std::process::ExitCode::from(3)
            } else {
                std::process::ExitCode::from(1)
            }
        }
    }
}

fn run(args: Args) -> anyhow::Result<()> {
    let timestamp = source_date_epoch()?;
    let mut rf = RevitFile::open_with_limits(
        &args.input,
        rvt::cli::open_limits(
            args.max_stream_bytes,
            args.max_inflate_bytes,
            args.large_model,
        ),
    )?;
    let quality_mode = ExportQualityMode::from(args.mode);
    let run = run_export_with_limits(
        &mut rf,
        &export_options(&args),
        walker_limits_from_args(&args),
    )?;
    let model = run.model;
    let diagnostics = run.diagnostics;
    for note in &run.notes {
        // The room fallback names a phase, the project's own text.
        if args.anonymize {
            eprintln!("rvt-ifc: {}", note.anonymized());
        } else {
            eprintln!("rvt-ifc: {note}");
        }
    }

    let out_path = args.output.clone().unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("ifc");
        p
    });

    if !run.unread.is_empty() {
        eprintln!(
            "rvt-ifc: {} stream(s) not read whole, so the export is missing what they hold:",
            run.unread.len()
        );
        for line in rvt::cli::unread_stream_report(&run.unread) {
            eprintln!("{line}");
        }
        if !args.allow_partial {
            if let Some(diagnostics_path) = &args.diagnostics {
                write_diagnostics_sidecar(diagnostics_path, &diagnostics, args.anonymize)?;
            }
            return Err(PartialRead.into());
        }
        eprintln!("rvt-ifc: writing the incomplete export (--allow-partial)");
    }

    if let Err(err) = quality_mode.validate(&diagnostics) {
        if let Some(diagnostics_path) = &args.diagnostics {
            write_diagnostics_sidecar(diagnostics_path, &diagnostics, args.anonymize)?;
        }
        anyhow::bail!("{err}");
    }

    warn_about_export_quality(&diagnostics);

    let options = StepOptions { timestamp };
    // The witness observation reads the text back; otherwise it is written as
    // it is built, after the file handle and the partitions it holds inflated
    // are released (#1075).
    let mut observed_step = None;
    drop(rf);
    let written = if args.observation.is_some() {
        // Built through the streaming writer, which refuses a model holding a
        // non-finite number rather than write a file no IFC reader opens.
        let mut bytes = Vec::new();
        if args.revit_set_placement {
            write_step_to_with_space_materials(&model, &options, &mut bytes)?;
        } else {
            write_step_to(&model, &options, &mut bytes)?;
        }
        let step = String::from_utf8(bytes)?;
        std::fs::write(&out_path, step.as_bytes())?;
        let written = step.len() as u64;
        observed_step = Some(step);
        written
    } else {
        // Written beside the output and renamed when whole, so a failed or
        // killed export never leaves a truncated IFC under the output's name.
        let mut partial = out_path.clone().into_os_string();
        partial.push(".partial");
        let partial = PathBuf::from(partial);
        let streamed = std::fs::File::create(&partial)
            .and_then(|file| {
                if args.revit_set_placement {
                    write_step_to_with_space_materials(&model, &options, file)
                } else {
                    write_step_to(&model, &options, file)
                }
            })
            .and_then(|written| std::fs::rename(&partial, &out_path).map(|()| written));
        match streamed {
            Ok(written) => written,
            Err(error) => {
                let _ = std::fs::remove_file(&partial);
                return Err(error.into());
            }
        }
    };
    eprintln!("rvt-ifc: wrote {written} bytes to {}", out_path.display());
    for line in export_summary(
        &diagnostics,
        args.diagnostics.is_some(),
        is_family_file(&args.input),
    ) {
        eprintln!("{line}");
    }
    if let Some(diagnostics_path) = &args.diagnostics {
        write_diagnostics_sidecar(diagnostics_path, &diagnostics, args.anonymize)?;
    }
    if let (Some(observation_path), Some(step)) = (&args.observation, &observed_step) {
        write_observation(
            observation_path,
            &args.input,
            args.artifact_id.as_deref(),
            &diagnostics,
            step,
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

/// The switches that change the model: the document exporter by default,
/// `--placeholder` or `--diagnostic-proxies` for the others; the saved-graphics
/// bodies are every boxed one on any release with `--saved-meshes`, none with
/// `--no-saved-meshes`, and otherwise every boxed one on Revit 2023 to 2027
/// and the walls that embed a curtain wall where Revit raised their base on
/// Revit 2024 to 2027 (RE-340).
fn export_options(args: &Args) -> ExportOptions {
    let kind = if args.placeholder {
        ExportKind::Placeholder
    } else if args.diagnostic_proxies {
        ExportKind::DiagnosticProxies
    } else {
        ExportKind::Document
    };
    let saved_meshes = if args.no_saved_meshes {
        SavedMeshChoice::Off
    } else if args.saved_meshes {
        SavedMeshChoice::All
    } else {
        SavedMeshChoice::Default
    };
    ExportOptions::default()
        .with_kind(kind)
        .with_mode(args.mode.into())
        .with_saved_meshes(saved_meshes)
        .with_family_extrusions(args.family_extrusions)
        .with_revit_property_sets(args.revit_property_sets)
        .with_revit_set_placement(args.revit_set_placement)
        .with_no_base_quantities(args.no_base_quantities)
        .with_phase(args.phase.clone())
        .with_element_provenance(args.element_provenance)
        .with_export_decisions(args.export_decisions)
}

fn walker_limits_from_args(args: &Args) -> WalkerLimits {
    let defaults = WalkerLimits::default();
    WalkerLimits {
        max_scan_bytes: args.max_walker_scan_bytes.unwrap_or(if args.large_model {
            rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES
        } else {
            defaults.max_scan_bytes
        }),
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

/// The time `SOURCE_DATE_EPOCH` sets for the header, or `None` when it is
/// unset or empty (reproducible-builds.org).
fn source_date_epoch() -> anyhow::Result<Option<i64>> {
    match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| anyhow::anyhow!("SOURCE_DATE_EPOCH must be Unix seconds, not {value:?}")),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(value)) => Err(anyhow::anyhow!(
            "SOURCE_DATE_EPOCH must be Unix seconds, not {value:?}"
        )),
    }
}

fn write_diagnostics_sidecar(
    path: &std::path::Path,
    diagnostics: &ExportDiagnostics,
    anonymize: bool,
) -> anyhow::Result<()> {
    let json = if anonymize {
        let mut anonymized = diagnostics.clone();
        anonymized.anonymize();
        serde_json::to_vec_pretty(&anonymized)?
    } else {
        serde_json::to_vec_pretty(diagnostics)?
    };
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
    let input_hash = lower_hex(&Sha256::digest(&bytes));
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
        "relations": relation_pairs(step),
        "storeys": storey_set(step),
        "exported_building_elements": diagnostics.exported.by_ifc_type,
        "storey_count": diagnostics.exported.storey_count,
        "material_count": diagnostics.exported.material_count,
        "building_elements_with_geometry": diagnostics.exported.building_elements_with_geometry,
    });
    let canonical = serde_json::to_string(&payload)?;
    let observation = serde_json::json!({
        "schema_version": "1.1.0",
        "witness_id": "rvt-rs",
        "witness_version": env!("CARGO_PKG_VERSION"),
        "artifact_id": artifact_id,
        "input_role": "source",
        "input_file": input.file_name().and_then(|n| n.to_str()),
        "input_hash_sha256": input_hash,
        "deterministic": true,
        "semantic_surface_covered": ["entity_counts", "relations", "storeys"],
        "observation": payload,
        "observation_hash_sha256": lower_hex(&Sha256::digest(canonical.as_bytes())),
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

/// Attribute index of `Tag` on every `IfcElement` subtype in IFC4.
///
/// `IfcElement` adds exactly one attribute to `IfcProduct`'s seven
/// (`GlobalId`, `OwnerHistory`, `Name`, `Description`, `ObjectType`,
/// `ObjectPlacement`, `Representation`), so the index is the same for
/// `IfcWall`, `IfcDoor`, `IfcWindow` and every other element type.
const IFC_ELEMENT_TAG_INDEX: usize = 7;

/// One STEP instance: its type keyword and its top-level attributes.
struct StepInstance<'a> {
    ifc_type: &'a str,
    attributes: Vec<&'a str>,
}

/// Split a STEP attribute list on top-level commas.
///
/// Commas inside a nested list or a quoted string do not separate
/// attributes. STEP escapes an apostrophe by doubling it, which this
/// scanner handles for free: the pair closes and immediately reopens
/// the string with nothing between the two quotes.
fn split_attributes(args: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut in_string = false;
    let mut start = 0usize;
    for (index, ch) in args.char_indices() {
        match ch {
            '\'' => in_string = !in_string,
            '(' if !in_string => depth += 1,
            ')' if !in_string => depth = depth.saturating_sub(1),
            ',' if !in_string && depth == 0 => {
                out.push(args[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    out.push(args[start..].trim());
    out
}

/// Index the STEP data section by instance id. One instance per line
/// — the same assumption the `entity_counts` scan above makes, and
/// the shape `ifc::write_step` emits.
fn parse_step_instances(step: &str) -> std::collections::BTreeMap<u32, StepInstance<'_>> {
    let mut out = std::collections::BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id_text, after_eq)) = rest.split_once('=') else {
            continue;
        };
        let Ok(id) = id_text.trim().parse::<u32>() else {
            continue;
        };
        let after_eq = after_eq.trim_start();
        let Some((ifc_type, args)) = after_eq.split_once('(') else {
            continue;
        };
        let Some(args) = args.trim_end().strip_suffix(';').and_then(|a| {
            let a = a.trim_end();
            a.strip_suffix(')')
        }) else {
            continue;
        };
        out.insert(
            id,
            StepInstance {
                ifc_type: ifc_type.trim(),
                attributes: split_attributes(args),
            },
        );
    }
    out
}

/// Resolve `#123` to its instance id.
fn entity_ref(attribute: &str) -> Option<u32> {
    attribute.trim().strip_prefix('#')?.trim().parse().ok()
}

/// Decode a STEP string literal, inverting `ifc::step_writer::escape`.
///
/// Returns `None` for `$` or anything that is not a quoted literal —
/// an unset `Tag` is not a string.
fn step_string(attribute: &str) -> Option<String> {
    let body = attribute.trim().strip_prefix('\'')?.strip_suffix('\'')?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            // Doubled apostrophe — consume the second half.
            if chars.peek() == Some(&'\'') {
                chars.next();
            }
            out.push('\'');
            continue;
        }
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('X') | Some('x') => {
                let width = match chars.peek() {
                    Some('\\') => {
                        chars.next();
                        2
                    }
                    Some('2') => {
                        chars.next();
                        chars.next();
                        4
                    }
                    Some('4') => {
                        chars.next();
                        chars.next();
                        8
                    }
                    _ => return None,
                };
                loop {
                    let mut digits = String::with_capacity(width);
                    for _ in 0..width {
                        digits.push(chars.next()?);
                    }
                    let code = u32::from_str_radix(&digits, 16).ok()?;
                    out.push(char::from_u32(code)?);
                    if width == 2 {
                        break;
                    }
                    // `\X2\` / `\X4\` runs continue until the `\X0\`
                    // terminator the writer always emits.
                    if chars.peek() == Some(&'\\') {
                        chars.next();
                        chars.next();
                        chars.next();
                        chars.next();
                        break;
                    }
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

/// `IfcRelFillsElement` host/filling `Tag` pairs, canonically sorted.
///
/// The chain is Revit's own: `IfcRelVoidsElement` binds an opening to
/// the element it voids, `IfcRelFillsElement` binds that opening to
/// the element that fills it, so the pair `[host Tag, filling Tag]`
/// is the door/window ↔ host-wall relation as an IFC reader sees it
/// (OctetProof §7.2, field class *relation pair sets*).
///
/// A `Tag` that is unset, or an opening with no `IfcRelVoidsElement`,
/// yields an empty string rather than dropping the pair: a missing
/// half must surface as a disagreement, never as a silent omission.
/// Duplicates are kept, so the value is a sorted multiset.
fn relation_pairs(step: &str) -> std::collections::BTreeMap<String, Vec<Vec<String>>> {
    let instances = parse_step_instances(step);
    let tag_of = |id: Option<u32>| -> String {
        id.and_then(|id| instances.get(&id))
            .and_then(|instance| instance.attributes.get(IFC_ELEMENT_TAG_INDEX))
            .and_then(|attribute| step_string(attribute))
            .unwrap_or_default()
    };
    let mut voided_by: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
    for instance in instances.values() {
        if !instance.ifc_type.eq_ignore_ascii_case("IFCRELVOIDSELEMENT") {
            continue;
        }
        // (GlobalId, OwnerHistory, Name, Description,
        //  RelatingBuildingElement, RelatedOpeningElement)
        if let (Some(host), Some(opening)) = (
            instance.attributes.get(4).and_then(|a| entity_ref(a)),
            instance.attributes.get(5).and_then(|a| entity_ref(a)),
        ) {
            voided_by.insert(opening, host);
        }
    }
    let mut pairs: Vec<Vec<String>> = Vec::new();
    for instance in instances.values() {
        if !instance.ifc_type.eq_ignore_ascii_case("IFCRELFILLSELEMENT") {
            continue;
        }
        // (GlobalId, OwnerHistory, Name, Description,
        //  RelatingOpeningElement, RelatedBuildingElement)
        let opening = instance.attributes.get(4).and_then(|a| entity_ref(a));
        let filling = instance.attributes.get(5).and_then(|a| entity_ref(a));
        let host = opening.and_then(|id| voided_by.get(&id).copied());
        pairs.push(vec![tag_of(host), tag_of(filling)]);
    }
    pairs.sort();
    let mut out = std::collections::BTreeMap::new();
    out.insert("IFCRELFILLSELEMENT".to_string(), pairs);
    out
}

/// Attribute index of `Name` on `IfcBuildingStorey` (`IfcRoot` +2).
const IFC_BUILDING_STOREY_NAME_INDEX: usize = 2;

/// Attribute index of `Elevation` on `IfcBuildingStorey`: the tenth and
/// last attribute of the IFC4 entity.
const IFC_BUILDING_STOREY_ELEVATION_INDEX: usize = 9;

/// Metres in one international foot, exactly.
const METRES_PER_FOOT: f64 = 0.3048;

/// Parse a STEP `REAL`/`INTEGER` literal.
fn step_number(attribute: &str) -> Option<f64> {
    attribute.trim().parse::<f64>().ok()
}

/// Multiplier that takes this model's declared `LENGTHUNIT` to metres.
///
/// Walks `IfcProject.UnitsInContext` → `IfcUnitAssignment.Units` and
/// resolves the first length unit: an `IfcSIUnit` contributes its SI
/// prefix, an `IfcConversionBasedUnit` its
/// `IfcMeasureWithUnit.ValueComponent` times the SI unit that measure
/// is expressed in. `None` when the model declares no length unit —
/// the caller must not guess, because a wrong scale silently moves
/// every storey.
fn length_unit_scale_to_metres(
    instances: &std::collections::BTreeMap<u32, StepInstance<'_>>,
) -> Option<f64> {
    fn si_prefix_multiplier(prefix: &str) -> f64 {
        match prefix
            .trim()
            .trim_matches('.')
            .to_ascii_uppercase()
            .as_str()
        {
            "EXA" => 1e18,
            "PETA" => 1e15,
            "TERA" => 1e12,
            "GIGA" => 1e9,
            "MEGA" => 1e6,
            "KILO" => 1e3,
            "HECTO" => 1e2,
            "DECA" => 1e1,
            "DECI" => 1e-1,
            "CENTI" => 1e-2,
            "MILLI" => 1e-3,
            "MICRO" => 1e-6,
            "NANO" => 1e-9,
            "PICO" => 1e-12,
            "FEMTO" => 1e-15,
            "ATTO" => 1e-18,
            _ => 1.0,
        }
    }
    fn enum_value(attribute: &str) -> String {
        attribute.trim().trim_matches('.').to_ascii_uppercase()
    }
    fn si_scale(instance: &StepInstance<'_>) -> Option<f64> {
        if !instance.ifc_type.eq_ignore_ascii_case("IFCSIUNIT") {
            return None;
        }
        Some(match instance.attributes.get(2) {
            Some(prefix) if prefix.trim() != "$" => si_prefix_multiplier(prefix),
            _ => 1.0,
        })
    }

    let project = instances
        .values()
        .find(|instance| instance.ifc_type.eq_ignore_ascii_case("IFCPROJECT"))?;
    let assignment = instances.get(&entity_ref(project.attributes.get(8)?)?)?;
    let units = assignment.attributes.first()?.trim();
    let units = units.strip_prefix('(')?.strip_suffix(')')?;
    for reference in split_attributes(units) {
        let Some(unit) = entity_ref(reference).and_then(|id| instances.get(&id)) else {
            continue;
        };
        let Some(unit_type) = unit.attributes.get(1).map(|a| enum_value(a)) else {
            continue;
        };
        if unit_type != "LENGTHUNIT" {
            continue;
        }
        if let Some(scale) = si_scale(unit) {
            return Some(scale);
        }
        if unit.ifc_type.eq_ignore_ascii_case("IFCCONVERSIONBASEDUNIT") {
            let measure = instances.get(&entity_ref(unit.attributes.get(3)?)?)?;
            let value = measure.attributes.first()?.trim();
            let ratio = value
                .split_once('(')
                .and_then(|(_, rest)| rest.strip_suffix(')'))
                .and_then(step_number)
                .or_else(|| step_number(value))?;
            let base = instances.get(&entity_ref(measure.attributes.get(1)?)?)?;
            return Some(ratio * si_scale(base)?);
        }
    }
    None
}

/// `IfcBuildingStorey` `[Name, Elevation]` pairs, canonically sorted.
///
/// The elevation is converted from the model's declared `LENGTHUNIT`
/// to feet and rendered with six decimals, so the field compares
/// across witnesses whose files declare different units — Revit's own
/// export of this corpus declares `FOOT` while rvt-rs writes `METRE`
/// (OctetProof §7.2, field class *storey sets*). The name is the
/// decoded `Name` string verbatim; an unset `Name` contributes an
/// empty string rather than dropping the storey, so a missing half
/// surfaces as a disagreement and never as a silent omission.
///
/// An empty list is returned when the model declares no length unit:
/// a storey whose unit is unknown is not a measurement.
fn storey_set(step: &str) -> std::collections::BTreeMap<String, Vec<Vec<String>>> {
    let instances = parse_step_instances(step);
    let mut storeys: Vec<Vec<String>> = Vec::new();
    if let Some(scale) = length_unit_scale_to_metres(&instances) {
        for instance in instances.values() {
            if !instance.ifc_type.eq_ignore_ascii_case("IFCBUILDINGSTOREY") {
                continue;
            }
            let name = instance
                .attributes
                .get(IFC_BUILDING_STOREY_NAME_INDEX)
                .and_then(|attribute| step_string(attribute))
                .unwrap_or_default();
            let elevation = instance
                .attributes
                .get(IFC_BUILDING_STOREY_ELEVATION_INDEX)
                .and_then(|attribute| step_number(attribute));
            let feet = match elevation {
                Some(value) => format_elevation_feet(value * scale / METRES_PER_FOOT),
                None => String::new(),
            };
            storeys.push(vec![name, feet]);
        }
    }
    storeys.sort();
    let mut out = std::collections::BTreeMap::new();
    out.insert("IFCBUILDINGSTOREY".to_string(), storeys);
    out
}

/// Render an elevation in feet at 1e-6 ft, as a string.
///
/// A string, not a JSON number, because the canonical form
/// (OctetProof §7.3) is defined over integers and strings only — two
/// runtimes must not be trusted to print the same `f64` the same way.
/// `-0` is normalised to `0`.
fn format_elevation_feet(feet: f64) -> String {
    if !feet.is_finite() {
        return String::new();
    }
    let rendered = format!("{feet:.6}");
    if rendered == "-0.000000" {
        return "0.000000".to_string();
    }
    rendered
}

/// How many IFC types the summary names before "and N more".
const SUMMARY_TYPES: usize = 6;

/// Whether `path` names a family (`.rfa`) or a family template (`.rft`), whose
/// elements are read from `Global/Latest` and not from the project element
/// records.
fn is_family_file(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            ["rfa", "rft"]
                .iter()
                .any(|family| extension.eq_ignore_ascii_case(family))
        })
}

/// A few lines saying what the IFC holds: building elements by IFC type,
/// what was left out on purpose, and the readiness level.
///
/// `family` is whether the input is a family file, for which the element
/// records of a project say nothing.
fn export_summary(diagnostics: &ExportDiagnostics, has_sidecar: bool, family: bool) -> Vec<String> {
    let mut lines = Vec::new();
    let exported = &diagnostics.exported;
    if exported.building_elements > 0 {
        let carried = match exported.building_elements_carried_by_parts {
            0 => String::new(),
            n => format!(" and {n} carried by their parts"),
        };
        lines.push(format!(
            "rvt-ifc: {} building element(s), {} with geometry{carried}, on {} storey(s)",
            exported.building_elements,
            exported.building_elements_with_geometry,
            exported.storey_count
        ));
        let mut types: Vec<(&String, &usize)> = exported.by_ifc_type.iter().collect();
        types.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        let mut named: Vec<String> = types
            .iter()
            .take(SUMMARY_TYPES)
            .map(|(name, count)| format!("{count} {}", ifc_type_display(name)))
            .collect();
        if types.len() > SUMMARY_TYPES {
            named.push(format!("and {} more type(s)", types.len() - SUMMARY_TYPES));
        }
        lines.push(format!("  {}", named.join(", ")));
        // #409: say how much of the model is a stand-in box.
        if exported.bounding_box_bodies > 0 {
            lines.push(format!(
                "  {} of the bodies are their element record's bounding box: placed and sized, shape not decoded",
                exported.bounding_box_bodies
            ));
        }
        // RE-313: say how many spaces are a box, as their area is the box's.
        if exported.bounding_box_spaces > 0 {
            lines.push(format!(
                "  {} of {} spaces are their record's box: the area of their body is the box's, not the space's (SpaceOutlineSource record_box)",
                exported.bounding_box_spaces,
                exported.by_ifc_type.get("IFCSPACE").copied().unwrap_or(0)
            ));
        }
        // RE-316: rooms read from their stored solid by a rule that is not
        // the exact one.
        if exported.resolved_outline_spaces > 0 {
            lines.push(format!(
                "  {} rooms take an outline read from the faces their stored solid lists, where the exact walk did not close them (SpaceOutlineSource stored_solid_faces)",
                exported.resolved_outline_spaces
            ));
        }
        // RE-374 (#90): rooms read from their solid's edges.
        if exported.edge_outline_spaces > 0 {
            lines.push(format!(
                "  {} rooms take the outline of their stored solid's floor, read from its edges where neither the walk nor the span closed it (SpaceOutlineSource stored_solid_edges)",
                exported.edge_outline_spaces
            ));
        }
        if exported.approximate_outline_spaces > 0 {
            lines.push(format!(
                "  {} rooms take an approximate outline, a column that overlaps a wall taken off it or up to 0.02 ft short of their record box on a side: the area is within about 1% of the space's, not 0.1% (SpaceOutlineSource stored_solid_approximate)",
                exported.approximate_outline_spaces
            ));
        }
        // #1437: a space that says nothing of what it is.
        if exported.spaces_without_kind > 0 {
            lines.push(format!(
                "  {} of {} spaces have no SpaceKind: their property set is not RvtElementRecordGeometry, so the export does not say what they are",
                exported.spaces_without_kind,
                exported.by_ifc_type.get("IFCSPACE").copied().unwrap_or(0)
            ));
        }
    }
    let left_out: Vec<String> = diagnostics
        .skipped
        .iter()
        .filter_map(|item| {
            let what = match item.reason.as_str() {
                "element_record_without_volume" => "without a 3D body",
                "element_record_in_non_primary_design_option" => "in non-primary design options",
                "element_record_empty_curtain_panel" => "empty curtain panels",
                "room_not_in_export_phase" => "rooms in a phase other than the export phase",
                "element_record_family_type_draws_nothing" => {
                    "plumbing fixtures whose type draws nothing visible"
                }
                "element_record_family_space_without_body" => {
                    "family instances written as spaces whose family draws no model form"
                }
                _ => return None,
            };
            Some(format!("{} {what}", item.count))
        })
        .collect();
    if !left_out.is_empty() {
        lines.push(format!(
            "  left out as Revit's own export leaves them out: {}",
            left_out.join(", ")
        ));
    }
    // #1389: a Revit 2008 to 2023 project none of whose element records was
    // written says where they stopped, on a line of its own (it can be the only
    // line, when nothing else was exported). A family file reads its elements
    // from `Global/Latest`, so for one it says nothing.
    if let Some(stages) = diagnostics
        .decoded
        .element_record_stages
        .as_ref()
        .filter(|_| !family)
    {
        if let Some(stopped) = &stages.stopped_at {
            lines.push(format!(
                "rvt-ifc: no element record of this Revit {} layout was written: {} marker(s) found, they stopped at {stopped} (decoded.element_record_stages in the diagnostics)",
                stages.layout_release, stages.total.markers
            ));
        }
    }
    // A scaffold-only export already warned with the same advice.
    let hint = if has_sidecar || diagnostics.confidence.level == "scaffold" {
        ""
    } else {
        ". Add --diagnostics <path> for the full report"
    };
    lines.push(format!(
        "rvt-ifc: readiness {} (score {:.2}){hint}",
        diagnostics.confidence.level, diagnostics.confidence.score
    ));
    lines
}

/// `IFCWALL` as `IfcWall`, the spelling IFC documentation and viewers use.
/// A type outside the list keeps the STEP spelling.
fn ifc_type_display(upper: &str) -> String {
    const NAMES: &[&str] = &[
        "IfcAirTerminal",
        "IfcAlarm",
        "IfcBeam",
        "IfcBuildingElementProxy",
        "IfcCableCarrierFitting",
        "IfcCableCarrierSegment",
        "IfcColumn",
        "IfcCovering",
        "IfcCurtainWall",
        "IfcDoor",
        "IfcDuctFitting",
        "IfcDuctSegment",
        "IfcElectricAppliance",
        "IfcFlowController",
        "IfcFooting",
        "IfcFurniture",
        "IfcLightFixture",
        "IfcMember",
        "IfcPipeFitting",
        "IfcPipeSegment",
        "IfcPlate",
        "IfcRailing",
        "IfcRamp",
        "IfcReinforcingBar",
        "IfcRoof",
        "IfcSanitaryTerminal",
        "IfcShadingDevice",
        "IfcSlab",
        "IfcSpace",
        "IfcStair",
        "IfcStairFlight",
        "IfcSystemFurnitureElement",
        "IfcTransportElement",
        "IfcWall",
        "IfcWindow",
    ];
    NAMES
        .iter()
        .find(|name| name.eq_ignore_ascii_case(upper))
        .map_or_else(|| upper.to_string(), |name| (*name).to_string())
}

fn warn_about_export_quality(diagnostics: &ExportDiagnostics) {
    if diagnostics.confidence.level == "scaffold" {
        eprintln!(
            "warning: export confidence is scaffold-only; no validated building elements were exported. \
             Re-run with `--diagnostics <path>` for a shareable readiness report."
        );
    }
    let unexported = diagnostics.confidence.unexported_element_records;
    if unexported > 0 {
        eprintln!(
            "warning: the IFC is incomplete: {unexported} model element record(s) \
             use a layout whose ElementId this release cannot locate and were not exported (RE-30)."
        );
    }
}

/// `bytes` as lowercase hex, two digits a byte.
fn lower_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
