//! Native saved geometry with current identity, material and source provenance.
use anyhow::{Result, ensure};
use clap::Parser;
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};
#[derive(Parser)]
#[command(
    name = "rvt-native-saved-scene",
    about = "Extract current saved RVT meshes and material provenance without vendor runtime"
)]
struct Args {
    file: PathBuf,
    #[arg(long)]
    json: PathBuf,
    #[arg(long)]
    glb: Option<PathBuf>,
    #[arg(long, value_delimiter = ',')]
    ids: Vec<u64>,
    #[arg(long, default_value_t = 1_000_000)]
    max_graph_values: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_graph_objects: usize,
    #[arg(long,default_value_t=3,value_parser=clap::value_parser!(i64).range(1..=3))]
    detail_level: i64,
}
fn main() -> Result<()> {
    let a = Args::parse();
    ensure!(!a.json.exists(), "JSON output exists");
    if let Some(p) = &a.glb {
        ensure!(!p.exists(), "GLB output exists")
    }
    let source_sha256 = format!("{:x}", Sha256::digest(std::fs::read(&a.file)?));
    let mut f = rvt::RevitFile::open(&a.file)?;
    let opts = rvt::native_document::Options {
        selected_ids: a.ids.into_iter().collect(),
        max_graph_values: a.max_graph_values,
        max_graph_objects: a.max_graph_objects,
        ..Default::default()
    };
    let mut scene = rvt::native_saved_scene::extract_at_detail(&mut f, &opts, a.detail_level)?;
    scene.source_sha256 = Some(source_sha256.clone());
    let mut out = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(a.json)?;
    serde_json::to_writer(
        &mut out,
        &serde_json::json!({"source_sha256":source_sha256,"scene":scene}),
    )?;
    out.flush()?;
    if let Some(path) = a.glb {
        let bytes = rvt::native_saved_glb::encode(&scene, &|e, i, _p| {
            let m = e.render_materials.get(&i)?;
            if !m.diagnostics.is_empty() {
                return None;
            }
            Some(
                serde_json::json!({"name":m.name,"pbrMetallicRoughness":{"baseColorFactor":m.base_color,"metallicFactor":m.metallic_factor,"roughnessFactor":m.roughness_factor},"extras":{"native_material_id":m.material_id,"saved_opacity":m.saved_opacity}}),
            )
        })?;
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(path)?
            .write_all(&bytes)?;
    }
    eprintln!(
        "{} owners, {} mesh primitives; inspect diagnostics and material coverage",
        scene.elements.len(),
        scene
            .elements
            .iter()
            .map(|e| e.meshes.primitives.len())
            .sum::<usize>()
    );
    Ok(())
}
