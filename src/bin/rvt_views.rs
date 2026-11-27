//! `rvt-views` — list a project's views, sheets and the viewports placed on
//! them, as JSON.
//!
//! Reads the same native records the viewer and the probes read (#1117):
//! every `DBView*` record, the `Viewer` each names, and every `Viewport`
//! record. A placed viewport's sheet and view are the ids its record names;
//! its `centre` is `m_offset + m_scale * crop centre`, the rule measured on
//! one viewport against `Viewport.GetBoxCenter()` (#1117). See
//! `docs/views.md` for what is and is not measured.
//!
//! It reads `rvt::research::views`, so it builds only with the
//! `unstable-research` feature (`cargo install rvt --features
//! unstable-research`), is left out of the release archives, and is not one of
//! the CLIs RELEASE.md keeps stable.

use clap::Parser;
use rvt::RevitFile;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-views",
    version,
    about = "List views, sheets and placed viewports as JSON",
    after_help = "Examples:\n  \
        rvt-views model.rvt\n  \
        rvt-views large.rvt --large-model > views.json"
)]
struct Cli {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    file: PathBuf,

    /// Open with the large-model limits (`rvt-ifc --large-model`'s).
    #[arg(long)]
    large_model: bool,
}

fn main() -> std::process::ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let mut file = RevitFile::open_with_limits(
        &cli.file,
        rvt::cli::open_limits(None, None, cli.large_model),
    )?;
    let catalog = rvt::research::views::read(&mut file)?;
    println!("{}", serde_json::to_string_pretty(&catalog)?);
    Ok(())
}
