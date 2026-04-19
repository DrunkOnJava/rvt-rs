//! `rvt-history` — dump the document-version-upgrade history of a Revit file.
//!
//! Phase D v0: the first tangible result of schema-driven object-graph
//! parsing. Reads `Global/Latest` and returns every Revit release that has
//! ever opened and saved this file, in chronological order.

use clap::Parser;
use rvt::{object_graph::{self, DocumentHistory}, RevitFile};
use std::collections::BTreeMap;
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

    /// Dump ALL length-prefixed UTF-16LE string records from Global/Latest,
    /// not just the Revit version-upgrade timeline. Includes level names,
    /// sheet names, elevation labels, and other user-visible BIM metadata.
    #[arg(long = "all-strings")]
    all_strings: bool,
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

    if cli.all_strings {
        let records = object_graph::string_records_from_file(&mut rf)?;
        if cli.format == "json" {
            println!("{}", serde_json::to_string_pretty(&records)?);
        } else {
            println!("Global/Latest string records · {} entries", records.len());
            let mut by_tag: BTreeMap<u32, usize> = BTreeMap::new();
            for r in &records {
                *by_tag.entry(r.tag).or_insert(0) += 1;
            }
            println!("\nTag histogram (most common first):");
            let mut pairs: Vec<_> = by_tag.iter().collect();
            pairs.sort_by(|a, b| b.1.cmp(a.1));
            for (tag, count) in pairs.iter().take(10) {
                println!("  tag=0x{:08x}  {} records", tag, count);
            }
            println!("\nSample of records with tag=0x01 (often sheets, levels, elevations):");
            let sample: Vec<_> = records.iter().filter(|r| r.tag == 1).take(40).collect();
            for r in sample {
                let v = if r.value.len() > 60 {
                    format!("{}...", &r.value[..57])
                } else {
                    r.value.clone()
                };
                println!("  off=0x{:06x}  '{}'", r.offset, v);
            }
        }
        return Ok(());
    }

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
