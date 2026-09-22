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
//! rvt-gltf model.rvt                    # writes model.glb next to the input
//! rvt-gltf model.rvt -o out/model.glb --verbose
//! rvt-gltf --src model.rvt --dst model.glb   # older spelling, still accepted
//! ```

use clap::Parser;
use rvt::RevitFile;
use rvt::ifc::{Exporter, RvtDocExporter, gltf::model_to_glb};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-gltf",
    version,
    about = "Convert a Revit (.rvt, .rfa, .rte, .rft) file to a glTF 2.0 binary (.glb)",
    after_help = "Examples:\n  \
        rvt-gltf model.rvt                      writes model.glb next to the input\n  \
        rvt-gltf model.rvt -o viewer/model.glb --verbose"
)]
struct Cli {
    /// Revit file to convert (.rvt / .rfa / .rte / .rft).
    #[arg(
        value_name = "INPUT",
        required_unless_present = "src",
        conflicts_with = "src"
    )]
    input: Option<PathBuf>,

    /// Output .glb path. Default: the input path with a `.glb` extension.
    #[arg(short = 'o', long = "output", alias = "dst", short_alias = 'd')]
    output: Option<PathBuf>,

    /// Older spelling of INPUT (`--src model.rvt`), kept for scripts.
    #[arg(short = 's', long = "src", hide = true)]
    src: Option<PathBuf>,

    /// Print element count + output size on success.
    #[arg(long)]
    verbose: bool,
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

/// Input from INPUT or the legacy `--src`; output from `-o` or the input
/// path with `extension` swapped in. Refuses to overwrite the input.
fn resolve_paths(
    input: Option<&Path>,
    src: Option<&Path>,
    output: Option<&Path>,
    extension: &str,
) -> Result<(PathBuf, PathBuf), String> {
    let input = input.or(src).ok_or("no input file given")?.to_path_buf();
    let output = output
        .map(Path::to_path_buf)
        .unwrap_or_else(|| input.with_extension(extension));
    if output == input {
        return Err(format!(
            "refusing to overwrite the input {}; pass -o <path>",
            input.display()
        ));
    }
    Ok((input, output))
}

fn run(cli: &Cli) -> Result<(), String> {
    let (input, output) = resolve_paths(
        cli.input.as_deref(),
        cli.src.as_deref(),
        cli.output.as_deref(),
        "glb",
    )?;
    // Open errors from the library already name the file.
    let mut rf = RevitFile::open(&input).map_err(|e| e.to_string())?;
    let model = RvtDocExporter
        .export(&mut rf)
        .map_err(|e| format!("export {}: {e}", input.display()))?;
    let glb = model_to_glb(&model);
    fs::write(&output, &glb).map_err(|e| format!("write {}: {e}", output.display()))?;
    if cli.verbose {
        println!(
            "rvt-gltf: wrote {} bytes ({} building elements) to {}",
            glb.len(),
            model
                .entities
                .iter()
                .filter(|e| matches!(e, rvt::ifc::entities::IfcEntity::BuildingElement { .. }))
                .count(),
            output.display()
        );
    }
    Ok(())
}
