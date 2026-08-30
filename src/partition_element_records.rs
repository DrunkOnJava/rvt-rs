//! Partition element-record headers: ElementId + BuiltInCategory + bbox.
//!
//! Revit 2024 project partitions frame each *element* record with a
//! fixed 88-byte prologue whose first field is the record's own
//! `ElementId` and whose sixth field is the element's Revit
//! `BuiltInCategory` id (a negative, publicly documented constant).
//! The prologue is followed by the element's model-space bounding
//! box as six `f64` feet values.
//!
//! Layout observed on `2024_Core_Interior.rvt`
//! (sha256 `c805df44…`, Revit 2024, eight `Partitions/*` streams,
//! ~190 MiB inflated):
//!
//! ```text
//! +0x00  u64  ElementId of this record (declared in Global/ElemTable)
//! +0x08  u32  record flags (0x00e1 / 0x0111 / 0x0131 / 0x0141 / … )
//! +0x0c  u32  0x0000059f on every observed record of this shape
//! +0x10  u16  0
//! +0x12  i64  BuiltInCategory id, negative (OST_Columns = -2000100)
//! +0x1a  24B  0xff sentinel padding (three u64 slots, never set here)
//! +0x32  u64  container ElementId, 0xffff_ffff_ffff_ffff = none
//! +0x3a  u64  0xff sentinel (never set on observed records)
//! +0x42  u32  placement kind: 0xffffef7f placed / 0xffff8000 symbol
//! +0x46  u32  unattributed (0x0928 / 0x0929 / 0x1929 / 0x09a8 / …)
//! +0x4a  u16  unattributed (0x0766 / 0x0e55 / 0x0788 / 0x04d6)
//! +0x4c  u32  0xffffffff on every observed record of this shape
//! +0x50  8B   bbox marker 46 01 ff ff ff ff ab 05
//! +0x58  48B  bounding box: min x/y/z, max x/y/z, feet, f64 LE
//! ```
//!
//! The two fields at `+0x32` and `+0x42` are what separates the
//! records Revit's own exporter emits as building elements from the
//! rest — see [`PartitionElementRecord::is_exported_instance`] and
//! `reports/element-framing/RE-21-partition-element-record-instance-rule.md`.
//!
//! 24880 records of this shape were found on that file; 23470 of
//! them (94.3 %) carry the bbox marker at exactly `+0x50`, which is
//! why the marker offset is treated as fixed rather than searched.
//! Every `ElementId` recovered this way is declared in
//! `Global/ElemTable`, and the scan requires that join — an offset
//! whose leading `u64` is not a declared id is rejected, so a random
//! byte match cannot become an element.
//!
//! # Honesty
//!
//! - The `BuiltInCategory` ids are Autodesk's published API
//!   constants; nothing here is derived from Revit binaries.
//! - Only the fields above are claimed. The sentinel runs, the flags
//!   word, `0x059f`, `+0x46` and `+0x4a` are recorded, not interpreted.
//! - `+0x32` is named `container` because every non-sentinel value
//!   observed is an ElementId declared in `Global/ElemTable`, is
//!   lower than every member id that names it, owns a contiguous
//!   ElementId block spanning several categories at once, and has no
//!   element record of its own. What kind of container Revit calls it
//!   is not claimed.
//! - `+0x42` is named `placement_kind` because it takes exactly two
//!   values on the corpus and they partition the records into placed
//!   instances and type/symbol envelopes. The numbers are recorded,
//!   not decoded.
//! - Category membership alone is **not** an instance claim: a
//!   family symbol carries the same category as its instances.

use crate::{Result, RevitFile, compression};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Releases where this record shape is corpus-proven.
pub const PARTITION_ELEMENT_RECORD_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Autodesk `BuiltInCategory.OST_Columns` — architectural columns.
pub const OST_COLUMNS: i64 = -2_000_100;
/// Autodesk `BuiltInCategory.OST_Walls`.
pub const OST_WALLS: i64 = -2_000_011;
/// Autodesk `BuiltInCategory.OST_Doors`.
pub const OST_DOORS: i64 = -2_000_023;
/// Autodesk `BuiltInCategory.OST_Windows`.
pub const OST_WINDOWS: i64 = -2_000_014;
/// Autodesk `BuiltInCategory.OST_Floors` — floor slabs.
pub const OST_FLOORS: i64 = -2_000_032;
/// Autodesk `BuiltInCategory.OST_BuildingPad` — site/building pads.
///
/// Revit's own exporter emits a building pad as `IfcSlab`, which is
/// why it joins `OST_FLOORS` in the slab recovery: on
/// `2024_Core_Interior.rvt` the single exported slab with no
/// `OST_Floors` record (`Pad:Site Pad`, ElementId 21975) carries this
/// category instead (#212, RE-22).
pub const OST_BUILDING_PAD: i64 = -2_001_263;

/// Lower bound of the Revit `BuiltInCategory` id band.
pub const BUILTIN_CATEGORY_MIN: i64 = -2_100_000;
/// Upper bound of the Revit `BuiltInCategory` id band.
pub const BUILTIN_CATEGORY_MAX: i64 = -1_990_000;

/// Offset of the `BuiltInCategory` id from the record start.
pub const CATEGORY_OFFSET: usize = 0x12;
/// Offset of the container ElementId reference from the record start.
pub const CONTAINER_OFFSET: usize = 0x32;
/// Sentinel value of the container reference meaning "no container".
pub const CONTAINER_NONE: u64 = u64::MAX;
/// Offset of the placement-kind word from the record start.
pub const PLACEMENT_KIND_OFFSET: usize = 0x42;
/// Placement-kind value carried by a placed element instance.
pub const PLACEMENT_KIND_INSTANCE: u32 = 0xffff_ef7f;
/// Placement-kind value carried by a family/type symbol envelope.
pub const PLACEMENT_KIND_SYMBOL: u32 = 0xffff_8000;
/// Offset of the bounding-box marker from the record start.
pub const BBOX_MARKER_OFFSET: usize = 0x50;
/// Offset of the six bounding-box doubles from the record start.
pub const BBOX_OFFSET: usize = 0x58;
/// Minimum bytes a complete record header occupies.
pub const RECORD_MIN_LEN: usize = BBOX_OFFSET + 48;

/// Fixed marker that precedes the bounding box.
pub const BBOX_MARKER: [u8; 8] = [0x46, 0x01, 0xff, 0xff, 0xff, 0xff, 0xab, 0x05];

/// A decoded partition element-record header.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionElementRecord {
    /// Stream the record was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the record in the concatenated inflated stream.
    pub offset: usize,
    /// The record's own Revit ElementId (cross-checked against ElemTable).
    pub element_id: u32,
    /// Unattributed flags word at `+0x08`.
    pub flags: u32,
    /// Revit `BuiltInCategory` id (negative).
    pub builtin_category: i64,
    /// Raw container reference at `+0x32`; [`CONTAINER_NONE`] when unset.
    pub container: u64,
    /// Raw placement-kind word at `+0x42`.
    pub placement_kind: u32,
    /// Model bounding box in feet: `[min_x, min_y, min_z, max_x, max_y, max_z]`.
    pub bbox_feet: [f64; 6],
}

impl PartitionElementRecord {
    /// Plan-footprint origin, quantised to 1e-4 ft, for de-duplication.
    pub fn footprint_key(&self) -> (i64, i64, i64) {
        (
            quantise(self.bbox_feet[0]),
            quantise(self.bbox_feet[1]),
            quantise(self.bbox_feet[2]),
        )
    }

    /// Plan centre of the bounding box, in feet.
    pub fn plan_centre_feet(&self) -> (f64, f64) {
        (
            (self.bbox_feet[0] + self.bbox_feet[3]) * 0.5,
            (self.bbox_feet[1] + self.bbox_feet[4]) * 0.5,
        )
    }

    /// Bounding-box extents `(dx, dy, dz)` in feet.
    pub fn extents_feet(&self) -> (f64, f64, f64) {
        (
            self.bbox_feet[3] - self.bbox_feet[0],
            self.bbox_feet[4] - self.bbox_feet[1],
            self.bbox_feet[5] - self.bbox_feet[2],
        )
    }

    /// True when the bbox is expressed in family-local coordinates
    /// (centred on the plan origin) rather than project coordinates.
    ///
    /// Retained as a diagnostic. It is a *proxy* for
    /// [`Self::is_type_symbol`] and a strictly weaker one: on
    /// `2024_Core_Interior.rvt` it agrees on all 17 `OST_Columns`
    /// symbols but misses the 15 `OST_Doors`, 2 `OST_Windows` and 1
    /// `OST_Walls` symbol whose envelope is centred on only one axis.
    pub fn is_family_local(&self) -> bool {
        is_family_local_bbox(&self.bbox_feet)
    }

    /// The container ElementId this record belongs to, if any.
    ///
    /// `None` when `+0x32` carries [`CONTAINER_NONE`], and also when
    /// it carries a value outside the `u32` ElementId range — an
    /// unrecognised encoding is not turned into an id.
    pub fn container_element_id(&self) -> Option<u32> {
        if self.container == CONTAINER_NONE || self.container > u64::from(u32::MAX) {
            return None;
        }
        Some(self.container as u32)
    }

    /// True when `+0x32` is set, i.e. the record is a member of a
    /// container element rather than a standalone element.
    pub fn is_container_member(&self) -> bool {
        self.container != CONTAINER_NONE
    }

    /// True when `+0x42` marks a placed element instance.
    pub fn is_placed_instance(&self) -> bool {
        self.placement_kind == PLACEMENT_KIND_INSTANCE
    }

    /// True when `+0x42` marks a family/type symbol envelope.
    pub fn is_type_symbol(&self) -> bool {
        self.placement_kind == PLACEMENT_KIND_SYMBOL
    }

    /// The instance rule (#211): a record is a standalone placed
    /// instance — the thing Revit's own exporter emits as a building
    /// element — when it carries no container reference **and** its
    /// placement kind is [`PLACEMENT_KIND_INSTANCE`].
    ///
    /// Measured on `2024_Core_Interior.rvt` against the ElementId set
    /// Revit's full IFC export tags: `OST_Walls` 360/360, `OST_Doors`
    /// 132/132, `OST_Windows` 6/6, `OST_Columns` 256/256 — exact id
    /// sets, no false positives, no misses. It is a *test*, not a
    /// heuristic: it replaces the family-local bbox proxy and the
    /// highest-id-per-footprint collapse that #204 used for columns.
    ///
    /// RE-22 extended the same measurement to `OST_Floors` and found
    /// the rule exact there too, once the reference side is read
    /// correctly: the 99 selected ElementIds are *all* exported — 79
    /// as `IfcSlab` and 20 as `IfcShadingDevice` — so there are no
    /// false positives, and the 80th exported slab (`Pad:Site Pad`,
    /// 21975) simply carries [`OST_BUILDING_PAD`] rather than
    /// `OST_Floors`. `OST_Floors` + `OST_BuildingPad` under this rule
    /// reproduce the export's 80 `IFCSLAB` and 20 `IFCSHADINGDEVICE`
    /// id sets exactly (#212).
    pub fn is_exported_instance(&self) -> bool {
        !self.is_container_member() && self.is_placed_instance()
    }
}

fn quantise(v: f64) -> i64 {
    (v * 10_000.0).round() as i64
}

/// A bbox centred on the plan origin is a family/type definition
/// envelope, not a placed instance: Revit stores symbol geometry in
/// family coordinates, so `min_x == -max_x` and `min_y == -max_y`.
pub fn is_family_local_bbox(bbox: &[f64; 6]) -> bool {
    (bbox[0] + bbox[3]).abs() < 1e-6 && (bbox[1] + bbox[4]).abs() < 1e-6
}

/// Whether this release's partition element-record shape is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    PARTITION_ELEMENT_RECORD_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    buf.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    read_u64(buf, off).map(f64::from_bits)
}

/// Decode one record at `offset`, fail-closed.
///
/// `declared_ids` is the `Global/ElemTable` id set; a record whose
/// leading `u64` is not declared there is rejected outright.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
) -> Option<PartitionElementRecord> {
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
    if u16::from_le_bytes([buf[offset + 0x10], buf[offset + 0x11]]) != 0 {
        return None;
    }
    let builtin_category = read_u64(buf, offset + CATEGORY_OFFSET)? as i64;
    if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&builtin_category) {
        return None;
    }
    if buf[offset + BBOX_MARKER_OFFSET..offset + BBOX_MARKER_OFFSET + 8] != BBOX_MARKER {
        return None;
    }
    let mut bbox_feet = [0.0f64; 6];
    for (index, slot) in bbox_feet.iter_mut().enumerate() {
        let value = read_f64(buf, offset + BBOX_OFFSET + index * 8)?;
        if !value.is_finite() {
            return None;
        }
        *slot = value;
    }
    for axis in 0..3 {
        if bbox_feet[axis + 3] < bbox_feet[axis] {
            return None;
        }
    }
    let flags = read_u64(buf, offset + 0x08).map(|v| (v & 0xffff_ffff) as u32)?;
    let container = read_u64(buf, offset + CONTAINER_OFFSET)?;
    let placement_kind =
        read_u64(buf, offset + PLACEMENT_KIND_OFFSET).map(|v| (v & 0xffff_ffff) as u32)?;
    Some(PartitionElementRecord {
        stream: stream.to_string(),
        offset,
        element_id,
        flags,
        builtin_category,
        container,
        placement_kind,
        bbox_feet,
    })
}

/// Find every element record in `buf` carrying `builtin_category`.
///
/// The scan anchors on the 8-byte little-endian encoding of the
/// category id and back-references [`CATEGORY_OFFSET`] to the record
/// start, then validates through [`decode_at`].
pub fn find_category_records(
    stream: &str,
    buf: &[u8],
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Vec<PartitionElementRecord> {
    let needle = (builtin_category as u64).to_le_bytes();
    let mut out = Vec::new();
    if buf.len() < RECORD_MIN_LEN {
        return out;
    }
    let mut cursor = 0usize;
    while cursor + 8 <= buf.len() {
        let Some(found) = find_subslice(&buf[cursor..], &needle) else {
            break;
        };
        let hit = cursor + found;
        if hit >= CATEGORY_OFFSET {
            if let Some(record) = decode_at(stream, buf, hit - CATEGORY_OFFSET, declared_ids) {
                out.push(record);
            }
        }
        cursor = hit + 1;
    }
    out
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let first = needle[0];
    let last = haystack.len() - needle.len();
    let mut index = 0usize;
    while index <= last {
        let delta = haystack[index..=last].iter().position(|b| *b == first)?;
        let start = index + delta;
        if &haystack[start..start + needle.len()] == needle {
            return Some(start);
        }
        index = start + 1;
    }
    None
}

/// Scan every `Partitions/*` stream for records in `builtin_category`.
///
/// Returns an empty vector for unsupported releases (fail closed).
pub fn scan_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    builtin_category: i64,
    declared_ids: &BTreeSet<u32>,
) -> Result<Vec<PartitionElementRecord>> {
    if !supports_revit_version(revit_version) || declared_ids.is_empty() {
        return Ok(Vec::new());
    }
    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    let mut out = Vec::new();
    for stream in streams {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(&stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        out.extend(find_category_records(
            &stream,
            &concat,
            builtin_category,
            declared_ids,
        ));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synth_record(element_id: u32, category: i64, bbox: [f64; 6]) -> Vec<u8> {
        let mut buf = vec![0xffu8; RECORD_MIN_LEN];
        buf[0..8].copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[8..12].copy_from_slice(&0x0141u32.to_le_bytes());
        buf[12..16].copy_from_slice(&0x059fu32.to_le_bytes());
        buf[16..18].copy_from_slice(&0u16.to_le_bytes());
        buf[CATEGORY_OFFSET..CATEGORY_OFFSET + 8].copy_from_slice(&(category as u64).to_le_bytes());
        buf[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_INSTANCE.to_le_bytes());
        buf[BBOX_MARKER_OFFSET..BBOX_MARKER_OFFSET + 8].copy_from_slice(&BBOX_MARKER);
        for (index, value) in bbox.iter().enumerate() {
            let at = BBOX_OFFSET + index * 8;
            buf[at..at + 8].copy_from_slice(&value.to_le_bytes());
        }
        buf
    }

    fn declared(ids: &[u32]) -> BTreeSet<u32> {
        ids.iter().copied().collect()
    }

    #[test]
    fn decodes_a_well_formed_column_record() {
        let bbox = [135.5, 79.0, 0.0, 137.5, 81.0, 30.333];
        let buf = synth_record(22805, OST_COLUMNS, bbox);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.element_id, 22805);
        assert_eq!(record.builtin_category, OST_COLUMNS);
        assert_eq!(record.flags, 0x0141);
        let (dx, dy, _) = record.extents_feet();
        assert!((dx - 2.0).abs() < 1e-9);
        assert!((dy - 2.0).abs() < 1e-9);
        assert!(!record.is_family_local());
    }

    #[test]
    fn rejects_ids_absent_from_elem_table() {
        let buf = synth_record(22805, OST_COLUMNS, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[9999])).is_none());
    }

    #[test]
    fn rejects_missing_bbox_marker() {
        let mut buf = synth_record(22805, OST_COLUMNS, [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        buf[BBOX_MARKER_OFFSET] = 0x00;
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[22805])).is_none());
    }

    #[test]
    fn rejects_inverted_bounding_box() {
        let buf = synth_record(22805, OST_COLUMNS, [10.0, 0.0, 0.0, 1.0, 1.0, 1.0]);
        assert!(decode_at("Partitions/46", &buf, 0, &declared(&[22805])).is_none());
    }

    #[test]
    fn family_local_bbox_is_recognised() {
        let buf = synth_record(5755, OST_COLUMNS, [-1.0, -1.0, 0.0, 1.0, 1.0, 9.0]);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[5755])).expect("decodes");
        assert!(record.is_family_local());
    }

    #[test]
    fn placed_standalone_record_is_an_exported_instance() {
        let buf = synth_record(22805, OST_COLUMNS, [135.5, 79.0, 0.0, 137.5, 81.0, 30.3]);
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert_eq!(record.container, CONTAINER_NONE);
        assert_eq!(record.placement_kind, PLACEMENT_KIND_INSTANCE);
        assert!(record.container_element_id().is_none());
        assert!(!record.is_container_member());
        assert!(record.is_placed_instance());
        assert!(!record.is_type_symbol());
        assert!(record.is_exported_instance());
    }

    #[test]
    fn container_member_is_not_an_exported_instance() {
        let mut buf = synth_record(16347, OST_COLUMNS, [23.0, 109.0, 76.0, 25.0, 111.0, 91.0]);
        buf[CONTAINER_OFFSET..CONTAINER_OFFSET + 8].copy_from_slice(&16_229u64.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[16347])).expect("decodes");
        assert_eq!(record.container_element_id(), Some(16_229));
        assert!(record.is_container_member());
        assert!(record.is_placed_instance());
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn type_symbol_is_not_an_exported_instance() {
        // A door symbol envelope: centred on X only, so the
        // family-local bbox proxy misses it and `+0x42` does not.
        let mut buf = synth_record(17331, OST_DOORS, [-1.749, -0.332, 0.0, 1.749, 3.251, 8.249]);
        buf[PLACEMENT_KIND_OFFSET..PLACEMENT_KIND_OFFSET + 4]
            .copy_from_slice(&PLACEMENT_KIND_SYMBOL.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[17331])).expect("decodes");
        assert!(!record.is_family_local(), "proxy misses this symbol");
        assert!(record.is_type_symbol());
        assert!(!record.is_placed_instance());
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn out_of_range_container_reference_is_not_an_element_id() {
        let mut buf = synth_record(22805, OST_COLUMNS, [1.0, 2.0, 0.0, 3.0, 4.0, 5.0]);
        buf[CONTAINER_OFFSET..CONTAINER_OFFSET + 8]
            .copy_from_slice(&0x0001_0000_0000u64.to_le_bytes());
        let record = decode_at("Partitions/46", &buf, 0, &declared(&[22805])).expect("decodes");
        assert!(record.is_container_member());
        assert_eq!(record.container_element_id(), None);
        assert!(!record.is_exported_instance());
    }

    #[test]
    fn scan_finds_records_at_any_offset() {
        let bbox = [1.0, 2.0, 3.0, 3.0, 4.0, 13.0];
        let mut buf = vec![0u8; 37];
        buf.extend(synth_record(4242, OST_COLUMNS, bbox));
        buf.extend(vec![0u8; 11]);
        let found = find_category_records("Partitions/46", &buf, OST_COLUMNS, &declared(&[4242]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].element_id, 4242);
        assert_eq!(found[0].offset, 37);
    }

    #[test]
    fn unsupported_release_yields_nothing() {
        assert!(!supports_revit_version(2023));
        assert!(supports_revit_version(2024));
    }
}
