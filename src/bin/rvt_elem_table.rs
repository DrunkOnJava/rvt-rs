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
    about = "Dump Global/ElemTable header + records from a Revit file",
    after_help = "Examples:\n  \
        rvt-elem-table model.rvt --limit 50\n  \
        rvt-elem-table model.rvt --json"
)]
struct Cli {
    /// Path to a .rvt / .rfa / .rte / .rft file.
    file: PathBuf,

    /// Output format: `text` (human summary, default) or `json`.
    #[arg(short = 'f', long = "format", default_value = "text")]
    format: String,

    /// Shorthand for `--format json`.
    #[arg(long, conflicts_with = "format")]
    json: bool,

    /// How many records to print in text mode. Ignored for JSON.
    #[arg(long = "limit", default_value_t = 20)]
    limit: usize,

    /// Show the raw bytes of each printed record (hex). Expensive on
    /// 40 B records over a 26 K-record stream — scope with `--limit`.
    #[arg(long)]
    raw: bool,
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run() -> anyhow::Result<()> {
    let mut cli = Cli::parse();
    if cli.json {
        cli.format = "json".to_string();
    }
    let mut rf = RevitFile::open(&cli.file)?;
    let header = elem_table::parse_header(&mut rf)?;
    let layout = elem_table::read_layout(&mut rf)?;
    let records = elem_table::parse_records(&mut rf)?;
    let declared: std::collections::BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let with_owner = records.iter().filter(|r| r.owner_id.is_some()).count();
    let owner_declared = records
        .iter()
        .filter_map(|r| r.owner_id)
        .filter(|id| declared.contains(id))
        .count();

    if cli.format == "json" {
        #[derive(serde::Serialize)]
        struct Out<'a> {
            element_count: u16,
            record_count: u16,
            header_flag: u16,
            decompressed_bytes: usize,
            layout: elem_table::ElemTableLayout,
            parsed_records: usize,
            records_with_owner: usize,
            records: &'a [elem_table::ElemRecord],
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&Out {
                element_count: header.element_count,
                record_count: header.record_count,
                header_flag: header.header_flag,
                decompressed_bytes: header.decompressed_bytes,
                layout,
                parsed_records: records.len(),
                records_with_owner: with_owner,
                records: &records,
            })?
        );
        return Ok(());
    }

    println!("Global/ElemTable · {}", cli.file.display());
    println!(
        "  header: element_count={} (a per-release constant, not a count)  record_count={}",
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

    let layout_label = match layout.framing {
        elem_table::RecordFraming::Explicit { marker_len } => format!(
            "Explicit ({} B stride, {marker_len}-byte owner field at record +{})",
            layout.stride, layout.marker_offset
        ),
        elem_table::RecordFraming::Implicit => {
            format!("Implicit ({} B stride, no owner field)", layout.stride)
        }
    };
    println!(
        "  layout: {layout_label}  first record offset: 0x{:x}",
        layout.start
    );
    if matches!(layout.framing, elem_table::RecordFraming::Explicit { .. }) {
        println!(
            "  records naming an owner element: {with_owner} ({owner_declared} of them declared in this table)"
        );
    }

    let take = records.len().min(cli.limit);
    println!("\nFirst {take} records:");
    for r in records.iter().take(take) {
        if cli.raw {
            print!(
                "  off=0x{:06x}  id={} id2={}{}  raw=",
                r.offset,
                r.id_primary,
                r.id_secondary,
                owner_suffix(r.owner_id)
            );
            for b in &r.raw {
                print!("{:02x}", b);
            }
            println!();
        } else {
            println!(
                "  off=0x{:06x}  id={} id2={}{}",
                r.offset,
                r.id_primary,
                r.id_secondary,
                owner_suffix(r.owner_id)
            );
        }
    }

    Ok(())
}

fn owner_suffix(owner: Option<u32>) -> String {
    owner.map(|id| format!("  owner={id}")).unwrap_or_default()
}
