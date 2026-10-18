//! RE-189 (#760): what in a file names an ElemTable record's first id
//! where it differs from its second.
//!
//! FACT: where a 40-byte `Global/ElemTable` record's two ids differ, the
//! second is the element's ElementId (RE-41) and the first is the id it was
//! created with, which its UniqueId keeps (RE-48). No element record of the
//! project names a first id that is not also some element's current id. On
//! Autodesk's Snowdon Towers 2024 samples:
//! - **Architectural:** 84 records differ, and none of their first ids is
//!   another element's id. 43 are named only in loaded families' own
//!   documents, past the partitions' leading record chains (RE-135), and in
//!   `Global/ContentDocuments`; 41 are named nowhere.
//! - **Structural:** 27 records differ. 7 first ids are other elements'
//!   current ids, since its pairs run consecutively (666709 -> 666710 ->
//!   666711 ...); of the other 20, 2 are named only in loaded families'
//!   documents and 18 nowhere.
//! - **The other 28 files with 40-byte records** measured (Revit 2024 to
//!   2027: the other Autodesk sample projects and Snowdon models, Core
//!   Interior and the four RE1 models) have no record whose ids differ. The
//!   probe skips the 28-byte records of Revit 2023 and earlier, whose two ids
//!   RE-41 found equal on every record.
//! So `elem_table::declared_ids` declares a record's current id only.
//!
//! For every such record it searches every inflated `Partitions/*` stream in
//! one pass for both ids, as `u64` (the id followed by four zero bytes) and
//! as bare `u32`, little-endian. For each hit it names the record of the
//! partition's leading chain (RE-35) whose body holds it, by its ElementId,
//! and the hit's offset in that record; a hit past the chain is in a loaded
//! family's own document (RE-135). It prints how many first and second ids
//! are named anywhere and outside their own element's record (the record of
//! the second id). It then sorts the first ids: those that are another
//! element's current id (such a hit names that element), and of the rest,
//! those a project record names, with each record and offset, those only a
//! loaded family's document names, and those nothing names. Every other
//! stream but `Global/ElemTable` itself (`Global/Latest` among them) is
//! searched the same way, inflated where it is compressed, and the first ids
//! any of them names are listed with the streams. Last come each record's
//! hits, up to eight per id, with the sixteen bytes before and after each.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re189_first_id_references -- FILE.rvt [--records]

use rvt::RevitFile;
use rvt::partition_element_records::{enclosing_record, file_bbox_marker, partition_record_chain};
use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

/// Hits printed per id.
const SHOWN: usize = 8;

struct Hit {
    stream: String,
    offset: usize,
    wide: bool,
    record: Option<(u64, usize)>,
    context: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let pairs: Vec<(u32, u32)> = records
        .iter()
        .filter(|r| r.raw.len() == rvt::elem_table::RECORD_LEN_40)
        .filter(|r| r.id_secondary != 0 && r.id_primary != r.id_secondary)
        .map(|r| (r.id_primary, r.id_secondary))
        .collect();
    println!(
        "release {version}: {} ElemTable records, {} with differing ids",
        records.len(),
        pairs.len()
    );
    if pairs.is_empty() {
        return Ok(());
    }
    let Some(marker) = file_bbox_marker(&mut rf, version) else {
        println!("no record marker for release {version}");
        return Ok(());
    };
    let targets: HashSet<u32> = pairs.iter().flat_map(|(a, b)| [*a, *b]).collect();
    let mut hits: BTreeMap<u32, Vec<Hit>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = partition_record_chain(buf, &marker);
        for (offset, window) in buf.windows(4).enumerate() {
            let value = u32::from_le_bytes(window.try_into().expect("4 bytes"));
            if !targets.contains(&value) {
                continue;
            }
            let wide = buf.get(offset + 4..offset + 8) == Some(&[0, 0, 0, 0][..]);
            let record =
                enclosing_record(&chain, offset).map(|span| (span.element_id, offset - span.start));
            let context = format!(
                "{} [{}] {}",
                hex(&buf[offset.saturating_sub(16)..offset]),
                hex(&buf[offset..(offset + if wide { 8 } else { 4 }).min(buf.len())]),
                hex(&buf[(offset + if wide { 8 } else { 4 }).min(buf.len())
                    ..(offset + if wide { 24 } else { 20 }).min(buf.len())]),
            );
            hits.entry(value).or_default().push(Hit {
                stream: stream.clone(),
                offset,
                wide,
                record,
                context,
            });
        }
    }
    let partitions: HashSet<String> = rf.partition_stream_names().into_iter().collect();
    let mut elsewhere: BTreeMap<u32, BTreeMap<String, usize>> = BTreeMap::new();
    for stream in rf.stream_names() {
        if partitions.contains(&stream) || stream == rvt::streams::GLOBAL_ELEM_TABLE {
            continue;
        }
        let Ok(stored) = rf.read_stream(&stream) else {
            continue;
        };
        let bytes = match rvt::compression::inflate_stream_auto(&stream, &stored) {
            Ok((_, inflated)) => inflated,
            Err(_) => stored,
        };
        for window in bytes.windows(4) {
            let value = u32::from_le_bytes(window.try_into().expect("4 bytes"));
            if targets.contains(&value) {
                *elsewhere
                    .entry(value)
                    .or_default()
                    .entry(stream.clone())
                    .or_insert(0) += 1;
            }
        }
    }
    let outside = |id: u32, own: u32| {
        hits.get(&id).map_or(0, |list| {
            list.iter()
                .filter(|hit| hit.record.map(|(record, _)| record) != Some(u64::from(own)))
                .count()
        })
    };
    for (label, pick) in [("first", 0usize), ("second", 1usize)] {
        let ids: Vec<(u32, u32)> = pairs
            .iter()
            .map(|&(first, second)| (if pick == 0 { first } else { second }, second))
            .collect();
        let named = ids.iter().filter(|(id, _)| hits.contains_key(id)).count();
        let elsewhere = ids
            .iter()
            .filter(|(id, own)| outside(*id, *own) > 0)
            .count();
        let (mut wide, mut narrow) = (0, 0);
        for (id, _) in &ids {
            for hit in hits.get(id).into_iter().flatten() {
                if hit.wide {
                    wide += 1;
                } else {
                    narrow += 1;
                }
            }
        }
        println!(
            "{label} ids: {named} of {} named anywhere ({wide} u64 hits, {narrow} u32 hits); {elsewhere} named outside their element's record",
            ids.len()
        );
    }
    // An element's current id is its second where the two differ (RE-41).
    // A first id that is another element's current id names that element.
    let current: HashSet<u32> = records
        .iter()
        .map(|r| {
            if r.raw.len() == rvt::elem_table::RECORD_LEN_40 && r.id_secondary != 0 {
                r.id_secondary
            } else {
                r.id_primary
            }
        })
        .collect();
    let (taken, stray): (Vec<_>, Vec<_>) =
        pairs.iter().partition(|(first, _)| current.contains(first));
    let (mut in_chain, mut past_only, mut nowhere) = (Vec::new(), 0, 0);
    for (first, second) in &stray {
        let list = hits.get(first).map(Vec::as_slice).unwrap_or(&[]);
        let records: Vec<String> = list
            .iter()
            .filter_map(|hit| hit.record)
            .filter(|(record, _)| *record != u64::from(*second))
            .map(|(record, at)| format!("{record}+{at:#x}"))
            .collect();
        if !records.is_empty() {
            in_chain.push((*first, records));
        } else if list.is_empty() {
            nowhere += 1;
        } else {
            past_only += 1;
        }
    }
    println!(
        "first ids that are another element's current id: {}; no element's id: {} (named in a project record {}, only past the chain {past_only}, nowhere {nowhere})",
        taken.len(),
        stray.len(),
        in_chain.len()
    );
    for (first, records) in &in_chain {
        println!("  {first} named in {}", records.join(", "));
    }
    let named_elsewhere: Vec<String> = pairs
        .iter()
        .filter_map(|(first, _)| {
            let streams = elsewhere.get(first)?;
            let listed: Vec<String> = streams
                .iter()
                .map(|(stream, n)| format!("{stream} {n}"))
                .collect();
            Some(format!("{first} ({})", listed.join(", ")))
        })
        .collect();
    println!(
        "first ids named in a stream other than Partitions/* and Global/ElemTable: {}",
        named_elsewhere.len()
    );
    for line in &named_elsewhere {
        println!("  {line}");
    }
    for (first, second) in &pairs {
        for (label, id) in [("first", *first), ("second", *second)] {
            let list = hits.get(&id).map(Vec::as_slice).unwrap_or(&[]);
            println!(
                "  {label} {id} (record pair {first} -> {second}): {} hits, {} outside {second}'s record",
                list.len(),
                outside(id, *second)
            );
            for hit in list.iter().take(SHOWN) {
                let place = match hit.record {
                    Some((record, at)) => format!("in record {record} at +{at:#x}"),
                    None => "past the chain".into(),
                };
                println!(
                    "      {} {:#x} {} {place}  {}",
                    hit.stream,
                    hit.offset,
                    if hit.wide { "u64" } else { "u32" },
                    hit.context
                );
            }
        }
    }
    Ok(())
}
