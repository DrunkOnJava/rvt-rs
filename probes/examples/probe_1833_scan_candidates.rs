//! #1833, site R: how many candidates does the partition scanner keep on a real file?
//!
//! FACT under test: `ScanOptions::default()` sets `max_candidates: None`. A
//! candidate needs only a schema tag and the two zero bytes after it, so a stream
//! keeps at most one candidate for every four of its bytes, and each candidate
//! owns three strings on the heap. A stream that repeats one tag and two zero
//! bytes, with an envelope that scores 0.55 or more, would therefore hold many
//! times the file's length in memory. A default cap is safe only where no real
//! file reaches it, so this probe counts what the default scan keeps on each file
//! the scanner supports (Revit 2023 and 2024; it returns nothing on other
//! releases), per partition stream and in all, beside the stream's inflated length.
//!
//! Output (one model per run, counts and byte lengths only):
//!
//! ```text
//! file revit=<release> scan=<supported|unsupported> streams=<n> inflated_bytes=<n> candidates=<n> largest_stream=<n> fill_max=<percent> truncated=<0|1>
//! stream <name> inflated_bytes=<n> candidates=<n> bytes_per_candidate=<x|->
//! candidate_inline_bytes=<n>
//! ```
//!
//! `fill_max` is the largest share, over the streams, of the most candidates a
//! stream could keep (one per four bytes) that it does keep: a hostile stream
//! reaches 100, and a real one is the figure a default cap has to stay above.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_1833_scan_candidates -- model.rvt
//! ```

use std::collections::BTreeMap;

use rvt::research::partition_scanner::{PartitionRecordCandidate, ScanOptions, scan_partitions};

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_1833_scan_candidates FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let scan = scan_partitions(&mut rf, release, &ScanOptions::default())?;

    let mut kept: BTreeMap<&str, usize> = BTreeMap::new();
    for candidate in &scan.candidates {
        *kept.entry(candidate.stream.as_str()).or_default() += 1;
    }

    let names = rf.partition_stream_names();
    let mut inflated_total = 0usize;
    let mut largest = 0usize;
    let mut fill_max = 0.0_f64;
    let mut lines = Vec::new();
    for name in &names {
        let bytes = rf
            .inflated_partition(name)
            .map(|stream| stream.bytes().len())
            .unwrap_or(0);
        let count = kept.get(name.as_str()).copied().unwrap_or(0);
        inflated_total += bytes;
        largest = largest.max(count);
        if bytes >= 4 {
            fill_max = fill_max.max(100.0 * (4 * count) as f64 / bytes as f64);
        }
        let per = if count == 0 {
            "-".to_string()
        } else {
            format!("{:.1}", bytes as f64 / count as f64)
        };
        lines.push(format!(
            "stream {name} inflated_bytes={bytes} candidates={count} bytes_per_candidate={per}"
        ));
    }

    println!(
        "file revit={release} scan={} streams={} inflated_bytes={inflated_total} candidates={} largest_stream={largest} fill_max={fill_max:.3} truncated={}",
        if scan.status.is_supported() {
            "supported"
        } else {
            "unsupported"
        },
        names.len(),
        scan.candidates.len(),
        u8::from(scan.truncated),
    );
    for line in lines {
        println!("{line}");
    }
    println!(
        "candidate_inline_bytes={}",
        size_of::<PartitionRecordCandidate>()
    );
    Ok(())
}
