//! `rvt-corpus` — analyze multiple Revit files as a corpus.
//!
//! Phase B of the attack plan. Feed in 3+ versions of the same content and
//! get a structural classification of every byte position per stream:
//! invariant (structural constants), low-variance (type tags), size-correlated
//! (length fields), or variable (payload data).
//!
//! This drives Phase D (object graph parsing) by generating 80% of the
//! structural hypotheses automatically.

use clap::{Parser, ValueEnum};
use rvt::corpus::{Sample, analyze_corpus};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-corpus",
    version,
    about = "Corpus-wide delta analysis across multiple Revit files"
)]
struct Cli {
    /// 3+ Revit files (same logical content, different versions works best).
    #[arg(required = true, num_args = 3..)]
    files: Vec<PathBuf>,

    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "text", value_enum)]
    format: Format,

    /// Print up to N invariant runs per stream (each has offset/length/hex preview).
    #[arg(long = "runs", default_value_t = 5)]
    runs: usize,
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
    let mut samples: Vec<Sample> = cli
        .files
        .iter()
        .map(|p| Sample::open(p).map_err(anyhow::Error::from))
        .collect::<Result<_, _>>()?;

    let report = analyze_corpus(&mut samples)?;

    match cli.format {
        Format::Json => {
            println!("{}", serde_json::to_string_pretty(&report)?);
        }
        Format::Text => {
            println!("=== Corpus delta analysis ===");
            println!("samples: {}", report.samples.len());
            for s in &report.samples {
                println!("  · {}", s);
            }
            println!(
                "partition-number → year mapping: {} entries",
                report.partition_mapping.len()
            );
            for (num, year) in &report.partition_mapping {
                let y = year.map(|n| n.to_string()).unwrap_or_else(|| "?".into());
                println!("  Partitions/{}  → Revit {}", num, y);
            }
            println!();

            println!(
                "{:<32} {:>5}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}",
                "stream", "smp", "raw sz", "decomp sz", "invar", "low-var", "var"
            );
            println!("{}", "-".repeat(95));

            for s in &report.streams {
                let raw = format!("{}-{}", s.raw_size_min, s.raw_size_max);
                let decomp = match (s.decomp_size_min, s.decomp_size_max) {
                    (Some(mn), Some(mx)) => format!("{}-{}", mn, mx),
                    _ => "—".into(),
                };
                let inv = s.counts.get("Invariant").copied().unwrap_or(0);
                let lowvar = s.counts.get("LowVariance").copied().unwrap_or(0);
                let var = s.counts.get("Variable").copied().unwrap_or(0);
                let marker = if s.used_decompressed { "D" } else { "R" };
                println!(
                    "{:<32} {:>5}  {:>10}  {:>10}  {:>8}  {:>8}  {:>8}   [{}]",
                    s.name, s.samples, raw, decomp, inv, lowvar, var, marker
                );
            }

            println!();
            println!("=== Invariant runs (length ≥ 8 bytes, suggests structural markers) ===");
            for s in &report.streams {
                if s.invariant_runs.is_empty() {
                    continue;
                }
                println!("\n{} ({} runs total)", s.name, s.invariant_runs.len());
                for r in s.invariant_runs.iter().take(cli.runs) {
                    println!(
                        "  offset=0x{:06x}  len={:>4}  {}",
                        r.offset, r.length, r.hex_preview
                    );
                }
            }
        }
    }

    Ok(())
}
