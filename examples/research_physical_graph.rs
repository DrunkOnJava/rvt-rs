//! Research-only bounded physical records. Does not claim current liveness.
use anyhow::{Result, ensure};
use rvt::{compression, native_document, native_parameters, native_segments, schema_registry};
use serde_json::json;
use std::{collections::BTreeSet, io::Write};
fn main() -> Result<()> {
    let a = std::env::args().collect::<Vec<_>>();
    ensure!(a.len() == 5, "FILE CHANNEL ID,ID NEW_JSONL");
    let channel: u64 = a[2].parse()?;
    ensure!([101, 102, 103].contains(&channel), "channel");
    let ids = a[3]
        .split(',')
        .map(str::parse)
        .collect::<std::result::Result<BTreeSet<u64>, _>>()?;
    let mut out = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a[4])?;
    let mut f = rvt::RevitFile::open(&a[1])?;
    let s = native_document::read_single(&mut f, "Formats/Latest")?;
    let registry = schema_registry::parse(&s)?;
    for name in f
        .stream_names()
        .to_vec()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
    {
        let b = f.read_stream_with_limit(&name, 512 * 1024 * 1024)?;
        native_segments::walk(
            &compression::prepare_stream_for_inflate(&name, &b),
            &registry,
            256 * 1024 * 1024,
            |source, b| {
                if source.channel != channel || source.content_key.is_some() {
                    return Ok(());
                }
                let header = if channel == 101 { 12 } else { 16 };
                let mut pos = 0;
                let mut count = 0;
                let mut total = 0;
                while pos < b.len() {
                    ensure!(b.len() - pos >= header + 4, "shortphysicalrecord");
                    let id = u64::from_le_bytes(b[pos..pos + 8].try_into()?);
                    let n =
                        u32::from_le_bytes(b[pos + header - 4..pos + header].try_into()?) as usize;
                    let start = pos + header;
                    let end = start
                        .checked_add(n)
                        .ok_or_else(|| anyhow::anyhow!("overflow"))?;
                    ensure!(
                        end <= b.len() - 4
                            && u32::from_le_bytes(b[end..end + 4].try_into()?) as usize == n,
                        "dualphysical lengths"
                    );
                    if ids.contains(&id) {
                        let graph = native_parameters::decode_graph(&b[start..end], &registry);
                        let record = match graph {
                            Ok(g) => {
                                json!({"id":id,"stream":name,"source":source,"offset":pos,"body_bytes":n,"physical_only":true,"graph":g})
                            }
                            Err(e) => {
                                json!({"id":id,"stream":name,"source":source,"offset":pos,"body_bytes":n,"physical_only":true,"error":format!("{e:#}")})
                            }
                        };
                        serde_json::to_writer(&mut out, &record)?;
                        writeln!(&mut out)?;
                    }
                    count += 1;
                    total += n;
                    pos = end + 4;
                }
                ensure!(
                    count == source.declared_objects && total as u64 == source.declared_body_bytes,
                    "group population"
                );
                Ok(())
            },
        )?;
    }
    Ok(())
}
