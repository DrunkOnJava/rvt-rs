//! RE-38: family and type names from the partition name entries.
//!
//! FACT: Revit 2024 and 2025 partitions carry, for every loaded family and
//! family type, an entry `u64 ElementId · u32 n · n UTF-16 code units ·
//! i64 BuiltInCategory`. A placed element's type is the one id in its
//! reference list with an entry of the element's category, and the type's
//! own partition record names its family the same way. `Family:Type` is
//! exactly the name Revit's own IFC export gives the element (checked by
//! `tests/element_names.rs`).
//!
//! The probe prints how many ids carry an entry, how many placed instances
//! resolve a type and a family, and a sample of the names.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re38_names -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    CONTAINER_NONE, CONTAINER_OFFSET, PLACEMENT_KIND_INSTANCE, PLACEMENT_KIND_OFFSET,
    REFERENCE_LIST_OFFSET, bbox_marker, decode_reference_list,
};
use rvt::partition_names::{resolve_family, resolve_type};
use std::collections::BTreeMap;
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
    let names = rf.element_names();
    let with_family = names
        .type_family_candidates
        .keys()
        .filter(|&&t| resolve_family(&names, t).is_some())
        .count();
    println!(
        "release {version}: {} ids carry a name entry; {with_family} named types resolve one family",
        names.entries.len()
    );
    let ids = rf.second_prologue_ids();
    let declared: std::collections::BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();
    let (mut instances, mut typed, mut named) = (0usize, 0usize, 0usize);
    let mut sample: BTreeMap<String, usize> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
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
            let placed = u64_at(buf, offset + CONTAINER_OFFSET) == Some(CONTAINER_NONE)
                && u64_at(buf, offset + PLACEMENT_KIND_OFFSET)
                    .is_some_and(|v| (v & 0xffff_ffff) as u32 == PLACEMENT_KIND_INSTANCE);
            if !placed {
                continue;
            }
            let own = u64_at(buf, offset)
                .and_then(|v| u32::try_from(v).ok())
                .filter(|id| declared.contains(id))
                .or_else(|| ids.get(&stream).and_then(|m| m.get(&offset)).copied());
            let Some(own) = own else {
                continue;
            };
            instances += 1;
            let references =
                decode_reference_list(buf, offset + REFERENCE_LIST_OFFSET).unwrap_or_default();
            let Some(type_id) = resolve_type(&names, &references, category, own) else {
                continue;
            };
            typed += 1;
            let Some(family_id) = resolve_family(&names, type_id) else {
                continue;
            };
            named += 1;
            let label = format!(
                "{}:{}",
                names.entries[&family_id].name, names.entries[&type_id].name
            );
            *sample.entry(label).or_default() += 1;
        }
    }
    println!("{instances} placed instances; {typed} resolve a type, {named} a family and type");
    let mut common: Vec<(String, usize)> = sample.into_iter().collect();
    common.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (label, count) in common.into_iter().take(15) {
        println!("  {count:>5}  {label}");
    }
    Ok(())
}
