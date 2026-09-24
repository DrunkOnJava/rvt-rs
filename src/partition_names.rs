//! Family and type names from partition name entries (RE-38).
//!
//! Revit 2024 and 2025 partitions carry a name entry for every loaded
//! family and family type:
//!
//! ```text
//! u64  ElementId (declared in Global/ElemTable)
//! u32  n, the name's length in UTF-16 code units
//! n*2  the name, UTF-16LE
//! i64  the element's BuiltInCategory
//! ```
//!
//! On Autodesk's Snowdon Towers 2024 architectural sample 1,104 ElementIds
//! carry one such entry each. Every one that the VIM export of the model
//! also lists has exactly the VIM's name and category (198 family types,
//! 132 families, 13 mullion types, 5 panel types); on the structural sample
//! 57 of 57 agree the same way.
//!
//! An element record's type is the one id in its reference list with a
//! name entry of the record's own category ([`resolve_type`]). The type's
//! own partition record (RE-35) names its family the same way
//! ([`resolve_family`]). `Family:Type:ElementId` is how Revit's own IFC
//! export names an element; it reproduces that name exactly on every
//! Snowdon instance where both joins are unique.

use crate::partition_element_records::{
    BUILTIN_CATEGORY_MAX, BUILTIN_CATEGORY_MIN, PartitionRecordSpan, bbox_marker,
    partition_record_chain, supports_revit_version,
};
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Longest name, in UTF-16 code units, a name entry is searched for.
pub const NAME_MAX_UNITS: usize = 256;

/// One partition name entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEntry {
    /// The ElementId the name belongs to.
    pub element_id: u32,
    /// The name as stored.
    pub name: String,
    /// The element's `BuiltInCategory`.
    pub builtin_category: i64,
}

/// Names by ElementId, with the partition record of each named element.
#[derive(Debug, Clone, Default)]
pub struct ElementNames {
    /// Every declared ElementId with exactly one name entry.
    pub entries: BTreeMap<u32, NameEntry>,
    /// Named ids that are family candidates of each named type: the named
    /// ids of the type's category found in the type's own partition record.
    pub type_family_candidates: BTreeMap<u32, BTreeSet<u32>>,
}

fn read_u32(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Every name entry in `buf` whose ElementId is declared.
///
/// Anchors on the `i64` category that closes an entry: its top five bytes
/// are `0xff` for any id in the `BuiltInCategory` band. From there it looks
/// back for a `u32` length that matches the code units before the anchor
/// and a declared `u64` ElementId before that. A name with a control
/// character, an unpaired surrogate or only whitespace is rejected.
pub fn find_name_entries(buf: &[u8], declared_ids: &BTreeSet<u32>) -> Vec<NameEntry> {
    let mut out = Vec::new();
    for hit in memchr::memmem::find_iter(buf, &[0xff; 5]) {
        let Some(at) = hit.checked_sub(3) else {
            continue;
        };
        let Some(category) = read_u64(buf, at).map(|v| v as i64) else {
            continue;
        };
        if !(BUILTIN_CATEGORY_MIN..=BUILTIN_CATEGORY_MAX).contains(&category) {
            continue;
        }
        for units in 1..=NAME_MAX_UNITS {
            let Some(start) = at.checked_sub(2 * units) else {
                break;
            };
            let Some(length_at) = start.checked_sub(4) else {
                break;
            };
            if read_u32(buf, length_at) != Some(units as u32) {
                continue;
            }
            let Some(id_at) = length_at.checked_sub(8) else {
                break;
            };
            let Some(element_id) = read_u64(buf, id_at).and_then(|v| u32::try_from(v).ok()) else {
                continue;
            };
            if !declared_ids.contains(&element_id) {
                continue;
            }
            let code_units: Vec<u16> = buf[start..at]
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            let Ok(name) = String::from_utf16(&code_units) else {
                continue;
            };
            if name.trim().is_empty() || name.chars().any(char::is_control) {
                continue;
            }
            out.push(NameEntry {
                element_id,
                name,
                builtin_category: category,
            });
            break;
        }
    }
    out
}

/// Keep one entry per ElementId, dropping any id whose entries disagree.
fn unique_entries(entries: impl IntoIterator<Item = NameEntry>) -> BTreeMap<u32, NameEntry> {
    let mut by_id: BTreeMap<u32, Option<NameEntry>> = BTreeMap::new();
    for entry in entries {
        match by_id.get_mut(&entry.element_id) {
            None => {
                by_id.insert(entry.element_id, Some(entry));
            }
            Some(slot) => {
                if slot.as_ref() != Some(&entry) {
                    *slot = None;
                }
            }
        }
    }
    by_id
        .into_iter()
        .filter_map(|(id, entry)| entry.map(|e| (id, e)))
        .collect()
}

/// Named ids of `category` other than `own` that appear as a `u64` at any
/// offset of `body`.
fn named_ids_in(
    body: &[u8],
    entries: &BTreeMap<u32, NameEntry>,
    category: i64,
    own: u32,
) -> BTreeSet<u32> {
    let mut out = BTreeSet::new();
    for at in 0..body.len().saturating_sub(7) {
        let Some(id) = read_u64(body, at).and_then(|v| u32::try_from(v).ok()) else {
            continue;
        };
        if id != own
            && entries
                .get(&id)
                .is_some_and(|e| e.builtin_category == category)
        {
            out.insert(id);
        }
    }
    out
}

/// Name entries and type-to-family candidates for a whole file (RE-38).
/// Empty where the release's record shape is not proven.
/// [`crate::RevitFile::element_names`] memoises it.
pub fn compute_element_names(rf: &mut RevitFile) -> Result<ElementNames> {
    let mut out = ElementNames::default();
    let version = rf.basic_file_info()?.version;
    let Some(marker) = bbox_marker(version).filter(|_| supports_revit_version(version)) else {
        return Ok(out);
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return Ok(out),
    };
    let mut found = Vec::new();
    let mut spans: Vec<(String, PartitionRecordSpan)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        found.extend(find_name_entries(buf, &declared));
        spans.extend(
            partition_record_chain(buf, &marker)
                .into_iter()
                .map(|span| (stream.clone(), span)),
        );
    }
    out.entries = unique_entries(found);
    for (stream, span) in spans {
        let Ok(type_id) = u32::try_from(span.element_id) else {
            continue;
        };
        let Some(entry) = out.entries.get(&type_id) else {
            continue;
        };
        let category = entry.builtin_category;
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let Some(body) = inflated.bytes().get(span.start..span.end) else {
            continue;
        };
        let candidates = named_ids_in(body, &out.entries, category, type_id);
        out.type_family_candidates
            .entry(type_id)
            .or_default()
            .extend(candidates);
    }
    Ok(out)
}

/// The bytes that open an element's serialised data in a Revit 2024
/// partition, followed by the element's `u64` ElementId (#322).
pub const ELEMENT_DATA_HEADER: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0xd3, 0x02, 0x01, 0x00, 0x00, 0x00];

/// [`ELEMENT_DATA_HEADER`] on Revit 2025: the `u16` is `0x02ef`, measured
/// on the RE1 projects (#322). It does not follow from the release
/// constant the way the bbox marker does, so each release is listed.
pub const ELEMENT_DATA_HEADER_2025: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0xef, 0x02, 0x01, 0x00, 0x00, 0x00];

/// The element-data header of `revit_version`, or `None` where it is not
/// measured (fail closed).
pub fn element_data_header(revit_version: u32) -> Option<[u8; 10]> {
    match revit_version {
        2024 => Some(ELEMENT_DATA_HEADER),
        2025 => Some(ELEMENT_DATA_HEADER_2025),
        _ => None,
    }
}

/// How far past an element's data header its name is searched for.
pub const ELEMENT_DATA_NAME_WINDOW: usize = 0x600;

/// The names system-family types (wall, floor, roof, ceiling, railing
/// types) give themselves in their serialised data, by ElementId (#322).
///
/// Such a type has no name entry (RE-38). Its data opens with `header`
/// ([`element_data_header`]) and the type's ElementId, and its name is the
/// first framed string after that: `ff ff ff ff`, a `u16` field tag (never
/// `0xffff`), a `u32` length in UTF-16 code units and the name. On 2024
/// wall types frame it with tag `0x1002` and floor and pad types with
/// `0x0151`; on 2025 with `0x106a` and `0x015c`. Only ids in `wanted` are
/// read, and an id whose occurrences give different names is dropped.
pub fn find_element_data_names(
    buf: &[u8],
    header: &[u8; 10],
    wanted: &BTreeSet<u32>,
) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for hit in memchr::memmem::find_iter(buf, header) {
        let id_at = hit + header.len();
        let Some(id) = read_u64(buf, id_at).and_then(|v| u32::try_from(v).ok()) else {
            continue;
        };
        if !wanted.contains(&id) {
            continue;
        }
        let Some(name) = first_framed_name(buf, id_at + 8) else {
            continue;
        };
        match found.get_mut(&id) {
            None => {
                found.insert(id, Some(name));
            }
            Some(slot) => {
                if slot.as_deref() != Some(name.as_str()) {
                    *slot = None;
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect()
}

/// The class tag at `+0x47` from the ElementId of the serialised object that
/// run, stair, landing, support and railing types share (RE-52, RE-65,
/// RE-66): `01 00 00 00 · u64 id`, often held inside another element's data.
pub const TYPE_OBJECT_TAG: [u8; 2] = [0xdb, 0x0f];
/// See [`TYPE_OBJECT_TAG`].
pub const TYPE_OBJECT_TAG_OFFSET: usize = 0x47;
/// Where a railing type's name is looked for in its type object (RE-66,
/// Revit 2024), as offsets from the ElementId.
pub const RAILING_TYPE_NAME_WINDOW: std::ops::Range<usize> = 0x100..0x300;
/// The bytes that follow a railing type's name: an unset field frame with
/// tag `0x020b`.
pub const RAILING_TYPE_NAME_END: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0x0b, 0x02];

/// The names railing types keep in their type objects (RE-66), by
/// ElementId, for the ids in `wanted`.
///
/// Most railing types also have element data that [`find_element_data_names`]
/// reads. Those that do not keep their name only in their type object
/// ([`TYPE_OBJECT_TAG`]). The name is the one `u32 n · n UTF-16 units` that
/// ends where [`RAILING_TYPE_NAME_END`] first starts within
/// [`RAILING_TYPE_NAME_WINDOW`]. What comes before it varies: on Autodesk's
/// Snowdon Towers 2024 sample most types put it at `+0x1ae`, and the two
/// that carry a Uniformat code parameter put it at `+0x1cb`. An id whose
/// occurrences give different names is dropped.
pub fn find_railing_type_names(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for (id, id_at) in crate::partition_id_objects::find_id_objects(buf, wanted) {
        let tag_at = id_at + TYPE_OBJECT_TAG_OFFSET;
        if buf.get(tag_at..tag_at + 2) != Some(&TYPE_OBJECT_TAG[..]) {
            continue;
        }
        let Some(name) = railing_type_name_at(buf, id_at) else {
            continue;
        };
        match found.get_mut(&id) {
            None => {
                found.insert(id, Some(name));
            }
            Some(slot) => {
                if slot.as_deref() != Some(name.as_str()) {
                    *slot = None;
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect()
}

/// The ids in `wanted` that have a type object ([`TYPE_OBJECT_TAG`]) in
/// `buf` (RE-69).
pub fn find_type_object_ids(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeSet<u32> {
    crate::partition_id_objects::find_id_objects(buf, wanted)
        .into_iter()
        .filter(|(_, id_at)| {
            let tag_at = id_at + TYPE_OBJECT_TAG_OFFSET;
            buf.get(tag_at..tag_at + 2) == Some(&TYPE_OBJECT_TAG[..])
        })
        .map(|(id, _)| id)
        .collect()
}

/// The frame tag of a type object's own name on Revit 2024 (RE-67), as
/// material names are framed (RE-58): `ff ff ff ff 49 01 · u32 n · UTF-16`.
pub const TYPE_NAME_FRAME_TAG: [u8; 2] = [0x49, 0x01];
/// The frame tag of a text type's font on Revit 2024 (RE-67):
/// `ff ff ff ff eb 07 · u32 n · UTF-16`, such as "Trebuchet MS".
pub const FONT_FRAME_TAG: [u8; 2] = [0xeb, 0x07];
/// Where a text type's name and font are looked for, as offsets from the
/// ElementId.
pub const TEXT_TYPE_WINDOW: std::ops::Range<usize> = 0x40..0x200;

/// The names of the text types among `wanted` (RE-67), by ElementId.
///
/// A text type, such as the one model text uses, is a type object
/// ([`TYPE_OBJECT_TAG`]) that frames its own name with
/// [`TYPE_NAME_FRAME_TAG`] and then its font with [`FONT_FRAME_TAG`], both
/// within [`TEXT_TYPE_WINDOW`]. A type object without a font is not read.
/// On Autodesk's Snowdon Towers 2024 sample, the two such types are the
/// types of all 7 model texts: "10\" Trebuchet MS" and "18\" Trebuchet MS",
/// both in Trebuchet MS. An id whose occurrences give different names is
/// dropped.
pub fn find_text_type_names(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for (id, id_at) in crate::partition_id_objects::find_id_objects(buf, wanted) {
        let tag_at = id_at + TYPE_OBJECT_TAG_OFFSET;
        if buf.get(tag_at..tag_at + 2) != Some(&TYPE_OBJECT_TAG[..]) {
            continue;
        }
        let start = id_at + TEXT_TYPE_WINDOW.start;
        let stop = (id_at + TEXT_TYPE_WINDOW.end).min(buf.len());
        let Some((name, name_end)) = framed_string(buf, start, stop, TYPE_NAME_FRAME_TAG) else {
            continue;
        };
        if framed_string(buf, name_end, stop, FONT_FRAME_TAG).is_none() {
            continue;
        }
        match found.get_mut(&id) {
            None => {
                found.insert(id, Some(name));
            }
            Some(slot) => {
                if slot.as_deref() != Some(name.as_str()) {
                    *slot = None;
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect()
}

/// The first `ff ff ff ff · tag · u32 n · n UTF-16 units` string that
/// starts in `buf[start..stop]`, and where it ends.
fn framed_string(buf: &[u8], start: usize, stop: usize, tag: [u8; 2]) -> Option<(String, usize)> {
    let frame = [0xff, 0xff, 0xff, 0xff, tag[0], tag[1]];
    let window = buf.get(start..stop)?;
    let at = start + memchr::memmem::find(window, &frame)? + frame.len();
    let units = read_u32(buf, at)? as usize;
    if !(1..=NAME_MAX_UNITS).contains(&units) {
        return None;
    }
    let end = at + 4 + 2 * units;
    let code_units: Vec<u16> = buf
        .get(at + 4..end)?
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let text = String::from_utf16(&code_units).ok()?;
    if text.trim().is_empty()
        || text
            .chars()
            .any(|c| c.is_control() || matches!(c, '\u{fffd}' | '\u{fffe}' | '\u{ffff}'))
    {
        return None;
    }
    Some((text, end))
}

/// The one name that ends at the first [`RAILING_TYPE_NAME_END`] of the
/// type object whose ElementId starts at `id_at`.
fn railing_type_name_at(buf: &[u8], id_at: usize) -> Option<String> {
    let start = id_at.checked_add(RAILING_TYPE_NAME_WINDOW.start)?;
    let stop = id_at
        .checked_add(RAILING_TYPE_NAME_WINDOW.end)?
        .min(buf.len());
    let end = start + memchr::memmem::find(buf.get(start..stop)?, &RAILING_TYPE_NAME_END)?;
    let mut names = (1..=NAME_MAX_UNITS).filter_map(|units| {
        let at = end.checked_sub(4 + 2 * units)?;
        if read_u32(buf, at)? as usize != units {
            return None;
        }
        let code_units: Vec<u16> = buf
            .get(at + 4..end)?
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let name = String::from_utf16(&code_units).ok()?;
        (!name.trim().is_empty()
            && !name
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{fffd}' | '\u{fffe}' | '\u{ffff}')))
        .then_some(name)
    });
    let name = names.next()?;
    names.next().is_none().then_some(name)
}

/// The first `ff ff ff ff · u16 tag · u32 n · n UTF-16 units` string that
/// starts in `buf[start..start + ELEMENT_DATA_NAME_WINDOW]`.
fn first_framed_name(buf: &[u8], start: usize) -> Option<String> {
    let end = start
        .saturating_add(ELEMENT_DATA_NAME_WINDOW)
        .min(buf.len());
    let window = buf.get(start..end)?;
    for at in memchr::memmem::find_iter(window, &[0xff; 4]) {
        let at = start + at;
        let Some(tag) = buf.get(at + 4..at + 6) else {
            continue;
        };
        if tag == [0xff, 0xff] {
            continue;
        }
        let Some(units) = read_u32(buf, at + 6).map(|n| n as usize) else {
            continue;
        };
        if !(1..=NAME_MAX_UNITS).contains(&units) {
            continue;
        }
        let Some(bytes) = buf.get(at + 10..at + 10 + units * 2) else {
            continue;
        };
        let code_units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(name) = String::from_utf16(&code_units) else {
            continue;
        };
        // U+FFFF / U+FFFE fill unset slots; they are never part of a name.
        if name.trim().is_empty()
            || name
                .chars()
                .any(|c| c.is_control() || matches!(c, '\u{fffd}' | '\u{fffe}' | '\u{ffff}'))
        {
            continue;
        }
        return Some(name);
    }
    None
}

/// The one id in `references`, other than `own`, that carries a name entry
/// of `category`: the element's type. `None` when there is none or more
/// than one.
pub fn resolve_type(
    names: &ElementNames,
    references: &[u64],
    category: i64,
    own: u32,
) -> Option<u32> {
    let mut found = references
        .iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|&id| id != own)
        .filter(|id| {
            names
                .entries
                .get(id)
                .is_some_and(|e| e.builtin_category == category)
        })
        .collect::<BTreeSet<u32>>()
        .into_iter();
    let first = found.next()?;
    found.next().is_none().then_some(first)
}

/// The type's family: the named id of the type's category in the type's
/// own partition record.
///
/// A type of a family that nests other families of its category names
/// those too (#324). Then the family is the one candidate whose own
/// partition record names every other candidate, the families nested in it
/// (RE-42). `None` when there is no candidate, or when no candidate, or
/// more than one, names all the others.
///
/// A family type names its family the way a family names a nested one, so
/// a type record that also named a sibling type would pick that type. No
/// such record occurs on the files measured: every name the rule adds on
/// RE1 Electrical and Snowdon Towers is Revit's own.
pub fn resolve_family(names: &ElementNames, type_id: u32) -> Option<u32> {
    let candidates = names.type_family_candidates.get(&type_id)?;
    let mut iter = candidates.iter();
    let first = *iter.next()?;
    if iter.next().is_none() {
        return Some(first);
    }
    let mut hosts = candidates.iter().copied().filter(|family| {
        names
            .type_family_candidates
            .get(family)
            .is_some_and(|nested| {
                candidates
                    .iter()
                    .all(|other| other == family || nested.contains(other))
            })
    });
    let host = hosts.next()?;
    hosts.next().is_none().then_some(host)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry_bytes(id: u32, name: &str, category: i64) -> Vec<u8> {
        let units: Vec<u16> = name.encode_utf16().collect();
        let mut out = u64::from(id).to_le_bytes().to_vec();
        out.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        out.extend_from_slice(&(category as u64).to_le_bytes());
        out
    }

    const PANELS: i64 = -2_000_170;

    /// #322: a wall type's name is the first framed string after its data
    /// header; an id outside `wanted`, or whose copies disagree, gives none.
    #[test]
    fn a_2025_type_name_skips_frames_whose_length_is_unset() {
        // Wall type 381192 on RE1-Architecture.rvt (Revit 2025): the id,
        // 0xff padding, two frames whose length is 0xffffffff, then the
        // name "-" framed with tag 0x106a.
        let mut buf = ELEMENT_DATA_HEADER_2025.to_vec();
        buf.extend_from_slice(&381_192u64.to_le_bytes());
        buf.extend_from_slice(&[0xff; 58]);
        buf.extend_from_slice(&[0x00, 0x00, 0x00]);
        buf.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x3c, 0x10, 0xff, 0xff, 0xff, 0xff]);
        buf.extend_from_slice(&[0xff; 4]);
        buf.extend_from_slice(&[0x09, 0xd1, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00]);
        buf.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x43, 0x03, 0xff, 0xff, 0xff, 0xff]);
        buf.extend_from_slice(&[0xff; 20]);
        buf.extend_from_slice(&[0xff, 0xff, 0xff, 0xff, 0x6a, 0x10, 0x01, 0x00, 0x00, 0x00]);
        buf.extend_from_slice(&[b'-', 0x00]);
        let wanted = BTreeSet::from([381_192]);
        let names = find_element_data_names(&buf, &ELEMENT_DATA_HEADER_2025, &wanted);
        assert_eq!(names.get(&381_192).map(String::as_str), Some("-"));
        assert!(find_element_data_names(&buf, &ELEMENT_DATA_HEADER, &wanted).is_empty());
        assert_eq!(element_data_header(2025), Some(ELEMENT_DATA_HEADER_2025));
        assert_eq!(element_data_header(2023), None);
    }

    #[test]
    fn a_system_type_name_follows_its_element_data_header() {
        let framed = |tag: u16, name: &str| {
            let units: Vec<u16> = name.encode_utf16().collect();
            let mut out = vec![0xff; 4];
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&(units.len() as u32).to_le_bytes());
            for unit in units {
                out.extend_from_slice(&unit.to_le_bytes());
            }
            out
        };
        let data = |id: u32, name: &str| {
            let mut out = ELEMENT_DATA_HEADER.to_vec();
            out.extend_from_slice(&u64::from(id).to_le_bytes());
            // Unset slots and a field that is not a name come first.
            out.extend_from_slice(&[0xff; 12]);
            out.extend_from_slice(&[0x00, 0x10]);
            out.extend(framed(0x1002, name));
            out
        };
        let mut buf = vec![0u8; 7];
        buf.extend(data(17328, "8\" Interior Partition 3 Hour"));
        buf.extend(data(22757, "Basement Slab"));
        let names =
            find_element_data_names(&buf, &ELEMENT_DATA_HEADER, &BTreeSet::from([17328, 22757]));
        assert_eq!(
            names.get(&17328).map(String::as_str),
            Some("8\" Interior Partition 3 Hour")
        );
        assert_eq!(names.get(&22757).map(String::as_str), Some("Basement Slab"));
        assert!(
            find_element_data_names(&buf, &ELEMENT_DATA_HEADER, &BTreeSet::from([1])).is_empty()
        );
        buf.extend(data(17328, "Another name"));
        let names = find_element_data_names(&buf, &ELEMENT_DATA_HEADER, &BTreeSet::from([17328]));
        assert!(names.is_empty(), "disagreeing copies give no name");
    }

    #[test]
    fn a_name_entry_decodes_between_filler() {
        let declared: BTreeSet<u32> = [21944, 21943].into_iter().collect();
        let buf = [
            vec![0xff; 20],
            entry_bytes(21944, "Empty", PANELS),
            vec![0u8; 7],
            entry_bytes(21943, "Empty System Panel", PANELS),
            vec![0xff; 9],
        ]
        .concat();
        let found = find_name_entries(&buf, &declared);
        let names: Vec<(u32, &str)> = found
            .iter()
            .map(|e| (e.element_id, e.name.as_str()))
            .collect();
        assert_eq!(names, vec![(21944, "Empty"), (21943, "Empty System Panel")]);
        assert!(found.iter().all(|e| e.builtin_category == PANELS));
    }

    #[test]
    fn an_undeclared_id_or_a_category_outside_the_band_is_not_an_entry() {
        let declared: BTreeSet<u32> = [7].into_iter().collect();
        let buf = [
            entry_bytes(8, "Undeclared", PANELS),
            entry_bytes(7, "Not a category", -5),
        ]
        .concat();
        assert!(find_name_entries(&buf, &declared).is_empty());
    }

    #[test]
    fn a_non_ascii_name_decodes_and_a_control_character_does_not() {
        let declared: BTreeSet<u32> = [1, 2].into_iter().collect();
        let buf = [
            entry_bytes(1, "Porta simples 0,90 × 2,10", -2_000_023),
            entry_bytes(2, "bad\u{7}name", -2_000_023),
        ]
        .concat();
        let found = find_name_entries(&buf, &declared);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Porta simples 0,90 × 2,10");
    }

    #[test]
    fn disagreeing_entries_for_one_id_are_dropped() {
        let a = NameEntry {
            element_id: 5,
            name: "A".into(),
            builtin_category: PANELS,
        };
        let b = NameEntry {
            name: "B".into(),
            ..a.clone()
        };
        let c = NameEntry {
            element_id: 6,
            ..a.clone()
        };
        let unique = unique_entries([a.clone(), a.clone(), b, c.clone()]);
        assert_eq!(unique.into_values().collect::<Vec<_>>(), vec![c]);
    }

    #[test]
    fn type_and_family_resolve_only_when_unique() {
        let entry = |id, name: &str, category| {
            (
                id,
                NameEntry {
                    element_id: id,
                    name: name.into(),
                    builtin_category: category,
                },
            )
        };
        let names = ElementNames {
            entries: BTreeMap::from([
                entry(100, "Single-Flush", -2_000_023),
                entry(101, "36\" x 84\"", -2_000_023),
                entry(102, "30\" x 84\"", -2_000_023),
                entry(103, "Bi-Fold", -2_000_023),
                entry(200, "Wall type", -2_000_011),
            ]),
            type_family_candidates: BTreeMap::from([
                (101, BTreeSet::from([100])),
                // Two families, neither naming the other (RE-42).
                (102, BTreeSet::from([100, 103])),
            ]),
        };
        // The door's list names its type 101 and a wall type; only the
        // same-category id counts.
        assert_eq!(
            resolve_type(&names, &[3, 101, 200, 500], -2_000_023, 500),
            Some(101)
        );
        // Two same-category candidates: no type.
        assert_eq!(
            resolve_type(&names, &[3, 101, 102, 500], -2_000_023, 500),
            None
        );
        assert_eq!(resolve_family(&names, 101), Some(100));
        assert_eq!(resolve_family(&names, 102), None);
    }

    /// RE-42: among several candidates, the family is the one whose own
    /// record names every other candidate. No such candidate, or two, give
    /// no family.
    #[test]
    fn a_nesting_family_is_told_from_the_families_nested_in_it() {
        let mut names = ElementNames::default();
        let set = |ids: &[u32]| ids.iter().copied().collect::<BTreeSet<u32>>();
        // Type 444100 names its family 755868 and the nested 51892;
        // 755868's own record names 51892.
        names
            .type_family_candidates
            .insert(444_100, set(&[51_892, 755_868]));
        names.type_family_candidates.insert(755_868, set(&[51_892]));
        assert_eq!(resolve_family(&names, 444_100), Some(755_868));
        // A single candidate needs no nesting evidence.
        names.type_family_candidates.insert(21_944, set(&[21_943]));
        assert_eq!(resolve_family(&names, 21_944), Some(21_943));
        // Neither candidate names the other: no family.
        names.type_family_candidates.insert(1, set(&[10, 20]));
        names.type_family_candidates.insert(10, set(&[30]));
        assert_eq!(resolve_family(&names, 1), None);
        // Both name each other: ambiguous, no family.
        names.type_family_candidates.insert(2, set(&[40, 50]));
        names.type_family_candidates.insert(40, set(&[50]));
        names.type_family_candidates.insert(50, set(&[40]));
        assert_eq!(resolve_family(&names, 2), None);
    }

    #[test]
    fn named_ids_are_found_at_any_offset_of_a_record_body() {
        let names = BTreeMap::from([(
            21943,
            NameEntry {
                element_id: 21943,
                name: "Empty System Panel".into(),
                builtin_category: PANELS,
            },
        )]);
        let mut body = vec![0xffu8; 3];
        body.extend_from_slice(&21943u64.to_le_bytes());
        body.extend_from_slice(&21944u64.to_le_bytes());
        assert_eq!(
            named_ids_in(&body, &names, PANELS, 21944),
            BTreeSet::from([21943])
        );
    }
}
