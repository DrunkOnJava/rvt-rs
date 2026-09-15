//! Capture bounded class bodies and root prefixes; physical records, not liveness.
use anyhow::{Result, ensure};
use rvt::{compression, native_document, native_parameters, native_segments, schema_registry};
use serde_json::json;
use std::{fs, path::PathBuf};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() == 4, "FILE CLASS NEW_OUTPUT_DIR");
    let out = PathBuf::from(&args[3]);
    fs::create_dir(&out)?;
    let mut file = rvt::RevitFile::open(&args[1])?;
    let version = file.basic_file_info()?.version;
    ensure!(
        matches!(version, 2023 | 2024 | 2027),
        "unsupported Revit version {version}"
    );
    let schema = native_document::read_single(&mut file, "Formats/Latest")?;
    fs::write(out.join("schema.bin"), &schema)?;
    let registry = schema_registry::parse(&schema)?;
    let tag = registry
        .named(&args[2])
        .ok_or_else(|| anyhow::anyhow!("class absent"))?
        .tag;
    let mut records = Vec::new();
    for name in file
        .stream_names()
        .to_vec()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
    {
        let bytes = file.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &bytes);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            if source.channel != 102 || source.content_key.is_some() {
                return Ok(());
            }
            let id_bytes = if version == 2023 { 4 } else { 8 };
            let header = id_bytes + 8;
            let mut pos = 0usize;
            let mut count = 0u64;
            let mut total = 0u64;
            while pos < bytes.len() {
                ensure!(bytes.len() - pos >= header + 4, "short record");
                let id = if id_bytes == 4 {
                    u64::from(u32::from_le_bytes(bytes[pos..pos + 4].try_into()?))
                } else {
                    u64::from_le_bytes(bytes[pos..pos + 8].try_into()?)
                };
                let len =
                    u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into()?) as usize;
                let start = pos + header;
                let end = start
                    .checked_add(len)
                    .ok_or_else(|| anyhow::anyhow!("record overflow"))?;
                ensure!(end <= bytes.len() - 4, "record outside group");
                ensure!(
                    u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize == len,
                    "record check length"
                );
                let body = &bytes[start..end];
                if body.len() >= 2 && u16::from_le_bytes(body[..2].try_into()?) == tag {
                    let path = format!("record-{}.bin", records.len());
                    fs::write(out.join(&path), body)?;
                    let root = native_parameters::decode_object_fields(body, &registry, 2, tag);
                    let graph = native_parameters::decode_graph(body, &registry);
                    records.push(json!({"id":id,"stream":name,"group":source,"offset":pos,"body":path,"root":root.as_ref().ok(),"root_error":root.err().map(|e|format!("{e:#}")),"graph":graph.as_ref().ok(),"graph_error":graph.err().map(|e|format!("{e:#}"))}));
                }
                count += 1;
                total += len as u64;
                pos = end + 4;
            }
            ensure!(
                count == source.declared_objects && total == source.declared_body_bytes,
                "group totals mismatch"
            );
            Ok(())
        })?;
    }
    fs::write(
        out.join("records.json"),
        serde_json::to_vec_pretty(&records)?,
    )?;
    println!("{} bounded physical class records", records.len());
    Ok(())
}
