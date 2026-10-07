//! Revit *type* records recovered from the partition streams (#88, RE-28).
//!
//! A Revit element type — the `WallType` a wall is placed from, the
//! `Material` a type names — is an element like any other: it carries
//! an `ElementId` declared in `Global/ElemTable` and a
//! `BuiltInCategory` at the same `+0x12` every element record uses.
//! What it does not carry is a bounding box, because a type is not
//! placed anywhere. Its record therefore ends where a wall record's
//! bbox marker would start, exactly as a `Level` record does
//! ([`crate::partition_level_records`]), and what follows the marker
//! is a counted list of `u64` ElementId slots.
//!
//! # The record
//!
//! Observed on `2024_Core_Interior.rvt` (sha256 `c805df44…`, Revit
//! 2024). The prologue is byte-identical to the element-record
//! prologue up to `+0x4c`:
//!
//! ```text
//! +0x00  u64  ElementId of this record (declared in Global/ElemTable)
//! +0x08  u32  record flags (0x77 / 0x7f / 0x87 / 0x8f observed)
//! +0x0c  u32  0x0000059f, as on every Revit 2024 record prologue
//! +0x10  u16  0
//! +0x12  i64  BuiltInCategory id, negative
//! +0x1a  24B  0xff sentinel padding
//! +0x32  u64  container ElementId, 0xffff_ffff_ffff_ffff = none
//! +0x3a  u64  0xff sentinel
//! +0x42  u32  placement kind
//! +0x46  u32  unattributed
//! +0x4a  u16  unattributed
//! +0x4c  u32  unattributed
//! +0x50  6B   record marker ff ff ff ff ab 05
//! +0x56  u32  slot-list length n
//! +0x5a  n*8  n u64 ElementId slots, ascending, including this id
//! ```
//!
//! The placement-kind word takes a **third** value on these records.
//! [`crate::partition_element_records`] recorded `0xffffef7f` for a
//! placed instance and `0xffff8000` for a family/type symbol
//! *envelope*; a type record whose category is `OST_Walls` carries
//! [`PLACEMENT_KIND_TYPE_DEFINITION`] (`0xffff8080`), and one whose
//! category is [`OST_MATERIALS`] carries `0xffff8020`. Only the first
//! is tested here; the second is recorded by the probe and not
//! claimed.
//!
//! # Measured on `2024_Core_Interior.rvt`
//!
//! Eight records carry `OST_Walls` with this shape and
//! [`PLACEMENT_KIND_TYPE_DEFINITION`]:
//!
//! ```text
//! 1710  1711  3897  11356  17328  17337  17339  17341
//! ```
//!
//! Revit's own export of the same file writes four `IfcWallType`
//! rows, tagged `3897`, `17328`, `17337`, `17341` — a **subset** of
//! the eight. The four extras are wall types the project defines and
//! never places, which Revit's exporter does not write; the set is
//! therefore a superset by construction and is *not* used as an
//! `IfcWallType` recovery on its own.
//!
//! What it is used for is the join. Every one of the 360 exported
//! wall instance records carries **exactly one** slot from that
//! eight-id set in its `+0x88` reference list, and that slot is the
//! `IfcWallType.Tag` Revit's own export assigns to the wall — **360
//! of 360, no wrong answer, no miss**. See
//! [`unique_type_reference`] and
//! `reports/element-framing/RE-28-wall-type-records.md`.
//!
//! That closes RE-26 §9's open `18" Basement` question. RE-26 read
//! the type from a *position* (`refs1[1]`) and scored 356 of 360,
//! because the four `18" Basement` walls carry `1851` there where the
//! export's `IfcWallType.Tag` is `3897`. Their list is
//! `[3, 1851, 3897, 20268, 20273, …]`: `1851` is a record of a
//! different category (`-2009014`) and is not in the type-record set,
//! `3897` is, and the set test picks it.
//!
//! # Honesty
//!
//! - The `BuiltInCategory` ids are Autodesk's published API
//!   constants. `OST_Materials = -2000700` is read from the same
//!   public enumeration listings the rest of the repo cites; nothing
//!   here comes from Autodesk or ODA source.
//! - The flags word, `+0x46`, `+0x4a` and `+0x4c` are recorded, not
//!   interpreted, exactly as the sibling modules record their own.
//! - `0xffff8080` is named `PLACEMENT_KIND_TYPE_DEFINITION` because
//!   the eight `OST_Walls` records carrying it are a superset of the
//!   four `IfcWallType` Revit exports and because every exported wall
//!   instance names exactly one of them. What Revit calls the word is
//!   not claimed.
//! - The slot list is named a *slot list* and nothing more. On the
//!   recorded edge it is ascending, contains the record's own id, and
//!   on `3897` its leading slot (`3652`) is an [`OST_MATERIALS`]
//!   record — but three of the four exported wall types carry no
//!   material slot at all, so no material rule is claimed and none is
//!   shipped.
//! - **No compound-layer structure is decoded here.** A 128 KiB span
//!   centred on each of the four exported wall-type records carries
//!   no run of consecutive `f64` summing to that type's nominal
//!   width, and the reference export of the same file is a
//!   ReferenceView_V1.2 document with zero `IfcMaterialLayerSet`, so
//!   the quantity has neither a carrier in the bytes nor an oracle to
//!   check against. See the report's negative sections; nothing in
//!   this module invents a layer.

use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Releases where this record shape is corpus-proven: 2024 on Core
/// Interior and the Snowdon Towers samples, 2025 on the RE1 projects,
/// whose records carry the 2025 marker and prologue constant (#322).
pub const PARTITION_TYPE_RECORD_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Autodesk `BuiltInCategory.OST_Materials`.
pub const OST_MATERIALS: i64 = -2_000_700;

/// Placement-kind word carried by a wall *type* definition record.
pub const PLACEMENT_KIND_TYPE_DEFINITION: u32 = 0xffff_8080;

/// Offset of the `0x0000059f` word every Revit 2024 prologue carries.
pub const PROLOGUE_MAGIC_OFFSET: usize = 0x0c;

/// The word itself, on Revit 2024. Every release's is its
/// [`crate::partition_element_records::record_prologue_constant`]
/// (`0x05c7` on 2025).
pub const PROLOGUE_MAGIC: u32 = 0x0000_059f;

/// Offset of the bbox-less record marker.
pub const RECORD_MARKER_OFFSET: usize = 0x50;

/// The bbox-less record marker, shared with `OST_Levels` records.
///
/// An element record carries `46 01` *before* these six bytes and
/// then a bounding box, so testing this marker at
/// [`RECORD_MARKER_OFFSET`] rejects the element shape outright. This is
/// the Revit 2024 marker; every release's is the last six bytes of its
/// bbox marker ([`record_marker`]).
pub const RECORD_MARKER: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xab, 0x05];

/// The bbox-less record marker of the release whose element-record bbox
/// marker is `bbox_marker`: its last six bytes, `ff ff ff ff` and the
/// release constant (`ab 05` on 2024, `d3 05` on 2025).
pub fn record_marker(bbox_marker: &[u8; 8]) -> [u8; 6] {
    [
        bbox_marker[2],
        bbox_marker[3],
        bbox_marker[4],
        bbox_marker[5],
        bbox_marker[6],
        bbox_marker[7],
    ]
}

/// Offset of the slot-list `u32` length prefix.
pub const SLOT_COUNT_OFFSET: usize = 0x56;

/// Offset of the first slot.
pub const SLOT_LIST_OFFSET: usize = 0x5a;

/// Largest slot count the decoder will accept.
///
/// The longest list observed on `2024_Core_Interior.rvt` is four
/// slots; the bound is far above that so a garbage length word cannot
/// make the scan read megabytes.
pub const SLOT_LIST_MAX_ENTRIES: usize = 1024;

/// Minimum bytes a complete type-record header occupies.
pub const RECORD_MIN_LEN: usize = SLOT_LIST_OFFSET + 8;

/// A decoded partition *type* record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionTypeRecord {
    /// Stream the record was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the record in the concatenated inflated stream.
    pub offset: usize,
    /// The record's own Revit ElementId.
    pub element_id: u32,
    /// Unattributed flags word at `+0x08`.
    pub flags: u32,
    /// Revit `BuiltInCategory` id (negative).
    pub builtin_category: i64,
    /// Raw container reference at `+0x32`; `u64::MAX` when unset.
    pub container: u64,
    /// Raw placement-kind word at `+0x42`.
    pub placement_kind: u32,
    /// The counted `u64` slot list at `+0x56`, verbatim.
    pub slots: Vec<u64>,
}

impl PartitionTypeRecord {
    /// True when `+0x42` carries [`PLACEMENT_KIND_TYPE_DEFINITION`].
    pub fn is_type_definition(&self) -> bool {
        self.placement_kind == PLACEMENT_KIND_TYPE_DEFINITION
    }

    /// True when `+0x32` is set, i.e. the record is a member of a
    /// container element rather than a standalone type.
    pub fn is_container_member(&self) -> bool {
        self.container != crate::partition_element_records::CONTAINER_NONE
    }
}

/// Whether this release's type-record shape is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    PARTITION_TYPE_RECORD_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u16(buf: &[u8], at: usize) -> Option<u16> {
    buf.get(at..at + 2)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// Decode one type record at `offset`, fail-closed.
///
/// `declared_ids` is the `Global/ElemTable` id set; a record whose
/// leading `u64` is not declared there is rejected outright, exactly
/// as [`crate::partition_element_records::decode_at`] rejects one.
///
/// Also rejected: a `+0x0c` word that is not [`PROLOGUE_MAGIC`], a
/// non-zero `+0x10`, a `BuiltInCategory` outside the published band,
/// a missing [`RECORD_MARKER`], a slot count of zero or above
/// [`SLOT_LIST_MAX_ENTRIES`], a list that runs past the buffer, and a
/// list that does not contain the record's own ElementId.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
) -> Option<PartitionTypeRecord> {
    decode_at_with_marker(
        stream,
        buf,
        offset,
        declared_ids,
        &crate::partition_element_records::BBOX_MARKER,
    )
}

/// [`decode_at`] for the release whose element-record bbox marker is
/// `bbox_marker` ([`crate::partition_element_records::bbox_marker`]).
pub fn decode_at_with_marker(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
    bbox_marker: &[u8; 8],
) -> Option<PartitionTypeRecord> {
    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    let raw_id = read_u64(buf, offset)?;
    if crate::partition_element_records::carries_no_element_id(raw_id) {
        return None;
    }
    let element_id = raw_id as u32;
    if !declared_ids.contains(&element_id) {
        return None;
    }
    let magic = u32::from(crate::partition_element_records::record_prologue_constant(
        bbox_marker,
    ));
    decode_frame_as(stream, buf, offset, element_id, Some(magic), bbox_marker)
}

/// A type record whose frame carries no ElementId at `+0x00` (the second
/// prologue, RE-30), decoded as `element_id`: the id of the partition
/// record it sits in (RE-35). Its slot list must name that id, as a
/// first-prologue record's names its own (#322). The second prologue does
/// not carry the `0x059f` magic at `+0x0c`, so it is not checked.
/// `bbox_marker` is the release's element-record bbox marker.
pub fn decode_second_prologue_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    element_id: u32,
    bbox_marker: &[u8; 8],
) -> Option<PartitionTypeRecord> {
    if !crate::partition_element_records::carries_no_element_id(read_u64(buf, offset)?) {
        return None;
    }
    decode_frame_as(stream, buf, offset, element_id, None, bbox_marker)
}

fn decode_frame_as(
    stream: &str,
    buf: &[u8],
    offset: usize,
    element_id: u32,
    magic: Option<u32>,
    bbox_marker: &[u8; 8],
) -> Option<PartitionTypeRecord> {
    use crate::partition_element_records as per;

    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    let own = u64::from(element_id);
    if let Some(magic) = magic {
        if read_u32(buf, offset + PROLOGUE_MAGIC_OFFSET)? != magic {
            return None;
        }
    }
    if read_u16(buf, offset + 0x10)? != 0 {
        return None;
    }
    let builtin_category = read_u64(buf, offset + per::CATEGORY_OFFSET)? as i64;
    if !(per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&builtin_category) {
        return None;
    }
    let marker = record_marker(bbox_marker);
    if buf.get(offset + RECORD_MARKER_OFFSET..offset + RECORD_MARKER_OFFSET + marker.len())?
        != marker
    {
        return None;
    }
    let count = read_u32(buf, offset + SLOT_COUNT_OFFSET)? as usize;
    if count == 0 || count > SLOT_LIST_MAX_ENTRIES {
        return None;
    }
    let start = offset.checked_add(SLOT_LIST_OFFSET)?;
    let end = start.checked_add(count.checked_mul(8)?)?;
    if end > buf.len() {
        return None;
    }
    let mut slots = Vec::with_capacity(count);
    for index in 0..count {
        slots.push(read_u64(buf, start + index * 8)?);
    }
    if !slots.contains(&own) {
        return None;
    }
    Some(PartitionTypeRecord {
        stream: stream.to_string(),
        offset,
        element_id,
        flags: read_u32(buf, offset + 0x08)?,
        builtin_category,
        container: read_u64(buf, offset + per::CONTAINER_OFFSET)?,
        placement_kind: read_u32(buf, offset + per::PLACEMENT_KIND_OFFSET)?,
        slots,
    })
}

/// Find every type record in `buf` carrying `builtin_category`.
///
/// The scan anchors on the 8-byte little-endian encoding of the
/// category id, exactly as
/// [`crate::partition_element_records::find_category_records`] does,
/// and validates through [`decode_at`].
pub fn find_type_records(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Vec<PartitionTypeRecord> {
    find_type_records_with_marker(
        stream,
        buf,
        builtin_category,
        declared_ids,
        &crate::partition_element_records::BBOX_MARKER,
    )
}

/// [`find_type_records`] for the release whose element-record bbox
/// marker is `bbox_marker`.
pub fn find_type_records_with_marker(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
    bbox_marker: &[u8; 8],
) -> Vec<PartitionTypeRecord> {
    use crate::partition_element_records as per;

    let needle = (builtin_category as u64).to_le_bytes();
    let mut out = Vec::new();
    if buf.len() < RECORD_MIN_LEN {
        return out;
    }
    let mut chain: Option<Vec<per::PartitionRecordSpan>> = None;
    let mut cursor = 0usize;
    while cursor + needle.len() <= buf.len() {
        let Some(found) = memchr::memmem::find(&buf[cursor..], &needle) else {
            break;
        };
        let hit = cursor + found;
        cursor = hit + 1;
        if hit < per::CATEGORY_OFFSET {
            continue;
        }
        let offset = hit - per::CATEGORY_OFFSET;
        if let Some(record) = decode_at_with_marker(stream, buf, offset, declared_ids, bbox_marker)
        {
            out.push(record);
            continue;
        }
        // RE-35: a second-prologue type record takes the id of the
        // partition record it sits in.
        let chain = chain.get_or_insert_with(|| per::partition_record_chain(buf, bbox_marker));
        let Some(span) = per::enclosing_record(chain, offset) else {
            continue;
        };
        let Ok(element_id) = u32::try_from(span.element_id) else {
            continue;
        };
        if !declared_ids.contains(&element_id) {
            continue;
        }
        if let Some(record) =
            decode_second_prologue_at(stream, buf, offset, element_id, bbox_marker)
        {
            out.push(record);
        }
    }
    out
}

/// Scan every `Partitions/*` stream for type records in
/// `builtin_category`.
///
/// Returns an empty vector for unsupported releases (fail closed).
pub fn scan_type_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Result<Vec<PartitionTypeRecord>> {
    if !supports_revit_version(revit_version) || declared_ids.is_empty() {
        return Ok(Vec::new());
    }
    let Some(bbox_marker) = crate::partition_element_records::bbox_marker(revit_version) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        out.extend(find_type_records_with_marker(
            &stream,
            inflated.bytes(),
            builtin_category,
            declared_ids,
            &bbox_marker,
        ));
    }
    Ok(out)
}

/// The ElementId set of the type-definition records in `records`.
pub fn type_definition_ids(records: &[PartitionTypeRecord]) -> BTreeSet<u32> {
    records
        .iter()
        .filter(|record| record.is_type_definition())
        .map(|record| record.element_id)
        .collect()
}

/// The type an instance is placed from: the single slot of its
/// `+0x88` reference list that is a type-definition record (#88,
/// RE-28).
///
/// Fail-closed: `None` unless the reference list names **exactly
/// one** distinct id from `type_ids`. Two different type ids in one
/// list is an ambiguity the bytes do not resolve, so the caller gets
/// nothing rather than a guess.
///
/// Measured on `2024_Core_Interior.rvt`: all 360 exported wall
/// instance records name exactly one, and it equals the
/// `IfcWallType.Tag` Revit's own export assigns — 360 of 360, no
/// wrong answer, no miss. This supersedes the positional
/// `references[1]` read RE-26 §3 scored at 356 of 360.
pub fn unique_type_reference(references: &[u64], type_ids: &BTreeSet<u32>) -> Option<u32> {
    let mut found: Option<u32> = None;
    for slot in references {
        if *slot > u64::from(u32::MAX) {
            continue;
        }
        let candidate = *slot as u32;
        if !type_ids.contains(&candidate) {
            continue;
        }
        match found {
            None => found = Some(candidate),
            Some(existing) if existing == candidate => {}
            Some(_) => return None,
        }
    }
    found
}
