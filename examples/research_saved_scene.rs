//! Current saved geometry, extracted directly from RVT with the Rust library.
use anyhow::{Result, ensure};
use std::io::Write;
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 3 || a.len() == 4, "FILE NEW_JSONL [ID,ID]");
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[2])?;
    let mut opts = rvt::native_document::Options {
        max_graph_values: 1_000_000,
        ..Default::default()
    };
    if a.len() == 4 {
        opts.selected_ids = a[3]
            .split(',')
            .map(str::parse)
            .collect::<std::result::Result<_, _>>()?
    }
    let mut file = rvt::RevitFile::open(&a[1])?;
    let scene = rvt::native_saved_scene::extract(&mut file, &opts)?;
    for element in scene.elements {
        let mut row = serde_json::to_value(&element)?;
        row["units"] = serde_json::json!(scene.units);
        row["coordinate_frame"] = serde_json::json!(scene.coordinate_frame);
        serde_json::to_writer(&mut out, &row)?;
        writeln!(&mut out)?;
    }
    eprintln!("{}", serde_json::to_string(&scene.summaries)?);
    Ok(())
}
