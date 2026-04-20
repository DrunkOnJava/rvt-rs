//! `rvt-ifc` — convert a Revit file to IFC4.
//!
//! This is the first end-to-end `rvt → ifc` pipeline. Current scope:
//! document-level metadata export (IfcProject, framework entities,
//! units, geometric context). Element-level geometry is pending walker
//! expansion; this command's output is spec-valid IFC4 that readers
//! (IfcOpenShell, BlenderBIM) accept — just sparse until more of the
//! schema is walked.

use clap::Parser;
use rvt::{
    RevitFile,
    ifc::{Exporter, PlaceholderExporter, RvtDocExporter, write_step},
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Convert a Revit file to IFC4 (document-level export)"
)]
struct Args {
    /// Path to a `.rvt` / `.rfa` / `.rte` / `.rft` file.
    input: PathBuf,
    /// Output path. If omitted, writes `<input>.ifc` next to the input.
    #[arg(short, long)]
    output: Option<PathBuf>,
    /// Use the placeholder exporter (empty project body) instead of the
    /// document exporter. Mostly useful for testing the STEP writer.
    /// Kept as `--null` for backward compatibility with earlier versions.
    #[arg(long, alias = "null")]
    placeholder: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut rf = RevitFile::open(&args.input)?;

    let model = if args.placeholder {
        PlaceholderExporter.export(&mut rf)?
    } else {
        RvtDocExporter.export(&mut rf)?
    };

    let step = write_step(&model);

    let out_path = args.output.clone().unwrap_or_else(|| {
        let mut p = args.input.clone();
        p.set_extension("ifc");
        p
    });
    std::fs::write(&out_path, step.as_bytes())?;
    eprintln!(
        "rvt-ifc: wrote {} bytes to {}",
        step.len(),
        out_path.display()
    );
    if model.project_name.is_none() {
        eprintln!(
            "note: no project name extracted; output IFC uses \"Untitled\". \
             Title typically comes from PartAtom — this file may not have one."
        );
    }
    Ok(())
}
