//! `rvt-dump` — extract every OLE stream from a Revit file, decompress what
//! can be decompressed, and write each to its own file under an output dir.
//!
//! Decompression goes through the same decoders the library uses, so a
//! `.decomp` file is byte-for-byte what the parsers see: checksum-paged
//! streams (`Global/ElemTable`, `Global/Latest`, `Partitions/NN`, …) have
//! their page trailers stripped before inflating, and `Partitions_NN.decomp`
//! is every gzip member of the stream concatenated with nothing in between,
//! identical to `RevitFile::inflated_partition` — offsets quoted in the
//! `reports/` write-ups index it directly.
//!
//! Useful for:
//!   - feeding decompressed streams to Ghidra / IDA / radare2
//!   - diffing streams with `xxd` / `hexdump` between versions
//!   - building a corpus for future Phase D work

use clap::Parser;
use rvt::{RevitFile, compression};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-dump",
    version,
    about = "Extract + decompress every OLE stream from a Revit file",
    after_help = "Examples:\n  \
        rvt-dump model.rvt -o streams/\n  \
        rvt-dump model.rvt -o streams/ --raw"
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
    let cli = Cli::parse();
    let mut rf = RevitFile::open(&cli.file)?;
    fs::create_dir_all(&cli.out)?;

    let streams = rf.stream_names();
    println!(
        "Dumping {} streams from {}",
        streams.len(),
        cli.file.display()
    );

    for name in &streams {
        let safe = name.replace('/', "_");
        let raw = rf.read_stream(name)?;

        let decoded = if raw.is_empty() {
            Err(Undecoded::Empty)
        } else if name.starts_with("Partitions/") {
            let inflated = rf.inflated_partition(name)?;
            if inflated.chunk_count() > 0 {
                Ok(Decoded::new(
                    inflated.bytes().to_vec(),
                    inflated.chunk_count(),
                ))
            } else {
                Err(undecoded(name, &raw))
            }
        } else {
            decompress(name, &raw)
        };
        match decoded {
            Ok(decoded) => {
                let path = cli.out.join(format!("{safe}.decomp"));
                fs::write(&path, &decoded.bytes)?;
                let members = if decoded.members > 1 {
                    format!(" ({} gzip members)", decoded.members)
                } else {
                    String::new()
                };
                println!(
                    "  {:<30}  raw={} bytes  decomp={} bytes{members}  -> {}",
                    name,
                    raw.len(),
                    decoded.bytes.len(),
                    short_path(&path)
                );
            }
            Err(Undecoded::Empty) => println!("  {:<30}  raw=0 bytes  (empty)", name),
            Err(Undecoded::NotCompressed) => println!(
                "  {:<30}  raw={} bytes  (not gzip-compressed)",
                name,
                raw.len()
            ),
            Err(Undecoded::Failed) => println!(
                "  {:<30}  raw={} bytes  (gzip header found, but it did not inflate)",
                name,
                raw.len()
            ),
        }

        if cli.raw {
            let path = cli.out.join(format!("{safe}.raw"));
            fs::write(&path, &raw)?;
        }
    }

    Ok(())
}

struct Decoded {
    bytes: Vec<u8>,
    members: usize,
}

impl Decoded {
    fn new(bytes: Vec<u8>, members: usize) -> Self {
        Self { bytes, members }
    }
}

enum Undecoded {
    Empty,
    NotCompressed,
    Failed,
}

/// Inflate one non-partition stream the way the parsers do: page
/// checksums stripped first when the stream is paged, then the gzip member
/// at the usual prefix offsets, then every member in order.
fn decompress(name: &str, stored: &[u8]) -> Result<Decoded, Undecoded> {
    let prepared = compression::prepare_stream_for_inflate(name, stored);
    let data = prepared.as_ref();
    for off in [0, 4, 8, 16] {
        if compression::has_gzip_magic(data, off) {
            if let Ok(out) = compression::inflate_at(data, off) {
                return Ok(Decoded::new(out, 1));
            }
        }
    }
    let chunks = compression::inflate_all_chunks(data);
    if !chunks.is_empty() {
        let members = chunks.len();
        return Ok(Decoded::new(chunks.concat(), members));
    }
    Err(undecoded(name, stored))
}

/// Why a stream produced no bytes: no gzip header at all, or one that
/// did not inflate.
fn undecoded(name: &str, stored: &[u8]) -> Undecoded {
    let prepared = compression::prepare_stream_for_inflate(name, stored);
    if compression::find_gzip_offsets(prepared.as_ref()).is_empty() {
        Undecoded::NotCompressed
    } else {
        Undecoded::Failed
    }
}

fn short_path(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.display().to_string())
}
