//! Extract raw saved graphics UV maps without an API sidecar or vendor runtime.
use anyhow::{Result, ensure};
use std::collections::BTreeSet;
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    ensure!(
        a.len() == 4,
        "usage: research_texture_mapping RVT ID,ID NEW_OUTPUT_JSON"
    );
    let ids = a[2]
        .split(',')
        .map(str::parse)
        .collect::<std::result::Result<BTreeSet<u64>, _>>()?;
    let mut file = rvt::RevitFile::open(&a[1])?;
    let output = rvt::native_texture_mapping::read(&mut file, ids)?;
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[3])?,
        &output,
    )?;
    Ok(())
}
