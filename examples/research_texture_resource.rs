//! Decode an explicitly supplied texture resource; never follows saved paths.
use anyhow::{Result, ensure};
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    ensure!(
        a.len() == 3,
        "usage: research_texture_resource IMAGE NEW_OUTPUT_JSON"
    );
    let image = rvt::native_texture::TextureImage::from_encoded(&std::fs::read(&a[1])?)?;
    serde_json::to_writer_pretty(
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&a[2])?,
        &image,
    )?;
    Ok(())
}
