//! Streaming diagnostic extraction from a caller-supplied document.
use anyhow::{Context, Result};
use clap::Parser;
use rvt::native_document::{self, Options, Summary};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{BufWriter, Read, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "rvt-native-document",
    about = "Stream native current-record graphs to JSONL with explicit coverage diagnostics",
    after_help = "Outputs must not exist. Exit codes: 0 selected graph coverage complete; 2 incomplete coverage; 1 extraction or output failure. Graph coverage is not evaluated semantics or document geometry."
)]
struct Args {
    file: PathBuf,
    /// One serialized native record per line; may contain partial results on failure.
    #[arg(long)]
    jsonl: PathBuf,
    /// Run provenance, final coverage, and fatal error (if any).
    #[arg(long)]
    summary: PathBuf,
    /// Current element ID; repeat or comma-separate. Omit to select all indexed IDs.
    #[arg(long = "id", value_delimiter = ',')]
    ids: Vec<u64>,
    /// Native record channel; repeat or comma-separate (default 102).
    #[arg(long = "channel", value_delimiter = ',', default_value = "102", value_parser = channel)]
    channels: Vec<u64>,
    /// Maximum stored bytes per partition stream (implementation resource budget).
    #[arg(long, default_value_t = 512 * 1024 * 1024, value_parser = clap::value_parser!(u64).range(1..))]
    max_stream_bytes: u64,
    /// Maximum inflated bytes per continuation group (implementation resource budget).
    #[arg(long, default_value_t = 256 * 1024 * 1024)]
    max_group_bytes: usize,
    /// Maximum decoded values per object graph (implementation budget).
    #[arg(long, default_value_t = 100_000)]
    max_graph_values: usize,
    /// Maximum decoded objects per graph (independent implementation budget).
    #[arg(long, default_value_t = 100_000)]
    max_graph_objects: usize,
}

fn channel(value: &str) -> Result<u64, String> {
    match value {
        "101" => Ok(101),
        "102" => Ok(102),
        "103" => Ok(103),
        _ => Err("channel must be 101, 102, or 103".into()),
    }
}

#[derive(Serialize)]
struct Report<'a> {
    format: &'static str,
    status: &'static str,
    source: &'a Path,
    source_sha256: Option<String>,
    selected_ids: &'a [u64],
    channels: &'a [u64],
    max_stream_bytes: u64,
    max_group_bytes: usize,
    max_graph_values: usize,
    max_graph_objects: usize,
    emitted_records: usize,
    expected_selected_channel_records: Option<usize>,
    missing_selected_channel_records: Option<usize>,
    complete_document_geometry: bool,
    serialized_values_not_evaluated: bool,
    coverage: Option<Summary>,
    error: Option<String>,
}

fn source_hash(path: &Path) -> Result<String> {
    let mut file = File::open(path).context("open source for hashing")?;
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hash.update(&chunk[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn new_output(path: &Path) -> Result<BufWriter<File>> {
    Ok(BufWriter::new(
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .with_context(|| format!("create new output {}", path.display()))?,
    ))
}

fn run(args: Args) -> Result<u8> {
    anyhow::ensure!(
        args.max_group_bytes > 0,
        "group byte budget must be positive"
    );
    anyhow::ensure!(args.jsonl != args.summary, "output paths must be distinct");
    let mut summary_output = new_output(&args.summary)?;
    let mut report = Report {
        format: "rvt-native-document/v1",
        status: "failed",
        source: &args.file,
        source_sha256: None,
        selected_ids: &args.ids,
        channels: &args.channels,
        max_stream_bytes: args.max_stream_bytes,
        max_group_bytes: args.max_group_bytes,
        max_graph_values: args.max_graph_values,
        max_graph_objects: args.max_graph_objects,
        emitted_records: 0,
        expected_selected_channel_records: None,
        missing_selected_channel_records: None,
        complete_document_geometry: false,
        serialized_values_not_evaluated: true,
        coverage: None,
        error: None,
    };
    let result = (|| -> Result<Summary> {
        let mut output = new_output(&args.jsonl)?;
        report.source_sha256 = Some(source_hash(&args.file)?);
        let mut file = rvt::RevitFile::open(&args.file)?;
        let options = Options {
            selected_ids: args.ids.iter().copied().collect(),
            channels: args.channels.iter().copied().collect(),
            max_stream_bytes: args.max_stream_bytes,
            max_group_bytes: args.max_group_bytes,
            max_graph_values: args.max_graph_values,
            max_graph_objects: args.max_graph_objects,
        };
        let extraction = native_document::extract(&mut file, &options, |record| {
            serde_json::to_writer(&mut output, &record)?;
            output.write_all(b"\n")?;
            report.emitted_records += 1;
            Ok(())
        });
        output.flush().context("flush record JSONL")?;
        extraction
    })();
    let code = match result {
        Ok(coverage) => {
            let expected = coverage
                .selected_indexed_elements
                .checked_mul(args.channels.iter().collect::<BTreeSet<_>>().len())
                .context("selected record count overflow")?;
            report.expected_selected_channel_records = Some(expected);
            report.missing_selected_channel_records =
                Some(expected.saturating_sub(coverage.emitted_records));
            let complete = coverage.emitted_records > 0
                && coverage.emitted_records == expected
                && coverage.unsupported_graph_records == 0
                && coverage.requested_ids_absent_from_index.is_empty()
                && coverage.selected_ids_without_records.is_empty();
            report.status = if complete {
                "selected_graphs_complete"
            } else {
                "incomplete"
            };
            report.coverage = Some(coverage);
            if complete { 0 } else { 2 }
        }
        Err(error) => {
            report.error = Some(format!("{error:#}"));
            1
        }
    };
    serde_json::to_writer_pretty(&mut summary_output, &report)?;
    summary_output.write_all(b"\n")?;
    summary_output.flush().context("flush summary JSON")?;
    eprintln!(
        "{}: {} records; {}",
        report.status,
        report.emitted_records,
        report
            .error
            .as_deref()
            .unwrap_or("see summary for coverage")
    );
    Ok(code)
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("rvt-native-document: {error:#}");
            ExitCode::FAILURE
        }
    }
}
