//! Inspect bounded Global/Latest graph framing and schema storage; research only.
use anyhow::{Result, ensure};
use rvt::{native_parameters, schema_registry};
use std::{fs, path::PathBuf};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(a.len() == 4, "CAPTURE_DIR NEW_OUTPUT_JSON TRAILER_BYTES");
    let p = PathBuf::from(&a[1]);
    let (b, schema) = if p.is_file() {
        let mut file = rvt::RevitFile::open(&p)?;
        (
            rvt::native_document::read_single(&mut file, "Global/Latest")?,
            rvt::native_document::read_single(&mut file, "Formats/Latest")?,
        )
    } else {
        (
            fs::read(p.join("global-latest.bin"))?,
            fs::read(p.join("schema.bin"))?,
        )
    };
    let trailer: usize = a[3].parse()?;
    ensure!(trailer <= b.len(), "trailer beyond body");
    let r = schema_registry::parse(&schema)?;
    let g = native_parameters::decode_graph_with_limits(
        &b[..b.len() - trailer],
        &r,
        &native_parameters::GraphLimits {
            max_values: 2_000_000,
            ..Default::default()
        },
    );
    let result = serde_json::json!({"global_bytes":b.len(),"excluded_research_trailer": &b[b.len()-trailer..],"graph":g.as_ref().ok(),"error":g.as_ref().err().map(|e|format!("{e:#}")),"scope":"Research framing hypothesis; no production truncation permitted"});
    let f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[2])?;
    serde_json::to_writer(f, &result)?;
    Ok(())
}
