//! Research replay of bounded current-record graphs; source provenance retained.
use anyhow::{Result, ensure};
use std::io::{BufRead, Write};
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().collect();
    ensure!(
        a.len() == 4,
        "CURRENT_GRAPHS SELECTED_MESH_JSONL NEW_OUTPUT"
    );
    let ids = std::io::BufReader::new(std::fs::File::open(&a[2])?)
        .lines()
        .map(|l| -> Result<u64> {
            Ok(serde_json::from_str::<serde_json::Value>(&l?)?["id"]
                .as_u64()
                .unwrap())
        })
        .collect::<Result<std::collections::BTreeSet<_>>>()?;
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[3])?;
    for l in std::io::BufReader::new(std::fs::File::open(&a[1])?).lines() {
        let r: serde_json::Value = serde_json::from_str(&l?)?;
        let id = r["identity"]["element_id"].as_u64().unwrap();
        if !ids.contains(&id) {
            continue;
        }
        let g: rvt::native_parameters::ObjectGraph = serde_json::from_value(r["graph"].clone())?;
        serde_json::to_writer(
            &mut out,
            &serde_json::json!({"id":id,"identity":r["identity"],"source":r["source"],"status":r["status"],"meshes":rvt::native_saved_mesh::graphics(&g)}),
        )?;
        writeln!(&mut out)?;
    }
    Ok(())
}
