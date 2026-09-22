//! `rvt-capabilities` — emit an honest capability / relation-domain snapshot.
//!
//! Doctor-style CLI for Phase 1 leftovers. Prints JSON (default) or a short
//! text summary. Does **not** claim ES remapping, compound openings, or
//! converter-grade IFC.

use clap::{Parser, ValueEnum};
use rvt::capability::CapabilityManifest;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-capabilities",
    version,
    about = "Emit honest rvt-rs capability + relation-domain snapshot (no invented successes)",
    after_help = "Examples:\n  \
        rvt-capabilities\n  \
        rvt-capabilities -f text"
)]
struct Cli {
    /// Output format.
    #[arg(short = 'f', long = "format", default_value = "json", value_enum)]
    format: Format,

    /// Shorthand for `--format json`.
    #[arg(long, conflicts_with = "format")]
    json: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum Format {
    Json,
    Text,
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    let mut cli = Cli::parse();
    if cli.json {
        cli.format = Format::Json;
    }
    let manifest = CapabilityManifest::honest_snapshot();
    match cli.format {
        Format::Json => match manifest.to_json_string() {
            Ok(s) => {
                println!("{s}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(1)
            }
        },
        Format::Text => {
            println!("=== rvt-rs capabilities (honest snapshot) ===");
            println!("manifest: {}", manifest.manifest_id);
            println!("experimental: {}", manifest.experimental);
            println!();
            for c in &manifest.capabilities {
                println!(
                    "- {} · {} · {}",
                    c.capability_id,
                    c.status.as_str(),
                    c.evidence_tier.as_str()
                );
                for n in &c.non_claims {
                    println!("    non-claim: {n}");
                }
            }
            println!();
            println!("relation domains (experimental):");
            for d in &manifest.relation_domains.domains {
                println!("- {} · {:?}", d.id, d.status);
            }
            for h in &manifest.honesty {
                println!("honesty: {h}");
            }
            ExitCode::SUCCESS
        }
    }
}
