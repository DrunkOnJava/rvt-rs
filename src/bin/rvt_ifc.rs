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

fn warn_about_export_quality(diagnostics: &ExportDiagnostics) {
    if diagnostics.confidence.level == "scaffold" {
        eprintln!(
            "warning: export confidence is scaffold-only; no validated building elements were exported. \
             Re-run with `--diagnostics <path>` for a shareable readiness report."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_split_on_top_level_commas_only() {
        let attrs = split_attributes("'a,b',#3,(1,2,3),$,.OPENING.");
        assert_eq!(attrs, vec!["'a,b'", "#3", "(1,2,3)", "$", ".OPENING."]);
    }

    #[test]
    fn a_doubled_apostrophe_does_not_end_the_string() {
        let attrs = split_attributes("'it''s, one',#3");
        assert_eq!(attrs, vec!["'it''s, one'", "#3"]);
        assert_eq!(step_string(attrs[0]).as_deref(), Some("it's, one"));
    }

    #[test]
    fn step_strings_decode_the_writers_escapes() {
        assert_eq!(step_string("'20827'").as_deref(), Some("20827"));
        assert_eq!(step_string("'a\\\\b'").as_deref(), Some("a\\b"));
        assert_eq!(step_string("'\\X\\09'").as_deref(), Some("\t"));
        assert_eq!(step_string("'\\X2\\00E9\\X0\\'").as_deref(), Some("é"));
        assert_eq!(step_string("$"), None);
    }

    #[test]
    fn relation_pairs_follow_the_void_fill_chain() {
        let step = concat!(
            "#1=IFCWALL('g',#2,'Wall-360',$,$,#3,$,'360',.NOTDEFINED.);\n",
            "#4=IFCDOOR('g',#2,'Door-42',$,$,#3,$,'42',$,$,.DOOR.,$,$);\n",
            "#5=IFCOPENINGELEMENT('g',#2,'Opening for Door-42',$,$,#3,$,'42',.OPENING.);\n",
            "#6=IFCRELVOIDSELEMENT('g',#2,$,$,#1,#5);\n",
            "#7=IFCRELFILLSELEMENT('g',#2,$,$,#5,#4);\n",
        );
        let pairs = relation_pairs(step);
        assert_eq!(
            pairs.get("IFCRELFILLSELEMENT"),
            Some(&vec![vec!["360".to_string(), "42".to_string()]])
        );
    }

    #[test]
    fn an_unvoided_opening_still_yields_a_pair_with_an_empty_host() {
        let step = concat!(
            "#4=IFCDOOR('g',#2,'Door-42',$,$,#3,$,'42',$,$,.DOOR.,$,$);\n",
            "#5=IFCOPENINGELEMENT('g',#2,'Opening',$,$,#3,$,'42',.OPENING.);\n",
            "#7=IFCRELFILLSELEMENT('g',#2,$,$,#5,#4);\n",
        );
        let pairs = relation_pairs(step);
        assert_eq!(
            pairs.get("IFCRELFILLSELEMENT"),
            Some(&vec![vec![String::new(), "42".to_string()]])
        );
    }

    #[test]
    fn a_model_without_openings_declares_an_empty_relation_set() {
        let pairs = relation_pairs("#1=IFCWALL('g',#2,'Wall-1',$,$,#3,$,'1',.NOTDEFINED.);\n");
        assert_eq!(pairs.get("IFCRELFILLSELEMENT"), Some(&Vec::new()));
    }

    /// A metric model: the elevation is already in metres, so feet is
    /// the raw value divided by 0.3048.
    #[test]
    fn storeys_are_reported_in_feet_from_an_si_model() {
        let step = concat!(
            "#6=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
            "#24=IFCUNITASSIGNMENT((#6));\n",
            "#30=IFCPROJECT('g',#5,'p',$,$,$,$,(#29),#24);\n",
            "#36=IFCBUILDINGSTOREY('g',#5,'Basement 2',$,$,#35,$,'Basement 2',.ELEMENT.,-12.192000);\n",
            "#40=IFCBUILDINGSTOREY('g',#5,'Level 1',$,$,#39,$,'Level 1',.ELEMENT.,0.000000);\n",
        );
        assert_eq!(
            storey_set(step).get("IFCBUILDINGSTOREY"),
            Some(&vec![
                vec!["Basement 2".to_string(), "-40.000000".to_string()],
                vec!["Level 1".to_string(), "0.000000".to_string()],
            ])
        );
    }

    /// Revit's own export of an imperial project: `FOOT` as an
    /// `IfcConversionBasedUnit` over `METRE`, so the raw `Elevation`
    /// is already in feet and must come back unchanged.
    #[test]
    fn a_conversion_based_foot_model_reports_the_same_feet() {
        let step = concat!(
            "#19=IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.);\n",
            "#21=IFCMEASUREWITHUNIT(IFCRATIOMEASURE(0.30480000000000002),#19);\n",
            "#22=IFCCONVERSIONBASEDUNIT(#20,.LENGTHUNIT.,'FOOT',#21);\n",
            "#92=IFCUNITASSIGNMENT((#22));\n",
            "#100=IFCPROJECT('g',#18,'',$,$,'','',(#95),#92);\n",
            "#109=IFCBUILDINGSTOREY('g',#18,'Basement 2',$,'Level:Level 1',#108,$,'Basement 2',.ELEMENT.,-40.);\n",
            "#164=IFCBUILDINGSTOREY('g',#18,'Level 13',$,'Level:Level 1',#163,$,'Level 13',.ELEMENT.,185.5);\n",
        );
        assert_eq!(
            storey_set(step).get("IFCBUILDINGSTOREY"),
            Some(&vec![
                vec!["Basement 2".to_string(), "-40.000000".to_string()],
                vec!["Level 13".to_string(), "185.500000".to_string()],
            ])
        );
    }

    /// A model with no resolvable length unit is not a measurement:
    /// the storey set is empty rather than silently metres.
    #[test]
    fn a_model_without_a_length_unit_reports_no_storeys() {
        let step = concat!(
            "#30=IFCPROJECT('g',#5,'p',$,$,$,$,(#29),$);\n",
            "#36=IFCBUILDINGSTOREY('g',#5,'Level 1',$,$,#35,$,'Level 1',.ELEMENT.,0.);\n",
        );
        assert_eq!(storey_set(step).get("IFCBUILDINGSTOREY"), Some(&Vec::new()));
    }

    #[test]
    fn negative_zero_normalises() {
        assert_eq!(format_elevation_feet(-0.0), "0.000000");
        assert_eq!(format_elevation_feet(f64::NAN), "");
    }
}
