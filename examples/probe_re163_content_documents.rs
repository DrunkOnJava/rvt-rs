//! RE-163 (probe): which keyed blocks `Global/ContentDocuments` lists, and
//! why RE-148 found 60 of RE1 Electrical's block keys missing from it (#421).
//!
//! puzzbobb's account (#421, 2026-10-03): a loaded family is a `Family`
//! element of the project plus its own document, embedded as a keyed block
//! of a partition (RE-148); a `Family` record names the block by its 16-byte
//! key, and the documents of families nested in it are named by `Family`
//! records inside its block. The rule stated:
//!
//! 1. every block's key is held by a `Family` record, outside every block for
//!    a loaded family or inside another block for a nested one;
//! 2. a block's key is in `Global/ContentDocuments` exactly when the `Family`
//!    at the top of that chain (the first one outside every block) is an id
//!    `Global/ElemTable` declares. On RE1 Electrical the 60 unlisted blocks
//!    sit under three undeclared, deleted families.
//!
//! This probe is written from that statement with rvt-rs's own walkers. Per
//! file it prints: the keyed blocks; how many keys occur in the inflated
//! `ContentDocuments` bytes (RE-148's membership test); how many keys no
//! `Family` record holds, or two or more do; and a table of listed against
//! the top `Family`'s declared state. For each unlisted block whose top
//! `Family` is undeclared it gives that id and the block's channel-101
//! record count, up to 10.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re163_content_documents -- MODEL.rvt ...

use rvt::compression::{inflate_all_chunks, prepare_stream_for_inflate};
use rvt::streams::GLOBAL_CONTENT_DOCUMENTS;
use rvt::{RevitFile, elem_table, native_document, native_segments, schema_registry};
use std::collections::{BTreeMap, BTreeSet};

type Key = [u8; 16];

const EXAMPLES: usize = 10;
/// Longest chain of nested families followed before giving up.
const MAX_DEPTH: usize = 64;

/// A group's records as `(id, body)`, at the id width (8 or 4 bytes) at which
/// its framing walks exactly: `id · [u32] · u32 length · body · u32 length`,
/// the extra `u32` on channels 102 and 103.
fn records(channel: u64, bytes: &[u8]) -> Option<Vec<(u64, &[u8])>> {
    let extra = if channel == 101 { 0 } else { 4 };
    'width: for width in [8usize, 4] {
        let header = width + extra + 4;
        let mut out = Vec::new();
        let mut pos = 0;
        while pos < bytes.len() {
            if bytes.len() - pos < header + 4 {
                continue 'width;
            }
            let id = if width == 4 {
                u64::from(u32::from_le_bytes(bytes[pos..pos + 4].try_into().ok()?))
            } else {
                u64::from_le_bytes(bytes[pos..pos + 8].try_into().ok()?)
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
            out.push((id, &bytes[pos + header..end]));
            pos = end + 4;
        }
        return Some(out);
    }
    None
}

/// A `Family` record: the block it sits in (`None` outside every block), its
/// id, and its body.
struct FamilyRecord {
    container: Option<Key>,
    id: u64,
    body: Vec<u8>,
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
    let family_tag = registry
        .named("Family")
        .ok_or_else(|| anyhow::anyhow!("no Family class in the schema"))?
        .tag;
    let declared: BTreeSet<u64> = elem_table::declared_element_ids(&mut rf)?
        .into_iter()
        .map(u64::from)
        .collect();
    let stored = rf.read_stream(GLOBAL_CONTENT_DOCUMENTS)?;
    let prepared = prepare_stream_for_inflate(GLOBAL_CONTENT_DOCUMENTS, &stored);
    let documents: Vec<u8> = inflate_all_chunks(prepared.as_ref())
        .into_iter()
        .flatten()
        .collect();
    anyhow::ensure!(
        !documents.is_empty(),
        "Global/ContentDocuments inflates to nothing"
    );

    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let mut block_records: BTreeMap<Key, usize> = BTreeMap::new();
    let mut families: Vec<FamilyRecord> = Vec::new();
    let mut unsplit = 0usize;
    for name in &names {
        let stored = rf.read_stream(name)?;
        let prepared = prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            if let Some(key) = source.content_key {
                let count = block_records.entry(key).or_default();
                if source.channel == 101 {
                    *count += records(101, bytes).map_or(0, |r| r.len());
                }
            }
            if !(101..=103).contains(&source.channel) {
                return Ok(());
            }
            let Some(split) = records(source.channel, bytes) else {
                unsplit += 1;
                return Ok(());
            };
            for (id, body) in split {
                if body.get(..2) == Some(&family_tag.to_le_bytes()[..]) {
                    families.push(FamilyRecord {
                        container: source.content_key,
                        id,
                        body: body.to_vec(),
                    });
                }
            }
            Ok(())
        })?;
    }

    // For each block key, the Family records whose body holds its 16 bytes.
    let mut holders: BTreeMap<Key, Vec<(Option<Key>, u64)>> = BTreeMap::new();
    for family in &families {
        for key in block_records.keys() {
            if memchr::memmem::find(&family.body, key).is_some() {
                holders
                    .entry(*key)
                    .or_default()
                    .push((family.container, family.id));
            }
        }
    }
    // The id of the first Family outside every block on the chain above
    // `key`, following each block's first holder.
    let top = |key: &Key| -> Option<u64> {
        let mut at = *key;
        for _ in 0..MAX_DEPTH {
            let (container, id) = *holders.get(&at)?.first()?;
            match container {
                None => return Some(id),
                Some(up) => at = up,
            }
        }
        None
    };

    let mut table: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    let (mut no_holder, mut several_holders, mut listed_count) = (0usize, 0usize, 0usize);
    let mut unlisted_undeclared = Vec::new();
    for (key, records) in &block_records {
        let listed = memchr::memmem::find(&documents, key).is_some();
        listed_count += usize::from(listed);
        match holders.get(key).map_or(0, Vec::len) {
            0 => no_holder += 1,
            1 => {}
            _ => several_holders += 1,
        }
        let state = match top(key) {
            None => "top not found",
            Some(id) if declared.contains(&id) => "top declared",
            Some(_) => "top undeclared",
        };
        *table
            .entry((if listed { "listed" } else { "unlisted" }, state))
            .or_default() += 1;
        if !listed && state == "top undeclared" && unlisted_undeclared.len() < EXAMPLES {
            unlisted_undeclared.push(format!(
                "{{\"top_family\":{},\"records\":{records}}}",
                top(key).unwrap_or_default()
            ));
        }
    }
    let table = table
        .iter()
        .map(|((listed, state), n)| format!("\"{listed} / {state}\":{n}"))
        .collect::<Vec<_>>()
        .join(",");
    Ok(format!(
        "{{\"file\":{file:?},\"revit\":{revit},\"blocks\":{},\"listed\":{listed_count},\
         \"family_records\":{},\"blocks_no_family_record\":{no_holder},\
         \"blocks_several_family_records\":{several_holders},\"table\":{{{table}}},\
         \"unsplit_groups\":{unsplit},\"unlisted_under_undeclared\":[{}]}}",
        block_records.len(),
        families.len(),
        unlisted_undeclared.join(",")
    ))
}

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: probe_re163_content_documents MODEL.rvt ...");
        std::process::exit(2);
    }
    for path in paths.iter().filter(|p| !p.starts_with("--")) {
        match probe(path) {
            Ok(line) => println!("{line}"),
            Err(error) => println!("{{\"file\":{path:?},\"error\":{:?}}}", format!("{error:#}")),
        }
    }
}
