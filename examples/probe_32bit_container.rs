//! A Revit 2023 element record's container reference is the `u32` 22 bytes
//! before its marker (16 after the category), not the `u64` at 2024's
//! place relative to the marker.
//!
//! The probe joins a 2023 file with the 2024 copy of the same model on
//! ElementId (Autodesk re-saves its sample projects in every release, and
//! ElementIds survive the upgrade). For every record both files hold, it
//! compares the 2024 record's container (`u64` at `+0x32`) with two readings
//! of the 2023 record:
//! - the `u64` 30 bytes before the marker, where 2024's sits relative to it;
//! - the `u32` 22 bytes before the marker that
//!   `partition_element_records_2023::scan_records` now reads.
//!
//! `0xffff_ffff` and `0xffff_ffff_ffff_ffff` are the unset values.
//!
//! ```bash
//! tools/fetch-autodesk-samples.sh /tmp/samples 2023 2024
//! cargo run --release --example probe_32bit_container -- \
//!     /tmp/samples/2023-racbasicsampleproject.rvt /tmp/samples/2024-racbasicsampleproject.rvt
//! ```
//!
//! Measured on Autodesk's three sample projects of 2023 against their 2024
//! copies, the `u32` agrees with 2024 on every shared record and the `u64`
//! misses each record that is a container member.

use rvt::RevitFile;
use rvt::partition_element_records::{
    BBOX_MARKER_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CATEGORY_OFFSET,
    CONTAINER_OFFSET,
};
use std::collections::BTreeMap;

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?))
}

/// The 2024 file's containers by ElementId, read at the 2024 offsets.
fn containers_2024(path: &str) -> anyhow::Result<BTreeMap<u64, u64>> {
    let mut rf = RevitFile::open(path)?;
    let version = rf.basic_file_info()?.version;
    let marker = rvt::partition_element_records::file_bbox_marker(&mut rf, version)
        .ok_or_else(|| anyhow::anyhow!("{path}: no 2024-shaped record marker"))?;
    let mut out = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &marker) {
            let Some(start) = at.checked_sub(BBOX_MARKER_OFFSET) else {
                continue;
            };
            let (Some(id), Some(category), Some(container)) = (
                u64_at(buf, start),
                u64_at(buf, start + CATEGORY_OFFSET),
                u64_at(buf, start + CONTAINER_OFFSET),
            ) else {
                continue;
            };
            if (BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&(category as i64)) {
                out.insert(id, container);
            }
        }
    }
    Ok(out)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [old, reference] = args.as_slice() else {
        anyhow::bail!("usage: probe_32bit_container FILE_2023.rvt FILE_2024.rvt");
    };
    let expected = containers_2024(reference)?;
    let mut rf = RevitFile::open(old)?;
    let version = rf.basic_file_info()?.version;
    let (marker, _) = rvt::partition_element_records_2023::record_marker(&mut rf)
        .ok_or_else(|| anyhow::anyhow!("{old}: no Outline/ElementParents in the schema"))?;
    let (mut shared, mut set, mut wide_agrees, mut narrow_agrees, mut set_wide) = (0, 0, 0, 0, 0);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &marker) {
            let (Some(id_at), Some(category_at)) = (at.checked_sub(52), at.checked_sub(38)) else {
                continue;
            };
            let (Some(id), Some(category)) = (u32_at(buf, id_at), u64_at(buf, category_at)) else {
                continue;
            };
            if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&(category as i64)) {
                continue;
            }
            let Some(&want) = expected.get(&u64::from(id)) else {
                continue;
            };
            let (Some(wide), Some(narrow)) = (u64_at(buf, at - 30), u32_at(buf, at - 22)) else {
                continue;
            };
            let narrow = if narrow == u32::MAX {
                u64::MAX
            } else {
                u64::from(narrow)
            };
            shared += 1;
            set += usize::from(want != u64::MAX);
            wide_agrees += usize::from(wide == want);
            narrow_agrees += usize::from(narrow == want);
            set_wide += usize::from(want != u64::MAX && wide == want);
        }
    }
    println!(
        "{version} {old}: {shared} records shared with the 2024 copy, {set} in a container there; \
         u64 at marker-30 agrees on {wide_agrees} ({set_wide} of the container members), \
         u32 at marker-22 on {narrow_agrees}"
    );
    Ok(())
}
