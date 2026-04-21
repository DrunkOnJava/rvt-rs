//! `rvt-gltf` — convert a Revit file to a glTF 2.0 binary (.glb).
//!
//! VW1-16 CLI wrapping the VW1-04 exporter. Reads `src`, runs the
//! `RvtDocExporter` to produce an `IfcModel` (project metadata +
//! whatever per-element info the walker can resolve), then emits
//! the glb via `ifc::gltf::model_to_glb`.
//!
//! Usage:
//!
//! ```bash
//! rvt-gltf --src input.rfa --dst output.glb
//! rvt-gltf --src input.rfa --dst output.glb --verbose
//! ```

use clap::Parser;
use rvt::RevitFile;
use rvt::ifc::{Exporter, RvtDocExporter, gltf::model_to_glb};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-gltf",
    version,
    about = "Convert a Revit (.rvt, .rfa, .rte, .rft) file to a glTF 2.0 binary (.glb)"
)]
struct Cli {
    /// Source Revit file path.
    #[arg(short, long)]
    src: PathBuf,

    /// Destination .glb file path.
    #[arg(short, long)]
    dst: PathBuf,

    /// Print element count + output size on success.
    #[arg(long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rvt-gltf: {e}");
            ExitCode::from(1)
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let mut rf =
        RevitFile::open(&cli.src).map_err(|e| format!("open {}: {e}", cli.src.display()))?;
    let model = RvtDocExporter
        .export(&mut rf)
        .map_err(|e| format!("export: {e}"))?;
    let glb = model_to_glb(&model);
    fs::write(&cli.dst, &glb).map_err(|e| format!("write {}: {e}", cli.dst.display()))?;
    if cli.verbose {
        println!(
            "rvt-gltf: wrote {} bytes ({} building elements) to {}",
            glb.len(),
            model
                .entities
                .iter()
                .filter(|e| matches!(e, rvt::ifc::entities::IfcEntity::BuildingElement { .. }))
                .count(),
            cli.dst.display()
        );
    }
    Ok(())
}
