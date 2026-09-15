//! Read-only native index audit; raw row fields are evidence, not parsed owners.
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();
    anyhow::ensure!(args.len() == 3, "MODEL.rvt OUTPUT_DIRECTORY");
    let model = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let mut file = rvt::RevitFile::open(&model)?;
    let name = "Global/ElemTable";
    let raw = file.read_stream(name)?;
    let prepared = rvt::compression::prepare_stream_for_inflate(name, &raw);
    let offsets = rvt::compression::find_gzip_offsets(&prepared);
    anyhow::ensure!(offsets.len() == 1, "index segmentation unsupported");
    let bytes = rvt::compression::inflate_at(&prepared, offsets[0])?;
    std::fs::write(out.join("ElemTable.inflated"), &bytes)?;
    let u32at = |p: usize| u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap());
    anyhow::ensure!(bytes.len() >= 30, "short index");
    let count = u32at(2) as usize;
    anyhow::ensure!(
        count.checked_mul(40).and_then(|n| n.checked_add(30)) == Some(bytes.len()),
        "not exact 40byte table"
    );
    let mut seen = HashMap::new();
    let mut duplicate_count = 0;
    let mut invalid_identity = 0;
    let mut duplicate_samples = Vec::new();
    let row = |p: usize| (0..10).map(|i| u32at(p + i * 4)).collect::<Vec<_>>();
    for (i, b) in bytes[30..].chunks_exact(40).enumerate() {
        let p = 30 + i * 40;
        let id = u64::from_le_bytes(b[16..24].try_into()?);
        let valid = u64::from(u32at(p + 36)) == id;
        if !valid {
            invalid_identity += 1;
        }
        if let Some(first) = seen.insert(id, p) {
            duplicate_count += 1;
            if duplicate_samples.len() < 32 {
                duplicate_samples.push(serde_json::json!({"candidate_id":id,"first_offset":first,"next_offset":p,"first_row_u32":row(first),"next_row_u32":row(p),"next_repeated_id_matches":valid}));
            }
        }
    }
    let result = serde_json::json!({"version":file.basic_file_info()?.version,"stream":name,"prepared_gzip_offset":offsets[0],"inflated_sha256":format!("{:x}",Sha256::digest(&bytes)),"inflated_bytes":bytes.len(),"header_tag":u16::from_le_bytes(bytes[..2].try_into()?),"declared_rows":count,"unique_candidate_ids":seen.len(),"duplicate_count":duplicate_count,"rows_with_mismatched_identity_fields":invalid_identity,"duplicate_samples":duplicate_samples});
    std::fs::write(
        out.join("index-audit.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    println!(
        "{} rows; {} duplicates; {} mismatched identity rows",
        count, duplicate_count, invalid_identity
    );
    Ok(())
}
