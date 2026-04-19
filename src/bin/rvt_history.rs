//! `rvt-history` — dump the document-version-upgrade history of a Revit file.
//!
//! Phase D v0: the first tangible result of schema-driven object-graph
//! parsing. Reads `Global/Latest` and returns every Revit release that has
//! ever opened and saved this file, in chronological order.

use clap::Parser;
use rvt::{object_graph::{self, DocumentHistory}, RevitFile};
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

    /// Also scan the version-specific `Partitions/NN` stream — where the
    /// bulk Revit content lives (categories, OmniClass codes, Uniformat
    /// codes, Autodesk spec namespaces, asset references, localized strings).
    /// Implies --all-strings.
    #[arg(long = "partitions")]
    partitions: bool,

    /// Redact PII — Windows usernames, Autodesk-internal paths, and
    /// project-ID folder names — from every string record before
    /// display. Safe default for sharing output publicly.
    #[arg(long)]
    redact: bool,
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

fn fmt_trunc(s: &str, n: usize) -> String {
    let c: Vec<char> = s.chars().collect();
    if c.len() <= n {
        s.to_string()
    } else {
        format!("{}...", c[..n.saturating_sub(3)].iter().collect::<String>())
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut rf = RevitFile::open(&cli.file)?;

    if cli.all_strings || cli.partitions {
        let mut global_records = object_graph::string_records_from_file(&mut rf)?;
        let mut partition_records = if cli.partitions {
            object_graph::string_records_from_partitions(&mut rf).unwrap_or_default()
        } else {
            Vec::new()
        };
        if cli.redact {
            for r in global_records.iter_mut().chain(partition_records.iter_mut()) {
                r.value = rvt::redact::redact_sensitive(&r.value);
            }
        }

        if cli.format == "json" {
            #[derive(serde::Serialize)]
            struct Out<'a> {
                global_latest: &'a [object_graph::StringRecord],
                partitions: &'a [object_graph::StringRecord],
            }
            println!(
                "{}",
                serde_json::to_string_pretty(&Out {
                    global_latest: &global_records,
                    partitions: &partition_records,
                })?
            );
            return Ok(());
        }

        println!(
            "Global/Latest · {} string records",
            global_records.len()
        );
        if !partition_records.is_empty() {
            println!("Partitions/NN · {} string records", partition_records.len());
        }

        // Interesting samples from global
        let tag1_samples: Vec<_> = global_records.iter().filter(|r| r.tag == 1 && !r.value.is_empty()).take(20).collect();
        if !tag1_samples.is_empty() {
            println!("\nGlobal tag=0x01 records (sheets, levels, elevations):");
            for r in tag1_samples {
                println!("  off=0x{:06x}  '{}'", r.offset, fmt_trunc(&r.value, 60));
            }
        }

        if !partition_records.is_empty() {
            // Classify partitions content
            let mut units = Vec::new();
            let mut specs = Vec::new();
            let mut groups = Vec::new();
            let mut omniclass = Vec::new();
            let mut uniformat = Vec::new();
            let mut categories = Vec::new();
            for r in &partition_records {
                let v = &r.value;
                if v.starts_with("autodesk.unit.") {
                    units.push(v);
                } else if v.starts_with("autodesk.spec.") {
                    specs.push(v);
                } else if v.starts_with("autodesk.parameter.group") {
                    groups.push(v);
                } else if v.chars().all(|c| c.is_ascii_digit() || c == '.') && v.contains('.') {
                    omniclass.push(v);
                } else if (v.starts_with('A')
                    || v.starts_with('B')
                    || v.starts_with('C')
                    || v.starts_with('D')
                    || v.starts_with('E')
                    || v.starts_with('F')
                    || v.starts_with('G')
                    || v.starts_with('Z'))
                    && v.len() >= 4
                    && v[1..].chars().all(|c| c.is_ascii_digit())
                {
                    uniformat.push(v);
                } else if v.len() >= 4
                    && !v.starts_with("autodesk.")
                    && !v.starts_with('%')
                    && v.chars().any(|c| c.is_ascii_alphabetic())
                {
                    categories.push(v);
                }
            }

            println!(
                "\nPartitions/NN content classified:"
            );
            println!("  Autodesk unit  identifiers: {}", units.len());
            println!("  Autodesk spec  identifiers: {}", specs.len());
            println!("  Autodesk param groups     : {}", groups.len());
            println!("  OmniClass-shaped codes    : {}", omniclass.len());
            println!("  Uniformat-shaped codes    : {}", uniformat.len());
            println!("  Revit categories / misc   : {}", categories.len());

            fn pick<'a>(v: &[&'a String], n: usize) -> Vec<&'a String> {
                v.iter().take(n).copied().collect()
            }
            for (label, sample) in [
                ("Autodesk unit identifiers", pick(&units, 6)),
                ("Autodesk spec identifiers", pick(&specs, 6)),
                ("Autodesk parameter groups", pick(&groups, 6)),
                ("Uniformat codes", pick(&uniformat, 6)),
                ("Revit categories / misc", pick(&categories, 12)),
            ] {
                if !sample.is_empty() {
                    println!("\n  {}:", label);
                    for v in sample {
                        println!("    · {}", fmt_trunc(v, 80));
                    }
                }
            }
        }

        return Ok(());
    }

    let mut history = DocumentHistory::from_revit_file(&mut rf)?;
    if cli.redact {
        for e in history.entries.iter_mut() {
            *e = rvt::redact::redact_sensitive(e);
        }
    }

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
