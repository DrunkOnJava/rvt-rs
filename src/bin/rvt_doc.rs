//! `rvt-doc` — dump ADocument's instance fields as JSON or text.
//!
//! This is the first end-user-facing command that reads actual
//! instance data out of a Revit file (not just schema or metadata).
//! Coverage is currently limited: reliable on Revit 2024–2026; older
//! releases return "ADocument record not locatable" until the
//! entry-point detector is extended (see `docs/rvt-moat-break-
//! reconnaissance.md` §Q6.5).

use clap::Parser;
use rvt::walker::{ADocumentInstance, InstanceField, read_adocument};
use rvt::{RevitFile, redact};
use serde::Serialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Dump ADocument's instance fields from a Revit file")]
struct Args {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    path: PathBuf,
    /// Emit JSON instead of human-readable text.
    #[arg(long)]
    json: bool,
    /// Scrub usernames and Autodesk-internal paths from any string
    /// fields surfaced.
    #[arg(long)]
    redact: bool,
}

#[derive(Serialize)]
struct OutField {
    name: String,
    kind: String,
    value: serde_json::Value,
}

#[derive(Serialize)]
struct Output {
    path: String,
    version: u32,
    adocument_entry_offset: Option<String>,
    fields: Vec<OutField>,
    notes: Vec<String>,
}

fn render(inst: &ADocumentInstance) -> Vec<OutField> {
    inst.fields
        .iter()
        .map(|(name, v)| match v {
            InstanceField::Pointer { raw } => OutField {
                name: name.clone(),
                kind: "pointer".into(),
                value: serde_json::json!({ "slot_a": raw[0], "slot_b": raw[1] }),
            },
            InstanceField::ElementId { tag, id } => OutField {
                name: name.clone(),
                kind: "element_id".into(),
                value: serde_json::json!({ "tag": tag, "id": id }),
            },
            InstanceField::RefContainer { col_a, col_b } => OutField {
                name: name.clone(),
                kind: "ref_container".into(),
                value: serde_json::json!({
                    "count": col_a.len(),
                    "col_a": col_a,
                    "col_b": col_b,
                }),
            },
            InstanceField::Bytes(b) => OutField {
                name: name.clone(),
                kind: "bytes".into(),
                value: serde_json::json!({ "len": b.len() }),
            },
        })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let path_str = if args.redact {
        redact::redact_path_str(&args.path.display().to_string())
    } else {
        args.path.display().to_string()
    };

    let mut rf = RevitFile::open(&args.path)?;
    let maybe_inst = read_adocument(&mut rf)?;
    let version = rf.basic_file_info().ok().map(|b| b.version).unwrap_or(0);

    let (entry, fields, notes) = match maybe_inst {
        Some(inst) => {
            let e = format!("0x{:06x}", inst.entry_offset);
            let f = render(&inst);
            let mut n = Vec::new();
            if !(2024..=2026).contains(&inst.version) {
                n.push(
                    "Walker currently validated on Revit 2024–2026. \
                     Values for this release are experimental."
                        .into(),
                );
            }
            (Some(e), f, n)
        }
        None => (
            None,
            Vec::new(),
            vec![
                "ADocument record not locatable in this release's stream \
                 layout. This is expected for Revit 2016–2023 today \
                 (see §Q6.5 of docs/rvt-moat-break-reconnaissance.md \
                 for current status)."
                    .into(),
            ],
        ),
    };

    let out = Output {
        path: path_str,
        version,
        adocument_entry_offset: entry,
        fields,
        notes,
    };

    if args.json {
        serde_json::to_writer_pretty(std::io::stdout(), &out)?;
        println!();
    } else {
        println!("path:              {}", out.path);
        println!("version:           {}", out.version);
        if let Some(e) = &out.adocument_entry_offset {
            println!("adocument entry:   {}", e);
        } else {
            println!("adocument entry:   <not located>");
        }
        println!();
        for f in &out.fields {
            match f.kind.as_str() {
                "pointer" => {
                    let a = f.value["slot_a"].as_u64().unwrap_or(0);
                    let b = f.value["slot_b"].as_u64().unwrap_or(0);
                    println!(
                        "  {:36} :: Pointer    [a=0x{:08x}, b=0x{:08x}]",
                        f.name, a, b
                    );
                }
                "element_id" => {
                    let tag = f.value["tag"].as_u64().unwrap_or(0);
                    let id = f.value["id"].as_u64().unwrap_or(0);
                    println!("  {:36} :: ElementId  [tag={}, id={}]", f.name, tag, id);
                }
                "ref_container" => {
                    let count = f.value["count"].as_u64().unwrap_or(0);
                    let col_a = f.value["col_a"].as_array().map(|a| a.len()).unwrap_or(0);
                    let col_b = f.value["col_b"].as_array().map(|a| a.len()).unwrap_or(0);
                    println!(
                        "  {:36} :: Container  [count={count}, col_a={col_a}, col_b={col_b}]",
                        f.name
                    );
                }
                _ => {
                    println!("  {:36} :: {}", f.name, f.kind);
                }
            }
        }
        if !out.notes.is_empty() {
            println!();
            for n in &out.notes {
                println!("note: {n}");
            }
        }
    }

    Ok(())
}
