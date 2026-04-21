//! `rvt-sheet` — render a 2D plan view of a Revit file as SVG.
//!
//! Fourth shipping binary in the rvt-rs toolkit. Wraps the VW1-11
//! sheet renderer. Useful for:
//!
//! - embedding a plan in a markdown README or a PDF report
//! - quick visual sanity check after `rvt-write` edits
//! - any workflow that wants a picture without a full 3D viewer
//!
//! Usage:
//!
//! ```bash
//! rvt-sheet --src input.rfa --dst plan.svg
//! rvt-sheet --src input.rfa --dst plan.svg --width 2000 --height 1500
//! rvt-sheet --src input.rfa --dst plan.svg --no-labels --no-background
//! ```

use clap::Parser;
use rvt::RevitFile;
use rvt::ifc::{
    Exporter, RvtDocExporter,
    sheet::{SheetOptions, render_plan_svg},
};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "rvt-sheet",
    version,
    about = "Render a 2D plan view of a Revit file as SVG"
)]
struct Cli {
    /// Source Revit file path.
    #[arg(short, long)]
    src: PathBuf,

    /// Destination .svg file path.
    #[arg(short, long)]
    dst: PathBuf,

    /// Output width in pixels.
    #[arg(long, default_value_t = 1200)]
    width: u32,

    /// Output height in pixels.
    #[arg(long, default_value_t = 800)]
    height: u32,

    /// Interior margin in pixels.
    #[arg(long, default_value_t = 40.0)]
    margin: f32,

    /// Omit element name labels (useful for dense plans).
    #[arg(long)]
    no_labels: bool,

    /// Omit the white background (emit a transparent SVG).
    #[arg(long)]
    no_background: bool,

    /// Print element count + output size on success.
    #[arg(long)]
    verbose: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rvt-sheet: {e}");
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
    let options = SheetOptions {
        width_px: cli.width,
        height_px: cli.height,
        margin_px: cli.margin,
        show_labels: !cli.no_labels,
        background: if cli.no_background {
            None
        } else {
            Some("#FFFFFF".into())
        },
    };
    let svg = render_plan_svg(&model, &options);
    fs::write(&cli.dst, svg.as_bytes()).map_err(|e| format!("write {}: {e}", cli.dst.display()))?;
    if cli.verbose {
        let drawn = model
            .entities
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    rvt::ifc::entities::IfcEntity::BuildingElement {
                        location_feet: Some(_),
                        extrusion: Some(_),
                        ..
                    }
                )
            })
            .count();
        println!(
            "rvt-sheet: wrote {} bytes ({} drawn elements) to {}",
            svg.len(),
            drawn,
            cli.dst.display()
        );
    }
    Ok(())
}
