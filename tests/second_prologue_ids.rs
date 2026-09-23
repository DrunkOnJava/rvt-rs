//! RE-34: the ElementId of a second-prologue frame, inferred from its
//! reference list and the ascending frame order, measured with the answer
//! hidden.
//!
//! Every element record on `2024_Core_Interior.rvt` carries its ElementId
//! at `+0x00`, so the file is its own oracle: overwrite each id with the
//! `0x1_ffffffff` a Snowdon second-prologue frame holds there, run
//! [`assign_second_prologue_ids`], and compare. A wrong id is worse than
//! none, so the bar is zero wrong.
//!
//! Needs `RVT_PROJECT_CORPUS_DIR`; skips without it.

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    assign_second_prologue_ids, bbox_marker, context_element_ids,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn core_interior() -> Option<PathBuf> {
    let path =
        PathBuf::from(std::env::var_os("RVT_PROJECT_CORPUS_DIR")?).join("2024_Core_Interior.rvt");
    path.exists().then_some(path)
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

#[test]
fn hidden_element_ids_come_back_with_none_wrong() {
    let Some(path) = core_interior() else {
        eprintln!("skipping: RVT_PROJECT_CORPUS_DIR has no 2024_Core_Interior.rvt");
        return;
    };
    let mut rf = RevitFile::open(&path).expect("open");
    let marker = bbox_marker(2024).expect("2024 marker");
    let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)
        .expect("ElemTable")
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

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
            truth.push((offset, id as u32));
            hidden[offset..offset + 8].copy_from_slice(&0x0000_0001_ffff_ffff_u64.to_le_bytes());
        }
        hidden_streams.push((hidden, truth));
    }
    let context = context_element_ids(
        hidden_streams.iter().map(|(hidden, _)| hidden.as_slice()),
        &marker,
        &declared,
    );
    let (mut correct, mut wrong) = (0usize, 0usize);
    for (hidden, truth) in &hidden_streams {
        let assigned = assign_second_prologue_ids(hidden, &marker, &declared, &context);
        for (offset, id) in truth {
            match assigned.get(offset) {
                Some(picked) if picked == id => correct += 1,
                Some(_) => wrong += 1,
                None => {}
            }
        }
    }
    eprintln!("hold-out: {correct} correct, {wrong} wrong");
    assert_eq!(wrong, 0, "a hidden ElementId came back wrong");
    assert!(correct >= 3_000, "recall regressed: {correct} correct");
}
