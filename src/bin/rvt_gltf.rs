//! `rvt-gltf` — convert a Revit file to a glTF 2.0 binary (.glb).
//!
//! The command-line front end of `rvt::ifc::gltf`. Reads `src`, runs the
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
use rvt::ifc::{ExportQualityMode, RvtDocExporter, gltf::model_to_glb};
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
        rvt-gltf model.rvt -o viewer/model.glb --verbose\n\n\
        Exit status 3: a resource limit kept part of the file from being read, so\n\
        no glb was written. Re-run with --large-model, or --allow-partial to write\n\
        it anyway."
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

    /// Draw every element whose body would be its record's bounding box
    /// (furniture, fixtures, equipment and the rest) from the mesh Revit
    /// saved for display (#255). Doors and windows are drawn so by default
    /// on Revit 2023 to 2027. Slower on large files.
    #[arg(long, conflicts_with = "no_saved_meshes")]
    saved_meshes: bool,

    /// Keep doors and windows as their record's bounding box instead of the
    /// mesh Revit saved for them.
    #[arg(long)]
    no_saved_meshes: bool,

    /// Print progress to stderr as the conversion goes
    /// (`progress: <stage> <done>/<total>`, as `rvt-elements --progress`).
    #[arg(long)]
    progress: bool,

    /// Maximum bytes of one stream read from the file (default 256 MiB). A
    /// larger stream is not read, and nothing is written unless
    /// `--allow-partial` (#1075).
    #[arg(long, value_name = "BYTES")]
    max_stream_bytes: Option<u64>,

    /// Maximum inflated bytes of one compressed member (default 256 MiB). The
    /// members of one stream may inflate to this or 1 GiB together,
    /// whichever is larger.
    #[arg(long, value_name = "BYTES")]
    max_inflate_bytes: Option<usize>,

    /// Raise the stream, inflate and walker-scan limits for large models:
    /// streams up to 2 GiB, 8 GiB inflated. A model whose largest partition
    /// is 1 GiB stored needs several GB of memory.
    #[arg(long)]
    large_model: bool,

    /// Write the glb even when a resource limit kept part of the file from
    /// being read.
    #[arg(long)]
    allow_partial: bool,
}

/// Why a conversion stopped.
enum Failure {
    /// Exit status 1.
    Error(String),
    /// A resource limit kept part of the input from being read and
    /// `--allow-partial` was not given (#1075): exit status 3.
    PartialRead,
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Error(message)
    }
}

fn main() -> ExitCode {
    rvt::cli::exit_quietly_on_broken_pipe();
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(Failure::Error(e)) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
        Err(Failure::PartialRead) => {
            eprintln!("error: no glb written: part of the input was not read (see above)");
            ExitCode::from(3)
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

fn run(cli: &Cli) -> Result<(), Failure> {
    let (input, output) = resolve_paths(
        cli.input.as_deref(),
        cli.src.as_deref(),
        cli.output.as_deref(),
        "glb",
    )?;
    // Open errors from the library already name the file.
    let limits =
        rvt::cli::open_limits(cli.max_stream_bytes, cli.max_inflate_bytes, cli.large_model);
    let mut rf = RevitFile::open_with_limits(&input, limits).map_err(|e| e.to_string())?;
    let mut walker_limits = rvt::walker::WalkerLimits::default();
    if cli.large_model {
        walker_limits.max_scan_bytes = rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES;
    }
    let control = if cli.progress {
        rvt::control::WalkerControl::new().with_progress(|event| {
            let total = event.total.map(|t| format!("/{t}")).unwrap_or_default();
            eprintln!("progress: {} {}{total}", event.stage.as_str(), event.done);
        })
    } else {
        rvt::control::WalkerControl::new()
    };
    let mut model = RvtDocExporter
        .export_with_mode_limits_and_control(
            &mut rf,
            ExportQualityMode::Scaffold,
            walker_limits,
            &control,
        )
        .map_err(|e| format!("export {}: {e}", input.display()))?;
    let scope = if cli.no_saved_meshes {
        None
    } else if cli.saved_meshes {
        Some(rvt::ifc::saved_meshes::SavedMeshScope::All)
    } else {
        // Doors and windows by default (the maintainer, 2026-10-29), on the
        // releases whose saved graphics are read.
        rf.basic_file_info()
            .ok()
            .filter(|info| (2023..=2027).contains(&info.version))
            .map(|_| rvt::ifc::saved_meshes::SavedMeshScope::DoorsAndWindows)
    };
    if let Some(scope) = scope {
        if let Err(e) =
            rvt::ifc::saved_meshes::attach_scoped_with_control(&mut rf, &mut model, scope, &control)
        {
            eprintln!("rvt-gltf: saved graphics not read, bodies unchanged: {e}");
        }
    }
    let unread = rf.unread_streams().len();
    if unread > 0 {
        eprintln!(
            "rvt-gltf: {unread} stream(s) not read whole, so the model is missing what they hold:"
        );
        for line in rvt::cli::unread_stream_report(rf.unread_streams()) {
            eprintln!("{line}");
        }
        if !cli.allow_partial {
            return Err(Failure::PartialRead);
        }
        eprintln!("rvt-gltf: writing the incomplete model (--allow-partial)");
    }
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
