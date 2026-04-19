//! `rvt-diff` — compare two Revit files stream-by-stream.
//!
//! Phase B of the RVT reverse-engineering attack: take two (or more)
//! versions of the *same logical* content and identify what's invariant
//! vs what changes. The invariant bytes are format structure; the varying
//! bytes are payload. This is the delta-analysis step that makes object
//! graph inference tractable without a spec.

use clap::Parser;
use rvt::{RevitFile, compression};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-diff",
    version,
    about = "Compare Revit files stream-by-stream"
)]
struct Cli {
    /// Two or more Revit files to compare.
    #[arg(required = true, num_args = 2..)]
    files: Vec<PathBuf>,

    /// Also decompress each stream and diff the inflated bytes.
    #[arg(long = "decompress")]
    decompress: bool,

    /// Bytes to show per differing region.
    #[arg(long = "preview", default_value_t = 32)]
    preview: usize,
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
    let mut files: Vec<(PathBuf, RevitFile)> = cli
        .files
        .iter()
        .map(|p| RevitFile::open(p).map(|rf| (p.clone(), rf)))
        .collect::<Result<_, _>>()?;

    // Collect stream name union
    let all_streams: std::collections::BTreeSet<String> =
        files.iter().flat_map(|(_, rf)| rf.stream_names()).collect();

    println!("=== comparing {} files ===", files.len());
    for (p, rf) in &files {
        let names = rf.stream_names();
        println!(
            "  {} · {} streams · partitions={}",
            p.file_name().unwrap().to_string_lossy(),
            names.len(),
            rf.partition_stream_name().unwrap_or_else(|| "none".into())
        );
    }

    println!();
    println!(
        "{:<34} {}",
        "stream",
        files
            .iter()
            .map(|(p, _)| p
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .chars()
                .take(12)
                .collect::<String>())
            .collect::<Vec<_>>()
            .join("  ")
    );
    println!("{}", "-".repeat(34 + files.len() * 14));

    for name in &all_streams {
        let mut row = format!("{name:<34}");
        let mut sizes: Vec<Option<(usize, Option<usize>)>> = Vec::with_capacity(files.len());
        for (_, rf) in files.iter_mut() {
            match rf.read_stream(name) {
                Ok(bytes) => {
                    let decompressed_len = if cli.decompress {
                        compression::inflate_at(&bytes, 0).ok().map(|v| v.len())
                    } else {
                        None
                    };
                    sizes.push(Some((bytes.len(), decompressed_len)));
                }
                Err(_) => sizes.push(None),
            }
        }
        for s in &sizes {
            match s {
                None => row.push_str(&format!("{:>12}  ", "—")),
                Some((raw, Some(inflated))) => row.push_str(&format!("{raw:>5}→{inflated:<6}  ")),
                Some((raw, None)) => row.push_str(&format!("{raw:>12}  ")),
            }
        }
        println!("{row}");
    }

    // If exactly 2 files, do a byte-level diff on shared streams
    if files.len() == 2 {
        println!("\n=== byte-level diff (shared streams) ===");
        let shared: Vec<String> = all_streams
            .iter()
            .filter(|name| files.iter_mut().all(|(_, rf)| rf.read_stream(name).is_ok()))
            .cloned()
            .collect();
        for name in shared {
            let a = files[0].1.read_stream(&name)?;
            let b = files[1].1.read_stream(&name)?;
            let mut diffs = 0usize;
            let min_len = a.len().min(b.len());
            let mut first_diff: Option<usize> = None;
            for i in 0..min_len {
                if a[i] != b[i] {
                    diffs += 1;
                    if first_diff.is_none() {
                        first_diff = Some(i);
                    }
                }
            }
            let length_delta = (a.len() as isize) - (b.len() as isize);
            let pct = if min_len == 0 {
                100.0
            } else {
                100.0 * diffs as f64 / min_len as f64
            };
            if diffs == 0 && length_delta == 0 {
                println!("  {:<34} IDENTICAL", name);
            } else {
                println!(
                    "  {:<34} Δlen={:+}  Δbytes={}/{} ({:.1}%)  first@{}",
                    name,
                    length_delta,
                    diffs,
                    min_len,
                    pct,
                    first_diff
                        .map(|i| format!("0x{i:x}"))
                        .unwrap_or_else(|| "—".into())
                );
                if let Some(off) = first_diff {
                    let preview = cli.preview.min(min_len - off);
                    print!("         a[{off:>6x}..] ");
                    for b in &a[off..off + preview] {
                        print!("{:02x} ", b);
                    }
                    println!();
                    print!("         b[{off:>6x}..] ");
                    for bb in &b[off..off + preview] {
                        print!("{:02x} ", bb);
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}
