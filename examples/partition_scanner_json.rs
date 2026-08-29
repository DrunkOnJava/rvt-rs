//! Emit JSON [`PartitionRecordCandidate`]s for issue attachments (M3-03).
//!
//! Usage:
//! ```text
//! cargo run --release --example partition_scanner_json -- model.rvt
//! cargo run --release --example partition_scanner_json -- model.rvt --arcwall-only
//! cargo run --release --example partition_scanner_json -- model.rvt --unlocated
//! ```

use clap::Parser;
use rvt::partition_scanner::{
    ScanOptions, declared_but_unlocated_ids, element_id_partition_index, linkage_coverage,
    scan_partitions,
};
use rvt::{RevitFile, elem_table};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "partition_scanner_json")]
struct Cli {
    /// Path to a `.rvt` / `.rfa` file.
    path: PathBuf,
    /// Restrict the scan to ArcWall tag `0x0191` (2023 envelope path).
    #[arg(long)]
    arcwall_only: bool,
    /// Also print declared-but-unlocated ElemTable ids.
    #[arg(long)]
    unlocated: bool,
    /// Minimum confidence (default 0.55).
    #[arg(long, default_value_t = 0.55)]
    min_confidence: f32,
    /// Cap emitted candidates.
    #[arg(long)]
    max_candidates: Option<usize>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut rf = RevitFile::open(&cli.path)?;
    let version = rf.basic_file_info()?.version;

    let mut options = if cli.arcwall_only {
        ScanOptions::arcwall_2023_only()
    } else {
        ScanOptions::default()
    };
    options.min_confidence = cli.min_confidence;
    options.max_candidates = cli.max_candidates;

    let scan = scan_partitions(&mut rf, version, &options)?;
    let partition_index = element_id_partition_index(&scan.candidates);

    let report = serde_json::json!({
        "path": cli.path.display().to_string(),
        "revit_version": version,
        "status": scan.status,
        "candidate_count": scan.candidates.len(),
        "candidates_with_element_id": partition_index.len(),
        "candidates": scan.candidates,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);

    if cli.unlocated {
        let declared = elem_table::declared_element_ids(&mut rf).unwrap_or_default();
        let missing = declared_but_unlocated_ids(&declared, &partition_index);
        let coverage = linkage_coverage(&declared, &partition_index);
        let sidecar = serde_json::json!({
            "declared_element_ids": declared.len(),
            "linked": partition_index.len(),
            "coverage": coverage,
            "declared_but_unlocated": missing,
        });
        eprintln!("{}", serde_json::to_string_pretty(&sidecar)?);
    }

    Ok(())
}
