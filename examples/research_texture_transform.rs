//! Reproduce explicit planar transform-constructor inputs; no vendor runtime.
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    ensure!(
        a.len() == 3,
        "usage: research_texture_transform INPUT_JSON NEW_OUTPUT_JSON"
    );
    let v: Value = serde_json::from_slice(&std::fs::read(&a[1])?)?;
    let mut cases = Vec::new();
    for c in v["cases"].as_array().context("cases absent")? {
        let output = (|| -> Result<Value> {
            let r: [f64; 3] = serde_json::from_value(c["rotation"].clone())?;
            ensure!(r[0] == 0. && r[1] == 0., "nonplanar rotation unqualified");
            let m = rvt::native_texture::planar_transform(
                serde_json::from_value(c["translation"].clone())?,
                r[2],
                serde_json::from_value(c["scale"].clone())?,
            )?;
            Ok(json!((0..16).map(|i| m[i % 4][i / 4]).collect::<Vec<_>>()))
        })();
        cases.push(match output {
            Ok(m) => json!({"name":c["name"],"raw_matrix_f32":m}),
            Err(e) => json!({"name":c["name"],"refusal":format!("{e:#}")}),
        });
    }
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[2])?,
        &json!({"cases":cases}),
    )?;
    Ok(())
}
