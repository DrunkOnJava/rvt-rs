//! `rvt-info` — dump Revit file metadata as text or JSON. No Autodesk software required.

use clap::{Parser, ValueEnum};
use rvt::RevitFile;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-info",
    version,
    about = "Read Revit (.rvt, .rfa, .rte, .rft) metadata without requiring Revit"
)]
struct Cli {
    /// Path to a Revit file.
    file: PathBuf,

    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "text", value_enum)]
    format: Format,

    /// Include a sample of class/schema names.
    #[arg(long = "show-classes")]
    show_classes: bool,

    /// Include full raw class list (one per line after the summary).
    #[arg(long = "all-classes")]
    all_classes: bool,

    /// Extract the PNG preview thumbnail to this path.
    #[arg(long = "extract-preview")]
    extract_preview: Option<PathBuf>,

    /// Redact PII — Windows usernames, Autodesk-internal paths, and
    /// project-ID folder names — before rendering. Safe default for
    /// sharing output publicly.
    #[arg(long)]
    redact: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Text,
    Json,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut rf = RevitFile::open(&cli.file)?;
    let mut summary = rf.summarize()?;
    if cli.redact {
        redact_summary(&mut summary);
    }

    if let Some(preview_path) = &cli.extract_preview {
        let png = rf.preview_png()?;
        std::fs::write(preview_path, &png)?;
        eprintln!("preview PNG written to {}", preview_path.display());
    }

    match cli.format {
        Format::Json => {
            let s = serde_json::to_string_pretty(&summary)?;
            println!("{s}");
        }
        Format::Text => {
            print_text(&summary, cli.show_classes);
            if cli.all_classes {
                let names = rf.class_names()?;
                println!("\n=== all class names ({}) ===", names.len());
                for n in names {
                    println!("{n}");
                }
            }
        }
    }
    Ok(())
}

fn redact_summary(s: &mut rvt::reader::Summary) {
    if let Some(p) = &s.original_path {
        s.original_path = Some(rvt::redact::redact_sensitive(p));
    }
}

fn print_text(s: &rvt::reader::Summary, show_classes: bool) {
    println!("Revit file ·");
    println!("  version:       {}", s.version);
    if let Some(b) = &s.build {
        println!("  build:         {}", b);
    }
    if let Some(l) = &s.locale {
        println!("  locale:        {}", l);
    }
    if let Some(g) = &s.guid {
        println!("  file GUID:     {}", g);
    }
    if let Some(p) = &s.original_path {
        println!("  original path: {}", p);
    }
    if let Some(p) = &s.partition_stream {
        println!("  partition:     {}", p);
    }
    println!(
        "  streams:       {} ({} total bytes)",
        s.streams.len(),
        s.file_size
    );

    if let Some(pa) = &s.partatom {
        println!("\nPartAtom ·");
        if let Some(t) = &pa.title {
            println!("  title:     {}", t);
        }
        if let Some(i) = &pa.id {
            println!("  id:        {}", i);
        }
        if let Some(u) = &pa.updated {
            println!("  updated:   {}", u);
        }
        if let Some(oc) = &pa.omniclass {
            println!("  omniclass: {}", oc);
        }
        if !pa.categories.is_empty() {
            println!("  categories:");
            for c in &pa.categories {
                if let Some(sch) = &c.scheme {
                    println!("    - {} (scheme: {})", c.term, sch);
                } else {
                    println!("    - {}", c.term);
                }
            }
        }
        if !pa.taxonomies.is_empty() {
            println!("  taxonomies:");
            for t in &pa.taxonomies {
                println!("    - {} ({})", t.label, t.term);
            }
        }
    }

    println!("\nSchema ·");
    println!("  class names (inferred): {}", s.class_name_count);
    if show_classes && !s.class_name_sample.is_empty() {
        println!("  sample:");
        for n in &s.class_name_sample {
            println!("    - {}", n);
        }
    }

    println!("\nStreams ·");
    for name in &s.streams {
        println!("  {}", name);
    }
}
