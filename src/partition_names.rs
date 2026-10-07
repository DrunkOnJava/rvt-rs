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

/// Longest name, in UTF-16 code units, a Revit 2023 name entry is read with
/// (RE-109).
pub const NAME_MAX_UNITS_2023: usize = 128;

/// Revit 2023 name entries (RE-109): `01 00 00 00 · u32 ElementId · u32 n
/// · UTF-16 × n`, with no category after the name as 2024 writes one. Only
/// a name of printable characters counts: the same shape also frames a
/// lone `U+FFFF`. The ElementIds of `declared` with exactly one such name.
pub fn name_entries_2023(rf: &mut RevitFile, declared: &BTreeSet<u32>) -> BTreeMap<u32, String> {
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let (Some(id), Some(units)) = (read_u32(buf, at + 4), read_u32(buf, at + 8)) else {
                continue;
            };
            let units = units as usize;
            if !(1..=NAME_MAX_UNITS_2023).contains(&units) || !declared.contains(&id) {
                continue;
            }
            let Some(raw) = buf.get(at + 12..at + 12 + 2 * units) else {
                continue;
            };
            let code: Vec<u16> = raw
                .chunks_exact(2)
                .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
                .collect();
            let Ok(name) = String::from_utf16(&code) else {
                continue;
            };
            let printable = name.chars().all(|c| {
                !c.is_control() && !('\u{e000}'..='\u{f8ff}').contains(&c) && c < '\u{fff0}'
            });
            if !printable {
                continue;
            }
            names
                .entry(id)
                .and_modify(|held| {
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                })
                .or_insert(Some(name));
        }
    }
    names
        .into_iter()
        .filter_map(|(id, name)| Some((id, name?)))
        .collect()
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

/// [`ELEMENT_DATA_HEADER`] on Revit 2026: the `u16` is `0x02fd`, the tag of
/// `CellList` in the file's own schema, as `0x02d3` and `0x02ef` are on 2024
/// and 2025 (RE-124).
pub const ELEMENT_DATA_HEADER_2026: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0xfd, 0x02, 0x01, 0x00, 0x00, 0x00];

/// [`ELEMENT_DATA_HEADER`] on Revit 2027: `0x0310`, `CellList`'s tag in the
/// 2027 schema (RE-177).
pub const ELEMENT_DATA_HEADER_2027: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0x10, 0x03, 0x01, 0x00, 0x00, 0x00];

/// The element-data header of `revit_version`, or `None` where it is not
/// measured (fail closed).
pub fn element_data_header(revit_version: u32) -> Option<[u8; 10]> {
    match revit_version {
        2024 => Some(ELEMENT_DATA_HEADER),
        2025 => Some(ELEMENT_DATA_HEADER_2025),
        2026 => Some(ELEMENT_DATA_HEADER_2026),
        2027 => Some(ELEMENT_DATA_HEADER_2027),
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
    element_data_names(buf, header, wanted, |buf, at| {
        read_u64(buf, at).and_then(|v| u32::try_from(v).ok())
    })
}

/// [`ELEMENT_DATA_HEADER`] on Revit 2023 (RE-111): the `u16` is `0x02c0`,
/// the schema tag of `CellList` on 2023 as `0x02d3` and `0x02ef` are on
/// 2024 and 2025, and the ElementId after it is a `u32` followed by four
/// bytes that are not zero.
pub const ELEMENT_DATA_HEADER_2023: [u8; 10] =
    [0xff, 0xff, 0xff, 0xff, 0xc0, 0x02, 0x01, 0x00, 0x00, 0x00];

/// How an element's serialised data opens on a release: its header and the
/// width of the ElementId after it (RE-111). On every release the data
/// proper starts 8 bytes past the id's first byte: a `u64` id, or a `u32`
/// id and four more bytes on Revit 2023.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementDataLayout {
    /// The header, ending in `01 00 00 00`.
    pub header: [u8; 10],
    /// Whether the id is a `u64` (Revit 2024 and later) rather than a `u32`.
    pub wide_id: bool,
}

impl ElementDataLayout {
    /// The ElementId at `at`, right after the header.
    pub fn id_at(&self, buf: &[u8], at: usize) -> Option<u32> {
        if self.wide_id {
            read_u64(buf, at).and_then(|id| u32::try_from(id).ok())
        } else {
            read_u32(buf, at)
        }
    }
}

/// The [`ElementDataLayout`] of `revit_version`, where it is measured.
pub fn element_data_layout(revit_version: u32) -> Option<ElementDataLayout> {
    match revit_version {
        2023 => Some(ElementDataLayout {
            header: ELEMENT_DATA_HEADER_2023,
            wide_id: false,
        }),
        _ => element_data_header(revit_version).map(|header| ElementDataLayout {
            header,
            wide_id: true,
        }),
    }
}

/// [`find_element_data_names`] on Revit 2023 (RE-111): the header is
/// [`ELEMENT_DATA_HEADER_2023`] and the ElementId a `u32`.
pub fn find_element_data_names_2023(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeMap<u32, String> {
    element_data_names(buf, &ELEMENT_DATA_HEADER_2023, wanted, read_u32)
}

fn element_data_names(
    buf: &[u8],
    header: &[u8; 10],
    wanted: &BTreeSet<u32>,
    read_id: impl Fn(&[u8], usize) -> Option<u32>,
) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for hit in memchr::memmem::find_iter(buf, header) {
        let id_at = hit + header.len();
        let Some(id) = read_id(buf, id_at) else {
            continue;
        };
        if !wanted.contains(&id) {
            continue;
        }
        let Some(name) = first_framed_name(buf, id_at + 8, header) else {
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
/// The class whose tag sits at [`TYPE_OBJECT_TAG_OFFSET`] of a railing
/// type's object: [`TYPE_OBJECT_TAG`] on Revit 2024, `0x103c` on RE1's 2025
/// schema (B45).
pub const TYPE_OBJECT_CLASS: &str = "SymbolInfo";
/// The class of the unset field frame that follows a railing type's name:
/// `0x020b` on Revit 2024, `0x0220` on RE1's 2025 schema (B45).
pub const RAILING_TYPE_NAME_END_CLASS: &str = "BalusterPattern";

/// The tags a railing type's object is found and its name ended by
/// ([`TYPE_OBJECT_CLASS`], [`RAILING_TYPE_NAME_END_CLASS`]). A class's tag
/// is its definition ordinal, which moves between releases, so both are
/// read from the file's own schema.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailingTypeTags {
    pub type_object: u16,
    pub name_end: u16,
}

impl RailingTypeTags {
    /// Both tags from a file's schema, `None` when it lacks either class.
    pub fn from_classes(classes: &crate::formats::SchemaClasses) -> Option<Self> {
        let tag = |name: &str| {
            classes
                .classes
                .iter()
                .find(|class| class.name == name)
                .map(|class| class.tag)
        };
        Some(Self {
            type_object: tag(TYPE_OBJECT_CLASS)?,
            name_end: tag(RAILING_TYPE_NAME_END_CLASS)?,
        })
    }

    /// The bytes that follow a railing type's name: the unset frame
    /// `ff ff ff ff · name_end`.
    fn name_end_frame(self) -> [u8; 6] {
        let [low, high] = self.name_end.to_le_bytes();
        [0xff, 0xff, 0xff, 0xff, low, high]
    }
}

/// The names railing types keep in their type objects (RE-66), by
/// ElementId, for the ids in `wanted`.
///
/// Most railing types also have element data that [`find_element_data_names`]
/// reads. Those that do not keep their name only in their type object
/// (`tags.type_object` at [`TYPE_OBJECT_TAG_OFFSET`]). The name is the one
/// `u32 n · n UTF-16 units` that ends where the frame of `tags.name_end`
/// first starts within [`RAILING_TYPE_NAME_WINDOW`]. What comes before it varies: on Autodesk's
/// Snowdon Towers 2024 sample most types put it at `+0x1ae`, and the two
/// that carry a Uniformat code parameter put it at `+0x1cb`. An id whose
/// occurrences give different names is dropped.
pub fn find_railing_type_names(
    buf: &[u8],
    wanted: &BTreeSet<u32>,
    tags: RailingTypeTags,
) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for (id, id_at) in crate::partition_id_objects::find_id_objects(buf, wanted) {
        let tag_at = id_at + TYPE_OBJECT_TAG_OFFSET;
        if buf.get(tag_at..tag_at + 2) != Some(&tags.type_object.to_le_bytes()[..]) {
            continue;
        }
        let Some(name) = railing_type_name_at(buf, id_at, tags) else {
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

/// Bytes from a pipe or duct type's ElementId to the second copy of it
/// before its name (RE-130).
pub const MEP_CURVE_TYPE_ID_REPEAT: usize = 56;

/// Bytes from a pipe or duct type's ElementId to its name's `u32` length
/// (RE-130); the UTF-16 name follows it.
pub const MEP_CURVE_TYPE_NAME_OFFSET: usize = 306;

/// The `u32 0` right before a pipe or duct type's name length (RE-177): on
/// all six of RE1 Plumbing's and Mechanical's type names it is zero, and on
/// the one place in Autodesk's 2027 `rme_basic` where an element's
/// reference lists repeat an id 56 bytes apart and four UTF-16 units follow
/// at [`MEP_CURVE_TYPE_NAME_OFFSET`], it is not.
pub const MEP_CURVE_TYPE_NAME_GUARD: [u8; 4] = [0; 4];

/// For each id in `wanted`, the pipe or duct type name stored after it
/// (RE-130): the ElementId, the same ElementId again
/// [`MEP_CURVE_TYPE_ID_REPEAT`] bytes on, and at
/// [`MEP_CURVE_TYPE_NAME_OFFSET`], after [`MEP_CURVE_TYPE_NAME_GUARD`], a
/// `u32 n` and `n` UTF-16 units. These
/// types have no name entry. On RE1 Mechanical and Plumbing (Revit 2025)
/// that places all five pipe and duct type names Revit's export gives. An
/// id whose occurrences give different names is dropped.
pub fn find_mep_curve_type_names(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeMap<u32, String> {
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    let (Some(&low), Some(&high)) = (wanted.first(), wanted.last()) else {
        return BTreeMap::new();
    };
    // B70: one pass, each offset first asked whether a `u64` there repeats
    // MEP_CURVE_TYPE_ID_REPEAT bytes on, rather than a search per id.
    let last = buf.len().saturating_sub(MEP_CURVE_TYPE_ID_REPEAT + 8);
    for at in 0..last {
        let value = read_u64(buf, at).expect("in bounds");
        if value < u64::from(low)
            || value > u64::from(high)
            || read_u64(buf, at + MEP_CURVE_TYPE_ID_REPEAT) != Some(value)
        {
            continue;
        }
        let id = value as u32;
        if !wanted.contains(&id) {
            continue;
        }
        let guard_at = at + MEP_CURVE_TYPE_NAME_OFFSET - MEP_CURVE_TYPE_NAME_GUARD.len();
        if buf.get(guard_at..guard_at + MEP_CURVE_TYPE_NAME_GUARD.len())
            != Some(&MEP_CURVE_TYPE_NAME_GUARD[..])
        {
            continue;
        }
        let Some(units) = read_u32(buf, at + MEP_CURVE_TYPE_NAME_OFFSET)
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| (1..=NAME_MAX_UNITS).contains(n))
        else {
            continue;
        };
        let start = at + MEP_CURVE_TYPE_NAME_OFFSET + 4;
        let Some(bytes) = buf.get(start..start + units * 2) else {
            continue;
        };
        let code_units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(name) = String::from_utf16(&code_units) else {
            continue;
        };
        if name.trim().is_empty() || name.chars().any(char::is_control) {
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

/// Where a curtain panel type's material follows its ElementId: the `u64`
/// right after its [`TYPE_OBJECT_TAG`] (#309). On Snowdon Towers the
/// "Glazed" and "Solid" system panel types name "Glass" and "Default", and
/// the "Empty" type of the "Empty System Panel" family names none (`ff` ×
/// 8).
pub const PANEL_TYPE_MATERIAL_OFFSET: usize = TYPE_OBJECT_TAG_OFFSET + 2;

/// For each id in `wanted` with a type object in `buf`, the material its
/// [`PANEL_TYPE_MATERIAL_OFFSET`] names, `None` where it is unset (#309).
pub fn find_panel_type_materials(buf: &[u8], wanted: &BTreeSet<u32>) -> BTreeMap<u32, Option<u64>> {
    crate::partition_id_objects::find_id_objects(buf, wanted)
        .into_iter()
        .filter_map(|(id, id_at)| {
            let tag_at = id_at + TYPE_OBJECT_TAG_OFFSET;
            (buf.get(tag_at..tag_at + 2) == Some(&TYPE_OBJECT_TAG[..])).then_some(())?;
            let raw = buf
                .get(id_at + PANEL_TYPE_MATERIAL_OFFSET..id_at + PANEL_TYPE_MATERIAL_OFFSET + 8)?;
            let material = u64::from_le_bytes(raw.try_into().ok()?);
            Some((id, (material != u64::MAX).then_some(material)))
        })
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

/// The one name that ends at the first name-end frame of the type object
/// whose ElementId starts at `id_at`.
fn railing_type_name_at(buf: &[u8], id_at: usize, tags: RailingTypeTags) -> Option<String> {
    let start = id_at.checked_add(RAILING_TYPE_NAME_WINDOW.start)?;
    let stop = id_at
        .checked_add(RAILING_TYPE_NAME_WINDOW.end)?
        .min(buf.len());
    let end = start + memchr::memmem::find(buf.get(start..stop)?, &tags.name_end_frame())?;
    name_ending_at(buf, end)
}

/// A railing type's name in its `StairsRailingAttr` data object (RE-153,
/// B45, Revit 2025): the one name that ends at the object's first name-end
/// frame, as in RE-66's type objects. RE1 Architecture's type 446543 holds
/// "-" there, at +571 of its 1,253-byte object.
pub fn railing_type_name_in(object: &[u8], tags: RailingTypeTags) -> Option<String> {
    let end = memchr::memmem::find(object, &tags.name_end_frame())?;
    name_ending_at(object, end)
}

/// The one `u32 n · n UTF-16 units` name that ends at `end` of `buf`.
fn name_ending_at(buf: &[u8], end: usize) -> Option<String> {
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
fn first_framed_name(buf: &[u8], start: usize, header: &[u8; 10]) -> Option<String> {
    let end = start
        .saturating_add(ELEMENT_DATA_NAME_WINDOW)
        .min(buf.len());
    let window = buf.get(start..end)?;
    for at in memchr::memmem::find_iter(window, &[0xff; 4]) {
        let at = start + at;
        let Some(tag) = buf.get(at + 4..at + 6) else {
            continue;
        };
        // Another element's data header frames its ElementId, not a name
        // (RE-111).
        if tag == [0xff, 0xff] || buf.get(at..at + header.len()) == Some(&header[..]) {
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
