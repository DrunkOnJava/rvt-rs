//! Room number / name / host Level from the Revit 2024 partition
//! parameter block a room owns (#90, RE-29).
//!
//! RE-22 found that a per-instance parameter *value* sits at a fixed
//! negative offset from two copies of the owning `ElementId`, and used
//! that to read the `IFC Export As` override. A room's number and name
//! are carried by the same shape, read from the other end: anchor on
//! the owning `ElementId` and walk **forward**.
//!
//! ```text
//! +0x000  u64  owning room ElementId
//! +0x03c  u64  owning room ElementId   (confirmation; must agree)
//! +0x044  u64  host Level ElementId
//! +0x1f5  u32  room number length, in UTF-16 code units
//! +0x1f9  2*n  room Number, UTF-16LE
//! +…+ 8   u32  room name length, in UTF-16 code units   (8 bytes past
//! +…+12   2*m  room Name, UTF-16LE                       the number)
//! ```
//!
//! The name's offset is not fixed because the number's length is not:
//! the name's length prefix sits [`NAME_GAP_AFTER_NUMBER`] bytes past
//! the end of the number string, which is why the block is parsed
//! forward from the owner rather than backward from either string.
//!
//! # Honesty
//!
//! - Every offset here is **measured**, on one file, over the 232
//!   blocks that frame its 116 rooms. The parameter block's own header
//!   is not decoded; these are fixed displacements inside it, exactly
//!   as RE-22's two owner offsets are.
//! - The scan is anchored on an ElementId the caller supplies, and it
//!   requires the confirmation copy at [`OWNER_CONFIRM_OFFSET`] to
//!   carry the same id. Measured over every one of the 26 425 ids
//!   `Global/ElemTable` declares on `2024_Core_Interior.rvt`: the
//!   framing accepts **116** of them and they are exactly the 116
//!   rooms Revit's own export emits as `IfcSpace` — no other declared
//!   id accepts it at all.
//! - Recovery is **all or nothing per room**. A room framed by two
//!   blocks that disagree resolves to nothing rather than to either.
//!   On the recorded edge all 116 rooms are framed twice and no pair
//!   disagrees.
//! - The recovered `(number, name)` pairs were scored against the
//!   reference export: 116 of 116 names equal the `IfcSpace.LongName`
//!   Revit writes and 116 of 116 numbers equal its `IfcSpace.Name`.
//! - The Level slot at [`LEVEL_OFFSET`] is returned verbatim. Nothing
//!   here checks that it is a Level — the caller does, against the
//!   `OST_Levels` id set, the same way
//!   [`crate::element_record_level_refs`] does.
//!
//! See `reports/element-framing/RE-29-room-instance-and-envelope.md`.

use crate::{Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this framing is corpus-proven.
pub const ROOM_PARAMETER_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Bytes from the owning ElementId to its confirmation copy.
pub const OWNER_CONFIRM_OFFSET: usize = 0x3c;

/// Bytes from the owning ElementId to the host Level ElementId.
pub const LEVEL_OFFSET: usize = 0x44;

/// Bytes from the owning ElementId to the number's `u32` length prefix.
pub const NUMBER_LENGTH_OFFSET: usize = 505;

/// Bytes from the end of the number string to the name's length prefix.
pub const NAME_GAP_AFTER_NUMBER: usize = 8;

/// Longest number / name the scan will accept, in UTF-16 code units.
pub const MAX_VALUE_CHARS: usize = 64;

/// Value recorded for where a recovered room number / name came from.
pub const ROOM_PARAMETER_SOURCE: &str = "partition_room_parameter_block";

/// One accepted room parameter block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomParameters {
    /// Stream the block was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the owning ElementId in the inflated stream.
    pub offset: usize,
    /// The room this block belongs to.
    pub element_id: u32,
    /// Room number, e.g. `"66"`.
    pub number: String,
    /// Room name, e.g. `"Stair 2"`.
    pub name: String,
    /// Raw Level slot at [`LEVEL_OFFSET`], verbatim.
    pub level_slot: u64,
}

impl RoomParameters {
    /// The Level ElementId this block names, when it is one of
    /// `level_ids`. Fail-closed on a slot outside the `u32` range or
    /// outside the recovered Level set.
    pub fn level_element_id(&self, level_ids: &BTreeSet<u32>) -> Option<u32> {
        let id = u32::try_from(self.level_slot).ok()?;
        level_ids.contains(&id).then_some(id)
    }
}

/// Whether this release's room parameter framing is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    ROOM_PARAMETER_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// A UTF-16LE string of `chars` code units at `at`, rejected when it
/// holds a control character.
fn utf16_at(buf: &[u8], at: usize, chars: usize) -> Option<String> {
    if !(1..=MAX_VALUE_CHARS).contains(&chars) {
        return None;
    }
    let bytes = buf.get(at..at.checked_add(chars.checked_mul(2)?)?)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    if text.chars().any(char::is_control) {
        return None;
    }
    Some(text)
}

/// Decode the room parameter block whose owning ElementId sits at
/// `owner_at`, fail-closed.
///
/// Returns `None` unless the confirmation copy at
/// [`OWNER_CONFIRM_OFFSET`] carries the same `element_id` and both
/// strings decode as non-empty, control-free UTF-16LE.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    owner_at: usize,
    element_id: u32,
) -> Option<RoomParameters> {
    if read_u64(buf, owner_at)? != u64::from(element_id) {
        return None;
    }
    let confirm_at = owner_at.checked_add(OWNER_CONFIRM_OFFSET)?;
    if read_u64(buf, confirm_at)? != u64::from(element_id) {
        return None;
    }
    let level_slot = read_u64(buf, owner_at.checked_add(LEVEL_OFFSET)?)?;

    let number_len_at = owner_at.checked_add(NUMBER_LENGTH_OFFSET)?;
    let number_chars = read_u32(buf, number_len_at)? as usize;
    let number_at = number_len_at.checked_add(4)?;
    let number = utf16_at(buf, number_at, number_chars)?;

    let name_len_at = number_at
        .checked_add(number_chars.checked_mul(2)?)?
        .checked_add(NAME_GAP_AFTER_NUMBER)?;
    let name_chars = read_u32(buf, name_len_at)? as usize;
    let name = utf16_at(buf, name_len_at.checked_add(4)?, name_chars)?;

    Some(RoomParameters {
        stream: stream.to_string(),
        offset: owner_at,
        element_id,
        number,
        name,
        level_slot,
    })
}

/// Every accepted room parameter block in `buf` for an id in `rooms`.
pub fn find_room_parameters(
    stream: &str,
    buf: &[u8],
    rooms: &BTreeSet<u32>,
) -> Vec<RoomParameters> {
    let mut out = Vec::new();
    let (Some(&low), Some(&high)) = (rooms.first(), rooms.last()) else {
        return out;
    };
    if buf.len() < 8 {
        return out;
    }
    // Most offsets hold no room id; the range test settles them before
    // the set lookup.
    let (low, high) = (u64::from(low), u64::from(high));
    for owner_at in 0..=(buf.len() - 8) {
        let raw = u64::from_le_bytes(buf[owner_at..owner_at + 8].try_into().expect("8 bytes"));
        if raw == 0 || raw < low || raw > high {
            continue;
        }
        let element_id = raw as u32;
        if !rooms.contains(&element_id) {
            continue;
        }
        if let Some(block) = decode_at(stream, buf, owner_at, element_id) {
            out.push(block);
        }
    }
    out
}

/// Scan every `Partitions/*` stream for the parameter block of each
/// room in `rooms`, keeping only rooms whose blocks all agree.
///
/// Returns an empty map for unsupported releases (fail closed).
pub fn scan_room_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, RoomParameters>> {
    if !supports_revit_version(revit_version) || rooms.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut found: BTreeMap<u32, Vec<RoomParameters>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for block in find_room_parameters(&stream, inflated.bytes(), rooms) {
            found.entry(block.element_id).or_default().push(block);
        }
    }
    Ok(resolve_unique(found))
}

/// Keep one block per room, and only when every block found for it
/// agrees on the number, the name and the Level slot.
pub fn resolve_unique(found: BTreeMap<u32, Vec<RoomParameters>>) -> BTreeMap<u32, RoomParameters> {
    let mut out = BTreeMap::new();
    for (id, blocks) in found {
        let Some(first) = blocks.first() else {
            continue;
        };
        let agree = blocks.iter().all(|block| {
            block.number == first.number
                && block.name == first.name
                && block.level_slot == first.level_slot
        });
        if agree {
            out.insert(id, first.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic block laid out exactly as the measured framing.
    fn synth_block(element_id: u32, number: &str, name: &str, level: u64) -> Vec<u8> {
        let number_units: Vec<u16> = number.encode_utf16().collect();
        let name_units: Vec<u16> = name.encode_utf16().collect();
        let name_len_at = NUMBER_LENGTH_OFFSET + 4 + number_units.len() * 2 + NAME_GAP_AFTER_NUMBER;
        let mut buf = vec![0xabu8; name_len_at + 4 + name_units.len() * 2 + 16];

        buf[0..8].copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[OWNER_CONFIRM_OFFSET..OWNER_CONFIRM_OFFSET + 8]
            .copy_from_slice(&u64::from(element_id).to_le_bytes());
        buf[LEVEL_OFFSET..LEVEL_OFFSET + 8].copy_from_slice(&level.to_le_bytes());

        buf[NUMBER_LENGTH_OFFSET..NUMBER_LENGTH_OFFSET + 4]
            .copy_from_slice(&(number_units.len() as u32).to_le_bytes());
        for (index, unit) in number_units.iter().enumerate() {
            let at = NUMBER_LENGTH_OFFSET + 4 + index * 2;
            buf[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        buf[name_len_at..name_len_at + 4].copy_from_slice(&(name_units.len() as u32).to_le_bytes());
        for (index, unit) in name_units.iter().enumerate() {
            let at = name_len_at + 4 + index * 2;
            buf[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        buf
    }

    #[test]
    fn decodes_a_well_formed_room_block() {
        let buf = synth_block(20_822, "66", "Stair 2", 20_308);
        let block = decode_at("Partitions/46", &buf, 0, 20_822).expect("decodes");
        assert_eq!(block.element_id, 20_822);
        assert_eq!(block.number, "66");
        assert_eq!(block.name, "Stair 2");
        assert_eq!(block.level_slot, 20_308);
        assert_eq!(
            block.level_element_id(&[20_308u32].into_iter().collect()),
            Some(20_308)
        );
    }

    /// The name's offset moves with the number's length, so a longer
    /// number must not shift the name out of reach.
    #[test]
    fn a_longer_number_still_finds_the_name() {
        let buf = synth_block(60_221, "1005", "Cooridor Suite 1", 20_304);
        let block = decode_at("Partitions/59", &buf, 0, 60_221).expect("decodes");
        assert_eq!(block.number, "1005");
        assert_eq!(block.name, "Cooridor Suite 1");
    }

    #[test]
    fn a_disagreeing_confirmation_slot_fails_closed() {
        let mut buf = synth_block(20_822, "66", "Stair 2", 20_308);
        buf[OWNER_CONFIRM_OFFSET..OWNER_CONFIRM_OFFSET + 8]
            .copy_from_slice(&20_823u64.to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, 0, 20_822).is_none());
    }

    #[test]
    fn a_wrong_anchor_id_fails_closed() {
        let buf = synth_block(20_822, "66", "Stair 2", 20_308);
        assert!(decode_at("Partitions/46", &buf, 0, 20_823).is_none());
    }

    #[test]
    fn an_implausible_length_fails_closed() {
        let mut buf = synth_block(20_822, "66", "Stair 2", 20_308);
        buf[NUMBER_LENGTH_OFFSET..NUMBER_LENGTH_OFFSET + 4]
            .copy_from_slice(&9_999u32.to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, 0, 20_822).is_none());
        buf[NUMBER_LENGTH_OFFSET..NUMBER_LENGTH_OFFSET + 4].copy_from_slice(&0u32.to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, 0, 20_822).is_none());
    }

    #[test]
    fn a_control_character_in_a_value_fails_closed() {
        let buf = synth_block(20_822, "66", "Stair\u{7}2", 20_308);
        assert!(decode_at("Partitions/46", &buf, 0, 20_822).is_none());
    }

    #[test]
    fn a_truncated_block_fails_closed() {
        let buf = synth_block(20_822, "66", "Stair 2", 20_308);
        for cut in [8usize, OWNER_CONFIRM_OFFSET + 4, NUMBER_LENGTH_OFFSET + 2] {
            assert!(decode_at("Partitions/46", &buf[..cut], 0, 20_822).is_none());
        }
    }

    #[test]
    fn the_scan_only_accepts_ids_it_was_given() {
        let buf = synth_block(20_822, "66", "Stair 2", 20_308);
        let hits = find_room_parameters("Partitions/46", &buf, &[20_822u32].into_iter().collect());
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "Stair 2");
        assert!(find_room_parameters("Partitions/46", &buf, &BTreeSet::new()).is_empty());
        assert!(
            find_room_parameters("Partitions/46", &buf, &[999u32].into_iter().collect()).is_empty()
        );
    }

    #[test]
    fn two_blocks_that_agree_resolve_to_one() {
        let block = |offset: usize| RoomParameters {
            stream: "Partitions/46".into(),
            offset,
            element_id: 20_822,
            number: "66".into(),
            name: "Stair 2".into(),
            level_slot: 20_308,
        };
        let found = [(20_822u32, vec![block(0), block(4096)])]
            .into_iter()
            .collect();
        let resolved = resolve_unique(found);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[&20_822].name, "Stair 2");
    }

    #[test]
    fn two_blocks_that_disagree_resolve_to_nothing() {
        let mut a = RoomParameters {
            stream: "Partitions/46".into(),
            offset: 0,
            element_id: 20_822,
            number: "66".into(),
            name: "Stair 2".into(),
            level_slot: 20_308,
        };
        let b = RoomParameters {
            name: "Stair 3".into(),
            offset: 4096,
            ..a.clone()
        };
        a.stream = "Partitions/46".into();
        let found = [(20_822u32, vec![a, b])].into_iter().collect();
        assert!(resolve_unique(found).is_empty());
    }

    #[test]
    fn an_unsupported_release_recovers_nothing() {
        assert!(!supports_revit_version(2023));
        assert!(supports_revit_version(2024));
    }

    #[test]
    fn a_level_slot_outside_the_recovered_set_binds_nothing() {
        let buf = synth_block(20_822, "66", "Stair 2", 99_999);
        let block = decode_at("Partitions/46", &buf, 0, 20_822).expect("decodes");
        assert_eq!(
            block.level_element_id(&[20_308u32].into_iter().collect()),
            None
        );
        let huge = synth_block(20_822, "66", "Stair 2", u64::MAX);
        let block = decode_at("Partitions/46", &huge, 0, 20_822).expect("decodes");
        assert_eq!(
            block.level_element_id(&[20_308u32].into_iter().collect()),
            None
        );
    }
}
