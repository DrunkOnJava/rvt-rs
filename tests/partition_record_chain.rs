//! RE-35: every element frame sits in an ElementId-keyed partition record.
//!
//! The partitions of a Revit 2024 or 2025 project open with a chain of
//! records whose headers carry ElementIds
//! ([`partition_record_chain`]). A frame with its ElementId at `+0x00`
//! starts its record, so on a file whose frames all carry their id the
//! file is its own oracle: every such frame must start a record of the
//! same id, and the chains together must hold (nearly) every ElementId the
//! ElemTable declares. That is what lets a second-prologue frame, which has
//! no id at `+0x00`, take its enclosing record's.
//!
//! Runs on every 2024 / 2025 `.rvt` under `RVT_PROJECT_CORPUS_DIR`
//! (`2024_Core_Interior.rvt` and the RE1 models in CI); skips without it.

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET, bbox_marker,
    enclosing_record, partition_record_chain,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

fn corpus_models() -> Vec<PathBuf> {
    let Some(dir) = std::env::var_os("RVT_PROJECT_CORPUS_DIR") else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut models: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.extension().is_some_and(|ext| ext == "rvt"))
        .collect();
    models.sort();
    models
}

#[test]
fn every_first_prologue_frame_starts_a_record_of_its_own_id() {
    let mut checked = 0;
    for path in corpus_models() {
        let mut rf = RevitFile::open(&path).expect("open");
        let version = rf.basic_file_info().expect("BasicFileInfo").version;
        let Some(marker) = bbox_marker(version) else {
            continue;
        };
        let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)
            .expect("ElemTable")
            .into_iter()
            .map(|r| r.id_primary)
            .collect();
        let mut with_record = BTreeSet::new();
        let (mut frames, mut starting) = (0usize, 0usize);
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            let chain = partition_record_chain(buf, &marker);
            with_record.extend(
                chain
                    .iter()
                    .filter_map(|record| u32::try_from(record.element_id).ok())
                    .filter(|id| declared.contains(id)),
            );
            for hit in memchr::memmem::find_iter(buf, &marker) {
                let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                    continue;
                };
                let Some(category) = u64_at(buf, offset + CATEGORY_OFFSET) else {
                    continue;
                };
                if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&(category as i64)) {
                    continue;
                }
                let Some(id) = u64_at(buf, offset).and_then(|v| u32::try_from(v).ok()) else {
                    continue;
                };
                if !declared.contains(&id) {
                    continue;
                }
                frames += 1;
                if enclosing_record(&chain, offset)
                    .is_some_and(|r| r.start == offset && r.element_id == u64::from(id))
                {
                    starting += 1;
                }
            }
        }
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        assert!(frames > 0, "{name}: no first-prologue frames");
        assert_eq!(starting, frames, "{name}: frames starting their record");
        // One declared id per file may have no record (4,667 of 4,668 on
        // RE1 Architecture); 26,425 of 26,425 on Core Interior.
        assert!(
            with_record.len() + 1 >= declared.len(),
            "{name}: {} of {} declared ids have a record",
            with_record.len(),
            declared.len()
        );
        checked += 1;
    }
    if checked == 0 {
        eprintln!("skipping: no 2024 / 2025 .rvt under RVT_PROJECT_CORPUS_DIR");
    }
}
