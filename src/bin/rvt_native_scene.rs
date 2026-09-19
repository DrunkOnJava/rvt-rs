//! Rust-only native RVT geometry subset extraction.
use clap::Parser;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
#[derive(Parser)]
#[command(
    name = "rvt-native-scene",
    about = "Extract a validated native geometry subset to JSON and GLB; unsupported features are explicit"
)]
struct Args {
    file: PathBuf,
    #[arg(long)]
    json: PathBuf,
    #[arg(long)]
    glb: Option<PathBuf>,
}
fn source_hash(path: &Path) -> anyhow::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hash.update(&chunk[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let mut file = rvt::RevitFile::open(&args.file)?;
    let mut scene = rvt::native_scene::extract(&mut file)?;
    scene.source_sha256 = Some(source_hash(&args.file)?);
    std::fs::write(&args.json, serde_json::to_vec_pretty(&scene)?)?;
    if let Some(path) = args.glb {
        std::fs::write(path, scene.to_glb()?)?;
    }
    let meshes: usize = scene.elements.iter().map(|e| e.meshes.len()).sum();
    eprintln!(
        "{}: {} native elements, {} meshes; document geometry incomplete (see JSON diagnostics)",
        scene.status,
        scene.elements.len(),
        meshes
    );
    if meshes == 0 {
        std::process::exit(2);
    }
    Ok(())
}
