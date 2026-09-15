//! Native equipment inventory with source-qualified type and instance claims.
use anyhow::{Context, Result};
use clap::Parser;
use rvt::{native_document, native_equipment};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "rvt-native-equipment",
    about = "Extract a native equipment inventory with explicit ownership and coverage",
    after_help = "No external parser or API witness input. Output must not exist. Exit 0 means bounded extraction completed; 2 means incomplete document graph coverage; 1 means extraction failed. Neither 0 nor 2 establishes full equipment attribute parity."
)]
struct Args {
    file: PathBuf,
    #[arg(long)]
    json: PathBuf,
    #[arg(long, default_value_t = 100_000)]
    max_graph_values: usize,
}

#[derive(Serialize)]
struct Report {
    format: &'static str,
    source: PathBuf,
    source_sha256: Option<String>,
    status: &'static str,
    complete_world_model: bool,
    coverage: Option<native_document::Summary>,
    inventory: Option<native_equipment::Inventory>,
    error: Option<String>,
}

fn run(args: Args) -> Result<u8> {
    anyhow::ensure!(
        args.max_graph_values > 0,
        "graph value budget must be positive"
    );
    let mut output = BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&args.json)
            .context("create new inventory output")?,
    );
    let mut report = Report {
        format: "rvt-native-equipment/v1",
        source: args.file.clone(),
        source_sha256: None,
        status: "failed",
        complete_world_model: false,
        coverage: None,
        inventory: None,
        error: None,
    };
    let result = (|| -> Result<()> {
        let mut source = File::open(&args.file)?;
        let mut digest = Sha256::new();
        let mut chunk = [0u8; 65536];
        loop {
            let count = source.read(&mut chunk)?;
            if count == 0 {
                break;
            }
            digest.update(&chunk[..count]);
        }
        report.source_sha256 = Some(format!("{:x}", digest.finalize()));
        let mut file = rvt::RevitFile::open(&args.file)?;
        let mut builder = native_equipment::InventoryBuilder::default();
        let options = native_document::Options {
            max_graph_values: args.max_graph_values,
            ..Default::default()
        };
        report.coverage = Some(native_document::extract(&mut file, &options, |record| {
            builder.ingest(&record)
        })?);
        report.inventory = Some(builder.finish()?);
        Ok(())
    })();
    let code = match result {
        Err(error) => {
            report.error = Some(format!("{error:#}"));
            1
        }
        Ok(()) => {
            let coverage = report
                .coverage
                .as_ref()
                .expect("successful extraction has coverage");
            let incomplete = !report
                .inventory
                .as_ref()
                .expect("successful extraction has inventory")
                .complete_selected_equipment
                || coverage.unsupported_graph_records > 0
                || coverage.unsupported_metadata_records > 0
                || !coverage.definition_diagnostics.is_empty()
                || !coverage.selected_ids_without_records.is_empty()
                || coverage.emitted_records != coverage.selected_indexed_elements;
            report.status = if incomplete {
                "partial_native_inventory"
            } else {
                "bounded_inventory_extracted"
            };
            if incomplete { 2 } else { 0 }
        }
    };
    serde_json::to_writer_pretty(&mut output, &report)?;
    output.write_all(b"\n")?;
    output.flush()?;
    Ok(code)
}
fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("{error:#}");
            ExitCode::FAILURE
        }
    }
}
