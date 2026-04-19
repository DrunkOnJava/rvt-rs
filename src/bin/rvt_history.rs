//! `rvt-history` — dump the document-version-upgrade history of a Revit file.
//!
//! Phase D v0: the first tangible result of schema-driven object-graph
//! parsing. Reads `Global/Latest` and returns every Revit release that has
//! ever opened and saved this file, in chronological order.

use clap::Parser;
use rvt::{object_graph::DocumentHistory, RevitFile};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-history",
    version,
    about = "Dump document upgrade history from a Revit file"
)]
struct Cli {
    file: PathBuf,

    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut rf = RevitFile::open(&cli.file)?;
    let history = DocumentHistory::from_revit_file(&mut rf)?;

    if cli.format == "json" {
        println!("{}", serde_json::to_string_pretty(&history)?);
    } else {
        println!(
            "Document history · {} entries (string section begins at 0x{:x}):",
            history.entries.len(),
            history.string_section_offset
        );
        for (i, e) in history.entries.iter().enumerate() {
            println!("  {i:>2}.  {e}");
        }
    }
    Ok(())
}
