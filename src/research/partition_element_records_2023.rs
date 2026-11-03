//! Revit 2023 partition element records (RE-81, #421), and Revit 2008 to
//! 2022's, which take the same layout (RE-180, RE-181, RE-259, RE-260).
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
//! decodes on top of the records is read for 2023 as far as wall joins,
//! whose reference-list partners trim walls as on 2024 (RE-120), and, on 2016
//! to 2023, design options, whose id sits in the frame 26 bytes before the
//! marker and whose sets [`crate::partition_design_options`] reads
//! ([`EXPORT_FILTER_RELEASES`], RE-315). Family
//! instances' family and type names are read (RE-109), and so are
//! system-family types and their layers (RE-111, RE-112). Door and window
//! hosts are: the reference lists carry them as on 2024 (RE-85), and so they
//! carry each element's Level (RE-107).
//!
//! A frame with no declared ElementId 52 bytes before its marker (the
//! second prologue, RE-30) sits inside its element's partition record, as
//! on 2024 (RE-35). The record is `u32` ElementId, `u32` size, the tag of
//! `ElementHeader` and a `u16` count, the body, and a 12-byte trailer: a
//! `u32`, a flag word and the size again. [`record_spans`] walks them, and
//! [`scan_records`] gives a placed-instance frame the id of the declared
//! record around it when the frame's own reference list names that id
//! (RE-315, #1261).

use crate::RevitFile;
use crate::le_bytes;
use crate::partition_element_records::{
    BBOX_MARKER_OFFSET, BBOX_OFFSET, BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, CLASS_TAG_OFFSET,
    CONTAINER_NONE, OST_CURTAIN_WALL_PANELS, PLACEMENT_KIND_INSTANCE, PLACEMENT_KIND_OFFSET,
    PartitionElementRecord,
};
use std::collections::BTreeSet;

/// The release this layout is first measured on.
pub const REVIT_2023: u32 = 2023;

/// The releases whose records take this layout: Revit 2008 to 2023
/// (RE-180, RE-181, RE-259, RE-260). On Autodesk's 2019, 2020, 2021 and 2022 sample projects
/// every marker of the file's own schema has a declared `u32` ElementId 52
/// bytes before it and the tag of `ElementHeader` 8 bytes after that, as on
/// 2023, but for 4 to 109 per file, as many as on the file's 2023 copy but on
/// 2021 `rac_advanced` (`probes/examples/probe_re180_revit_2019_2022.rs`). On their
/// 2016, 2017 and 2018 copies it is the same, with as many markers without
/// one as on the 2019 copy, and on three of Autodesk's 2015 templates all but
/// 0 to 68 per file (`probes/examples/probe_re181_revit_2015_2018.rs`). On every
/// marker of eight local Revit 2011 to 2014 files, among them Autodesk's
/// 2013 `rac_advanced_sample_project`, the id is 52 bytes before it too
/// (`probes/examples/probe_re259_revit_2011_2014.rs`). Those files are closed
/// source and measured locally only. So do local Revit 2009 and 2010
/// files, and a local Revit 2008 file puts the id 48 bytes before the
/// marker ([`envelope_shortfall`], `probes/examples/probe_re260_revit_2008_2010.rs`).
/// No 2008 to 2010 file has a copy of its model to check its fields
/// against: those releases rest on their own files' consistency (RE-260).
pub const REVIT_32BIT_RELEASES: std::ops::RangeInclusive<u32> = 2008..=2023;

/// Whether `revit_version` writes this module's 32-bit records and the
/// 32-bit name, material, layer and Level frames that go with them.
pub fn is_32bit_release(revit_version: u32) -> bool {
    REVIT_32BIT_RELEASES.contains(&revit_version)
}

/// The releases whose elements outside the export phase and in a non-primary
/// design option are left out (RE-315): Revit 2016 to 2023. The design option
/// layout is read on Autodesk's `rst_basic_sample_project` of each of them,
/// and the phase filter's only evidence that it is right on a 32-bit file is
/// a 2018 project. 2008 to 2015 have no evidence, and keep every element.
pub const EXPORT_FILTER_RELEASES: std::ops::RangeInclusive<u32> = 2016..=2023;

/// Whether `revit_version` leaves out the elements outside the export phase
/// and in a non-primary design option ([`EXPORT_FILTER_RELEASES`]).
pub fn applies_export_filters(revit_version: u32) -> bool {
    EXPORT_FILTER_RELEASES.contains(&revit_version)
}

/// The schema tags the 32-bit readers match, each the tag of the named
/// class in the release's own schema. The constants measured on 2023
/// (RE-111 to RE-116) are 2023's tags of these classes, and 2024's
/// counterparts are its tags of the same classes (RE-180).
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
/// schema), or `None` outside [`REVIT_32BIT_RELEASES`]. The 2008 to 2014
/// rows are read from local files of each release (RE-259, RE-260), every file of
/// a release giving the same tags. No 2015 sample
/// project is published: that row is read from three of Autodesk's 2015
/// templates (RE-181).
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
        2008 => tags(0x0179, 0x04fb, 0x0615, 0x00fd, 0x0864),
        2009 => tags(0x01a3, 0x0556, 0x06a8, 0x011d, 0x094d),
        2010 => tags(0x01b1, 0x05b5, 0x06e4, 0x012c, 0x09e0),
        2011 => tags(0x01d5, 0x0668, 0x07ab, 0x013d, 0x0aab),
        2012 => tags(0x0217, 0x0702, 0x0865, 0x0183, 0x0b8c),
        2013 => tags(0x0250, 0x07e2, 0x095e, 0x01b9, 0x0cfc),
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

/// The [`SchemaTags32`] of the file's own schema, or `None` when the
/// schema does not parse or lacks one of the classes. On a file saved by
/// the release it was made in they are that release's [`schema_tags`].
pub fn file_schema_tags(rf: &mut RevitFile) -> Option<SchemaTags32> {
    let classes = rf.memoised_schema_classes().ok()?;
    let tag = |name: &str| {
        classes
            .classes
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.tag)
    };
    Some(SchemaTags32 {
        cell_list: tag("CellList")?,
        material: tag("Material")?,
        physical_param_set: tag("PhysicalParamSet")?,
        pattern_helper: tag("PatternHelper")?,
        vertical_regions_structure: tag("VerticalRegionsStructure")?,
    })
}

/// Bytes from the ElementId to the marker.
const ID_BEFORE_MARKER: usize = 52;
/// Bytes the envelope of a Revit 2008 record lacks between the category
/// and the placement kind: there the id is 48 bytes before the marker and
/// the category 34, and every byte from the placement kind to the marker
/// is where 2009's is. On every record of the local 2008 file the bytes
/// between the category and the placement kind are `ff`, 12 of them where
/// 2009 to 2023 write 16 (RE-260); which four are missing is not shown, and
/// the container reference, read where it is from 2009 on, is unset there.
const ENVELOPE_SHORTER_2008: usize = 4;

/// How many bytes shorter than 2009 to 2023's the envelope of a
/// `revit_version` record is between its category and its placement kind.
pub fn envelope_shortfall(revit_version: u32) -> usize {
    if revit_version == 2008 {
        ENVELOPE_SHORTER_2008
    } else {
        0
    }
}
/// Bytes from the `BuiltInCategory` to the marker.
const CATEGORY_BEFORE_MARKER: usize = 38;
/// Bytes from the container reference to the marker. The reference is a
/// `u32` 16 bytes after the category, followed by `ff ff ff ff`. Joined on
/// ElementId with the 2024 copy of Autodesk's same sample project, it gives
/// 2024's container on every shared record of the three 2023 samples
/// (1,201, 5,858 and 8,901 records), where the `u64` at 2024's place
/// relative to the marker (8 bytes after the category) disagrees on 9,
/// 4,183 and 95, every container member among them
/// (`probes/examples/probe_32bit_container.rs`).
const CONTAINER_BEFORE_MARKER: usize = 22;
/// Bytes from the design option reference to the marker. The 8 bytes after
/// the category are two `u32` slots; the second holds the ElementId of the
/// design option the element belongs to, and `ff ff ff ff` for the main
/// model (RE-315). The first holds the id of an object a line belongs to on
/// the frames that set it, and is not read.
const DESIGN_OPTION_BEFORE_MARKER: usize = 26;
/// Offset of the `ElementHeader` tag after the ElementId (after the `u32`
/// id and the `u32` size).
const HEADER_TAG_AFTER_ID: usize = 8;
/// Longest reference list accepted, as on 2024.
const MAX_REFERENCES: usize = 1024;

/// The element-record marker of a file, from its schema (RE-80), or `None`
/// when the schema has no `Outline` or `ElementParents` class.
pub fn record_marker(rf: &mut RevitFile) -> Option<([u8; 8], u16)> {
    let classes = rf.memoised_schema_classes().ok()?;
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

/// Length of the header that opens a 32-bit partition record: the `u32`
/// ElementId, the `u32` record size, the tag of `ElementHeader` and a `u16`
/// count (RE-315).
pub const RECORD_HEADER_LEN: usize = 12;

/// Length of the trailer that closes a 32-bit partition record: a `u32`, a
/// flag word and the record size again (RE-315). The flag word is `0`,
/// `0x0100_0000` or, on a record that holds more, another value
/// (`0x0100_11cc` on one record of a Revit 2018 project), so it is not read.
pub const RECORD_TRAILER_LEN: usize = 12;

/// One record of a 32-bit partition: where it starts, where its body ends
/// (its trailer starts there) and the ElementId in its header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordSpan {
    /// Offset of the record header.
    pub start: usize,
    /// Offset one past the record body, where its trailer starts.
    pub end: usize,
    /// The ElementId the header opens with, declared in `Global/ElemTable`.
    pub element_id: u32,
}

/// The record that starts at `start`, as `(element_id, end)`, or `None` when
/// `start` is not a record header whose trailer echoes its size.
fn record_at(buf: &[u8], start: usize, header_tag: u16) -> Option<(u32, usize)> {
    let element_id = le_bytes::u32_at(buf, start)?;
    let size = le_bytes::u32_at(buf, start.checked_add(4)?)? as usize;
    if le_bytes::u16_at(buf, start.checked_add(HEADER_TAG_AFTER_ID)?)? != header_tag
        || size < RECORD_HEADER_LEN
    {
        return None;
    }
    let end = start.checked_add(size)?;
    let echo = le_bytes::u32_at(buf, end.checked_add(8)?)? as usize;
    (echo == size).then_some((element_id, end))
}

/// The records of one inflated 32-bit partition whose ElementId is in
/// `declared`, in file order (RE-315).
///
/// A partition is runs of back-to-back records ([`RECORD_HEADER_LEN`],
/// body, [`RECORD_TRAILER_LEN`]) with stretches between them that hold no
/// record whose trailer echoes its size. The walk starts at offset 0, steps
/// over each record that validates, and where one does not resumes at the
/// next place the `ElementHeader` tag sits 8 bytes into a record that
/// validates. A record of a loaded family's own document (RE-135) is stepped
/// over and left out, as its id is not declared. A record is never entered,
/// so the spans do not overlap and each step moves forward, which makes the
/// walk linear in `buf`.
pub fn record_spans(buf: &[u8], header_tag: u16, declared: &BTreeSet<u32>) -> Vec<RecordSpan> {
    let needle = header_tag.to_le_bytes();
    let finder = memchr::memmem::Finder::new(&needle);
    let mut spans = Vec::new();
    let mut at = 0usize;
    loop {
        if let Some((element_id, end)) = record_at(buf, at, header_tag) {
            if declared.contains(&element_id) {
                spans.push(RecordSpan {
                    start: at,
                    end,
                    element_id,
                });
            }
            at = end + RECORD_TRAILER_LEN;
            continue;
        }
        let from = at + HEADER_TAG_AFTER_ID + 1;
        let Some(rest) = buf.get(from..) else {
            break;
        };
        let Some(hit) = finder.find(rest) else {
            break;
        };
        at = from + hit - HEADER_TAG_AFTER_ID;
    }
    spans
}

/// The span of `spans` (from [`record_spans`]) whose body holds `offset`.
pub fn enclosing_span(spans: &[RecordSpan], offset: usize) -> Option<&RecordSpan> {
    let index = spans.partition_point(|span| span.start <= offset);
    let span = spans.get(index.checked_sub(1)?)?;
    (offset < span.end).then_some(span)
}

/// The spans of one partition, walked the first time a frame needs them.
struct PartitionSpans<'a> {
    buf: &'a [u8],
    header_tag: u16,
    declared: &'a BTreeSet<u32>,
    spans: Option<Vec<RecordSpan>>,
}

impl<'a> PartitionSpans<'a> {
    fn new(buf: &'a [u8], header_tag: u16, declared: &'a BTreeSet<u32>) -> Self {
        Self {
            buf,
            header_tag,
            declared,
            spans: None,
        }
    }

    fn enclosing_id(&mut self, offset: usize) -> Option<u32> {
        let (buf, header_tag, declared) = (self.buf, self.header_tag, self.declared);
        let spans = self
            .spans
            .get_or_insert_with(|| record_spans(buf, header_tag, declared));
        enclosing_span(spans, offset).map(|span| span.element_id)
    }
}

/// Every 2008 to 2023 element record in the file whose ElementId is
/// declared in `Global/ElemTable`: a frame that opens with its declared id,
/// and a placed-instance frame without one (the second prologue) that sits
/// in a declared record whose id its reference list names (RE-315). Empty
/// for any other release (fail closed).
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
        let mut spans = PartitionSpans::new(buf, header_tag, &declared);
        for at in memchr::memmem::find_iter(buf, &marker) {
            if let Some(record) = decode_at_marker(
                &stream,
                buf,
                at,
                &declared,
                header_tag,
                envelope_shortfall(revit_version),
                &mut spans,
            ) {
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
    shortfall: usize,
    spans: &mut PartitionSpans<'_>,
) -> Option<PartitionElementRecord> {
    let id_at = at.checked_sub(ID_BEFORE_MARKER - shortfall)?;
    let leading = le_bytes::u32_at(buf, id_at)?;
    let own_id = (declared.contains(&leading)
        && le_bytes::u16_at(buf, id_at + HEADER_TAG_AFTER_ID) == Some(header_tag))
    .then_some(leading);
    let category_at = at - (CATEGORY_BEFORE_MARKER - shortfall);
    let builtin_category = le_bytes::i64_at(buf, category_at)?;
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
    let container_at = at.checked_sub(CONTAINER_BEFORE_MARKER)?;
    let container = match le_bytes::u32_at(buf, container_at)? {
        u32::MAX => u64::MAX,
        id => u64::from(id),
    };
    let placement_kind = le_bytes::u32_at(field(PLACEMENT_KIND_OFFSET, 4)?, 0)?;
    let class_tag = le_bytes::u16_at(field(CLASS_TAG_OFFSET, 2)?, 0)?;
    let mut bbox_feet = [0.0f64; 6];
    for (index, slot) in bbox_feet.iter_mut().enumerate() {
        let value = le_bytes::f64_at(field(BBOX_OFFSET + index * 8, 8)?, 0)?;
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
    let design_option = at
        .checked_sub(DESIGN_OPTION_BEFORE_MARKER)
        .and_then(|from| le_bytes::u32_at(buf, from))
        .filter(|option| *option != u32::MAX);
    let (element_id, id_from_enclosing_record) = match own_id {
        Some(id) => (id, false),
        None => {
            let id = enclosing_instance_id(
                spans,
                id_at,
                leading,
                builtin_category,
                container,
                placement_kind,
                &references,
            )?;
            (id, true)
        }
    };
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
        id_from_enclosing_record,
        design_option,
    })
}

/// The ElementId of a frame with no declared id of its own (the second
/// prologue, RE-30), from the declared record around it (RE-315).
///
/// Only a placed instance gets one (no container reference and the
/// placed-instance kind, the RE-21 rule), and only when the frame's own
/// reference list names the record's id, as an element's does (RE-23). On
/// Revit 2018's `ALD26 Before` each of the 3,350 placed-instance frames
/// without a declared id that a declared record encloses names it. A curtain
/// panel frame that opens with a count instead of `ff ff ff ff` gets none:
/// 96 of the 102 such frames sit in the record of a panel Revit's export does
/// not write (a panel's shell), and the 6 panels it does write that they
/// hold are left out with them; all 99 that open with `ff ff ff ff` are
/// panels it writes.
fn enclosing_instance_id(
    spans: &mut PartitionSpans<'_>,
    id_at: usize,
    leading: u32,
    builtin_category: i64,
    container: u64,
    placement_kind: u32,
    references: &[u64],
) -> Option<u32> {
    if container != CONTAINER_NONE || placement_kind != PLACEMENT_KIND_INSTANCE {
        return None;
    }
    if leading != u32::MAX && builtin_category == OST_CURTAIN_WALL_PANELS {
        return None;
    }
    let id = spans.enclosing_id(id_at)?;
    references.contains(&u64::from(id)).then_some(id)
}

/// A `u32`-counted list of `u32` ElementIds at `at`, widened to `u64`.
fn u32_list(buf: &[u8], at: usize) -> Option<Vec<u64>> {
    let count = le_bytes::u32_at(buf, at)? as usize;
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
