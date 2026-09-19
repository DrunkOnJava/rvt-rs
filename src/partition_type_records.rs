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

/// Releases where this record shape is corpus-proven.
pub const PARTITION_TYPE_RECORD_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Autodesk `BuiltInCategory.OST_Materials`.
pub const OST_MATERIALS: i64 = -2_000_700;

/// Placement-kind word carried by a wall *type* definition record.
pub const PLACEMENT_KIND_TYPE_DEFINITION: u32 = 0xffff_8080;

/// Offset of the `0x0000059f` word every Revit 2024 prologue carries.
pub const PROLOGUE_MAGIC_OFFSET: usize = 0x0c;

/// The word itself.
pub const PROLOGUE_MAGIC: u32 = 0x0000_059f;

/// Offset of the bbox-less record marker.
pub const RECORD_MARKER_OFFSET: usize = 0x50;

/// The bbox-less record marker, shared with `OST_Levels` records.
///
/// An element record carries `46 01` *before* these six bytes and
/// then a bounding box, so testing this marker at
/// [`RECORD_MARKER_OFFSET`] rejects the element shape outright.
pub const RECORD_MARKER: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xab, 0x05];

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
    use crate::partition_element_records as per;

    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    let raw_id = read_u64(buf, offset)?;
    if raw_id == 0 || raw_id > u64::from(u32::MAX) {
        return None;
    }
    let element_id = raw_id as u32;
    if !declared_ids.contains(&element_id) {
        return None;
    }
    if read_u32(buf, offset + PROLOGUE_MAGIC_OFFSET)? != PROLOGUE_MAGIC {
        return None;
    }
    if read_u16(buf, offset + 0x10)? != 0 {
        return None;
    }
    let builtin_category = read_u64(buf, offset + per::CATEGORY_OFFSET)? as i64;
    if !(per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&builtin_category) {
        return None;
    }
    if buf
        .get(offset + RECORD_MARKER_OFFSET..offset + RECORD_MARKER_OFFSET + RECORD_MARKER.len())?
        != RECORD_MARKER
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
    if !slots.contains(&raw_id) {
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
    use crate::partition_element_records as per;

    let needle = (builtin_category as u64).to_le_bytes();
    let mut out = Vec::new();
    if buf.len() < RECORD_MIN_LEN {
        return out;
    }
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
        if let Some(record) = decode_at(stream, buf, hit - per::CATEGORY_OFFSET, declared_ids) {
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
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        out.extend(find_type_records(
            &stream,
            inflated.bytes(),
            builtin_category,
            declared_ids,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_element_records as per;

    /// Build a synthetic type record: prologue, marker, slot list.
    fn synth_type_record(
        element_id: u32,
        category: i64,
        placement_kind: u32,
        slots: &[u64],
    ) -> Vec<u8> {
        let mut buf = vec![0xffu8; SLOT_LIST_OFFSET + slots.len() * 8];
        buf[0..8].copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[8..12].copy_from_slice(&0x7fu32.to_le_bytes());
        buf[PROLOGUE_MAGIC_OFFSET..PROLOGUE_MAGIC_OFFSET + 4]
            .copy_from_slice(&PROLOGUE_MAGIC.to_le_bytes());
        buf[0x10..0x12].copy_from_slice(&0u16.to_le_bytes());
        buf[per::CATEGORY_OFFSET..per::CATEGORY_OFFSET + 8]
            .copy_from_slice(&(category as u64).to_le_bytes());
        buf[per::CONTAINER_OFFSET..per::CONTAINER_OFFSET + 8]
            .copy_from_slice(&per::CONTAINER_NONE.to_le_bytes());
        buf[per::PLACEMENT_KIND_OFFSET..per::PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&placement_kind.to_le_bytes());
        buf[RECORD_MARKER_OFFSET..RECORD_MARKER_OFFSET + RECORD_MARKER.len()]
            .copy_from_slice(&RECORD_MARKER);
        buf[SLOT_COUNT_OFFSET..SLOT_COUNT_OFFSET + 4]
            .copy_from_slice(&(slots.len() as u32).to_le_bytes());
        for (index, slot) in slots.iter().enumerate() {
            let at = SLOT_LIST_OFFSET + index * 8;
            buf[at..at + 8].copy_from_slice(&slot.to_le_bytes());
        }
        buf
    }

    /// The element-record shape: `46 01` then the same six marker
    /// bytes, then a bounding box.
    fn synth_element_record(element_id: u32, category: i64) -> Vec<u8> {
        let mut buf = vec![0xffu8; per::RECORD_MIN_LEN];
        buf[0..8].copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[PROLOGUE_MAGIC_OFFSET..PROLOGUE_MAGIC_OFFSET + 4]
            .copy_from_slice(&PROLOGUE_MAGIC.to_le_bytes());
        buf[0x10..0x12].copy_from_slice(&0u16.to_le_bytes());
        buf[per::CATEGORY_OFFSET..per::CATEGORY_OFFSET + 8]
            .copy_from_slice(&(category as u64).to_le_bytes());
        buf[per::BBOX_MARKER_OFFSET..per::BBOX_MARKER_OFFSET + 8]
            .copy_from_slice(&per::BBOX_MARKER);
        for index in 0..6 {
            let at = per::BBOX_OFFSET + index * 8;
            buf[at..at + 8].copy_from_slice(&0.0f64.to_le_bytes());
        }
        buf
    }

    fn declared(ids: &[u32]) -> BTreeSet<u32> {
        ids.iter().copied().collect()
    }

    #[test]
    fn decodes_a_wall_type_record() {
        let buf = synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17328, 17329],
        );
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[17328]))
            .expect("the synthetic type record decodes");
        assert_eq!(record.element_id, 17328);
        assert_eq!(record.builtin_category, per::OST_WALLS);
        assert_eq!(record.slots, vec![17328, 17329]);
        assert!(record.is_type_definition());
        assert!(!record.is_container_member());
    }

    #[test]
    fn rejects_an_undeclared_id() {
        let buf = synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17328],
        );
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[999])).is_none());
    }

    #[test]
    fn rejects_the_element_record_shape() {
        // An element record carries `46 01` where a type record's
        // marker starts, so the marker test must reject it outright.
        let buf = synth_element_record(20796, per::OST_WALLS);
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[20796])).is_none());
    }

    #[test]
    fn rejects_a_slot_list_without_the_records_own_id() {
        let buf = synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17329, 17330],
        );
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[17328])).is_none());
    }

    #[test]
    fn rejects_a_slot_list_that_runs_past_the_buffer() {
        let mut buf = synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17328],
        );
        buf[SLOT_COUNT_OFFSET..SLOT_COUNT_OFFSET + 4].copy_from_slice(&64u32.to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[17328])).is_none());
    }

    #[test]
    fn rejects_an_absurd_slot_count() {
        let mut buf = synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17328],
        );
        buf[SLOT_COUNT_OFFSET..SLOT_COUNT_OFFSET + 4]
            .copy_from_slice(&(SLOT_LIST_MAX_ENTRIES as u32 + 1).to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[17328])).is_none());
    }

    #[test]
    fn finds_type_records_by_category_and_skips_element_records() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0u8; 32]);
        buf.extend_from_slice(&synth_element_record(20796, per::OST_WALLS));
        buf.extend_from_slice(&[0u8; 16]);
        buf.extend_from_slice(&synth_type_record(
            17328,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[17328, 17329],
        ));
        buf.extend_from_slice(&[0u8; 64]);
        let found = find_type_records(
            "Partitions/46",
            &buf,
            per::OST_WALLS,
            &declared(&[17328, 20796]),
        );
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].element_id, 17328);
    }

    #[test]
    fn type_definition_ids_filters_on_the_placement_kind() {
        let buf = synth_type_record(
            1710,
            per::OST_WALLS,
            PLACEMENT_KIND_TYPE_DEFINITION,
            &[1710],
        );
        let kept = decode_at("Partitions/46", &buf, 0, &declared(&[1710])).expect("decodes");
        let other = synth_type_record(
            3652,
            OST_MATERIALS,
            per::PLACEMENT_KIND_SYMBOL,
            &[3651, 3652],
        );
        let dropped = decode_at("Partitions/46", &other, 0, &declared(&[3652])).expect("decodes");
        let ids = type_definition_ids(&[kept, dropped]);
        assert_eq!(ids, declared(&[1710]));
    }

    #[test]
    fn unique_type_reference_picks_the_single_type_slot() {
        // The `18" Basement` shape: `1851` is not a type record and
        // `3897` is, so the set test picks `3897` where RE-26's
        // positional read picked `1851`.
        let refs = vec![3, 1851, 3897, 20268, 20273, 22771, 22773, 22777];
        let types = declared(&[1710, 1711, 3897, 11356, 17328, 17337, 17339, 17341]);
        assert_eq!(unique_type_reference(&refs, &types), Some(3897));
    }

    #[test]
    fn unique_type_reference_declines_two_different_types() {
        let refs = vec![3, 17328, 17341, 20796];
        let types = declared(&[17328, 17341]);
        assert_eq!(unique_type_reference(&refs, &types), None);
    }

    #[test]
    fn unique_type_reference_tolerates_a_repeated_slot() {
        let refs = vec![3, 17328, 20796, 17328];
        let types = declared(&[17328]);
        assert_eq!(unique_type_reference(&refs, &types), Some(17328));
    }

    #[test]
    fn unique_type_reference_is_none_without_a_type_slot() {
        let refs = vec![3, 20268, 20796];
        let types = declared(&[17328]);
        assert_eq!(unique_type_reference(&refs, &types), None);
    }

    #[test]
    fn unique_type_reference_ignores_out_of_range_slots() {
        let refs = vec![u64::MAX, 17328];
        let types = declared(&[17328]);
        assert_eq!(unique_type_reference(&refs, &types), Some(17328));
    }

    #[test]
    fn unsupported_release_scans_nothing() {
        assert!(supports_revit_version(2024));
        assert!(!supports_revit_version(2023));
    }
}
