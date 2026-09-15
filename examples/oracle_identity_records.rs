//! Read-only bounded-record evidence for requested numeric IDs; no live-owner claim.
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::PathBuf};

fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() >= 4, "MODEL.rvt OUTPUT_DIRECTORY ID [ID ...]");
    let out = PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let ids: BTreeSet<u64> = args[3..]
        .iter()
        .map(|s| s.parse())
        .collect::<Result<_, _>>()?;
    let mut file = rvt::RevitFile::open(&args[1])?;
    let mut names: Vec<_> = file
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let mut records = Vec::new();
    let mut failures = Vec::new();
    let mut members = 0usize;
    let mut inflated_bytes = 0u64;
    for name in names {
        // Explicit research budget for the measured 465 MB architectural stream.
        // Production reader defaults remain unchanged.
        let raw = file.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
        let prepared = rvt::compression::prepare_stream_for_inflate(&name, &raw);
        for offset in rvt::compression::find_gzip_offsets(&prepared) {
            let bytes = match rvt::compression::inflate_at(&prepared, offset) {
                Ok(bytes) => bytes,
                Err(error) => {
                    failures.push(serde_json::json!({"stream":name,"offset":offset,"error":error.to_string()}));
                    continue;
                }
            };
            members += 1;
            inflated_bytes += bytes.len() as u64;
            let mut member_hash = None;
            for outer in 0..bytes.len().saturating_sub(20) {
                let id = u64::from_le_bytes(bytes[outer..outer + 8].try_into()?);
                if !ids.contains(&id) {
                    continue;
                }
                let length = u32::from_le_bytes(bytes[outer + 12..outer + 16].try_into()?) as usize;
                if !(2..=16 * 1024 * 1024).contains(&length) {
                    continue;
                }
                let start = outer + 16;
                let Some(end) = start.checked_add(length) else {
                    continue;
                };
                if end + 4 > bytes.len()
                    || u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize != length
                {
                    continue;
                }
                let body = &bytes[start..end];
                let repeated: Vec<_> = body
                    .windows(8)
                    .enumerate()
                    .filter_map(|(i, b)| (b == id.to_le_bytes()).then_some(i))
                    .collect();
                let filename = format!("record-{:05}-id-{id}.bin", records.len());
                std::fs::write(out.join(&filename), body)?;
                records.push(serde_json::json!({"id":id,"stream":name,"prepared_member_offset":offset,"inflated_member_bytes":bytes.len(),"member_sha256":member_hash.get_or_insert_with(||format!("{:x}",Sha256::digest(&bytes))),"record_offset":outer,"header_opaque_u32":u32::from_le_bytes(bytes[outer+8..outer+12].try_into()?),"body_bytes":length,"body_sha256":format!("{:x}",Sha256::digest(body)),"class_tag":u16::from_le_bytes(body[..2].try_into()?),"repeated_id_offsets":repeated,"body_file":filename}));
            }
        }
    }
    let report = serde_json::json!({"schema_version":1,"method":"dual-length-record-candidates-no-live-selection","requested_ids":ids,"decoded_members":members,"inflated_bytes":inflated_bytes,"inflate_failures":failures,"records":records});
    std::fs::write(
        out.join("identity-records.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!(
        "{} bounded candidates in {members} members ({inflated_bytes} inflated bytes)",
        records.len()
    );
    Ok(())
}
