//! RE-148 (probe): which loaded family a record after a project's chain
//! belongs to (RE-135's open question, #421).
//!
//! puzzbobb reported (#421, issuecomment-5956962887) that a keyed block of a
//! partition, the groups after a `ContentMarker` that owns a `ContentKey`,
//! belongs to one `Global/ContentDocuments` section:
//!
//! 1. every block's 16-byte key is the GUID of a `ContentDocuments` section
//!    (163 of 163 on every `rac_basic`, 52 of 52 on `rst_basic`, 121 of 121
//!    on `rac_advanced`);
//! 2. the block's `ContentMarker.m_nElementCount` is its number of
//!    channel-101 records less the one whose id is -1.
//!
//! This prints, per file: the distinct block keys, how many of them occur in
//! the inflated `Global/ContentDocuments` bytes (a byte search: it shows the
//! key is in the stream, not which field holds it), and for how many keys the
//! stated count equals the channel-101 records less those with id -1, with up
//! to five counterexamples.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re148_content_keys -- MODEL.rvt ...

use rvt::compression::{inflate_stream_at, prepare_stream_for_inflate};
use rvt::streams::GLOBAL_CONTENT_DOCUMENTS;
use rvt::{RevitFile, native_document, native_segments, schema_registry};
use std::collections::BTreeMap;

const EXAMPLES: usize = 5;

/// The ids of a channel-101 group's records, at the id width (8 or 4
/// bytes) at which they walk exactly; an id of all ones reads as `None`.
fn record_ids(bytes: &[u8]) -> Option<Vec<Option<u64>>> {
    'width: for width in [8usize, 4] {
        let header = width + 4;
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            if bytes.len() - pos < header + 4 {
                continue 'width;
            }
            let id = if width == 4 {
                let v = u32::from_le_bytes(bytes[pos..pos + 4].try_into().ok()?);
                (v != u32::MAX).then_some(u64::from(v))
            } else {
                let v = u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?);
                (v != u64::MAX).then_some(v)
            };
            let length =
                u32::from_le_bytes(bytes[pos + header - 4..pos + header].try_into().ok()?) as usize;
            let Some(end) = (pos + header).checked_add(length) else {
                continue 'width;
            };
            if end + 4 > bytes.len()
                || u32::from_le_bytes(bytes[end..end + 4].try_into().ok()?) as usize != length
            {
                continue 'width;
            }
            out.push(id);
            pos = end + 4;
        }
        return Some(out);
    }
    None
}

#[derive(Default)]
struct Block {
    stated: Option<u32>,
    records: usize,
    minus_one: usize,
}

fn hex(key: &[u8; 16]) -> String {
    key.iter().map(|b| format!("{b:02x}")).collect()
}

fn probe(path: &str) -> anyhow::Result<String> {
    let file = std::path::Path::new(path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut rf = RevitFile::open(path)?;
    let revit = rf.basic_file_info()?.version;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut rf, "Formats/Latest")?)?;
    let documents = inflate_stream_at(
        GLOBAL_CONTENT_DOCUMENTS,
        &rf.read_stream(GLOBAL_CONTENT_DOCUMENTS)?,
        0,
    )?;
    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let mut blocks: BTreeMap<[u8; 16], Block> = BTreeMap::new();
    let mut unsplit = 0usize;
    for name in &names {
        let stored = rf.read_stream(name)?;
        let prepared = prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            let Some(key) = source.content_key else {
                return Ok(());
            };
            let block = blocks.entry(key).or_default();
            if block.stated.is_none() {
                block.stated = source.content_element_count;
            }
            if source.channel != 101 {
                return Ok(());
            }
            match record_ids(bytes) {
                Some(ids) => {
                    block.records += ids.len();
                    block.minus_one += ids.iter().filter(|id| id.is_none()).count();
                }
                None => unsplit += 1,
            }
            Ok(())
        })?;
    }
    let in_documents = blocks
        .keys()
        .filter(|key| memchr::memmem::find(&documents, &key[..]).is_some())
        .count();
    let (mut count_agrees, mut counterexamples) = (0usize, Vec::new());
    for (key, block) in &blocks {
        let expected = block.records.saturating_sub(block.minus_one);
        if block.stated.map(|s| s as usize) == Some(expected) {
            count_agrees += 1;
        } else if counterexamples.len() < EXAMPLES {
            counterexamples.push(format!(
                "{{\"key\":\"{}\",\"stated\":{:?},\"records\":{},\"minus_one\":{}}}",
                hex(key),
                block.stated,
                block.records,
                block.minus_one
            ));
        }
    }
    let minus_one_per_block: BTreeMap<usize, usize> =
        blocks.values().fold(BTreeMap::new(), |mut m, b| {
            *m.entry(b.minus_one).or_default() += 1;
            m
        });
    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"content_documents_bytes\":{},\"blocks\":{},\
         \"keys_in_content_documents\":{in_documents},\"stated_count_agrees\":{count_agrees},\
         \"blocks_by_minus_one_records\":{minus_one_per_block:?},\"unsplit_groups\":{unsplit},\
         \"counterexamples\":[{}]}}",
        documents.len(),
        blocks.len(),
        counterexamples.join(",")
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re148_content_keys MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in paths.iter().filter(|p| !p.starts_with("--")) {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
