//! #1489: why two `Single-Flush` doors of `rac_basic`'s 2027 save write no
//! `Qto_DoorBaseQuantities` Width and Height.
//!
//! FACT under test: the quantities are written where the symbol that draws an
//! instance and the instance's type hold the same Width and Height
//! (`type_door_sizes` needs every copy of an entry to agree). RE-318's probe
//! shows doors 431064 and 485679 are drawn by symbol 431054 of type 232780 on
//! the 2025, 2026 and 2027 saves alike, so the symbol is not re-resolved on
//! 2027. This prints every partition's Width and Height entries for those
//! elements, with two other symbols of the type (490150 and 235155), whose
//! doors keep their quantities, as the control.
//!
//! Measure run 42555155286 (5a190e2f) found, on the 2027 save, three entries of
//! another layout (Width 1.5 m, Height 2.7 m) 11,053 to 13,263 bytes past
//! 431054's block mark with no mark between, which the 2026 save holds past
//! 420952's block instead; and every Width entry, owned or not, outside the
//! partition's record chain (RE-35), so the chain cannot bound a block. The
//! `block` section below asks what lies between a block's mark and the next
//! mark of any id: every length entry with the bytes before it, every declared
//! ElementId of at least 100000 stored as a u32, and those ids' classes.
//!
//! Output, one entry per line:
//!   entry <partition> <owner> width=<feet|-> height=<feet|->
//!   declared <owner> <bool>
//!   block <partition> <owner> @<mark> next=+<bytes> <next mark's id> declared=<bool>
//!     length +<offset> parameter=<id> value=<feet> pre=<hex of the 32 bytes before the value>
//!     id +<offset> <declared ElementId>            (first offset of each, at most 60)
//!   class <ElementId> <class name>                 (each id the blocks listed)
//!   held <partition> <owner> parameter=<id> value=<feet>
//!   sizes <owner> <width,height|none>   (what `type_door_sizes` returns)
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1489_door_size_entries -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::elem_table;
use rvt::research::partition_type_parameters::{
    VALUE_BLOCK_MARK, WINDOW_HEIGHT_PARAMETER, WINDOW_WIDTH_PARAMETER, scan_partition_lengths,
    type_door_sizes,
};

const OWNERS: [u32; 4] = [431054, 490150, 235155, 232780];
/// The block the 2026 save holds the three 1.5 m entries past.
const BLOCK_OWNERS: [u32; 5] = [431054, 490150, 235155, 232780, 420952];
const MAX_LENGTHS: usize = 40;
const MAX_IDS: usize = 60;
const MIN_ID: u32 = 100_000;

fn u64_at(bytes: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(bytes.get(at..at + 8)?.try_into().ok()?))
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let records = elem_table::parse_records(&mut rf)?;
    let declared = elem_table::declared_ids(&records);
    let parameters = BTreeSet::from([WINDOW_WIDTH_PARAMETER, WINDOW_HEIGHT_PARAMETER]);
    let owners: BTreeSet<u32> = OWNERS.into_iter().collect();
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            println!("unread {stream}");
            continue;
        };
        for (owner, held) in scan_partition_lengths(inflated.bytes(), &declared, &parameters) {
            if !owners.contains(&owner) {
                continue;
            }
            let value = |parameter: i64| {
                held.get(&parameter)
                    .map_or("-".to_string(), |feet| format!("{feet}"))
            };
            println!(
                "entry {stream} {owner} width={} height={}",
                value(WINDOW_WIDTH_PARAMETER),
                value(WINDOW_HEIGHT_PARAMETER)
            );
        }
    }
    for owner in OWNERS {
        println!("declared {owner} {}", declared.contains(&owner));
    }
    // What lies between each owner's block mark and the next mark of any id.
    let block_owners: BTreeSet<u64> = BLOCK_OWNERS.iter().map(|&id| u64::from(id)).collect();
    let mut listed: BTreeSet<u64> = BTreeSet::new();
    let mut found: Vec<(String, usize, i64, f64)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let bytes = inflated.bytes();
        let marks: Vec<usize> = memchr::memmem::find_iter(bytes, &VALUE_BLOCK_MARK)
            .filter(|&at| at >= 8 && bytes[at - 1] != 0xff)
            .collect();
        for (index, &mark) in marks.iter().enumerate() {
            let Some(owner) = u64_at(bytes, mark - 8).filter(|id| block_owners.contains(id)) else {
                continue;
            };
            let end = marks.get(index + 1).copied().unwrap_or(bytes.len());
            let next_id = marks
                .get(index + 1)
                .and_then(|&next| u64_at(bytes, next - 8))
                .map_or("end".to_string(), |id| id.to_string());
            let next_declared = marks
                .get(index + 1)
                .and_then(|&next| u64_at(bytes, next - 8))
                .and_then(|id| u32::try_from(id).ok())
                .is_some_and(|id| declared.contains(&id));
            println!(
                "block {stream} {owner} @{mark} next=+{} {next_id} declared={next_declared}",
                end - mark
            );
            let stretch = &bytes[mark..end];
            let mut lengths = 0;
            for at in memchr::memmem::find_iter(stretch, &[0xff; 8]) {
                if lengths == MAX_LENGTHS {
                    break;
                }
                if at < 8 || stretch[at - 1] == 0xff || stretch.get(at + 8) == Some(&0xff) {
                    continue;
                }
                let (Some(value), Some(parameter)) = (
                    stretch
                        .get(at - 8..at)
                        .and_then(|b| b.try_into().ok())
                        .map(f64::from_le_bytes),
                    stretch
                        .get(at + 8..at + 16)
                        .and_then(|b| b.try_into().ok())
                        .map(i64::from_le_bytes),
                ) else {
                    continue;
                };
                if !value.is_finite()
                    || value.abs() > 1.0e4
                    || !(-3_000_000..0).contains(&parameter)
                {
                    continue;
                }
                let from = (at - 8).saturating_sub(32);
                let pre: String = stretch[from..at - 8]
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect();
                println!(
                    "  length +{} parameter={parameter} value={value} pre={pre}",
                    at - 8
                );
                found.push((stream.clone(), mark + at - 8, parameter, value));
                lengths += 1;
            }
            listed.insert(owner);
            let mut ids: BTreeMap<u32, usize> = BTreeMap::new();
            for (at, window) in stretch.windows(4).enumerate() {
                let id = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
                if id >= MIN_ID && u64::from(id) != owner && declared.contains(&id) {
                    ids.entry(id).or_insert(at);
                }
            }
            let mut ids: Vec<(usize, u32)> = ids.into_iter().map(|(id, at)| (at, id)).collect();
            ids.sort_unstable();
            for &(at, id) in ids.iter().take(MAX_IDS) {
                println!("  id +{at} {id}");
                listed.insert(u64::from(id));
            }
        }
    }
    // The classes of the ids the blocks listed.
    if !listed.is_empty() {
        // No graph is decoded: only each record's class and where its bytes lie.
        let options = rvt::research::native_document::Options {
            selected_ids: listed.clone(),
            graph_classes: Some(BTreeSet::new()),
            ..rvt::research::native_document::Options::default()
        };
        let mut classes: BTreeMap<u64, String> = BTreeMap::new();
        let mut spans: Vec<(u64, String, usize, usize)> = Vec::new();
        let read = rvt::research::native_document::extract_graphs(&mut rf, &options, |record| {
            let class = record.class_name.clone().unwrap_or_else(|| "-".to_string());
            if let Some(start) = record
                .source
                .group
                .inflated_offset
                .and_then(|at| at.checked_add(record.source.group_record_offset))
            {
                println!(
                    "record {} {class} {} @{start}..{}",
                    record.identity.element_id,
                    record.source.stream,
                    start + record.source.body_bytes
                );
                spans.push((
                    record.identity.element_id,
                    record.source.stream.clone(),
                    start,
                    start + record.source.body_bytes,
                ));
            } else {
                println!(
                    "record {} {class} {} not in the partition's own bytes",
                    record.identity.element_id, record.source.stream
                );
            }
            classes.insert(record.identity.element_id, class);
            Ok(())
        });
        if let Err(error) = read {
            println!("class read failed: {error}");
        }
        for id in &listed {
            println!("class {id} {}", classes.get(id).map_or("-", String::as_str));
        }
        // Which record's bytes hold each length entry the blocks listed, and the
        // nearest record that starts before it.
        for (stream, at, parameter, value) in &found {
            let within: Vec<String> = spans
                .iter()
                .filter(|(_, s, start, end)| s == stream && start <= at && at < end)
                .map(|(id, ..)| id.to_string())
                .collect();
            let before = spans
                .iter()
                .filter(|(_, s, start, _)| s == stream && start <= at)
                .max_by_key(|(_, _, start, _)| *start)
                .map_or("none".to_string(), |(id, _, start, end)| {
                    format!("{id} @{start}..{end}")
                });
            println!(
                "in {stream} @{at} parameter={parameter} value={value} within={} before={before}",
                if within.is_empty() {
                    "none".to_string()
                } else {
                    within.join(",")
                }
            );
        }
    }
    // Every Width and Height entry each owner's blocks hold, value by value: the
    // scan drops a parameter whose entries in one partition disagree.
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let entries = rvt::research::partition_type_parameters::partition_length_entries(
            inflated.bytes(),
            &declared,
        );
        for owner in OWNERS {
            for (parameter, value) in entries.get(&owner).into_iter().flatten() {
                if parameters.contains(parameter) {
                    println!("held {stream} {owner} parameter={parameter} value={value}");
                }
            }
        }
    }
    let sizes = type_door_sizes(&mut rf, version, &owners);
    for owner in OWNERS {
        match sizes.get(&owner) {
            Some(size) => println!("sizes {owner} {},{}", size.width_feet, size.height_feet),
            None => println!("sizes {owner} none"),
        }
    }
    Ok(())
}
