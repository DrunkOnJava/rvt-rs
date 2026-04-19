//! `rvt-dump` — extract every OLE stream from a Revit file, decompress what
//! can be decompressed, and write each to its own file under an output dir.
//!
//! Useful for:
//!   - feeding decompressed streams to Ghidra / IDA / radare2
//!   - diffing streams with `xxd` / `hexdump` between versions
//!   - building a corpus for future Phase D work

use clap::Parser;
use rvt::{compression, RevitFile};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-dump",
    version,
    about = "Extract + decompress every OLE stream from a Revit file"
)]
struct Cli {
    /// Path to a Revit file.
    file: PathBuf,

    /// Output directory (created if missing).
    #[arg(short = 'o', long = "out", default_value = ".")]
    out: PathBuf,

    /// Also write the raw (compressed) bytes alongside the decompressed ones.
    #[arg(long = "raw")]
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
    fs::create_dir_all(&cli.out)?;

    let streams = rf.stream_names();
    println!("Dumping {} streams from {}", streams.len(), cli.file.display());

    for name in &streams {
        let safe = name.replace('/', "_");
        let raw = rf.read_stream(name)?;

        // Always write decompressed if we can
        if let Some(decomp) = try_decompress(&raw) {
            let path = cli.out.join(format!("{safe}.decomp"));
            fs::write(&path, &decomp)?;
            println!(
                "  {:<30}  raw={} bytes  decomp={} bytes  -> {}",
                name,
                raw.len(),
                decomp.len(),
                short_path(&path)
            );
        } else {
            println!("  {:<30}  raw={} bytes  (no gzip magic found)", name, raw.len());
        }

        if cli.raw {
            let path = cli.out.join(format!("{safe}.raw"));
            fs::write(&path, &raw)?;
        }
    }

    Ok(())
}

fn try_decompress(data: &[u8]) -> Option<Vec<u8>> {
    for off in [0, 4, 8, 16] {
        if compression::has_gzip_magic(data, off) {
            if let Ok(out) = compression::inflate_at(data, off) {
                return Some(out);
            }
        }
    }
    // fallback: scan for first gzip magic
    let chunks = compression::inflate_all_chunks(data);
    if chunks.is_empty() {
        None
    } else {
        // If multiple chunks (Partitions/NN), concatenate them with a
        // 16-byte separator so offsets remain recognizable.
        let mut out = Vec::new();
        for (i, c) in chunks.iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(&[0xFF; 16]);
            }
            out.extend_from_slice(c);
        }
        Some(out)
    }
}

fn short_path(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
