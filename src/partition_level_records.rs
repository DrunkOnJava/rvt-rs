//! Revit `Level` elements recovered from the partition streams (#218, RE-24).
//!
//! A Revit `Level` is an element like any other — it carries a
//! `BuiltInCategory` of [`OST_LEVELS`] and an `ElementId` declared in
//! `Global/ElemTable` — but its record does **not** carry the
//! bounding box [`crate::partition_element_records`] decodes, because
//! a level is a datum plane and not a solid. Its record therefore
//! ends where a column record's bbox marker would start, and its
//! *name* and *elevation* live in a separate parameter block keyed by
//! the level's own `ElementId`, in the same
//! owner-`ElementId`-at-a-fixed-negative-offset shape RE-22 found for
//! the per-instance `IFC Export As` overrides.
//!
//! # The record
//!
//! Observed on `2024_Core_Interior.rvt` (sha256 `c805df44…`, Revit
//! 2024). The prologue is byte-identical to the element-record
//! prologue up to `+0x4c`; from there the two shapes diverge:
//!
//! ```text
//! +0x00  u64  ElementId of this record (declared in Global/ElemTable)
//! +0x08  u32  record flags (0x87 on every standalone Level)
//! +0x0c  u32  0x0000059f, as on every element record
//! +0x10  u16  0
//! +0x12  i64  BuiltInCategory = OST_Levels (-2000240)
//! +0x1a  24B  0xff sentinel padding
//! +0x32  u64  container ElementId, 0xffff_ffff_ffff_ffff = none
//! +0x3a  u64  0xff sentinel
//! +0x42  u32  placement kind: 0xffffef7f placed / 0xffff8000 symbol
//! +0x46  u32  unattributed (0x0a on every observed Level)
//! +0x4a  u16  unattributed (0x0976 on every observed Level)
//! +0x4c  u32  0
//! +0x50  6B   record marker ff ff ff ff ab 05
//! +0x56  u16  record kind (2 on every standalone Level)
//! ```
//!
//! 75 records of this shape carry `OST_Levels` on that file. Exactly
//! fifteen of them are *standalone placed instances* under the same
//! #211 rule the element records use — no container reference and a
//! placed placement kind — and fifteen is the number of
//! `IfcBuildingStorey` Revit's own export of the same file writes.
//! The other sixty are members of nine containers (`16229`, `21920`,
//! `21984`, `23117`, `26863`, `26908`, `33696`, `81029`, `87754`,
//! `108205`) plus one type/symbol envelope (`1673`).
//!
//! # The name / elevation block
//!
//! ```text
//! V-0x47  u64  owning ElementId
//! V-0x3f  56B  0xff sentinel run
//! V-0x07  3B   0x00
//! V-0x04  u32  name length in UTF-16 code units
//! V       2*n  the name, UTF-16LE
//! …            variable-length parameter run
//! M       8B   elevation marker 05 00 00 00 48 02 00 00
//! M+55    f64  elevation, feet
//! M+208   f64  the same elevation again
//! ```
//!
//! The marker is searched forward from the end of the name, bounded
//! by [`ELEVATION_MARKER_SEARCH_BYTES`], because the run between the
//! two is variable-length: on the recorded file it is 347 bytes for
//! fourteen of the fifteen levels and 363 for `Basement 2`. The two
//! elevation copies must agree; a block where they do not is
//! discarded rather than resolved.
//!
//! # Measured
//!
//! On `2024_Core_Interior.rvt` this recovers exactly fifteen
//! `(ElementId, name, elevation)` triples, one per standalone Level
//! record, with no level owning two blocks and no block owned by a
//! non-Level:
//!
//! | ElementId | name | elevation (ft) |
//! |---:|---|---:|
//! | 20273 | `Basement 2` | −40 |
//! | 20272 | `Basement 1` | −20 |
//! | 20268 | `Level 1` | 0 |
//! | 20275 | `Mez 1-2` | 15 |
//! | 20274 | `Level 3 - Wall Layouts 1` | 31 |
//! | 20276 | `Level 4 - Wall Layouts 2` | 46 |
//! | 20277 | `Level 4 - Wall Layouts 3` | 61 |
//! | 20308 | `Level 6` | 76 |
//! | 20307 | `Level 7` | 91 |
//! | 20306 | `Level 8` | 106 |
//! | 20305 | `Level 9` | 121 |
//! | 20304 | `Level 10` | 136 |
//! | 20303 | `Level 11` | 151 |
//! | 20302 | `Level 12` | 166 |
//! | 65128 | `Level 13` | 185.5 |
//!
//! # RE-51: the same block on every 2024 and 2025 project
//!
//! On Snowdon Towers (2024) and the RE1 models (2025) the same block opens
//! the Level's element data (RE-44), and RE-24's reading needed two
//! corrections to find it there:
//!
//! - The marker is six bytes, `05 00 00 00` and a `u16` per release
//!   ([`elevation_marker`]). Four plan extents follow it, and RE-24's last
//!   two marker bytes were the first extent's low bytes, zero only on
//!   Core Interior's round extents.
//! - The confirming copy is found by the 24 bytes before it, which are the
//!   first copy's too ([`ELEVATION_CONFIRM_LEAD_IN`]): 153 bytes on on Core
//!   Interior, 161 on Snowdon, 246 and 181 on RE1.
//!
//! Levels in second-prologue frames take their id from their partition
//! record ([`decode_record_at_with`]). A Level inside another element names
//! that element after its own id ([`owned_level_ids`]) and is not a
//! storey. With those, every storey equals one of Revit's own export, name,
//! elevation and GlobalId: Snowdon 18 of 18, each RE1 model 2 of 2. See
//! `reports/element-framing/RE-51-level-names.md`. On Snowdon's structural
//! model 8 of 19 Levels carry `01` where the three bytes before the name
//! length are zero on every other Level measured; it does not decide a
//! storey, and since RE-118 it is accepted.
//!
//! # RE-118: Building Story
//!
//! A Level keeps Revit's Building Story setting past its name
//! ([`BUILDING_STORY_ANCHOR`]), and only a building story is a storey of
//! Revit's export. On Snowdon's structural model 7 of 19 Levels (Top of
//! Footing and the six "TOS" Levels) are not, and its storeys are the other
//! 12, the building stories of its VIM export. See
//! `reports/element-framing/RE-118-building-story.md`.
//!
//! Every name and every elevation equals the `Name` and `Elevation`
//! of an `IfcBuildingStorey` in Revit's own export of the same file —
//! all fifteen, exactly, including the four elevations (−40, −20, 15,
//! 185.5 ft) that carry no column record and that #213's bbox
//! distribution therefore could not see. See
//! `reports/element-framing/RE-24-level-records.md`.
//!
//! # Honesty
//!
//! - `OST_Levels` is Autodesk's published `BuiltInCategory` constant.
//! - The record fields at `+0x46`, `+0x4a`, `+0x4c` and `+0x56` are
//!   recorded, not interpreted, exactly as
//!   [`crate::partition_element_records`] records its own.
//! - The three block offsets (`0x47`, `0x3f`, `0x04`) and the
//!   elevation offsets (55, 208) are **measured**, on one file, over
//!   fifteen accepted entries. The block framing itself is not
//!   decoded: what the 8-byte marker's two words (`5`, `584`) mean is
//!   not claimed, only that they precede the elevation at a fixed
//!   distance on every accepted entry.
//! - A level with no accepted block is *not* emitted with a guessed
//!   elevation; the recovery is all-or-nothing per file at the
//!   caller's gate (see [`recovered_levels_are_a_storey_set`]).

use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this framing is corpus-proven: 2024 on Core Interior
/// (RE-24) and Snowdon Towers, 2025 on RE1 Architecture (RE-51), and 2023
/// on two projects in its 32-bit form (RE-107), as 2014 to 2022 write it
/// (RE-178, RE-179).
pub const PARTITION_LEVEL_SUPPORTED_REVIT_VERSIONS: &[u32] = &[
    2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027,
];

/// The release whose Level records and name blocks carry 32-bit
/// ElementIds (RE-107).
pub const REVIT_2023: u32 = 2023;

/// Autodesk `BuiltInCategory.OST_Levels`.
pub const OST_LEVELS: i64 = -2_000_240;

/// Offset of the record marker from the record start.
pub const RECORD_MARKER_OFFSET: usize = 0x50;

/// Fixed marker that ends a Level record's prologue.
///
/// The element-record shape carries `46 01` before the same six
/// bytes and then a bounding box; a Level record carries neither.
pub const RECORD_MARKER: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xab, 0x05];

/// Minimum bytes a Level record header occupies.
pub const RECORD_MIN_LEN: usize = RECORD_MARKER_OFFSET + RECORD_MARKER.len() + 2;

/// Bytes from the name string back to the owning `ElementId`.
pub const OWNER_OFFSET_BEFORE_NAME: usize = 0x47;

/// Length of the `0xff` sentinel run that separates the owner slot
/// from the name's length prefix.
pub const OWNER_SENTINEL_RUN_LEN: usize = 56;

/// Bytes from the name string back to its `u32` length prefix.
pub const LENGTH_PREFIX_OFFSET_BEFORE_NAME: usize = 4;

/// Longest name the scan will accept, in UTF-16 code units.
pub const MAX_NAME_CHARS: usize = 128;

/// Marker whose fixed distance to the elevation double is measured on
/// `2024_Core_Interior.rvt`. Its last two bytes are data, not marker: they
/// are the low bytes of the first plan extent that follows, zero on that
/// file's round extents (RE-51); [`elevation_marker`] is the marker.
pub const ELEVATION_MARKER: [u8; 8] = [0x05, 0x00, 0x00, 0x00, 0x48, 0x02, 0x00, 0x00];

/// The elevation marker per release: `05 00 00 00` and a release's `u16`
/// (RE-51). Four plan extents (`f64` feet) follow it, then, at
/// [`ELEVATION_OFFSET_AFTER_MARKER`], the elevation.
pub fn elevation_marker(revit_version: u32) -> Option<[u8; 6]> {
    match revit_version {
        2024 => Some([0x05, 0x00, 0x00, 0x00, 0x48, 0x02]),
        2025 => Some([0x05, 0x00, 0x00, 0x00, 0x5d, 0x02]),
        2026 => Some([0x05, 0x00, 0x00, 0x00, 0x65, 0x02]),
        2027 => Some([0x05, 0x00, 0x00, 0x00, 0x7a, 0x02]),
        _ => None,
    }
}

/// The elevation marker from the file's own schema (RE-107): `05 00 00 00`
/// and the tag of its `Plane` class. The `u16` [`elevation_marker`] gives
/// per release is that tag: `0x0248` on 2024, `0x025d` on 2025, and
/// `0x0235` on 2023, where no release constant is recorded.
pub fn schema_elevation_marker(rf: &mut RevitFile) -> Option<[u8; 6]> {
    let classes = rf.schema_classes().ok()?;
    let plane = classes.classes.iter().find(|c| c.name == "Plane")?.tag;
    let mut marker = [0x05, 0x00, 0x00, 0x00, 0x00, 0x00];
    marker[4..].copy_from_slice(&plane.to_le_bytes());
    Some(marker)
}

/// The Level record marker per release: the last six bytes of the
/// element-record bbox marker ([`crate::partition_element_records::bbox_marker`]),
/// [`RECORD_MARKER`] on 2024.
pub fn record_marker(revit_version: u32) -> Option<[u8; 6]> {
    let marker = crate::partition_element_records::bbox_marker(revit_version)?;
    let mut out = [0u8; 6];
    out.copy_from_slice(&marker[2..]);
    Some(out)
}

/// Bytes before the elevation that recur, with it, in its confirming copy
/// (RE-51).
pub const ELEVATION_CONFIRM_LEAD_IN: usize = 24;

/// Bytes from the elevation marker to the elevation double.
pub const ELEVATION_OFFSET_AFTER_MARKER: usize = 55;

/// Bytes from the first elevation copy to the confirmation copy on
/// `2024_Core_Interior.rvt`. The copy is found, not assumed (RE-51): it is
/// 161 bytes on Snowdon Towers and 246 or 181 on RE1.
pub const ELEVATION_CONFIRM_STRIDE: usize = 153;

/// How far past the name the elevation marker may sit.
pub const ELEVATION_MARKER_SEARCH_BYTES: usize = 2048;

/// Largest elevation magnitude the scan will accept, in feet.
///
/// Wider than any building and far inside `f64`; the bound exists so
/// a marker that landed in unrelated bytes cannot become a storey.
pub const MAX_ELEVATION_FEET: f64 = 1.0e7;

/// A decoded `OST_Levels` record header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartitionLevelRecord {
    /// Stream the record was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the record in the concatenated inflated stream.
    pub offset: usize,
    /// The record's own Revit ElementId.
    pub element_id: u32,
    /// Unattributed flags word at `+0x08`.
    pub flags: u32,
    /// Raw container reference at `+0x32`; `u64::MAX` when unset.
    pub container: u64,
    /// Raw placement-kind word at `+0x42`.
    pub placement_kind: u32,
}

impl PartitionLevelRecord {
    /// True when `+0x32` is set, i.e. the record is a member of a
    /// container element rather than a standalone Level.
    pub fn is_container_member(&self) -> bool {
        self.container != crate::partition_element_records::CONTAINER_NONE
    }

    /// True when `+0x42` marks a placed element instance.
    pub fn is_placed_instance(&self) -> bool {
        self.placement_kind == crate::partition_element_records::PLACEMENT_KIND_INSTANCE
    }

    /// The #211 instance rule, applied to Levels: a standalone placed
    /// record is a Level element, a container member or a type/symbol
    /// envelope is not.
    pub fn is_level_element(&self) -> bool {
        !self.is_container_member() && self.is_placed_instance()
    }
}

/// One recovered Revit `Level`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PartitionLevel {
    /// The Level's Revit ElementId.
    pub element_id: u32,
    /// The Level's display name, verbatim.
    pub name: String,
    /// The Level's elevation in feet.
    pub elevation_feet: f64,
    /// Revit's Building Story setting (RE-118), where it is read: only a
    /// building story is a storey of Revit's IFC export.
    #[serde(default)]
    pub building_story: Option<bool>,
}

/// Whether this release's Level framing is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    PARTITION_LEVEL_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    buf.get(off..off + 2)
        .map(|s| u16::from_le_bytes(s.try_into().expect("2 bytes")))
}

fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    buf.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    read_u64(buf, off).map(f64::from_bits)
}

/// Decode one Level record at `offset`, fail-closed.
///
/// `declared_ids` is the `Global/ElemTable` id set; a record whose
/// leading `u64` is not declared there is rejected outright, exactly
/// as [`crate::partition_element_records::decode_at`] rejects one.
pub fn decode_record_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
) -> Option<PartitionLevelRecord> {
    decode_record_at_with(stream, buf, offset, declared_ids, &RECORD_MARKER, &[])
}

/// [`decode_record_at`] for a release's `marker` ([`record_marker`]). A
/// frame with no ElementId at `+0x00` (RE-30's second prologue) takes the
/// id of the `chain` record it sits in (RE-35), when that id is declared.
pub fn decode_record_at_with(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 6],
    chain: &[crate::partition_element_records::PartitionRecordSpan],
) -> Option<PartitionLevelRecord> {
    use crate::partition_element_records as per;
    if offset.checked_add(RECORD_MIN_LEN)? > buf.len() {
        return None;
    }
    let raw_id = read_u64(buf, offset)?;
    let element_id = if per::carries_no_element_id(raw_id) {
        u32::try_from(per::enclosing_record(chain, offset)?.element_id).ok()?
    } else {
        raw_id as u32
    };
    if element_id == 0 || !declared_ids.contains(&element_id) {
        return None;
    }
    if read_u16(buf, offset + 0x10)? != 0 {
        return None;
    }
    if read_u64(buf, offset + per::CATEGORY_OFFSET)? as i64 != OST_LEVELS {
        return None;
    }
    if buf[offset + RECORD_MARKER_OFFSET..offset + RECORD_MARKER_OFFSET + marker.len()] != *marker {
        return None;
    }
    let flags = read_u32(buf, offset + 0x08)?;
    let container = read_u64(buf, offset + per::CONTAINER_OFFSET)?;
    let placement_kind = read_u32(buf, offset + per::PLACEMENT_KIND_OFFSET)?;
    Some(PartitionLevelRecord {
        stream: stream.to_string(),
        offset,
        element_id,
        flags,
        container,
        placement_kind,
    })
}

/// Find every `OST_Levels` record in one inflated stream.
pub fn find_level_records(
    stream: &str,
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
) -> Vec<PartitionLevelRecord> {
    find_level_records_with(stream, buf, declared_ids, &RECORD_MARKER, &[])
}

/// [`find_level_records`] for a release's record `marker`, with
/// second-prologue frames attributed through the partition's record
/// `chain` ([`decode_record_at_with`]).
pub fn find_level_records_with(
    stream: &str,
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    marker: &[u8; 6],
    chain: &[crate::partition_element_records::PartitionRecordSpan],
) -> Vec<PartitionLevelRecord> {
    use crate::partition_element_records as per;
    let needle = (OST_LEVELS as u64).to_le_bytes();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor + needle.len() <= buf.len() {
        let Some(found) = find_subslice(&buf[cursor..], &needle) else {
            break;
        };
        let hit = cursor + found;
        if hit >= per::CATEGORY_OFFSET {
            if let Some(record) = decode_record_at_with(
                stream,
                buf,
                hit - per::CATEGORY_OFFSET,
                declared_ids,
                marker,
                chain,
            ) {
                out.push(record);
            }
        }
        cursor = hit + 1;
    }
    out
}

/// A name/elevation block found in one inflated stream, before any
/// owner filtering.
#[derive(Debug, Clone, PartialEq)]
pub struct NameElevationBlock {
    /// ElementId the block belongs to.
    pub element_id: u32,
    /// The name string, verbatim.
    pub name: String,
    /// The elevation in feet, agreed by both copies.
    pub elevation_feet: f64,
    /// Revit's Building Story setting, read after the name (RE-118,
    /// [`BUILDING_STORY_ANCHOR`]).
    pub building_story: Option<bool>,
}

/// The bytes that open the Level data holding its Building Story setting
/// (RE-118): an unset `u64`, then `00 01`. An `f64`, a `u64` ElementId and
/// the setting follow.
pub const BUILDING_STORY_ANCHOR: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x01];

/// How far past a Level's name its Building Story setting is looked for.
pub const BUILDING_STORY_WINDOW: usize = 0x200;

/// A Level's Building Story setting (RE-118): the byte after the first
/// [`BUILDING_STORY_ANCHOR`] past its name (`name_end`), an `f64` (0 on most
/// Levels) and a `u64` ElementId, 1 for a building story and 0 otherwise.
/// Against the Building Story parameter of Snowdon Towers' VIM export it is
/// exact on all 18 architectural and 19 structural Levels (12 stories, 7
/// not), and it is 1 on Core Interior's 15 and RE1's 2 Levels, every one a
/// storey of Revit's export. `None` where the frame is not found.
fn building_story_after(buf: &[u8], name_end: usize) -> Option<bool> {
    let window = buf.get(name_end..(name_end + BUILDING_STORY_WINDOW).min(buf.len()))?;
    let anchor = name_end + find_subslice(window, &BUILDING_STORY_ANCHOR)?;
    let id_at = anchor + BUILDING_STORY_ANCHOR.len() + 8;
    if read_u64(buf, id_at)? > u64::from(u32::MAX) {
        return None;
    }
    match buf.get(id_at + 8)? {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

/// Decode the block whose `0xff` sentinel run starts at `run_start`.
///
/// Fail-closed at every step: a run that is not exactly
/// [`OWNER_SENTINEL_RUN_LEN`] long, a length prefix out of range, a
/// name that is not valid UTF-16, an owner that `Global/ElemTable`
/// does not declare, a missing elevation marker, a non-finite or
/// out-of-range elevation, or two elevation copies that disagree all
/// reject the block.
pub fn decode_name_block_at(
    buf: &[u8],
    run_start: usize,
    declared_ids: &BTreeSet<u32>,
) -> Option<NameElevationBlock> {
    decode_name_block_at_with(buf, run_start, declared_ids, &ELEVATION_MARKER[..6])
}

/// [`decode_name_block_at`] for a release's elevation `marker`
/// ([`elevation_marker`]). The elevation is confirmed by a second copy
/// anywhere in the next [`ELEVATION_MARKER_SEARCH_BYTES`] whose
/// [`ELEVATION_CONFIRM_LEAD_IN`] bytes before it are also the first copy's
/// (RE-51); on Core Interior that copy is [`ELEVATION_CONFIRM_STRIDE`]
/// bytes on.
pub fn decode_name_block_at_with(
    buf: &[u8],
    run_start: usize,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8],
) -> Option<NameElevationBlock> {
    decode_name_block(
        buf,
        run_start,
        declared_ids,
        marker,
        8,
        OWNER_SENTINEL_RUN_LEN,
    )
}

/// Length of the `0xff` sentinel run of a Revit 2023 name block, whose
/// owner is a `u32` (RE-107): half of [`OWNER_SENTINEL_RUN_LEN`].
pub const OWNER_SENTINEL_RUN_LEN_2023: usize = 28;

/// A name block whose owner is `id_len` bytes wide and whose sentinel run
/// is `run_len` bytes: `u64` and 56 on 2024 and 2025, `u32` and 28 on 2023
/// (RE-107). Three zero bytes and the name's `u32` length follow the run.
fn decode_name_block(
    buf: &[u8],
    run_start: usize,
    declared_ids: &BTreeSet<u32>,
    marker: &[u8],
    id_len: usize,
    run_len: usize,
) -> Option<NameElevationBlock> {
    let value = run_start.checked_add(run_len + 3 + LENGTH_PREFIX_OFFSET_BEFORE_NAME)?;
    let owner_at = run_start.checked_sub(id_len)?;
    let run = buf.get(run_start..run_start + run_len)?;
    if !run.iter().all(|byte| *byte == 0xff) {
        return None;
    }
    // The run must end here: a longer run means this is not the slot. The
    // pad's first byte is 1 on 8 of Snowdon Towers' 19 structural Levels,
    // stories and not alike, and 0 on every other Level measured (RE-118).
    let pad = buf.get(run_start + run_len..value - LENGTH_PREFIX_OFFSET_BEFORE_NAME)?;
    if !matches!(pad.first(), Some(0 | 1)) || !pad[1..].iter().all(|byte| *byte == 0) {
        return None;
    }
    let owner = if id_len == 4 {
        u64::from(read_u32(buf, owner_at)?)
    } else {
        read_u64(buf, owner_at)?
    };
    if owner == 0 || owner > u64::from(u32::MAX) {
        return None;
    }
    let element_id = owner as u32;
    if !declared_ids.contains(&element_id) {
        return None;
    }
    let chars = read_u32(buf, value - LENGTH_PREFIX_OFFSET_BEFORE_NAME)? as usize;
    if chars == 0 || chars > MAX_NAME_CHARS {
        return None;
    }
    let end = value.checked_add(chars.checked_mul(2)?)?;
    let raw = buf.get(value..end)?;
    let units: Vec<u16> = raw
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let name = String::from_utf16(&units).ok()?;
    if name.chars().any(|c| c.is_control()) {
        return None;
    }
    let window = buf.get(end..(end + ELEVATION_MARKER_SEARCH_BYTES).min(buf.len()))?;
    let at = end + find_subslice(window, marker)? + ELEVATION_OFFSET_AFTER_MARKER;
    let elevation_feet = read_f64(buf, at)?;
    if !elevation_feet.is_finite() || elevation_feet.abs() > MAX_ELEVATION_FEET {
        return None;
    }
    let copy = buf.get(at.checked_sub(ELEVATION_CONFIRM_LEAD_IN)?..at + 8)?;
    let after = buf.get(at + 8..(at + 8 + ELEVATION_MARKER_SEARCH_BYTES).min(buf.len()))?;
    find_subslice(after, copy)?;
    Some(NameElevationBlock {
        element_id,
        name,
        elevation_feet,
        building_story: building_story_after(buf, end),
    })
}

/// Find every name/elevation block in one inflated stream.
///
/// The scan walks maximal runs of `0xff` and tests each run's start:
/// the owner slot immediately before a block's run is an `ElementId`
/// whose high four bytes are zero, so the run can never start earlier
/// than the framing says it does.
pub fn find_name_blocks(buf: &[u8], declared_ids: &BTreeSet<u32>) -> Vec<NameElevationBlock> {
    find_name_blocks_with(buf, declared_ids, &ELEVATION_MARKER[..6])
}

/// [`find_name_blocks`] for a release's elevation `marker`.
pub fn find_name_blocks_with(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    marker: &[u8],
) -> Vec<NameElevationBlock> {
    find_name_blocks_sized(buf, declared_ids, marker, 8, OWNER_SENTINEL_RUN_LEN)
}

/// [`find_name_blocks_with`] for owners `id_len` bytes wide and sentinel
/// runs `run_len` bytes long ([`decode_name_block`]).
fn find_name_blocks_sized(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    marker: &[u8],
    id_len: usize,
    run_len: usize,
) -> Vec<NameElevationBlock> {
    // Each run of `0xff` at least `run_len` long, decoded at the run's
    // first byte. The search jumps to the next such run rather than
    // stepping every byte; walking back from the hit finds where the run
    // starts, never before the previous run's end.
    let mut out = Vec::new();
    let run = vec![0xffu8; run_len];
    let finder = memchr::memmem::Finder::new(&run);
    let mut index = 0usize;
    while let Some(found) = finder.find(&buf[index..]) {
        let mut run_start = index + found;
        while run_start > index && buf[run_start - 1] == 0xff {
            run_start -= 1;
        }
        index = run_start;
        while index < buf.len() && buf[index] == 0xff {
            index += 1;
        }
        if let Some(block) =
            decode_name_block(buf, run_start, declared_ids, marker, id_len, run_len)
        {
            out.push(block);
        }
    }
    out
}

/// First occurrence of `needle` in `haystack`, vectorised (as
/// `partition_element_records` searches).
fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    memchr::memmem::find(haystack, needle)
}

/// The Levels of `level_ids` whose name block names an owner: the
/// ElementId at the block's owner slot is followed by eight `0xff` bytes and
/// then another element's ElementId, where a project Level's block has an
/// unbroken sentinel run (RE-51). On Snowdon Towers these are two Levels
/// inside other elements, neither of them a storey of Revit's export.
pub fn owned_level_ids(buf: &[u8], level_ids: &BTreeSet<u32>) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for (id, owner_at) in crate::partition_id_objects::find_id_objects(buf, level_ids) {
        let value = owner_at + OWNER_OFFSET_BEFORE_NAME;
        let Some(chars) = read_u32(buf, value - LENGTH_PREFIX_OFFSET_BEFORE_NAME) else {
            continue;
        };
        if !(1..=MAX_NAME_CHARS as u32).contains(&chars) {
            continue;
        }
        let sentinel = buf.get(owner_at + 8..owner_at + 16);
        let owner = read_u64(buf, owner_at + 16);
        if sentinel.is_some_and(|run| run.iter().all(|b| *b == 0xff))
            && owner.is_some_and(|o| o != 0 && o < u64::from(u32::MAX))
        {
            out.insert(id);
        }
    }
    out
}

/// Join accepted blocks onto the Level element ids.
///
/// A level that owns two blocks naming different values is dropped:
/// a level that cannot be read unambiguously is not a level. Blocks
/// owned by anything that is not a standalone Level record — a
/// container member, a type symbol, or any other element — are
/// ignored.
pub fn levels_from_records_and_blocks(
    records: &[PartitionLevelRecord],
    blocks: impl IntoIterator<Item = NameElevationBlock>,
) -> Vec<PartitionLevel> {
    let level_ids: BTreeSet<u32> = records
        .iter()
        .filter(|record| record.is_level_element())
        .map(|record| record.element_id)
        .collect();
    let mut accepted: BTreeMap<u32, PartitionLevel> = BTreeMap::new();
    let mut conflicting: BTreeSet<u32> = BTreeSet::new();
    for block in blocks {
        if !level_ids.contains(&block.element_id) {
            continue;
        }
        let level = PartitionLevel {
            element_id: block.element_id,
            name: block.name,
            elevation_feet: block.elevation_feet,
            building_story: block.building_story,
        };
        match accepted.get(&level.element_id) {
            Some(existing)
                if existing.name != level.name
                    || existing.elevation_feet.to_bits() != level.elevation_feet.to_bits() =>
            {
                conflicting.insert(level.element_id);
            }
            Some(_) => {}
            None => {
                accepted.insert(level.element_id, level);
            }
        }
    }
    for id in conflicting {
        accepted.remove(&id);
    }
    let mut out: Vec<PartitionLevel> = accepted.into_values().collect();
    out.sort_by(|a, b| {
        a.elevation_feet
            .partial_cmp(&b.elevation_feet)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.element_id.cmp(&b.element_id))
    });
    out
}

/// Whether a recovered level list is usable as a storey set.
///
/// Fail closed: at least two levels, one block per Level, and no two
/// levels sharing an elevation. A partial recovery is not
/// silently emitted as a smaller building.
pub fn recovered_levels_are_a_storey_set(
    records: &[PartitionLevelRecord],
    levels: &[PartitionLevel],
) -> bool {
    // Distinct Levels: a Level can be written in more than one partition
    // (RE-107: modelo_bim holds Level 338608 twice).
    let level_records = records
        .iter()
        .filter(|record| record.is_level_element())
        .map(|record| record.element_id)
        .collect::<BTreeSet<u32>>()
        .len();
    if levels.len() < 2 || levels.len() != level_records {
        return false;
    }
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    levels
        .iter()
        .all(|level| seen.insert(level.elevation_feet.to_bits()))
}

/// Every ElementId that frames a standalone Revit `Level` record.
///
/// Cheaper than [`scan_partition_levels`] — it skips the name /
/// elevation blocks — and deliberately *broader*: it is the id set the
/// #219 reference-list join tests for uniqueness, so a Level that the
/// full recovery would drop still counts as a second named Level and
/// keeps the join fail-closed rather than turning an ambiguous record
/// into a confident one.
pub fn scan_partition_level_ids(
    rf: &mut RevitFile,
    revit_version: u32,
    declared_ids: &BTreeSet<u32>,
) -> Result<BTreeSet<u32>> {
    if !supports_revit_version(revit_version) || declared_ids.is_empty() {
        return Ok(BTreeSet::new());
    }
    Ok(level_records(rf, revit_version, declared_ids)
        .into_iter()
        .filter(PartitionLevelRecord::is_level_element)
        .map(|record| record.element_id)
        .collect())
}

/// Bytes from a Revit 2023 Level record's `u32` ElementId to its
/// `BuiltInCategory` (RE-107).
const ID_BEFORE_CATEGORY_2023: usize = 14;
/// Bytes from the ElementId to the `ElementHeader` tag, after the `u32` id
/// and the `u32` record size, as in [`crate::partition_element_records_2023`].
const HEADER_TAG_AFTER_ID_2023: usize = 8;
/// Bytes from the category to the container reference.
const CONTAINER_AFTER_CATEGORY_2023: usize = 8;
/// Bytes from the category to the placement kind.
const PLACEMENT_KIND_AFTER_CATEGORY_2023: usize = 24;
/// Bytes from the category to the record marker.
const MARKER_AFTER_CATEGORY_2023: usize = 38;

/// Every Revit 2023 `OST_Levels` record in one inflated stream (RE-107).
/// It is the 2024 record without the 24 sentinel bytes after the
/// category, opened by the 32-bit chain header 2023's element records
/// carry (RE-81): a `u32` ElementId, the record size and the
/// `ElementHeader` tag, 14 bytes before the category; the container
/// reference 8 bytes after it, the placement kind 24 after it, and the
/// record `marker` (`ff ff ff ff` and the `ElementParents` tag) 38 after
/// it.
pub fn find_level_records_2023(
    stream: &str,
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    header_tag: u16,
    marker: &[u8; 6],
) -> Vec<PartitionLevelRecord> {
    let needle = (OST_LEVELS as u64).to_le_bytes();
    memchr::memmem::find_iter(buf, &needle)
        .filter_map(|category| {
            let id_at = category.checked_sub(ID_BEFORE_CATEGORY_2023)?;
            let element_id = read_u32(buf, id_at)?;
            if element_id == 0 || !declared_ids.contains(&element_id) {
                return None;
            }
            if read_u16(buf, id_at + HEADER_TAG_AFTER_ID_2023)? != header_tag {
                return None;
            }
            let marker_at = category + MARKER_AFTER_CATEGORY_2023;
            if buf.get(marker_at..marker_at + marker.len())? != marker {
                return None;
            }
            Some(PartitionLevelRecord {
                stream: stream.to_string(),
                offset: id_at,
                element_id,
                flags: read_u32(buf, id_at + 4)?,
                container: read_u64(buf, category + CONTAINER_AFTER_CATEGORY_2023)?,
                placement_kind: read_u32(buf, category + PLACEMENT_KIND_AFTER_CATEGORY_2023)?,
            })
        })
        .collect()
}

/// The placement kind Revit 2014 to 2018 write on a Level element, where
/// 2019 and later write [`crate::partition_element_records::PLACEMENT_KIND_INSTANCE`]
/// (RE-179). Their other records carry the later releases' kinds: the 7
/// Levels of Autodesk's 2017 `rac_basic` hold it, and their 2019 copies
/// hold the instance kind, under the same ElementIds and bytes otherwise.
pub const LEVEL_PLACEMENT_KIND_2014: u32 = 0xffff_ef6f;

/// Every `OST_Levels` record of every partition, first- and
/// second-prologue. On Revit 2014 to 2018 a Level's own placement kind,
/// [`LEVEL_PLACEMENT_KIND_2014`], reads as the instance kind.
fn level_records(
    rf: &mut RevitFile,
    revit_version: u32,
    declared_ids: &BTreeSet<u32>,
) -> Vec<PartitionLevelRecord> {
    if crate::partition_element_records_2023::is_32bit_release(revit_version) {
        let Some((full, header_tag)) = crate::partition_element_records_2023::record_marker(rf)
        else {
            return Vec::new();
        };
        let mut marker = [0u8; 6];
        marker.copy_from_slice(&full[2..]);
        let mut records = Vec::new();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            records.extend(find_level_records_2023(
                &stream,
                inflated.bytes(),
                declared_ids,
                header_tag,
                &marker,
            ));
        }
        if revit_version < 2019 {
            for record in &mut records {
                if record.placement_kind == LEVEL_PLACEMENT_KIND_2014 {
                    record.placement_kind =
                        crate::partition_element_records::PLACEMENT_KIND_INSTANCE;
                }
            }
        }
        return records;
    }
    let (Some(marker), Some(bbox_marker)) = (
        record_marker(revit_version),
        crate::partition_element_records::bbox_marker(revit_version),
    ) else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let chain = crate::partition_element_records::partition_record_chain(buf, &bbox_marker);
        records.extend(find_level_records_with(
            &stream,
            buf,
            declared_ids,
            &marker,
            &chain,
        ));
    }
    records
}

/// Scan every `Partitions/*` stream for Revit `Level` elements.
///
/// Returns an empty vector for unsupported releases, and an empty
/// vector when the recovery does not satisfy
/// [`recovered_levels_are_a_storey_set`] (fail closed).
pub fn scan_partition_levels(
    rf: &mut RevitFile,
    revit_version: u32,
    declared_ids: &BTreeSet<u32>,
) -> Result<Vec<PartitionLevel>> {
    if !supports_revit_version(revit_version) || declared_ids.is_empty() {
        return Ok(Vec::new());
    }
    let Some(marker) = elevation_marker(revit_version).or_else(|| schema_elevation_marker(rf))
    else {
        return Ok(Vec::new());
    };
    let records = level_records(rf, revit_version, declared_ids);
    let level_ids: BTreeSet<u32> = records
        .iter()
        .filter(|record| record.is_level_element())
        .map(|record| record.element_id)
        .collect();
    // Each block with the number of the partition it is in: a Level
    // edited after an earlier save is rewritten into a higher-numbered
    // partition and the earlier copy left in place, as every element is
    // (RE-26 §3), so only its newest partition's blocks are its current
    // name and elevation (RE-107).
    let mut numbered: Vec<(u32, NameElevationBlock)> = Vec::new();
    let mut owned = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let number = stream
            .rsplit('/')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0);
        let mut blocks = Vec::new();
        let buf = inflated.bytes();
        if crate::partition_element_records_2023::is_32bit_release(revit_version) {
            // No Level inside another element is measured on 2014 to 2023.
            blocks.extend(find_name_blocks_sized(
                buf,
                declared_ids,
                &marker,
                4,
                OWNER_SENTINEL_RUN_LEN_2023,
            ));
        } else {
            blocks.extend(find_name_blocks_with(buf, declared_ids, &marker));
            owned.extend(owned_level_ids(buf, &level_ids));
        }
        numbered.extend(blocks.into_iter().map(|block| (number, block)));
    }
    let mut newest: BTreeMap<u32, u32> = BTreeMap::new();
    for (number, block) in &numbered {
        let held = newest.entry(block.element_id).or_insert(*number);
        *held = (*held).max(*number);
    }
    let blocks: Vec<NameElevationBlock> = numbered
        .into_iter()
        .filter(|(number, block)| newest.get(&block.element_id) == Some(number))
        .map(|(_, block)| block)
        .collect();
    // A Level inside another element is not a storey: it neither joins
    // the storey set nor counts against it.
    let records: Vec<PartitionLevelRecord> = records
        .into_iter()
        .filter(|record| !owned.contains(&record.element_id))
        .collect();
    let levels = levels_from_records_and_blocks(&records, blocks);
    if !recovered_levels_are_a_storey_set(&records, &levels) {
        return Ok(Vec::new());
    }
    Ok(levels)
}

/// Every declared object laid out `01 00 00 00 · u64 id · u64 Level id`
/// whose second id is one of `level_ids`, as its Level, by id (RE-60).
/// An id found with two different Levels is dropped. The objects are the
/// work planes and views a family instance can reference.
pub fn scan_level_objects(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
    level_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, u32> {
    let mut found: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let (Some(id), Some(level)) = (read_u64(buf, at + 4), read_u64(buf, at + 12)) else {
                continue;
            };
            let (Ok(id), Ok(level)) = (u32::try_from(id), u32::try_from(level)) else {
                continue;
            };
            if !declared.contains(&id) || level_ids.contains(&id) || !level_ids.contains(&level) {
                continue;
            }
            let held = found.entry(id).or_insert(Some(level));
            if *held != Some(level) {
                *held = None;
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, level)| level.map(|level| (id, level)))
        .collect()
}

/// [`scan_partition_levels`] with the `Global/ElemTable` id set read
/// from `rf`, for callers that hold no id set of their own.
pub fn recover_partition_levels(
    rf: &mut RevitFile,
    revit_version: u32,
) -> Result<Vec<PartitionLevel>> {
    if !supports_revit_version(revit_version) {
        return Ok(Vec::new());
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(Vec::new()),
    };
    scan_partition_levels(rf, revit_version, &declared)
}
