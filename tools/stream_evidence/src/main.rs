//! CLI for the stream-evidence harness (Discussion #112 / issue #151).
//!
//! ```text
//! cargo run -p stream-evidence --release -- \
//!   --file path.rvt --stream Formats/Latest -o report.json
//! ```

use clap::Parser;
use std::path::PathBuf;
use stream_evidence::{analyze_file, stream_names_from_args, write_report};

#[derive(Parser, Debug)]
#[command(
    name = "stream-evidence",
    version,
    about = "Control vs experimental page-strip evidence for Revit CFB streams (#112 / #151)"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    #[arg(long, short = 'f')]
    file: PathBuf,

    /// Stream name(s) to analyze (repeatable). Default: `Formats/Latest`
    /// when present; otherwise the first non-empty stream.
    #[arg(long = "stream", short = 's')]
    streams: Vec<String>,

    /// Analyze every suspected checksum-paged non-empty stream
    /// (`Formats/Latest`, `Global/*` tables, `Partitions/N`).
    #[arg(long, default_value_t = false)]
    all_paged: bool,

    /// Include empty streams in the report.
    #[arg(long, default_value_t = false)]
    include_empty: bool,

    /// Write JSON report to this path (pretty-printed). Prints to stdout
    /// when omitted.
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
        eprintln!(
            "wrote {} ({} stream(s), sha256={}…)",
            out.display(),
            report.streams.len(),
            &report.file.sample_hash_sha256[..12]
        );
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}
