//! `rvt-elem-table` — dump the Global/ElemTable contents of a Revit file.
//!
//! Shows the declared element-id count, detected record layout (implicit
//! 12 B on family files; 28 B or 40 B explicit on project files), and a
//! sample of the parsed records. Useful for cross-corpus validation and
//! for anyone investigating element-id distribution across files.
//!
//! See `docs/elem-table-record-layout-2026-04-21.md` for the record-layout
//! reverse-engineering notes.

use clap::Parser;
use rvt::{RevitFile, elem_table};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-elem-table",
    version,
    about = "Dump Global/ElemTable header + records from a Revit file"
)]
struct Cli {
    file: PathBuf,

    /// Output format: `text` (human summary, default) or `json`.
    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,

    /// How many records to print in text mode. Ignored for JSON.
    #[arg(long = "limit", default_value_t = 20)]
    limit: usize,

    /// Show the raw bytes of each printed record (hex). Expensive on
    /// 40 B records over a 26 K-record stream — scope with `--limit`.
    #[arg(long)]
    raw: bool,
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
    let header = elem_table::parse_header(&mut rf)?;
    let records = elem_table::parse_records(&mut rf)?;

    if cli.format == "json" {
        #[derive(serde::Serialize)]
        struct Out<'a> {
            element_count: u16,
            record_count: u16,
            header_flag: u16,
            decompressed_bytes: usize,
            parsed_records: usize,
            records: &'a [elem_table::ElemRecord],
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&Out {
                element_count: header.element_count,
                record_count: header.record_count,
                header_flag: header.header_flag,
                decompressed_bytes: header.decompressed_bytes,
                parsed_records: records.len(),
                records: &records,
            })?
        );
        return Ok(());
    }

    println!("Global/ElemTable · {}", cli.file.display());
    println!(
        "  declared element_count={}  declared record_count={}",
        header.element_count, header.record_count
    );
    println!(
        "  header_flag=0x{:04x}  decompressed={} B",
        header.header_flag, header.decompressed_bytes
    );
    println!(
        "  parsed records: {} (of {} declared)",
        records.len(),
        header.record_count
    );

    if records.is_empty() {
        return Ok(());
    }

    // Infer layout from first record for a friendly summary.
    let first = &records[0];
    let layout_label = match first.raw.first() {
        Some(b) if *b == 0xFF && first.raw.get(7) == Some(&0xFF) => {
            "Explicit (40 B stride, 8-byte FF marker)"
        }
        Some(b) if *b == 0xFF => "Explicit (28 B stride, 4-byte FF marker)",
        _ => "Implicit (12 B stride, no marker)",
    };
    println!("  layout: {layout_label}  first record offset: 0x{:x}", first.offset);

    let take = records.len().min(cli.limit);
    println!("\nFirst {take} records:");
    for r in records.iter().take(take) {
        if cli.raw {
            print!(
                "  off=0x{:06x}  id={} id2={}  raw=",
                r.offset, r.id_primary, r.id_secondary
            );
            for b in &r.raw {
                print!("{:02x}", b);
            }
            println!();
        } else {
            println!(
                "  off=0x{:06x}  id={} id2={}",
                r.offset, r.id_primary, r.id_secondary
            );
        }
    }

    Ok(())
}
