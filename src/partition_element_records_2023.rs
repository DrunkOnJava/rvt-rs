//! Revit 2023 partition element records (RE-81, #421).
//!
//! A 2023 record sits behind the same marker as 2024 and 2025,
//! `[tag of Outline][0xFF x 4][tag of ElementParents]` in the file's own
//! schema (RE-80). It opens with a chain-record header in 32-bit form: the
//! ElementId as a `u32`, the record size, the tag of `ElementHeader` and a
//! count, 52 bytes before the marker. The `BuiltInCategory` follows 38
//! bytes before the marker. The container reference, placement kind,
//! class tag and bounding box sit where 2024 has them relative to the
//! marker, so a record's `offset` here is the marker minus
//! [`crate::partition_element_records::BBOX_MARKER_OFFSET`], as on 2024.
//! After the box come two counted lists of `u32` ElementIds (2024 writes
//! `u64`).
//!
//! Measured on two 2023 projects against Revit's own IFC4 exports: every
//! element Tag of each export is a placed instance here
//! (`reports/element-framing/RE-81-2023-element-records.md`). What 2024
//! decodes on top of the records (names, types, joins, storeys, design
//! options, opening hosts) is not established for 2023 and is not read.

use crate::RevitFile;
use crate::partition_element_records::{
    BBOX_MARKER_OFFSET, BBOX_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CLASS_TAG_OFFSET,
    CONTAINER_OFFSET, PLACEMENT_KIND_OFFSET, PartitionElementRecord,
};
use std::collections::BTreeSet;

/// The release this layout is measured on.
pub const REVIT_2023: u32 = 2023;

/// Bytes from the ElementId to the marker.
const ID_BEFORE_MARKER: usize = 52;
/// Bytes from the `BuiltInCategory` to the marker.
const CATEGORY_BEFORE_MARKER: usize = 38;
/// Offset of the `ElementHeader` tag after the ElementId (after the `u32`
/// id and the `u32` size).
const HEADER_TAG_AFTER_ID: usize = 8;
/// Longest reference list accepted, as on 2024.
const MAX_REFERENCES: usize = 1024;

/// The element-record marker of a file, from its schema (RE-80), or `None`
/// when the schema has no `Outline` or `ElementParents` class.
pub fn record_marker(rf: &mut RevitFile) -> Option<([u8; 8], u16)> {
    let classes = rf.schema_classes().ok()?;
    let tag = |name: &str| {
        classes
            .classes
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.tag)
    };
    let (outline, parents, header) = (
        tag("Outline")?,
        tag("ElementParents")?,
        tag("ElementHeader")?,
    );
    let mut marker = [0xffu8; 8];
    marker[..2].copy_from_slice(&outline.to_le_bytes());
    marker[6..].copy_from_slice(&parents.to_le_bytes());
    Some((marker, header))
}

/// Every 2023 element record in the file whose ElementId is declared in
/// `Global/ElemTable`. Empty for any other release (fail closed).
pub fn scan_records(rf: &mut RevitFile, revit_version: u32) -> Vec<PartitionElementRecord> {
    if revit_version != REVIT_2023 {
        return Vec::new();
    }
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return Vec::new();
    };
    let declared = crate::elem_table::declared_ids(&table);
    let Some((marker, header_tag)) = record_marker(rf) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &marker) {
            if let Some(record) = decode_at_marker(&stream, buf, at, &declared, header_tag) {
                out.push(record);
            }
        }
    }
    out
}

/// Decode the 2023 record whose marker is at `at`.
fn decode_at_marker(
    stream: &str,
    buf: &[u8],
    at: usize,
    declared: &BTreeSet<u32>,
    header_tag: u16,
) -> Option<PartitionElementRecord> {
    let id_at = at.checked_sub(ID_BEFORE_MARKER)?;
    let element_id = u32::from_le_bytes(buf.get(id_at..id_at + 4)?.try_into().ok()?);
    if !declared.contains(&element_id) {
        return None;
    }
    let tag_at = id_at + HEADER_TAG_AFTER_ID;
    if u16::from_le_bytes(buf.get(tag_at..tag_at + 2)?.try_into().ok()?) != header_tag {
        return None;
    }
    let category_at = at - CATEGORY_BEFORE_MARKER;
    let builtin_category =
        i64::from_le_bytes(buf.get(category_at..category_at + 8)?.try_into().ok()?);
    if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&builtin_category) {
        return None;
    }
    // Relative to a 2024-shaped record start, which can lie before the
    // start of the partition: every field below is read from the marker.
    let base = at as isize - BBOX_MARKER_OFFSET as isize;
    let field = |offset: usize, len: usize| -> Option<&[u8]> {
        let from = usize::try_from(base + offset as isize).ok()?;
        buf.get(from..from + len)
    };
    let u64_at = |offset: usize| {
        field(offset, 8)
            .and_then(|b| b.try_into().ok())
            .map(u64::from_le_bytes)
    };
    let container = u64_at(CONTAINER_OFFSET)?;
    let placement_kind = u32::from_le_bytes(field(PLACEMENT_KIND_OFFSET, 4)?.try_into().ok()?);
    let class_tag = u16::from_le_bytes(field(CLASS_TAG_OFFSET, 2)?.try_into().ok()?);
    let mut bbox_feet = [0.0f64; 6];
    for (index, slot) in bbox_feet.iter_mut().enumerate() {
        let value = f64::from_le_bytes(field(BBOX_OFFSET + index * 8, 8)?.try_into().ok()?);
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    if (0..3).any(|axis| bbox_feet[axis + 3] < bbox_feet[axis]) {
        return None;
    }
    let list_at = at + 8 + 48;
    let references = u32_list(buf, list_at).unwrap_or_default();
    Some(PartitionElementRecord {
        stream: stream.to_string(),
        offset: id_at,
        element_id,
        flags: 0,
        builtin_category,
        container,
        placement_kind,
        class_tag,
        bbox_feet,
        preceding_reference: None,
        owner_reference: None,
        references,
        id_from_enclosing_record: false,
        design_option: None,
    })
}

/// A `u32`-counted list of `u32` ElementIds at `at`, widened to `u64`.
fn u32_list(buf: &[u8], at: usize) -> Option<Vec<u64>> {
    let count = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if count > MAX_REFERENCES {
        return None;
    }
    let body = buf.get(at + 4..at + 4 + count * 4)?;
    Some(
        body.chunks_exact(4)
            .map(|c| u64::from(u32::from_le_bytes([c[0], c[1], c[2], c[3]])))
            .collect(),
    )
}
