//! RE-397 (#1635): the partitions that hold a copy of a railing's and its post
//! symbol's saved graphics, and which copy the native path reads.
//!
//! FACT under test: the saved graphics of the post symbol of the `KAI-3`
//! railings of the BIM4LCA office (2620217) hold one cylinder of diameter
//! 0.328 ft, where Revit's own export draws a rectangular prism whose corners lie
//! on that circle. If the file holds more than one copy of that symbol's graphics
//! (a record is repeated in several partitions when an element is saved again, RE-145,
//! RE-216), the copy the route reads may be the older one. This lists, for each
//! channel-103 record of the railings and symbols named in `WANTED`, the
//! partitions that hold it, each copy's size and a hash of its bytes, and the
//! partition its stored revision routes to; then counts, over every channel-103
//! record, the elements with copies in several partitions and whether the copies
//! are byte-identical.
//!
//! Output:
//!   copy <id> partition=<n> bytes=<n> fnv=<hex> routed=<n>
//!   census elements=<n> single=<n> repeated=<n> identical=<n> differing=<n>
//!   differing_examples <id> <partitions> ... (the first twenty)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_graphics_copies -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::compression::prepare_stream_for_inflate;
use rvt::{
    RevitFile, research::native_document, research::native_index, research::native_segments,
    research::schema_registry,
};

/// The railings (2618205, 2614259), their symbols (2618206, 2614260) and the post
/// symbol they name (2620217), the `KAI-4` railing 1467318 and its symbol 1467319.
const WANTED: [u64; 7] = [
    2618205, 2614259, 2618206, 2614260, 2620217, 1467318, 1467319,
];
const CHANNEL: u64 = 103;

/// Split a group into `(id, body)` records at the id width (4 or 8 bytes) at which
/// every record's two length words agree and the group ends exactly. A channel 102
/// or 103 record's header is the id and two words, the second the body's length
/// (`native_document::group_records`).
fn split_group(bytes: &[u8]) -> Option<Vec<(u64, &[u8])>> {
    'width: for width in [8usize, 4] {
        let header = width + 8;
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
            let start = pos + header;
            let Some(end) = start.checked_add(length) else {
                continue 'width;
            };
            if end + 4 > bytes.len()
                || u32::from_le_bytes(bytes[end..end + 4].try_into().ok()?) as usize != length
            {
                continue 'width;
            }
            out.push((id, &bytes[start..end]));
            pos = end + 4;
        }
        return Some(out);
    }
    None
}

fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let registry =
        schema_registry::parse(&native_document::read_single(&mut rf, "Formats/Latest")?)?;
    let episodes = native_index::creation_episodes(
        &native_document::read_single(&mut rf, "Global/History")?,
        &registry,
    )?;
    let index = native_index::parse(
        &native_document::read_single(&mut rf, "Global/ElemTable")?,
        &registry,
        &episodes,
    )?;
    let increments = native_index::storage_increments(
        &native_document::read_single(&mut rf, "Global/DocumentIncrementTable")?,
        &registry,
    )?;
    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|n| n.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();
    let present: BTreeSet<u32> = names.iter().filter_map(|n| n[11..].parse().ok()).collect();

    // id -> (partition, bytes, hash) of each copy.
    let mut copies: BTreeMap<u64, Vec<(u32, usize, u64)>> = BTreeMap::new();
    let mut unsplit = 0usize;
    for name in &names {
        let partition: u32 = name[11..].parse()?;
        let stored = rf.read_stream(name)?;
        let prepared = prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            if source.content_key.is_some() || source.channel != CHANNEL {
                return Ok(());
            }
            match split_group(bytes) {
                Some(records) => {
                    for (id, body) in records {
                        copies
                            .entry(id)
                            .or_default()
                            .push((partition, body.len(), fnv(body)));
                    }
                }
                None => unsplit += 1,
            }
            Ok(())
        })?;
    }
    for id in WANTED {
        let routed = index
            .identities
            .get(&id)
            .and_then(|identity| {
                native_index::route_episode(identity.stored_revision, &increments, &present).ok()
            })
            .map_or_else(|| "none".to_string(), |partition| partition.to_string());
        match copies.get(&id) {
            Some(list) => {
                for (partition, bytes, hash) in list {
                    println!(
                        "copy {id} partition={partition} bytes={bytes} fnv={hash:016x} routed={routed}"
                    );
                }
            }
            None => println!("copy {id} none routed={routed}"),
        }
    }
    let (mut single, mut repeated, mut identical, mut differing) = (0usize, 0usize, 0usize, 0usize);
    let mut examples = Vec::new();
    for (id, list) in &copies {
        let partitions: BTreeSet<u32> = list.iter().map(|copy| copy.0).collect();
        if partitions.len() == 1 && list.len() == 1 {
            single += 1;
            continue;
        }
        repeated += 1;
        if list.iter().all(|copy| copy.2 == list[0].2) {
            identical += 1;
        } else {
            differing += 1;
            if examples.len() < 20 {
                examples.push(format!("{id} {:?}", partitions));
            }
        }
    }
    println!(
        "census elements={} single={single} repeated={repeated} identical={identical} differing={differing} unsplit_groups={unsplit}",
        copies.len()
    );
    for example in examples {
        println!("differing_examples {example}");
    }
    Ok(())
}
