//! RE-34 hold-out: hide every first-prologue ElementId and see whether
//! [`assign_second_prologue_ids`] gets it back.
//!
//! FACT: on a file whose element records all carry their ElementId at
//! `+0x00`, overwriting each id with the `0x1_ffffffff` a second-prologue
//! frame holds and re-running the order rule reproduces the ids it assigns.
//! Every wrong pick is printed with its reference list.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re34_holdout -- FILE.rvt

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    REFERENCE_LIST_OFFSET, assign_second_prologue_ids, bbox_marker, context_element_ids,
    decode_reference_list,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known bbox marker");
        return Ok(());
    };
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

    // Pass 1: hide every declared +0x00 id, keeping the answers.
    let mut hidden_streams = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let mut hidden = inflated.bytes().to_vec();
        let mut truth = Vec::new();
        let hits: Vec<usize> = memchr::memmem::find_iter(&hidden, &marker).collect();
        for hit in hits {
            let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(category) = u64_at(&hidden, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
                continue;
            };
            if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&category) {
                continue;
            }
            let Some(id) = u64_at(&hidden, offset) else {
                continue;
            };
            if id == 0 || id > u64::from(u32::MAX) || !declared.contains(&(id as u32)) {
                continue;
            }
            truth.push((offset, category, id as u32));
            hidden[offset..offset + 8].copy_from_slice(&0x0000_0001_ffff_ffff_u64.to_le_bytes());
        }
        hidden_streams.push((stream, hidden, truth));
    }
    // Pass 2: the context count spans every partition, as in the library.
    let context = context_element_ids(
        hidden_streams
            .iter()
            .map(|(_, hidden, _)| hidden.as_slice()),
        &marker,
        &declared,
    );
    let mut score: BTreeMap<i64, (usize, usize)> = BTreeMap::new();
    for (stream, hidden, truth) in &hidden_streams {
        let assigned = assign_second_prologue_ids(hidden, &marker, &declared, &context);
        for &(offset, category, id) in truth {
            let Some(&picked) = assigned.get(&offset) else {
                continue;
            };
            let entry = score.entry(category).or_default();
            if picked == id {
                entry.0 += 1;
            } else {
                entry.1 += 1;
                let refs = decode_reference_list(hidden, offset + REFERENCE_LIST_OFFSET)
                    .unwrap_or_default();
                println!(
                    "WRONG {stream} +{offset:#x} category {category} true {id} picked {picked} refs {refs:?}"
                );
            }
        }
    }
    let (correct, wrong) = score.values().fold((0, 0), |(c, w), (a, b)| (c + a, w + b));
    println!("release {version}: {correct} correct, {wrong} wrong");
    for (category, (c, w)) in &score {
        println!("{category:>12} {c:>6} {w:>4}");
    }
    Ok(())
}
