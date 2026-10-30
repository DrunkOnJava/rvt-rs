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
//!   ones (Revit 2023, RE-108) alike: right after the record's first id on
//!   40 bytes, after its two ids on 28. The element's episode GUID is entry
//!   `N - 1 - episode` of the list.
//!
//! Measured against Revit's own IFC exports, every element rvt-rs exports
//! that Revit's export also holds gets the GlobalId Revit gives it:
//! 854 of 854 on `2024_Core_Interior.rvt`, 281 of 281 on the RE1 models, 10
//! of 10 on `teste_export_2025` and `Projeto1`, 5,945 of 5,945 on Snowdon
//! Towers, and on Revit 2023 45 of 45 on `Exemplo_data` and 36 of 37 on
//! `modelo_bim` (RE-108). Revit also exports parts it synthesises (stair components, a
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
/// Bytes per episode entry: a GUID and the byte [`EPISODE_TERMINATOR`].
pub const EPISODE_ENTRY_LEN: usize = 17;
/// The byte that closes every episode entry.
pub const EPISODE_TERMINATOR: u8 = 0x28;
/// Offset of the episode number in a 40-byte `Global/ElemTable` record,
/// counted from the record's start at `0x06` (#152). It read `0x18` while
/// records were framed from `0x1E`, 16 bytes before each record's start.
pub const RECORD_EPISODE_OFFSET: usize = 8;
/// Offset of the episode number in a 28-byte `Global/ElemTable` record
/// (Revit 2023, RE-108), after the record's two ids. It read `0x0c` while
/// records were framed from `0x1E`, 4 bytes before each record's start.
pub const RECORD_EPISODE_OFFSET_28: usize = 8;

/// The episode GUIDs of a file, in stored order (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeHistory {
    guids: Vec<[u8; 16]>,
}

impl EpisodeHistory {
    /// Parse a decompressed `Global/History`, or `None` when its counts
    /// disagree, a count runs past the stream, or an entry is not closed by
    /// [`EPISODE_TERMINATOR`].
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
            if entry[16] != EPISODE_TERMINATOR {
                return None;
            }
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
/// 28 (Revit 2023 and earlier, measured on 2023) or its history does not
/// parse.
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
