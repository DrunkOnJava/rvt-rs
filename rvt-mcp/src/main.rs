//! `rvt-rs-mcp`: a local, read-only Model Context Protocol server for Revit
//! files.
//!
//! It links the `rvt` crate in process and speaks MCP over stdio only: it
//! opens no socket, makes no network call, and writes only the export files a
//! tool is asked for. stdout carries nothing but JSON-RPC; logs go to stderr.
//! The server exits when its client closes stdin.

#![deny(clippy::print_stdout, clippy::print_stderr)]

mod args;
mod config;
mod exports;
mod output;
mod pipeline;
mod schema;
mod server;
mod store;
mod views;

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use rmcp::ServiceExt;
use rmcp::transport::stdio;
use rvt::walker::WalkerLimits;

use crate::config::Config;
use crate::server::RvtServer;
use crate::store::Store;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl LogLevel {
    fn level(self) -> tracing::Level {
        match self {
            Self::Error => tracing::Level::ERROR,
            Self::Warn => tracing::Level::WARN,
            Self::Info => tracing::Level::INFO,
            Self::Debug => tracing::Level::DEBUG,
            Self::Trace => tracing::Level::TRACE,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    version,
    about = "Local, read-only MCP server for Revit files (.rvt, .rfa, .rte, .rft), over stdio",
    after_help = "The server speaks MCP on stdin and stdout and nothing else: no socket, no\n\
        network call. Add it to an MCP client as a stdio server whose command is\n\
        rvt-rs-mcp (docs/mcp.md has configuration examples).\n\n\
        The limits are rvt-ifc's: a stream larger than --max-stream-bytes, or a\n\
        member that inflates past --max-inflate-bytes, is not read, and the tools\n\
        report it. --large-model raises them for large models."
)]
struct Args {
    /// Raise the stream, inflate and walker-scan limits for large models:
    /// streams up to 2 GiB, 8 GiB inflated. A model whose largest partition
    /// is 1 GiB stored needs several GB of memory.
    #[arg(long)]
    large_model: bool,
    /// Maximum bytes of one stream read from the file (default 256 MiB).
    #[arg(long, value_name = "BYTES")]
    max_stream_bytes: Option<u64>,
    /// Maximum inflated bytes of one compressed member (default 256 MiB).
    #[arg(long, value_name = "BYTES")]
    max_inflate_bytes: Option<usize>,
    /// Maximum decompressed Global/Latest bytes scanned by the walker.
    #[arg(long, value_name = "BYTES")]
    max_walker_scan_bytes: Option<usize>,
    /// Maximum schema-scan candidates retained by the walker.
    #[arg(long, value_name = "N")]
    max_walker_candidates: Option<usize>,
    /// Maximum trial decodes attempted by the walker.
    #[arg(long, value_name = "N")]
    max_walker_trial_offsets: Option<usize>,
    /// Maximum bytes inspected while decoding one walker candidate.
    #[arg(long, value_name = "BYTES")]
    max_walker_record_decode_bytes: Option<usize>,
    /// Maximum records accepted in walker reference containers.
    #[arg(long, value_name = "N")]
    max_walker_container_records: Option<usize>,
    /// How many decoded models the server keeps; opening another drops the
    /// least recently used one. A large model peaks at several GB.
    #[arg(long, value_name = "N", default_value_t = 2)]
    max_open_models: usize,
    /// Write exports only inside this folder; a relative out_path is taken
    /// from it.
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// How much the server logs to stderr.
    #[arg(long, value_enum, default_value = "warn")]
    log_level: LogLevel,
}

impl Args {
    fn config(&self) -> Result<Config, String> {
        let defaults = WalkerLimits::default();
        let walker_limits = WalkerLimits {
            max_scan_bytes: self.max_walker_scan_bytes.unwrap_or(if self.large_model {
                rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES
            } else {
                defaults.max_scan_bytes
            }),
            max_candidates: self
                .max_walker_candidates
                .unwrap_or(defaults.max_candidates),
            max_trial_offsets: self
                .max_walker_trial_offsets
                .unwrap_or(defaults.max_trial_offsets),
            max_per_record_decode_bytes: self
                .max_walker_record_decode_bytes
                .unwrap_or(defaults.max_per_record_decode_bytes),
            max_container_records: self
                .max_walker_container_records
                .unwrap_or(defaults.max_container_records),
        };
        let output_dir = match &self.output_dir {
            Some(dir) => Some(
                std::fs::canonicalize(dir)
                    .map_err(|error| format!("--output-dir {}: {error}", dir.display()))?,
            ),
            None => None,
        };
        Ok(Config {
            open_limits: rvt::cli::open_limits(
                self.max_stream_bytes,
                self.max_inflate_bytes,
                self.large_model,
            ),
            walker_limits,
            max_open_models: self.max_open_models.max(1),
            output_dir,
        })
    }
}

fn main() -> ExitCode {
    let args = Args::parse();
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .with_max_level(args.log_level.level())
        .init();
    let config = match args.config() {
        Ok(config) => config,
        Err(message) => {
            tracing::error!("{message}");
            return ExitCode::from(2);
        }
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            tracing::error!("cannot start the async runtime: {error}");
            return ExitCode::from(1);
        }
    };
    let outcome = runtime.block_on(serve(config));
    // A decode still running on the blocking pool must not keep the process
    // alive once the client has gone.
    runtime.shutdown_background();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            tracing::error!("{message}");
            ExitCode::from(1)
        }
    }
}

/// Serve MCP on stdio until the client closes it.
async fn serve(config: Config) -> Result<(), String> {
    tracing::info!("rvt-rs-mcp {} serving on stdio", env!("CARGO_PKG_VERSION"));
    let server = RvtServer::new(Arc::new(Store::new(config)));
    let running = server
        .serve(stdio())
        .await
        .map_err(|error| format!("cannot start the MCP server: {error}"))?;
    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| format!("the MCP server stopped: {error}"))
}
