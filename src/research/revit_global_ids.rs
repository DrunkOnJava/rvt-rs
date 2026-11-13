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
//! - A `Global/ElemTable` record holds the element's episode number at
//!   `+8`, on the 40-byte records (Revit 2024 and later) and on the 28-byte
//!   ones (Revit 2015 to 2023: RE-108 on 2023, RE-459 on 2018 to 2021) alike:
//!   right after the record's first id on
//!   40 bytes, after its two ids on 28. The element's episode GUID is entry
//!   `N - 1 - episode` of the list.
//!
//! Measured against Revit's own IFC exports, every element rvt-rs exports
//! that Revit's export also holds gets the GlobalId Revit gives it:
//! 854 of 854 on `2024_Core_Interior.rvt`, 281 of 281 on the RE1 models, 10
//! of 10 on `teste_export_2025` and `Projeto1`, 5,945 of 5,945 on Snowdon
//! Towers, and on Revit 2023 45 of 45 on `Exemplo_data` and 36 of 37 on
//! `modelo_bim` (RE-108). On the 28-byte tables of earlier releases it is
//! Revit's on 434 of 435 elements of Autodesk's 2018 sample and on three
//! held-out files of Revit 2020 and 2021 (145 of 151, 181 of 181 and 1,696
//! of 1,698, counts only), and it gives none on two Revit 2018 projects (0 of
//! 1,314 and 0 of 1,441) and on BIMcollab's Revit 2015 example (0 of 241,
//! RE-208), whose ids are not episodes (RE-459). The storeys, project, site
//! and building of a file made before 2023 are not Revit's unless the file
//! stores them (RE-220). Revit also exports parts it synthesises (stair components, a
//! ramp's flight), with GlobalIds of its own making. rvt-rs does not write
//! those parts. The ElementId XORed in is the
//! element's first one: an element whose id has since changed (RE-41) keeps
//! the id it was created with in its UniqueId.

use crate::le_bytes;
use crate::{Result, RevitFile, compression, elem_table, streams};
use std::collections::{BTreeMap, BTreeSet};

/// Offset of the episode count in `Global/History`.
pub const EPISODE_COUNT_OFFSET: usize = 0x0e;
/// Offset of the `u32`-counted table that precedes the episode list.
pub const EPISODE_TABLE_OFFSET: usize = 0x66;
/// Bytes per episode entry: a GUID and one more byte, [`EPISODE_TERMINATOR`] on
/// nearly every entry.
pub const EPISODE_ENTRY_LEN: usize = 17;
/// The byte that closes an episode entry: every one of 1,127 entries of a
/// Revit 2018 project but four (RE-459), and every one of every other file read.
pub const EPISODE_TERMINATOR: u8 = 0x28;
/// Offset of the episode number in a 40-byte `Global/ElemTable` record,
/// counted from the record's start at `0x06` (#152). It read `0x18` while
/// records were framed from `0x1E`, 16 bytes before each record's start.
pub const RECORD_EPISODE_OFFSET: usize = 8;
/// Offset of the episode number in a 28-byte `Global/ElemTable` record
/// (Revit 2023, RE-108), after the record's two ids. It read `0x0c` while
/// records were framed from `0x1E`, 4 bytes before each record's start.
pub const RECORD_EPISODE_OFFSET_28: usize = 8;
/// Where `Global/History` holds the project's own GUID, the one every save of a
/// project keeps: three copies in a row from `0x16`, 16 zero bytes, and a fourth
/// that ends where the count at [`EPISODE_TABLE_OFFSET`] begins (RE-459).
pub const PROJECT_GUID_OFFSETS: [usize; 4] = [0x16, 0x26, 0x36, 0x56];
/// The releases whose project, building and site GlobalIds were measured to be the
/// project's GUID with a fake id XORed in: 63 of the 64 files of 2017 to 2027 that
/// have a Revit export of their own project (RE-459). Revit 2015 and 2016 have no
/// file to measure it on.
pub const PROJECT_GUID_RELEASES: std::ops::RangeFrom<u32> = 2017..;
/// The releases whose storeys were measured to be the project's GUID with the
/// Level's ElementId XORed in: 69 of 69 storeys on 11 of the 12 files of 2017 to 2022
/// with a Revit export of their own project. From 2023 they are 0 of 229 and each
/// Level's own episode's GUID is Revit's (RE-459).
pub const PROJECT_GUID_STOREY_RELEASES: std::ops::RangeInclusive<u32> = 2017..=2022;
/// The ElementId Revit's exporter XORs into the project's GUID for the IfcProject's
/// GlobalId, the IfcBuilding's and the IfcSite's: the fake ids of revit-ifc's
/// `GUIDUtil.ProjectLevelGUIDType` (RE-220, RE-459).
pub const PROJECT_ELEMENT_ID: i32 = -16;
/// See [`PROJECT_ELEMENT_ID`]: the building's.
pub const BUILDING_ELEMENT_ID: i32 = -15;
/// See [`PROJECT_ELEMENT_ID`]: the site's.
pub const SITE_ELEMENT_ID: i32 = -14;

/// The episode GUIDs of a file, in stored order (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeHistory {
    guids: Vec<[u8; 16]>,
}

impl EpisodeHistory {
    /// Parse a decompressed `Global/History`, or `None` when its counts
    /// disagree or a count runs past the stream. An entry's last byte is
    /// [`EPISODE_TERMINATOR`] on all but 4 of the 1,127 entries of a Revit 2018
    /// project (RE-459), so it is not required: refusing the history for it left
    /// every element of that project with a GlobalId derived from the document.
    pub fn parse(history: &[u8]) -> Option<Self> {
        let count = le_bytes::u32_at(history, EPISODE_COUNT_OFFSET)? as usize;
        let skipped = le_bytes::u32_at(history, EPISODE_TABLE_OFFSET)? as usize;
        let list_count_at = EPISODE_TABLE_OFFSET
            .checked_add(4)?
            .checked_add(skipped.checked_mul(4)?)?;
        if le_bytes::u32_at(history, list_count_at)? as usize != count {
            return None;
        }
        let start = list_count_at + 4;
        let end = start.checked_add(count.checked_mul(EPISODE_ENTRY_LEN)?)?;
        let list = history.get(start..end)?;
        let mut guids = Vec::with_capacity(count);
        for entry in list.chunks_exact(EPISODE_ENTRY_LEN) {
            let guid = entry.first_chunk::<16>()?;
            guids.push(*guid);
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

/// The project's own GUID in Windows byte order, from the header of a decompressed
/// `Global/History`: the same 16 bytes at each of [`PROJECT_GUID_OFFSETS`], not
/// all zero. `None` when the stream ends before the last copy, when a copy
/// differs, or when they are zero, so a header read wrongly gives no GlobalId
/// rather than a plausible one (RE-459).
pub fn project_guid(history: &[u8]) -> Option<[u8; 16]> {
    let copy =
        |at: usize| -> Option<[u8; 16]> { history.get(at..at.checked_add(16)?)?.try_into().ok() };
    let first = copy(PROJECT_GUID_OFFSETS[0])?;
    if first == [0u8; 16] {
        return None;
    }
    PROJECT_GUID_OFFSETS[1..]
        .iter()
        .all(|&at| copy(at) == Some(first))
        .then_some(first)
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

/// The digits of IFC's 22-character base-64 `GlobalId`, lowest value first.
const IFC_GUID_ALPHABET: &[u8; 64] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

/// A 128-bit value in IFC's 22-character base-64 `GlobalId` form.
pub fn compress_ifc_guid(value: [u8; 16]) -> String {
    let mut n = u128::from_be_bytes(value);
    let mut out = [0u8; 22];
    for slot in out.iter_mut().rev() {
        *slot = IFC_GUID_ALPHABET[(n & 63) as usize];
        n >>= 6;
    }
    out.iter().map(|&b| char::from(b)).collect()
}

/// A 22-character IFC `GlobalId` as the 128-bit value it writes, `None` when
/// it is not one.
pub fn expand_ifc_guid(global_id: &str) -> Option<[u8; 16]> {
    if global_id.len() != 22 {
        return None;
    }
    let mut n: u128 = 0;
    for byte in global_id.bytes() {
        let digit = IFC_GUID_ALPHABET.iter().position(|&a| a == byte)?;
        n = n.checked_mul(64)?.checked_add(digit as u128)?;
    }
    Some(n.to_be_bytes())
}

/// The GlobalId Revit's exporter makes from a hash key (revit-ifc
/// `GUIDUtil.GenerateIFCGuidFrom`): the MD5 of the key's UTF-8, read as a
/// .NET GUID (RE-167).
pub fn hashed_global_id(key: &str) -> String {
    use md5::{Digest, Md5};
    let digest: [u8; 16] = Md5::digest(key.as_bytes())
        .as_slice()
        .try_into()
        .expect("an MD5 digest is 16 bytes");
    compress_ifc_guid(canonical_guid(digest))
}

/// The GlobalId Revit's exporter gives sub-element `index` of the element
/// whose GlobalId is `global_id` (`ExporterIFCUtils.CreateSubElementGUID`):
/// the element's with `index` XORed, big-endian, into bytes 10 and 11 of the
/// GUID (RE-167: a column's type, index 2048 `InstanceAsType`, on all 164
/// columns of Core Interior that use their own geometry).
pub fn sub_element_global_id(global_id: &str, index: u16) -> Option<String> {
    let mut value = expand_ifc_guid(global_id)?;
    let [high, low] = index.to_be_bytes();
    value[10] ^= high;
    value[11] ^= low;
    Some(compress_ifc_guid(value))
}

/// The GlobalId Revit's exporter gives element `element_id` of an episode
/// whose GUID is `episode_guid` (Windows byte order).
pub fn revit_ifc_global_id(episode_guid: [u8; 16], element_id: u32) -> String {
    let mut value = canonical_guid(episode_guid);
    let low = u32::from_be_bytes([value[12], value[13], value[14], value[15]]) ^ element_id;
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

/// The project's own GUID of a file, in Windows byte order ([`project_guid`]), `None`
/// when its `Global/History` cannot be read or holds none.
pub fn project_guid_of(rf: &mut RevitFile) -> Option<[u8; 16]> {
    project_guid(&inflate_history(rf).ok()?)
}

/// Built-in parameter under which Revit's exporter stores an element's
/// IfcGUID (RE-208 §3).
pub const IFC_GUID_PARAMETER: i32 = -1_019_000;
/// Built-in parameter holding the project's stored IfcGUID (#878).
pub const IFC_PROJECT_GUID_PARAMETER: i32 = -1_019_002;
/// Built-in parameter holding the building's stored IfcGUID (#878).
pub const IFC_BUILDING_GUID_PARAMETER: i32 = -1_019_003;
/// Built-in parameter holding the site's stored IfcGUID (#878).
pub const IFC_SITE_GUID_PARAMETER: i32 = -1_019_004;
/// How far before a stored storey GlobalId its Level's id is looked for.
const LEVEL_ID_WINDOW: usize = 512;
/// The distance between the two copies of a Level's id before its stored
/// GlobalId, bytes.
const LEVEL_ID_REPEAT: usize = 58;

/// GlobalIds a file stores for its project, site and building, and for its
/// Levels, which Revit's export writes as they are (#878).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct StoredGlobalIds {
    pub project: Option<String>,
    pub site: Option<String>,
    pub building: Option<String>,
    /// By the Level's ElementId: its storey's GlobalId.
    pub levels: BTreeMap<u32, String>,
}

/// The 22-character GlobalId stored in UTF-16 at `at`, when it is one.
fn stored_global_id(buf: &[u8], at: usize) -> Option<String> {
    let units = buf.get(at..at.checked_add(44)?)?;
    let text: String = units
        .chunks_exact(2)
        .map(|unit| (unit[1] == 0).then_some(char::from(unit[0])))
        .collect::<Option<_>>()?;
    expand_ifc_guid(&text).map(|_| text)
}

/// The GlobalIds a file stores (#878), each read where a built-in
/// parameter's id and the length 22 precede a GlobalId in UTF-16:
///
/// - the project's, site's and building's under their own parameters, each
///   where the file holds one value; else, where values are stored under
///   no parameter id (`-1`), the one triple of them that Revit's export
///   derives from one GUID, the building's the project's with the last bit
///   flipped and the site's with the one before it (Core Interior, the
///   BIM4LCA office and the R2025 models all relate theirs so);
/// - a storey's under [`IFC_GUID_PARAMETER`], where its Level's id stands
///   twice, 58 bytes apart, in the 512 bytes before it, as
///   on BIMcollab's Revit 2015 example. A Level given two values gets none.
///
/// Measured on BIMcollab's Revit 2015 example (project, site, building and
/// 5 of 5 storeys) and both BIM4LCA office files (project, site and
/// building).
pub fn stored_global_ids(rf: &mut RevitFile, level_ids: &BTreeSet<u32>) -> StoredGlobalIds {
    let mut by_parameter: BTreeMap<i32, BTreeSet<String>> = BTreeMap::new();
    let mut levels: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    let parameters = [
        IFC_GUID_PARAMETER,
        IFC_PROJECT_GUID_PARAMETER,
        IFC_BUILDING_GUID_PARAMETER,
        IFC_SITE_GUID_PARAMETER,
        -1,
    ];
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for parameter in parameters {
            let mut needle = [0u8; 8];
            needle[..4].copy_from_slice(&parameter.to_le_bytes());
            needle[4..].copy_from_slice(&22u32.to_le_bytes());
            for hit in memchr::memmem::find_iter(buf, &needle) {
                let at = hit + needle.len();
                let Some(value) = stored_global_id(buf, at) else {
                    continue;
                };
                if parameter == IFC_GUID_PARAMETER {
                    let from = hit.saturating_sub(LEVEL_ID_WINDOW);
                    let level = (from..hit.saturating_sub(4))
                        .rev()
                        .filter_map(|p| Some((p, le_bytes::u32_at(buf, p)?)))
                        .find(|(p, id)| {
                            level_ids.contains(id)
                                && p.checked_sub(LEVEL_ID_REPEAT)
                                    .filter(|q| *q >= from)
                                    .and_then(|q| le_bytes::u32_at(buf, q))
                                    == Some(*id)
                        })
                        .map(|(_, id)| id);
                    if let Some(level) = level {
                        levels.entry(level).or_default().insert(value);
                    }
                } else {
                    by_parameter.entry(parameter).or_default().insert(value);
                }
            }
        }
    }
    let one = |parameter: i32| {
        by_parameter
            .get(&parameter)
            .and_then(|values| match values.len() {
                1 => values.iter().next().cloned(),
                _ => None,
            })
    };
    let mut out = StoredGlobalIds {
        project: one(IFC_PROJECT_GUID_PARAMETER),
        site: one(IFC_SITE_GUID_PARAMETER),
        building: one(IFC_BUILDING_GUID_PARAMETER),
        levels: levels
            .into_iter()
            .filter_map(|(id, values)| match values.len() {
                1 => Some((id, values.into_iter().next()?)),
                _ => None,
            })
            .collect(),
    };
    if out.project.is_none() && out.site.is_none() && out.building.is_none() {
        let untagged = by_parameter.get(&-1).cloned().unwrap_or_default();
        let flipped = |value: &str, bit: u8| {
            let mut bytes = expand_ifc_guid(value)?;
            bytes[15] ^= bit;
            Some(compress_ifc_guid(bytes))
        };
        let triples: Vec<(String, String, String)> = untagged
            .iter()
            .filter_map(|project| {
                let building = flipped(project, 1).filter(|v| untagged.contains(v))?;
                let site = flipped(project, 2).filter(|v| untagged.contains(v))?;
                Some((project.clone(), site, building))
            })
            .collect();
        if let [(project, site, building)] = triples.as_slice() {
            out.project = Some(project.clone());
            out.site = Some(site.clone());
            out.building = Some(building.clone());
        }
    }
    out
}

/// The GlobalId Revit's exporter gives each element `Global/ElemTable`
/// declares, by ElementId (both of a record's ids). The ElementId XORed in
/// is the record's first id: on Snowdon Towers the 37 elements whose
/// ElementId differs from it (RE-41) all match Revit's only that way. Empty when the file's
/// ElemTable records are neither 40 bytes long (Revit 2024 and later) nor
/// 28 (Revit 2015 to 2023: equal to Revit's on files of 2018, 2020, 2021 and
/// 2023, and not on two 2018 projects and a 2015 example, RE-459) or its
/// history does not parse.
pub fn revit_global_ids(rf: &mut RevitFile) -> Result<BTreeMap<u32, String>> {
    let records = elem_table::parse_records(rf)?;
    let episode_offset = match records.first().map(|record| record.raw.len()) {
        Some(elem_table::RECORD_LEN_40) => RECORD_EPISODE_OFFSET,
        Some(28) => RECORD_EPISODE_OFFSET_28,
        _ => return Ok(BTreeMap::new()),
    };
    let Some(history) = EpisodeHistory::parse(&inflate_history(rf)?) else {
        return Ok(BTreeMap::new());
    };
    let mut out = BTreeMap::new();
    for record in &records {
        let Some(episode) = le_bytes::u32_at(&record.raw, episode_offset) else {
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

    /// A history as `EpisodeHistory::parse` reads it: the count at `0x0e`, a table
    /// of `table` words, the count again and the entries (a GUID and a last byte).
    fn history(
        count_at_head: u32,
        table: u32,
        count_in_list: u32,
        entries: &[([u8; 16], u8)],
    ) -> Vec<u8> {
        let mut out = vec![0u8; EPISODE_TABLE_OFFSET];
        out[EPISODE_COUNT_OFFSET..EPISODE_COUNT_OFFSET + 4]
            .copy_from_slice(&count_at_head.to_le_bytes());
        out.extend_from_slice(&table.to_le_bytes());
        for _ in 0..table.min(64) {
            out.extend_from_slice(&0u32.to_le_bytes());
        }
        out.extend_from_slice(&count_in_list.to_le_bytes());
        for (guid, last) in entries {
            out.extend_from_slice(guid);
            out.push(*last);
        }
        out
    }

    #[test]
    fn a_history_that_ends_early_or_disagrees_is_refused() {
        let entries = [
            ([1u8; 16], EPISODE_TERMINATOR),
            ([2u8; 16], EPISODE_TERMINATOR),
        ];
        assert!(EpisodeHistory::parse(&[]).is_none());
        assert!(EpisodeHistory::parse(&[0u8; EPISODE_TABLE_OFFSET]).is_none());
        // The count at the head and the count before the list differ.
        assert!(EpisodeHistory::parse(&history(2, 3, 3, &entries)).is_none());
        // The list holds fewer entries than it says.
        assert!(EpisodeHistory::parse(&history(3, 3, 3, &entries)).is_none());
        // A table length that would run past the stream, and one that overflows.
        assert!(EpisodeHistory::parse(&history(2, 1_000, 2, &entries)).is_none());
        assert!(EpisodeHistory::parse(&history(2, u32::MAX, 2, &entries)).is_none());
    }

    #[test]
    fn an_entry_closed_by_another_byte_is_read_like_the_rest() {
        let entries = [
            ([1u8; 16], EPISODE_TERMINATOR),
            ([2u8; 16], 0x2a),
            ([3u8; 16], EPISODE_TERMINATOR),
        ];
        let parsed = EpisodeHistory::parse(&history(3, 2, 3, &entries)).expect("parses");
        assert_eq!(parsed.len(), 3);
        // Episodes count from the last entry, whatever byte closes each one.
        assert_eq!(parsed.episode_guid(0), Some([3u8; 16]));
        assert_eq!(parsed.episode_guid(1), Some([2u8; 16]));
        assert_eq!(parsed.episode_guid(2), Some([1u8; 16]));
        assert_eq!(parsed.episode_guid(3), None);
    }

    /// A history header with `copies` at the project GUID's four offsets.
    fn header(copies: [[u8; 16]; 4]) -> Vec<u8> {
        let mut out = vec![0u8; EPISODE_TABLE_OFFSET];
        for (at, copy) in PROJECT_GUID_OFFSETS.iter().zip(copies) {
            out[*at..*at + 16].copy_from_slice(&copy);
        }
        out
    }

    #[test]
    fn a_header_read_wrongly_gives_no_project_guid() {
        let guid = [7u8; 16];
        // Too short for the last copy, with or without the other three.
        assert_eq!(project_guid(&[]), None);
        assert_eq!(
            project_guid(&header([guid; 4])[..PROJECT_GUID_OFFSETS[3] + 15]),
            None
        );
        // Any one copy that differs.
        for odd in [0, 1, 2, 3] {
            let mut copies = [guid; 4];
            copies[odd] = [8u8; 16];
            assert_eq!(project_guid(&header(copies)), None, "copy {odd} differs");
        }
        // All four equal and zero: an unset header, not a GUID.
        assert_eq!(project_guid(&header([[0u8; 16]; 4])), None);
    }

    #[test]
    fn four_equal_copies_are_the_project_guid() {
        let guid: [u8; 16] = std::array::from_fn(|i| i as u8 + 1);
        assert_eq!(project_guid(&header([guid; 4])), Some(guid));
        // Whatever follows the header is not read.
        let mut longer = header([guid; 4]);
        longer.extend_from_slice(&[0xff; 40]);
        assert_eq!(project_guid(&longer), Some(guid));
    }

    #[test]
    fn an_empty_list_has_no_episode() {
        let parsed = EpisodeHistory::parse(&history(0, 0, 0, &[])).expect("parses");
        assert!(parsed.is_empty());
        assert_eq!(parsed.episode_guid(0), None);
    }
}
