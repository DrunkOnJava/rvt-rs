//! The IFC `GlobalId` Revit's own exporter gives an element, rebuilt from
//! the file (RE-48).
//!
//! Revit's exporter derives an element's GlobalId from the element's
//! UniqueId: the GUID of the editing episode that created it, with the
//! ElementId XORed into its last 32 bits, written in IFC's 22-character
//! base-64 form. Both halves are in the file:
//!
//! - `Global/History` lists every episode GUID, newest first:
//!
//!   ```text
//!   +0x0e       u32  N, the number of episodes
//!   +0x66       u32  M, then M u32 (not read)
//!   then        u32  N again, then N entries: 16-byte GUID (Windows
//!                    byte order) and the byte 0x28
//!   ```
//!
//! - A 40-byte `Global/ElemTable` record (Revit 2024 and later) holds the
//!   element's episode number at `+0x18`. The element's episode GUID is
//!   entry `N - 1 - episode` of the list.
//!
//! Measured against Revit's own IFC exports, every element rvt-rs exports
//! that Revit's export also holds gets the GlobalId Revit gives it:
//! 854 of 854 on `2024_Core_Interior.rvt`, 281 of 281 on the RE1 models, 10
//! of 10 on `teste_export_2025` and `Projeto1`, and 5,945 of 5,945 on Snowdon
//! Towers. Revit also exports parts it synthesises (stair components, a
//! ramp's flight), with GlobalIds of its own making. rvt-rs does not write
//! those parts. The ElementId XORed in is the
//! element's first one: an element whose id has since changed (RE-41) keeps
//! the id it was created with in its UniqueId.

use crate::{Result, RevitFile, compression, elem_table, streams};
use std::collections::BTreeMap;

/// Offset of the episode count in `Global/History`.
pub const EPISODE_COUNT_OFFSET: usize = 0x0e;
/// Offset of the `u32`-counted table that precedes the episode list.
pub const EPISODE_TABLE_OFFSET: usize = 0x66;
/// Bytes per episode entry: a GUID and the byte [`EPISODE_TERMINATOR`].
pub const EPISODE_ENTRY_LEN: usize = 17;
/// The byte that closes every episode entry.
pub const EPISODE_TERMINATOR: u8 = 0x28;
/// Offset of the episode number in a 40-byte `Global/ElemTable` record.
pub const RECORD_EPISODE_OFFSET: usize = 0x18;

/// The episode GUIDs of a file, in stored order (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeHistory {
    guids: Vec<[u8; 16]>,
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

impl EpisodeHistory {
    /// Parse a decompressed `Global/History`, or `None` when its counts
    /// disagree, a count runs past the stream, or an entry is not closed by
    /// [`EPISODE_TERMINATOR`].
    pub fn parse(history: &[u8]) -> Option<Self> {
        let count = read_u32(history, EPISODE_COUNT_OFFSET)? as usize;
        let skipped = read_u32(history, EPISODE_TABLE_OFFSET)? as usize;
        let list_count_at = EPISODE_TABLE_OFFSET
            .checked_add(4)?
            .checked_add(skipped.checked_mul(4)?)?;
        if read_u32(history, list_count_at)? as usize != count {
            return None;
        }
        let start = list_count_at + 4;
        let end = start.checked_add(count.checked_mul(EPISODE_ENTRY_LEN)?)?;
        let list = history.get(start..end)?;
        let mut guids = Vec::with_capacity(count);
        for entry in list.chunks_exact(EPISODE_ENTRY_LEN) {
            if entry[16] != EPISODE_TERMINATOR {
                return None;
            }
            guids.push(entry[..16].try_into().expect("16 bytes"));
        }
        Some(Self { guids })
    }

    /// Number of episodes.
    pub fn len(&self) -> usize {
        self.guids.len()
    }

    /// Whether the history lists no episode.
    pub fn is_empty(&self) -> bool {
        self.guids.is_empty()
    }

    /// The GUID of episode `episode`, in Windows byte order, or `None` when
    /// the history has no such episode.
    pub fn episode_guid(&self, episode: u32) -> Option<[u8; 16]> {
        let index = self
            .guids
            .len()
            .checked_sub(1)?
            .checked_sub(episode as usize)?;
        self.guids.get(index).copied()
    }
}

/// A GUID stored in Windows byte order (first three fields little-endian)
/// in the order its text form is written.
pub fn canonical_guid(stored: [u8; 16]) -> [u8; 16] {
    let mut out = stored;
    out[0..4].reverse();
    out[4..6].reverse();
    out[6..8].reverse();
    out
}

/// A 128-bit value in IFC's 22-character base-64 `GlobalId` form.
pub fn compress_ifc_guid(value: [u8; 16]) -> String {
    const ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";
    let mut n = u128::from_be_bytes(value);
    let mut out = [0u8; 22];
    for slot in out.iter_mut().rev() {
        *slot = ALPHABET[(n & 63) as usize];
        n >>= 6;
    }
    out.iter().map(|&b| char::from(b)).collect()
}

/// The GlobalId Revit's exporter gives element `element_id` of an episode
/// whose GUID is `episode_guid` (Windows byte order).
pub fn revit_ifc_global_id(episode_guid: [u8; 16], element_id: u32) -> String {
    let mut value = canonical_guid(episode_guid);
    let low = u32::from_be_bytes(value[12..16].try_into().expect("4 bytes")) ^ element_id;
    value[12..16].copy_from_slice(&low.to_be_bytes());
    compress_ifc_guid(value)
}

fn inflate_history(rf: &mut RevitFile) -> Result<Vec<u8>> {
    let stored = rf.read_stream(streams::GLOBAL_HISTORY)?;
    let prepared = compression::prepare_stream_for_inflate(streams::GLOBAL_HISTORY, &stored);
    let data = prepared.as_ref();
    let mut last = None;
    for offset in [0, 4, 8, 16] {
        if compression::has_gzip_magic(data, offset) {
            match compression::inflate_at(data, offset) {
                Ok(out) => return Ok(out),
                Err(error) => last = Some(error),
            }
        }
    }
    match last {
        Some(error) => Err(error),
        None => compression::inflate_at_auto(data).map(|(_, out)| out),
    }
}

/// The GlobalId Revit's exporter gives each element `Global/ElemTable`
/// declares, by ElementId (both of a record's ids). The ElementId XORed in
/// is the record's first id: on Snowdon Towers the 37 elements whose
/// ElementId differs from it (RE-41) all match Revit's only that way. Empty when the file's
/// ElemTable records are not 40 bytes long (Revit 2023 and earlier) or its
/// history does not parse.
pub fn revit_global_ids(rf: &mut RevitFile) -> Result<BTreeMap<u32, String>> {
    let records = elem_table::parse_records(rf)?;
    if records
        .first()
        .is_none_or(|record| record.raw.len() != elem_table::RECORD_LEN_40)
    {
        return Ok(BTreeMap::new());
    }
    let Some(history) = EpisodeHistory::parse(&inflate_history(rf)?) else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for record in &records {
        let Some(episode) = read_u32(&record.raw, RECORD_EPISODE_OFFSET) else {
            continue;
        };
        let Some(guid) = history.episode_guid(episode) else {
            continue;
        };
        // The UniqueId keeps the id the element was created with, the
        // record's first: an element whose ElementId has since changed (a
        // second id, RE-41) still XORs the first.
        let global_id = revit_ifc_global_id(guid, record.id_primary);
        for id in [record.id_primary, record.id_secondary] {
            if id != 0 {
                out.entry(id).or_insert_with(|| global_id.clone());
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The Snowdon episode whose walls 619340 and 619404 Revit exports as
    /// `1G_I00WlX2MBQekqB04dFi` and `1G_I00WlX2MBQekqB04dCi`.
    const EPISODE: [u8; 16] = [
        0x00, 0x20, 0xf9, 0x50, 0xf8, 0x82, 0x58, 0x42, 0xb6, 0xa8, 0xbb, 0x42, 0xc0, 0x1b, 0x00,
        0xa0,
    ];

    #[test]
    fn a_global_id_is_the_episode_guid_xor_the_element_id() {
        assert_eq!(
            revit_ifc_global_id(EPISODE, 619_340),
            "1G_I00WlX2MBQekqB04dFi"
        );
        assert_eq!(
            revit_ifc_global_id(EPISODE, 619_404),
            "1G_I00WlX2MBQekqB04dCi"
        );
        // 50f92000-82f8-4258-b6a8-bb42c01b00a0 in text order.
        assert_eq!(
            canonical_guid(EPISODE),
            [
                0x50, 0xf9, 0x20, 0x00, 0x82, 0xf8, 0x42, 0x58, 0xb6, 0xa8, 0xbb, 0x42, 0xc0, 0x1b,
                0x00, 0xa0
            ]
        );
        assert_eq!(compress_ifc_guid([0; 16]), "0000000000000000000000");
        assert_eq!(compress_ifc_guid([0xff; 16]), "3$$$$$$$$$$$$$$$$$$$$$");
    }

    fn history(guids: &[[u8; 16]], skipped: &[u32]) -> Vec<u8> {
        let mut out = vec![0u8; EPISODE_TABLE_OFFSET];
        out[EPISODE_COUNT_OFFSET..EPISODE_COUNT_OFFSET + 4]
            .copy_from_slice(&(guids.len() as u32).to_le_bytes());
        out.extend_from_slice(&(skipped.len() as u32).to_le_bytes());
        for value in skipped {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out.extend_from_slice(&(guids.len() as u32).to_le_bytes());
        for guid in guids {
            out.extend_from_slice(guid);
            out.push(EPISODE_TERMINATOR);
        }
        out.extend_from_slice(&[0; 4]);
        out
    }

    #[test]
    fn the_history_lists_episodes_newest_first() {
        let guids = [[1u8; 16], [2u8; 16], [3u8; 16]];
        let parsed = EpisodeHistory::parse(&history(&guids, &[472, 481])).expect("history");
        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed.episode_guid(0), Some([3u8; 16]));
        assert_eq!(parsed.episode_guid(2), Some([1u8; 16]));
        assert_eq!(parsed.episode_guid(3), None);
    }

    #[test]
    fn a_malformed_history_is_rejected() {
        let guids = [[1u8; 16], [2u8; 16]];
        let mut bytes = history(&guids, &[]);
        // The second count disagrees with the first.
        bytes[EPISODE_TABLE_OFFSET + 4] = 3;
        assert_eq!(EpisodeHistory::parse(&bytes), None);
        // An entry not closed by 0x28.
        let mut bytes = history(&guids, &[]);
        let last_terminator = bytes.len() - 5;
        bytes[last_terminator] = 0;
        assert_eq!(EpisodeHistory::parse(&bytes), None);
        // A count that runs past the stream.
        let mut bytes = history(&guids, &[]);
        bytes[EPISODE_COUNT_OFFSET..EPISODE_COUNT_OFFSET + 4]
            .copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(EpisodeHistory::parse(&bytes), None);
    }
}
