//! Research extraction preserving stream names, member boundaries, and failed candidates.
//! cargo run --release --example oracle_stream_dump -- MODEL.rvt NEW_OUTPUT_DIR
use rvt::{RevitFile, compression};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "expected MODEL.rvt NEW_OUTPUT_DIR");
    let out = Path::new(&args[2]);
    anyhow::ensure!(!out.exists(), "output directory already exists");
    let mut rf = RevitFile::open(&args[1])?;
    let version = rf.basic_file_info()?.version;
    fs::create_dir_all(out)?;
    let mut streams = Vec::new();
    let mut names = rf.stream_names();
    names.sort();
    for (index, name) in names.iter().enumerate() {
        let raw = rf.read_stream(name)?;
        let raw_path = format!("stream-{index:04}.raw");
        fs::write(out.join(&raw_path), &raw)?;
        let prepared = compression::prepare_stream_for_inflate(name, &raw);
        let mut members = Vec::new();
        for (candidate, offset) in compression::find_gzip_offsets(&prepared).iter().enumerate() {
            match compression::inflate_at(&prepared, *offset) {
                Ok(bytes) => {
                    let path = format!("stream-{index:04}-candidate-{candidate:04}.inflated");
                    fs::write(out.join(&path), &bytes)?;
                    members.push(json!({"prepared_offset": offset, "status": "inflated", "path": path, "length": bytes.len(), "sha256": hash(&bytes)}));
                }
                Err(error) => members.push(json!({"prepared_offset": offset, "status": "failed", "error": error.to_string()})),
            }
        }
        streams.push(json!({"name": name, "raw_path": raw_path, "raw_length": raw.len(), "raw_sha256": hash(&raw), "prepared_length": prepared.len(), "members": members}));
    }
    let manifest = json!({"schema_version": 1, "source": args[1], "source_sha256": hash(&fs::read(&args[1])?), "revit_version": version, "offset_space": "checksum-prepared stream; inflated offsets are member-local", "note": "Gzip candidates are observations, not validated stream segmentation. No synthetic separators or inferred ownership.", "streams": streams});
    fs::write(
        out.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    println!("{}", out.join("manifest.json").display());
    Ok(())
}
