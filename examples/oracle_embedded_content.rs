//! Research-only physical embedded namespace extraction. No current-state or
//! global UniqueId assertion is made without a content-local index/history join.
use anyhow::{Result, ensure};
use clap::Parser;
use rvt::{compression, native_document, native_parameters, native_segments, schema_registry};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Read, Write},
    path::PathBuf,
};

#[derive(Parser)]
struct Args {
    file: PathBuf,
    /// Raw on-disk 16-byte GUID as 32 hexadecimal digits (not display GUID order).
    #[arg(long)]
    content_hex: String,
    /// A new directory containing records.jsonl and summary.json.
    #[arg(long)]
    output: PathBuf,
}
fn raw_key(text: &str) -> Result<[u8; 16]> {
    ensure!(
        text.len() == 32 && text.is_ascii(),
        "content hex requires 32 ASCII hexadecimal digits"
    );
    let mut bytes = [0; 16];
    for (i, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16)?;
    }
    Ok(bytes)
}
fn hash_file(path: &std::path::Path) -> Result<String> {
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    let mut chunk = [0; 65536];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        };
        hash.update(&chunk[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn main() -> Result<()> {
    let args = Args::parse();
    let key = raw_key(&args.content_hex)?;
    fs::create_dir(&args.output)?;
    let mut output = BufWriter::new(File::create(args.output.join("records.jsonl"))?);
    let mut channels: BTreeMap<u64, BTreeMap<String, usize>> = BTreeMap::new();
    let mut classes: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut locators = BTreeMap::new();
    let mut partitions = BTreeMap::new();
    let mut source_hash = None;
    let mut schema_hash = None;
    let mut emitted = 0usize;
    let mut selected_groups = 0usize;
    let extraction = (|| -> Result<()> {
        source_hash = Some(hash_file(&args.file)?);
        let mut file = rvt::RevitFile::open(&args.file)?;
        ensure!(
            file.basic_file_info()?.version == 2027,
            "embedded research framing currently qualified only for 2027"
        );
        let registry =
            schema_registry::parse(&native_document::read_single(&mut file, "Formats/Latest")?)?;
        schema_hash = Some(registry.source_sha256.clone());
        let mut names: Vec<_> = file
            .stream_names()
            .iter()
            .filter(|n| n.starts_with("Partitions/"))
            .cloned()
            .collect();
        names.sort();
        for name in names {
            let stored = file.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
            let prepared = compression::prepare_stream_for_inflate(&name, &stored);
            let stats = native_segments::walk(
                &prepared,
                &registry,
                256 * 1024 * 1024,
                |source, bytes| {
                    if source.content_key != Some(key) {
                        return Ok(());
                    }
                    selected_groups += 1;
                    let header = match source.channel {
                        101 => 12,
                        102 | 103 => 16,
                        _ => anyhow::bail!("unknown embedded channel {}", source.channel),
                    };
                    let mut pos = 0usize;
                    let mut rows = Vec::new();
                    let mut sum = 0u64;
                    while pos < bytes.len() {
                        ensure!(
                            bytes.len() - pos >= header + 4,
                            "truncated embedded record header"
                        );
                        let id = u64::from_le_bytes(bytes[pos..pos + 8].try_into()?);
                        let length =
                            u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into()?)
                                as usize;
                        let start = pos + header;
                        let end = start
                            .checked_add(length)
                            .ok_or_else(|| anyhow::anyhow!("embedded record length overflow"))?;
                        ensure!(
                            end <= bytes.len().saturating_sub(4),
                            "truncated embedded body"
                        );
                        ensure!(
                            u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize == length,
                            "embedded dual lengths disagree"
                        );
                        rows.push((id, pos, start, end));
                        sum += length as u64;
                        pos = end + 4;
                    }
                    ensure!(
                        rows.len() as u64 == source.declared_objects
                            && sum == source.declared_body_bytes,
                        "embedded group count/body size mismatch"
                    );
                    for (id, offset, start, end) in rows {
                        let body = &bytes[start..end];
                        let class = body
                            .get(..2)
                            .and_then(|b| registry.class(u16::from_le_bytes([b[0], b[1]])))
                            .map(|c| c.name.clone());
                        let (status, graph, diagnostic) =
                            match native_parameters::decode_graph(body, &registry) {
                                Ok(g) => ("complete_bounded_graph", Some(g), None),
                                Err(e) => ("unsupported_graph", None, Some(format!("{e:#}"))),
                            };
                        *channels
                            .entry(source.channel)
                            .or_default()
                            .entry(status.into())
                            .or_default() += 1;
                        *classes
                            .entry(class.clone().unwrap_or_else(|| "<unknown>".into()))
                            .or_default()
                            .entry(status.into())
                            .or_default() += 1;
                        *locators.entry((source.channel, id)).or_insert(0usize) += 1;
                        let row = json!({"content_key_raw_hex":args.content_hex.to_lowercase(),"local_element_id":id,
                        "channel":source.channel,"class_name":class,"status":status,"graph":graph,"diagnostic":diagnostic,
                        "current_state_established":false,"source":{"stream":name,"group":source,"group_record_offset":offset,
                        "body_bytes":body.len(),"body_sha256":format!("{:x}",Sha256::digest(body))}});
                        serde_json::to_writer(&mut output, &row)?;
                        output.write_all(b"\n")?;
                        emitted += 1;
                    }
                    Ok(())
                },
            )?;
            partitions.insert(name, stats);
        }
        ensure!(selected_groups > 0, "requested content GUID absent");
        Ok(())
    })();
    output.flush()?;
    let duplicates:Vec<_>=locators.iter().filter(|(_,count)|**count>1).map(|((channel,id),count)|json!({"channel":channel,"local_element_id":id,"physical_occurrences":count})).collect();
    let error = extraction.err().map(|e| format!("{e:#}"));
    let unsupported: usize = channels
        .values()
        .map(|c| c.get("unsupported_graph").copied().unwrap_or(0))
        .sum();
    let status = if error.is_some() {
        "failed"
    } else if unsupported > 0 {
        "incomplete"
    } else {
        "selected_physical_graphs_complete"
    };
    let summary = json!({"format":"embedded-content-research/v1","status":status,"source":args.file,"source_sha256":source_hash,
        "schema_sha256":schema_hash,"content_key_raw_hex":args.content_hex.to_lowercase(),"selected_groups":selected_groups,
        "emitted_records":emitted,"channels":channels,"classes":classes,"duplicate_local_locators":duplicates,"partitions":partitions,
        "current_state_established":false,"main_project_unique_ids_assigned":false,"error":error});
    let mut summary_output = BufWriter::new(File::create(args.output.join("summary.json"))?);
    serde_json::to_writer_pretty(&mut summary_output, &summary)?;
    summary_output.write_all(b"\n")?;
    summary_output.flush()?;
    eprintln!("{status}: {emitted} embedded physical records");
    std::process::exit(if error.is_some() {
        1
    } else if unsupported > 0 {
        2
    } else {
        0
    });
}
