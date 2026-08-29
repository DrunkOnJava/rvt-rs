//! Example probe: stream-evidence harness (Discussion #112 / issue #151).
//!
//! Emits control-vs-experimental page-strip evidence JSON for a Revit file.
//! Library lives in `tools/stream_evidence` so production inflate is untouched.
//!
//! ```text
//! cargo run --release --example stream_evidence -- \
//!   --file corpus/tier1/architectural-2024/architectural-2024.rvt \
//!   -o /tmp/formats.json
//! ```
//!
//! Prefer the workspace binary when scripting:
//! `cargo run -p stream-evidence --release -- …`
//!
//! Credit: [@STE1200](https://github.com/STE1200) (Steffen).

use clap::Parser;
use std::path::PathBuf;
use stream_evidence::{analyze_file, stream_names_from_args, write_report};

#[derive(Parser, Debug)]
#[command(name = "stream_evidence", version)]
struct Cli {
    #[arg(long, short = 'f')]
    file: PathBuf,
    #[arg(long = "stream", short = 's')]
    streams: Vec<String>,
    #[arg(long, default_value_t = false)]
    all_paged: bool,
    #[arg(long, default_value_t = false)]
    include_empty: bool,
    #[arg(long, short = 'o')]
    output: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let filter = stream_names_from_args(&cli.streams);
    let report = analyze_file(
        &cli.file,
        filter.as_deref(),
        cli.all_paged,
        cli.include_empty,
    )?;
    if let Some(out) = cli.output.as_ref() {
        write_report(&report, out)?;
        eprintln!("wrote {}", out.display());
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
