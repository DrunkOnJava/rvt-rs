//! `rvt-schema` — dump the embedded serialization schema from a Revit file.
//!
//! Phase C of the attack plan. Revit ships its complete serialization
//! schema inside every `.rvt` / `.rfa` file in the `Formats/Latest` stream:
//! class names, field names, and full C++ type signatures including STL
//! generics like `std::pair< ElementId, double >`. This binary makes that
//! metadata available as structured JSON or text.

use clap::{Parser, ValueEnum};
use rvt::RevitFile;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-schema",
    version,
    about = "Dump the embedded serialization schema from a Revit file"
)]
struct Cli {
    /// Path to a Revit file.
    file: PathBuf,

    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "text", value_enum)]
    format: Format,

    /// Filter to classes whose names contain this substring (case-sensitive).
    #[arg(long = "grep")]
    grep: Option<String>,

    /// Only show classes that have at least one field.
    #[arg(long = "with-fields")]
    with_fields: bool,

    /// Show the top N classes by field count.
    #[arg(long = "top")]
    top: Option<usize>,
}

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Text,
    Json,
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
    let schema = rf.schema()?;

    let mut classes: Vec<_> = schema.classes.iter().collect();
    if let Some(pat) = &cli.grep {
        classes.retain(|c| c.name.contains(pat));
    }
    if cli.with_fields {
        classes.retain(|c| !c.fields.is_empty());
    }
    if let Some(n) = cli.top {
        classes.sort_by(|a, b| b.fields.len().cmp(&a.fields.len()));
        classes.truncate(n);
    }

    match cli.format {
        Format::Json => {
            // Filtered view for JSON output
            let filtered = rvt::formats::SchemaTable {
                classes: classes.iter().map(|c| (*c).clone()).collect(),
                cpp_types: schema.cpp_types.clone(),
                skipped_records: schema.skipped_records,
            };
            println!("{}", serde_json::to_string_pretty(&filtered)?);
        }
        Format::Text => {
            println!("Schema · {} classes", schema.classes.len());
            let total_fields: usize = schema.classes.iter().map(|c| c.fields.len()).sum();
            println!("          {} fields total", total_fields);
            println!(
                "          {} unique C++ type signatures",
                schema.cpp_types.len()
            );
            if schema.skipped_records > 0 {
                println!(
                    "          ({} records skipped during parse)",
                    schema.skipped_records
                );
            }
            println!();

            if cli.top.is_some() {
                println!("Top {} classes by field count:", classes.len());
            } else if let Some(p) = &cli.grep {
                println!("Classes matching /{p}/:");
            } else {
                println!("(showing first 40 classes; use --grep or --top to filter)");
                classes.truncate(40);
            }

            for c in &classes {
                println!(
                    "\n  {}  [offset 0x{:x}, {} fields]",
                    c.name,
                    c.offset,
                    c.fields.len()
                );
                for f in c.fields.iter().take(8) {
                    match &f.cpp_type {
                        Some(t) => println!("    . {} : {}", f.name, t),
                        None => println!("    . {}", f.name),
                    }
                }
                if c.fields.len() > 8 {
                    println!("    . ... ({} more)", c.fields.len() - 8);
                }
            }

            if !schema.cpp_types.is_empty() {
                println!("\nC++ type signatures (top 20):");
                for t in schema.cpp_types.iter().take(20) {
                    println!("  {t}");
                }
            }
        }
    }

    Ok(())
}
