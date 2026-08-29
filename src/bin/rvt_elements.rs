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
    iter_elements_with_limits,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-elements",
    version,
    about = "Dump production decoded elements or class counts as JSON"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,

    /// Emit `{total, by_class}` instead of the full element list.
    #[arg(long)]
    counts: bool,

    /// Omit the Lane Five MVP `typed` projection on each element.
    #[arg(long)]
    no_typed: bool,
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
    /// AProperty* values when this element is a value carrier; empty
    /// for host elements until host↔parameter joins recover (#35).
    parameters: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    typed: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct CountsOut {
    total: usize,
    by_class: BTreeMap<String, usize>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut rf = RevitFile::open(&cli.file)?;
    let elements: Vec<DecodedElement> = iter_elements_with_limits(
        &mut rf,
        PRODUCTION_ELEMENT_MIN_SCORE,
        WalkerLimits::default(),
    )?
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
                    .collect(),
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
    }
}
