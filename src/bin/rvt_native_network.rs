use anyhow::{Context, Result};
use clap::Parser;
use rvt::native_network_document::{
    ExtractionLimits, extract_selected_with_geometry_limits, write_json,
};
use std::{collections::BTreeSet, path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "rvt-native-network",
    about = "Extract a bounded explicit native connector closure"
)]
struct Args {
    file: PathBuf,
    #[arg(long)]
    ids: String,
    #[arg(long)]
    json: PathBuf,
    #[arg(long, default_value_t = 1)]
    max_depth: usize,
    #[arg(long, default_value_t = 10_000)]
    max_owners: usize,
    #[arg(long, default_value_t = 100_000)]
    max_graph_values: usize,
    #[arg(long, default_value_t = 100_000)]
    max_graph_objects: usize,
    #[arg(long, default_value_t = 1)]
    max_geometry_depth: usize,
    #[arg(long, default_value_t = 10_000)]
    max_geometry_owners: usize,
}

fn run(args: Args) -> Result<u8> {
    let ids = args
        .ids
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<u64>()
                .with_context(|| format!("invalid ElementId {s}"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let report = extract_selected_with_geometry_limits(
        &args.file,
        ids,
        ExtractionLimits {
            max_depth: args.max_depth,
            max_owners: args.max_owners,
            max_graph_values: args.max_graph_values,
            max_graph_objects: args.max_graph_objects,
            max_geometry_depth: args.max_geometry_depth,
            max_geometry_owners: args.max_geometry_owners,
        },
    )?;
    let code = if report.status == "bounded_network_extracted" {
        0
    } else {
        2
    };
    write_json(&report, &args.json)?;
    eprintln!("{}", report.status);
    Ok(code)
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("rvt-native-network: {error:#}");
            ExitCode::FAILURE
        }
    }
}
