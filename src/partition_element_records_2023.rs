//! Revit 2023 partition element records (RE-81, #421), and Revit 2014 to
//! 2022's, which take the same layout (RE-178, RE-179).
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
//! decodes on top of the records (design options) is not established for
//! 2023 and is not read, except wall joins, whose reference-list partners
//! trim walls as on 2024 (RE-120); family instances' family and type
//! names are (RE-109), and so are system-family types and their layers
//! (RE-111, RE-112). Door and window hosts are:
//! the reference lists carry them as on 2024 (RE-85), and so they carry
//! each element's Level (RE-107).

use crate::RevitFile;
use crate::partition_element_records::{
    BBOX_MARKER_OFFSET, BBOX_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CLASS_TAG_OFFSET,
    CONTAINER_OFFSET, PLACEMENT_KIND_OFFSET, PartitionElementRecord,
};
use std::collections::BTreeSet;

/// The release this layout is first measured on.
pub const REVIT_2023: u32 = 2023;

/// The releases whose records take this layout: Revit 2014 to 2023
/// (RE-178, RE-179). On Autodesk's 2019, 2020, 2021 and 2022 sample projects
/// every marker of the file's own schema has a declared `u32` ElementId 52
/// bytes before it and the tag of `ElementHeader` 8 bytes after that, as on
/// 2023, but for 4 to 109 per file, as many as on the file's 2023 copy but on
/// 2021 `rac_advanced` (`examples/probe_re178_revit_2019_2022.rs`). On their
/// 2016, 2017 and 2018 copies it is the same, with as many markers without
/// one as on the 2019 copy, and on one 2014 project and three 2015 templates
/// all but 0 to 68 per file (`examples/probe_re179_revit_2014_2018.rs`).
pub const REVIT_32BIT_RELEASES: std::ops::RangeInclusive<u32> = 2014..=2023;

/// Whether `revit_version` writes this module's 32-bit records and the
/// 32-bit name, material, layer and Level frames that go with them.
pub fn is_32bit_release(revit_version: u32) -> bool {
    REVIT_32BIT_RELEASES.contains(&revit_version)
}

/// The schema tags the 32-bit readers match, each the tag of the named
/// class in the release's own schema. The constants measured on 2023
/// (RE-111 to RE-116) are 2023's tags of these classes, and 2024's
/// counterparts are its tags of the same classes (RE-178).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaTags32 {
    /// `CellList`: opens an element's serialised data (RE-111).
    pub cell_list: u16,
    /// `Material`: a material object's class tag (RE-113).
    pub material: u16,
    /// `PhysicalParamSet`: ends a material's name (RE-116).
    pub physical_param_set: u16,
    /// `PatternHelper`: frames a category's object-styles entry (RE-115).
    pub pattern_helper: u16,
    /// `VerticalRegionsStructure`: frames a type's layer count (RE-112).
    pub vertical_regions_structure: u16,
}

/// The [`SchemaTags32`] of `revit_version`, read from Autodesk's sample
/// projects of each release (every file of a release has the same
/// schema), or `None` outside [`REVIT_32BIT_RELEASES`]. No 2014 or 2015
/// sample project is published: those rows are read from one 2014 project
/// and from three of Autodesk's 2015 templates (RE-179).
pub fn schema_tags(revit_version: u32) -> Option<SchemaTags32> {
    let tags = |cell_list, material, physical_param_set, pattern_helper, vertical_regions| {
        Some(SchemaTags32 {
            cell_list,
            material,
            physical_param_set,
            pattern_helper,
            vertical_regions_structure: vertical_regions,
        })
    };
    match revit_version {
        2014 => tags(0x0251, 0x080e, 0x09a7, 0x01bc, 0x0d5a),
        2015 => tags(0x024f, 0x0871, 0x0a0b, 0x01b9, 0x0dc4),
        2016 => tags(0x0255, 0x088b, 0x0a34, 0x01be, 0x0e06),
        2017 => tags(0x025d, 0x08b7, 0x0a73, 0x01bf, 0x0e5e),
        2018 => tags(0x0263, 0x08f8, 0x0aba, 0x01ca, 0x0eeb),
        2019 => tags(0x0264, 0x0924, 0x0aef, 0x01ca, 0x0f37),
        2020 => tags(0x026c, 0x094d, 0x0b24, 0x01d0, 0x0f76),
        2021 => tags(0x027d, 0x094b, 0x0b25, 0x01e1, 0x0f79),
        2022 => tags(0x028d, 0x09af, 0x0b90, 0x01f0, 0x1003),
        2023 => tags(0x02c0, 0x09fb, 0x0beb, 0x013f, 0x106f),
        _ => None,
    }
}

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

/// Every 2014 to 2023 element record in the file whose ElementId is
/// declared in `Global/ElemTable`. Empty for any other release (fail
/// closed).
pub fn scan_records(rf: &mut RevitFile, revit_version: u32) -> Vec<PartitionElementRecord> {
    if !is_32bit_release(revit_version) {
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
