//! RE-35: element frames sit inside ElementId-keyed partition records.
//!
//! FACT: every `Partitions/*` stream of a Revit 2024 or 2025 project opens
//! with a chain of records, back to back from offset 0:
//!
//! ```text
//! +0x00  u64  ElementId the record belongs to
//! +0x08  u32  record size, header included
//! +0x0c  u16  prologue constant (release constant + 28: 0x059f / 0x05c7)
//! +0x0e  u16  count
//! ...         body
//! end    u64  (unread)  u32 flag word 0 or 0x0100_0000  u32 size again
//! ```
//!
//! A first-prologue element frame starts its record, so its `+0x00`
//! ElementId is the record's. A second-prologue frame (RE-30) sits inside
//! its element's record body, so the record header carries the id the
//! frame lacks. The chain holds nearly every ElementId `Global/ElemTable`
//! declares; what follows it are loaded families' own documents, whose
//! records carry ids of their own.
//!
//! The probe prints, per partition, the chain and where every framed
//! element falls, and checks that each first-prologue frame starts a
//! record of its own id.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re35_record_wrapper -- FILE.rvt

use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE, PLACEMENT_KIND_OFFSET, bbox_marker,
    enclosing_record, partition_record_chain, record_prologue_constant,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
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
    println!(
        "release {version}; prologue constant {:#06x}; ElemTable declares {} ids",
        record_prologue_constant(&marker),
        declared.len()
    );
    let is_declared = |id: u64| u32::try_from(id).is_ok_and(|id| declared.contains(&id));

    let mut with_record: BTreeSet<u64> = BTreeSet::new();
    let (mut first_ok, mut first_bad, mut second_declared) = (0usize, 0usize, 0usize);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = partition_record_chain(buf, &marker);
        let declared_records = chain.iter().filter(|r| is_declared(r.element_id)).count();
        with_record.extend(
            chain
                .iter()
                .map(|r| r.element_id)
                .filter(|&id| is_declared(id)),
        );
        // first prologue: starts its record / does not
        // second prologue, placed: record id declared / undeclared / after the chain
        let mut counts = [0usize; 5];
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(offset) = hit.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let Some(category) = u64_at(buf, offset + CATEGORY_OFFSET).map(|v| v as i64) else {
                continue;
            };
            if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&category) {
                continue;
            }
            let Some(head) = u64_at(buf, offset) else {
                continue;
            };
            let record = enclosing_record(&chain, offset);
            if is_declared(head) {
                let starts = record.is_some_and(|r| r.start == offset && r.element_id == head);
                counts[usize::from(!starts)] += 1;
                continue;
            }
            if head != 0 && head <= u64::from(u32::MAX) {
                continue;
            }
            let placed = u64_at(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && u64_at(buf, offset + PLACEMENT_KIND_OFFSET)
                    .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE);
            if !placed {
                continue;
            }
            counts[match record {
                Some(r) if is_declared(r.element_id) => 2,
                Some(_) => 3,
                None => 4,
            }] += 1;
        }
        if chain.is_empty() && counts.iter().all(|&c| c == 0) {
            continue;
        }
        let end = chain.last().map_or(0, |r| r.end);
        println!(
            "{stream}: chain of {} records ({declared_records} declared) ending at {end:#x}",
            chain.len()
        );
        println!(
            "  first-prologue frames: {} start a record of their own id, {} do not",
            counts[0], counts[1]
        );
        println!(
            "  second-prologue placed instances: {} in a record of a declared id, {} of an undeclared id, {} after the chain",
            counts[2], counts[3], counts[4]
        );
        first_ok += counts[0];
        first_bad += counts[1];
        second_declared += counts[2];
    }
    println!(
        "total: {} of {} declared ids have a record; first-prologue frames {first_ok} starting their record, {first_bad} not; {second_declared} second-prologue instances resolved",
        with_record.len(),
        declared.len()
    );
    Ok(())
}
