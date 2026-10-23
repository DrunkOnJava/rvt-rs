//! Export a Revit file as IFC4, glTF and a CSV element schedule, and print
//! how many entities the model holds.
//!
//! ```sh
//! cargo run --example basics_export -- path/to/project.rvt out/
//! ```
//!
//! Without arguments it exports the synthetic project file the repository
//! ships, `corpus/tier1/architectural-2024/architectural-2024.rvt`, into the
//! system's temporary directory.

use std::path::PathBuf;

use rvt::RevitFile;
use rvt::ifc::schedule_csv::{CsvOptions, elements_csv};
use rvt::ifc::{RvtDocExporter, gltf, write_step};

const FIXTURE: &str = "corpus/tier1/architectural-2024/architectural-2024.rvt";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| FIXTURE.to_string());
    let out: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&out)?;

    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let model = &result.model;
    std::fs::write(out.join("model.ifc"), write_step(model))?;
    std::fs::write(out.join("model.glb"), gltf::model_to_glb(model))?;
    std::fs::write(
        out.join("elements.csv"),
        elements_csv(model, &CsvOptions::default()),
    )?;
    std::fs::write(
        out.join("diagnostics.json"),
        serde_json::to_string_pretty(&result.diagnostics)?,
    )?;

    println!(
        "{} entities written to {}",
        model.entities.len(),
        out.display()
    );
    Ok(())
}
