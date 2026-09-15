//! Read-only research capture of structural embedded header/index evidence.
use anyhow::{Result, ensure};
use rvt::{compression, native_document, native_parameters, native_segments, schema_registry};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufWriter, Write},
};
fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().collect();
    ensure!(args.len() == 4, "file content-hex output-directory");
    fs::create_dir(&args[3])?;
    let mut file = rvt::RevitFile::open(&args[1])?;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut file, "Formats/Latest")?)?;
    let content = native_document::read_single(&mut file, "Global/ContentDocuments")?;
    fs::write(format!("{}/content-documents.bin", args[3]), &content)?;
    let catalog = rvt::native_content_documents::parse(&content, &registry, 2_000_000)?;
    fs::write(
        format!("{}/catalog.json", args[3]),
        serde_json::to_vec_pretty(&catalog)?,
    )?;
    let ctag = u16::from_le_bytes(content[..2].try_into()?);
    let decoded = native_parameters::decode_object_fields(&content, &registry, 2, ctag);
    fs::write(
        format!("{}/content-documents.json", args[3]),
        serde_json::to_vec_pretty(
            &json!({"class":registry.class(ctag).map(|c|&c.name),"root":decoded.as_ref().ok().map(|v|&v.fields),"end":decoded.as_ref().ok().map(|v|v.end),"error":decoded.err().map(|e|e.to_string())}),
        )?,
    )?;
    let mut cp = 0usize;
    let mut cd = Vec::new();
    while cp < content.len() {
        let tag = u16::from_le_bytes(content[cp..cp + 2].try_into()?);
        ensure!(
            registry
                .class(tag)
                .is_some_and(|c| c.name == "ContentMarker"),
            "content document marker mismatch"
        );
        cp += 2;
        let token = u32::from_le_bytes(content[cp..cp + 4].try_into()?);
        cp += 4;
        if token == 0 {
            cd.push(json!({"terminal":content[cp..].iter().map(|b|format!("{b:02x}")).collect::<String>()}));
            break;
        }
        ensure!(token == u32::MAX, "unknown content marker token");
        let kt = u16::from_le_bytes(content[cp..cp + 2].try_into()?);
        cp += 2;
        ensure!(
            registry.class(kt).is_some_and(|c| c.name == "ContentKey"),
            "content key class"
        );
        let count = u32::from_le_bytes(content[cp..cp + 4].try_into()?);
        cp += 4;
        let key = hex(&content[cp..cp + 16]);
        cp += 16;
        let len = u32::from_le_bytes(content[cp..cp + 4].try_into()?) as usize;
        cp += 4;
        let end = cp + len;
        ensure!(end + 4 <= content.len(), "content doc overflow");
        ensure!(
            u32::from_le_bytes(content[end..end + 4].try_into()?) as usize == len,
            "content doc dual lengths"
        );
        let b = &content[cp..end];
        fs::write(format!("{}/document-{key}.bin", args[3]), b)?;
        let tag = u16::from_le_bytes(b[..2].try_into()?);
        let root = native_parameters::decode_object_fields(b, &registry, 2, tag);
        let graph = native_parameters::decode_graph(b, &registry);
        cd.push(json!({"key":key,"count":count,"start":cp,"bytes":len,"class":registry.class(tag).map(|c|&c.name),"root_fields":root.as_ref().ok().map(|r|&r.fields),"root_end":root.as_ref().ok().map(|r|r.end),"root_error":root.err().map(|e|e.to_string()),"graph":graph.as_ref().ok(),"graph_error":graph.err().map(|e|e.to_string())}));
        cp = end + 4;
    }
    fs::write(
        format!("{}/content-documents-parsed.json", args[3]),
        serde_json::to_vec_pretty(&cd)?,
    )?;
    let names = file.stream_names().to_vec();
    fs::write(
        format!("{}/stream-names.json", args[3]),
        serde_json::to_vec_pretty(&names)?,
    )?;
    let mut schemas = Vec::new();
    for name in [
        "ElementHeader",
        "ElemHistory",
        "ElementHistory",
        "ClassDefinitionRef",
        "ElemRec",
        "FamilyDocument",
        "Document",
        "IdentifierSource",
    ] {
        if let Some(c) = registry.named(name) {
            schemas.push(serde_json::to_value(c)?);
        }
    }
    fs::write(
        format!("{}/schemas.json", args[3]),
        serde_json::to_vec_pretty(&schemas)?,
    )?;
    let mut out = BufWriter::new(fs::File::create(format!("{}/headers.jsonl", args[3]))?);
    for name in names.into_iter().filter(|s| s.starts_with("Partitions/")) {
        let bytes = file.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
        let prepared = compression::prepare_stream_for_inflate(&name, &bytes);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            let Some(key) = source.content_key else {
                return Ok(());
            };
            let key: String = key.iter().map(|b| format!("{b:02x}")).collect();
            if key != args[2] {
                return Ok(());
            }
            let width = if source.channel == 101 { 12 } else { 16 };
            let mut pos = 0usize;
            let mut count = 0u64;
            let mut total = 0u64;
            while pos < bytes.len() {
                ensure!(bytes.len() - pos >= width + 4, "short record header");
                let id = u64::from_le_bytes(bytes[pos..pos + 8].try_into()?);
                let len =
                    u32::from_le_bytes(bytes[pos + width - 4..pos + width].try_into()?) as usize;
                let start = pos + width;
                let end = start
                    .checked_add(len)
                    .ok_or_else(|| anyhow::anyhow!("record overflow"))?;
                ensure!(end + 4 <= bytes.len(), "record outside group");
                ensure!(
                    u32::from_le_bytes(bytes[end..end + 4].try_into()?) as usize == len,
                    "record dual size mismatch"
                );
                let body = &bytes[start..end];
                let class = body
                    .get(..2)
                    .and_then(|b| registry.class(u16::from_le_bytes(b.try_into().unwrap())));
                let fields = class
                    .and_then(|c| {
                        native_parameters::decode_object_fields(body, &registry, 2, c.tag).ok()
                    })
                    .map(|x| x.fields);
                let row = json!({"key":key,"id":id,"source":source,"stream":name,"offset":pos,"header_hex":hex(&bytes[pos..start]),"body_length":len,"class":class.map(|c|&c.name),"prefix_hex":hex(&body[..body.len().min(200)]),"fields":fields,"body_sha256":format!("{:x}",Sha256::digest(body))});
                serde_json::to_writer(&mut out, &row)?;
                out.write_all(b"\n")?;
                if source.channel == 101 && id != u64::MAX {
                    fs::write(
                        format!(
                            "{}/header-{}-{}-{}.bin",
                            args[3],
                            name.replace('/', "_"),
                            source.first_marker_offset,
                            id
                        ),
                        body,
                    )?;
                }
                pos = end + 4;
                count += 1;
                total += len as u64;
            }
            ensure!(
                count == source.declared_objects && total == source.declared_body_bytes,
                "group bookkeeping mismatch"
            );
            Ok(())
        })?;
    }
    out.flush()?;
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
