//! `rvt-elements` — dump production decoded elements / class counts.
//!
//! Mirrors the Python `RevitFile.decoded_elements()` /
//! `RevitFile.element_counts()` surfaces so CLI and Python share one
//! JSON contract (`docs/schemas/decoded-elements.schema.json`,
//! `docs/schemas/element-counts.schema.json`).

use clap::Parser;
use rvt::RevitFile;
use rvt::elements::typed_json::mvp_typed_view;
use rvt::walker::{
    DecodedElement, InstanceField, PRODUCTION_ELEMENT_MIN_SCORE, WalkerLimits,
    iter_elements_with_control,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-elements",
    version,
    about = "Dump production decoded elements or class counts as JSON",
    after_help = "Examples:\n  \
        rvt-elements model.rvt --counts\n  \
        rvt-elements model.rvt --min-confidence 0.55 > elements.json"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,

    /// Emit `{total, by_class}` instead of the full element list.
    #[arg(long)]
    counts: bool,

    /// Omit the `typed` projection (the typed view of the decoded classes) on each element.
    #[arg(long)]
    no_typed: bool,

    /// Hide elements whose provenance confidence is below this floor
    ///. Default keeps everything; pass `0.55` to match the
    /// viewer/export low-confidence filter.
    // Tracker: M3-07.
    #[arg(long, default_value_t = 0.0)]
    min_confidence: f32,

    /// Print scan progress to stderr (`progress: <stage> <done>/<total>`).
    /// Useful on large project files where the walker runs for a while.
    #[arg(long)]
    progress: bool,
}

#[derive(Serialize)]
struct ByteRange {
    start: usize,
    end: usize,
}

#[derive(Serialize)]
struct ElementOut {
    id: Option<u32>,
    class_name: String,
    byte_range: ByteRange,
    fields: Vec<serde_json::Value>,
    /// AProperty* values when this element is a value carrier, then the
    /// saved parameters of its native record and its type's (#35, RE-207).
    parameters: Vec<serde_json::Value>,
    /// Decode confidence and provenance.
    provenance: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    typed: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct CountsOut {
    total: usize,
    by_class: BTreeMap<String, usize>,
}

fn main() -> std::process::ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(&cli.file)?;
    let control = if cli.progress {
        rvt::control::WalkerControl::new().with_progress(|event| {
            let total = event.total.map(|t| format!("/{t}")).unwrap_or_default();
            eprintln!("progress: {} {}{total}", event.stage.as_str(), event.done);
        })
    } else {
        rvt::control::WalkerControl::new()
    };
    let elements: Vec<DecodedElement> = iter_elements_with_control(
        &mut rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
        &control,
    )?
    .filter(|e| e.meets_confidence(cli.min_confidence))
    .collect();

    if cli.counts {
        let mut by_class = BTreeMap::<String, usize>::new();
        for element in &elements {
            *by_class.entry(element.class.clone()).or_insert(0) += 1;
        }
        let out = CountsOut {
            total: elements.len(),
            by_class,
        };
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }

    // Revit 2015 to 2027 only; other files keep their AProperty* entries.
    let saved = rvt::elements::parameters::saved_parameters_lossy(&mut rf, &elements);
    for warning in &saved.diagnostics.warnings {
        eprintln!("warning: {}", warning.message);
    }
    let saved = saved.value;

    let out: Vec<ElementOut> = elements
        .iter()
        .map(|element| {
            let typed = if cli.no_typed {
                None
            } else {
                mvp_typed_view(element)
            };
            ElementOut {
                id: element.id,
                class_name: element.class.clone(),
                byte_range: ByteRange {
                    start: element.byte_range.start,
                    end: element.byte_range.end,
                },
                fields: element
                    .fields
                    .iter()
                    .map(|(name, value)| field_json(name, value))
                    .collect(),
                parameters: rvt::elements::parameters::parameter_entries_from_decoded(element)
                    .iter()
                    .map(|e| e.to_json_value())
                    .chain(saved.entries_for(element))
                    .collect(),
                provenance: element.provenance.to_json_value(),
                typed,
            }
        })
        .collect();
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

fn field_json(name: &str, value: &InstanceField) -> serde_json::Value {
    match value {
        InstanceField::Pointer { raw } => serde_json::json!({
            "name": name,
            "kind": "pointer",
            "slot_a": raw[0],
            "slot_b": raw[1],
        }),
        InstanceField::ElementId { tag, id } => serde_json::json!({
            "name": name,
            "kind": "element_id",
            "tag": tag,
            "id": id,
        }),
        InstanceField::RefContainer { col_a, col_b } => serde_json::json!({
            "name": name,
            "kind": "ref_container",
            "count": col_a.len(),
            "col_a": col_a,
            "col_b": col_b,
        }),
        InstanceField::Integer {
            value,
            signed,
            size,
        } => serde_json::json!({
            "name": name,
            "kind": "integer",
            "value": value,
            "signed": signed,
            "size": size,
        }),
        InstanceField::Float { value, size } => serde_json::json!({
            "name": name,
            "kind": "float",
            "value": value,
            "size": size,
        }),
        InstanceField::Bool(v) => serde_json::json!({
            "name": name,
            "kind": "bool",
            "value": v,
        }),
        InstanceField::Guid(bytes) => serde_json::json!({
            "name": name,
            "kind": "guid",
            "bytes": bytes.as_slice(),
        }),
        InstanceField::String(s) => serde_json::json!({
            "name": name,
            "kind": "string",
            "value": s,
        }),
        InstanceField::Vector(items) => serde_json::json!({
            "name": name,
            "kind": "vector",
            "len": items.len(),
        }),
        InstanceField::Bytes(b) => serde_json::json!({
            "name": name,
            "kind": "bytes",
            "len": b.len(),
        }),
        _ => serde_json::json!({ "name": name, "kind": "unknown" }),
    }
}
