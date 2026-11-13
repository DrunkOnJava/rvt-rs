//! `rvt-rs-mcp`: a local, read-only Model Context Protocol server for Revit
//! files.
//!
//! It links the `rvt` crate in process and speaks MCP over stdio only: it
//! opens no socket, makes no network call, and writes only the export files a
//! tool is asked for. stdout carries nothing but JSON-RPC; logs go to stderr.
//! The server exits when its client closes stdin.

#![deny(clippy::print_stdout, clippy::print_stderr)]
// A failed tool call is an `Ok` protocol response whose `CallToolResult` has
// `isError` set, which rmcp's tool signatures take as the `Err` side; it is
// built once per failed call and never travels far.
#![allow(clippy::result_large_err)]

mod args;
mod bounds;
mod config;
mod exports;
mod gate;
mod output;
mod root;
mod schema;
mod server;
mod store;
mod views;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use rmcp::ServiceExt;
use rmcp::service::ServerInitializeError;
use rvt::walker::WalkerLimits;

use crate::config::Config;
use crate::gate::Gate;
use crate::root::{self, Reach};
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
        report it. --large-model raises them for large models.\n\n\
        An answer is bounded too: a name or value from the file longer than\n\
        --max-field-chars is cut, and an answer longer than --max-answer-bytes is\n\
        refused, so a crafted file cannot make a tool return without end.\n\n\
        Files are read and exports written inside one folder: --root (or\n\
        RVT_MCP_ROOT), else the folder the server is started in when that is not\n\
        too broad (`/`, the home folder, a folder above it). A path that leaves it,\n\
        by .. or a link, is refused, so text in a crafted file cannot steer the\n\
        client to other files. --unconfined lifts the limit; --output-dir moves\n\
        the exports elsewhere."
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
    /// least recently used one. A large model peaks at several GB, so this
    /// is also how many files are read and how many models decoded at once.
    #[arg(long, value_name = "N", default_value_t = 2)]
    max_open_models: usize,
    /// The most characters of one name or value from a file that an answer
    /// carries (a message, or a string of the diagnostics, eight times that);
    /// more are cut, and the cut says how many (default 1000).
    #[arg(long, value_name = "N")]
    max_field_chars: Option<usize>,
    /// The most bytes of JSON one answer may carry, counted once (the wire
    /// message carries it twice); a longer answer is refused with a message
    /// that says how to ask for less (default 4 MiB).
    #[arg(long, value_name = "BYTES")]
    max_answer_bytes: Option<usize>,
    /// The one folder models are read from and exports written to; a relative
    /// path is taken from it, and a path that leaves it, by `..` or a link, is
    /// refused. Also RVT_MCP_ROOT, for a client that passes only an
    /// environment. With neither, the folder the server is started in, unless
    /// that is the top of a file system, the home folder or a folder that holds
    /// it: then every tool that touches a file refuses until one is given.
    #[arg(long, value_name = "DIR", conflicts_with = "unconfined")]
    root: Option<PathBuf>,
    /// Lift the root: read any file the account can read, and write exports
    /// anywhere (inside --output-dir when one is given).
    #[arg(long)]
    unconfined: bool,
    /// Write exports only inside this folder, which need not be inside the
    /// root; a relative out_path is taken from it.
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
        let max_open_models = self.max_open_models.max(1);
        let reach = Reach::choose(
            self.unconfined,
            self.root.as_deref(),
            std::env::var_os(root::ROOT_ENV).as_deref(),
            std::env::current_dir(),
            &root::home_folders(),
        )?;
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
            max_open_models,
            reach,
            output_dir,
            max_answer_bytes: self
                .max_answer_bytes
                .unwrap_or(bounds::DEFAULT_ANSWER_BYTES),
            gate: Arc::new(Gate::new(max_open_models)),
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
    bounds::set_field_chars(args.max_field_chars.unwrap_or(bounds::DEFAULT_FIELD_CHARS));
    let config = match args.config() {
        Ok(config) => config,
        Err(message) => {
            tracing::error!("{message}");
            return ExitCode::from(2);
        }
    };
    match config.reach.limit() {
        Ok(Some(root)) => tracing::info!(
            "reads models from and writes exports to {} only",
            root.dir().display()
        ),
        Ok(None) => {
            tracing::info!("--unconfined: models are read from and exports written anywhere")
        }
        // The server starts, and each tool that touches a file says the same.
        Err(message) => tracing::warn!("{message}"),
    }
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
    // Every reply is a newline-terminated line, which std's stdout writes
    // through; flushing anyway keeps that from depending on it.
    let _ = std::io::stdout().flush();
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            tracing::error!("{message}");
            ExitCode::from(1)
        }
    }
}

/// Serve MCP on stdio until the client closes it. Closing stdin at any point,
/// before a session or during one, is a clean exit.
async fn serve(config: Config) -> Result<(), String> {
    tracing::info!("rvt-rs-mcp {} serving on stdio", env!("CARGO_PKG_VERSION"));
    let server = RvtServer::new(Arc::new(Store::new(config)));
    // stdin and stdout as one transport: rmcp's `stdio()` is this pair, but
    // lives behind the `transport-io` feature, which turns on tokio's `net`.
    let running = match server
        .serve((tokio::io::stdin(), tokio::io::stdout()))
        .await
    {
        Ok(running) => running,
        // `server/discover` and unsupported-version replies are answered before
        // a session exists, so a client that only probes (the release smoke
        // test) and closes stdin is leaving, not failing to start.
        Err(ServerInitializeError::ConnectionClosed(stage)) => {
            tracing::info!("stdin closed before a session began ({stage})");
            return Ok(());
        }
        Err(error) => return Err(format!("cannot start the MCP server: {error}")),
    };
    running
        .waiting()
        .await
        .map(|_| ())
        .map_err(|error| format!("the MCP server stopped: {error}"))
}
