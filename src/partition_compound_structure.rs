//! Host types' layers and which side of a wall is its exterior (RE-53).
//!
//! # Layers
//!
//! A wall, floor, roof or ceiling type's own serialised data (its
//! element-data header and ElementId, [`crate::partition_names`]) carries
//! its compound structure: a `u32` layer count and one
//! [`LAYER_RECORD_LEN`]-byte record per layer, exterior (or top) first:
//!
//! ```text
//! +0   f64  width, feet (0 only on a membrane)
//! +8   u64  material ElementId, ff × 8 = by category
//! +16  u64  deck profile ElementId, ff × 8 = none
//! +24  u32  function: 1 structure, 2 substrate, 3 thermal / air,
//!           4 finish 1, 5 finish 2, 100 membrane, 200 structural deck
//! +28  9 bytes not read
//! ```
//!
//! On Revit 2023 (RE-112) a record is 29 bytes and its ids are `u32`
//! ([`layer_layout`]):
//!
//! ```text
//! +0   f64  width, feet
//! +8   u32  function
//! +12  u32  not read (ff × 4 or 0)
//! +16  u32  material ElementId, ff × 4 = by category
//! +20  u32  deck profile ElementId, ff × 4 = none
//! +24  5 bytes not read
//! ```
//!
//! The count is framed one of two ways ([`find_layers`]):
//! - `ff ff ff ff` and a per-release tag, `0x10a6` on Revit 2024 and
//!   `0x110e` on Revit 2025 and `0x106f` on Revit 2023
//!   ([`layer_frame_tag`]). On 2024 and 2023 this follows a wall type's
//!   name directly.
//! - the type's name (`u32 k`, `k` UTF-16 code units), then `u32 0`:
//!   Revit 2025 floors and ceilings, whose types on RE1 are named "-",
//!   Revit 2024 roofs (RE-56), and Revit 2023 floors and roofs.
//!
//! Against the materials Revit's own IFC4 export gives the same types:
//! - Snowdon Towers (2024): 42 of 42 types give Revit's constituent
//!   sequence, once the 0-width membranes Revit leaves out are dropped;
//! - Core Interior (2024): 4 of 4, all by category;
//! - RE1 Architecture (2025): the wall, both floors and the ceiling give
//!   Revit's layer widths and materials.
//!
//! # Wall orientation
//!
//! A wall's data carries, after `ff ff ff ff 01 00 00 00`, three `u32`: its
//! location line (0 to 5), a word of 0 to 2, and a flip flag
//! ([`wall_orientation`]). With the flag set, the wall's exterior (its first
//! layer) lies to the right of its location line's direction; clear, to
//! the left. On Snowdon Towers this holds on all 416 walls whose first and
//! last layers Revit's IFC4 export places measurably apart. The one other
//! wall's two outer layers are 0.03 ft apart, too close to tell.
//!
//! The wall's stored location line (RE-49's bounded line) is its
//! centreline whatever its location-line setting: Revit's IFC4 body spans
//! exactly half the type's thickness either side of it on 964 of Snowdon's
//! 1,054 walls, all of them with word 1 (RE-54).

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases these layouts are measured on.
pub const COMPOUND_STRUCTURE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Releases a wall's location line (RE-49's bounded line) is read on. On
/// 2025 it is measured by the same house saved in 2024 and 2025, whose 50
/// walls read the same line, orientation and layers from both (RE-55); on
/// 2023 by the walls of two projects, whose lines are Revit's axes before
/// joins (RE-114); on 2019 to 2022 by Autodesk's sample projects, whose
/// walls read the line their 2023 copies do (RE-178), and on 2016 to 2018 by
/// the same projects, whose walls read the line their 2019 copies do
/// (RE-179).
pub const WALL_LINE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[
    2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026, 2027,
];

/// Each wall's location line, by ElementId, on a release in
/// [`WALL_LINE_SUPPORTED_REVIT_VERSIONS`]; empty elsewhere.
pub fn scan_wall_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, crate::partition_beam_axes::BoundedLine>> {
    if !WALL_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Ok(BTreeMap::new());
    }
    crate::partition_beam_axes::scan_first_bounded_lines(rf, revit_version, walls)
}

/// Each curved wall's location arc, by ElementId, where its data's first
/// curve record is an arc (RE-75), on the releases [`scan_wall_lines`]
/// reads. Like the line, it is the wall's centreline: Revit's IFC4 body lies
/// half the type's thickness either side of it on 21 of Snowdon Towers' 24
/// curved walls.
pub fn scan_wall_arcs(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, crate::partition_beam_axes::BoundedArc>> {
    if !WALL_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Ok(BTreeMap::new());
    }
    crate::partition_beam_axes::scan_first_bounded_arcs(rf, revit_version, walls)
}

/// Bytes per layer record, on Revit 2024 and 2025.
pub const LAYER_RECORD_LEN: usize = 37;

/// Where a layer record's fields are on a release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerLayout {
    /// Bytes per record.
    pub record_len: usize,
    /// The `u32` function.
    pub function_at: usize,
    /// The material ElementId.
    pub material_at: usize,
    /// The deck profile ElementId.
    pub deck_at: usize,
    /// Whether the ids are `u64` (Revit 2024 and later) rather than `u32`.
    pub wide_ids: bool,
}

/// The layer record layout of `revit_version`, where it is measured.
pub fn layer_layout(revit_version: u32) -> Option<LayerLayout> {
    match revit_version {
        // Revit 2014 to 2022 write 2023's records (RE-178, RE-179).
        2014..=2023 => Some(LayerLayout {
            record_len: 29,
            function_at: 8,
            material_at: 16,
            deck_at: 20,
            wide_ids: false,
        }),
        2024 | 2025 => Some(LayerLayout {
            record_len: LAYER_RECORD_LEN,
            function_at: 24,
            material_at: 8,
            deck_at: 16,
            wide_ids: true,
        }),
        // Revit 2026 adds a `u32` after the function, repeating it, so a
        // record is 41 bytes; the fields read keep their offsets (RE-124).
        // Revit 2027 writes the same records: Autodesk's 2026 and 2027
        // `rac_basic` hold byte-identical layer lists (RE-177).
        2026 | 2027 => Some(LayerLayout {
            record_len: LAYER_RECORD_LEN + 4,
            function_at: 24,
            material_at: 8,
            deck_at: 16,
            wide_ids: true,
        }),
        _ => None,
    }
}

/// Most layers a type is taken to have.
pub const MAX_LAYERS: usize = 32;

/// How far into a type's data the layers are looked for.
pub const LAYER_WINDOW: usize = 0x2000;

/// Longest type name, in UTF-16 code units, a name-framed layer count is
/// looked for behind.
pub const MAX_NAME_UNITS: usize = 256;

/// The bytes a wall's location line, a word and its flip flag follow.
pub const WALL_FLIP_ANCHOR: [u8; 8] = [0xff, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00];

/// The tag framing a type's layer count on `revit_version`: the tag of
/// `VerticalRegionsStructure` in the release's schema (RE-178).
pub fn layer_frame_tag(revit_version: u32) -> Option<[u8; 2]> {
    if let Some(tags) = crate::partition_element_records_2023::schema_tags(revit_version) {
        return Some(tags.vertical_regions_structure.to_le_bytes());
    }
    match revit_version {
        2024 => Some([0xa6, 0x10]),
        2025 => Some([0x0e, 0x11]),
        2026 => Some([0x65, 0x11]),
        2027 => Some([0xab, 0x11]),
        _ => None,
    }
}

/// One layer of a type's compound structure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompoundLayer {
    /// Feet.
    pub width_feet: f64,
    /// The layer's material; `None` where the layer takes its category's.
    pub material: Option<u32>,
    /// The layer's function (see the module docs).
    pub function: u32,
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// The layers whose count starts at `count_at`, or `None` unless every
/// record is one a type can have: a width under 10 ft that is positive (0
/// only on a membrane or a structural deck), a material in `materials` or by category, a deck
/// profile that is none or a declared id, and a known function.
pub fn layers_at(
    buf: &[u8],
    count_at: usize,
    layout: LayerLayout,
    materials: &BTreeSet<u32>,
    declared: &BTreeSet<u32>,
) -> Option<Vec<CompoundLayer>> {
    let id_at = |at: usize| -> Option<u64> {
        if layout.wide_ids {
            u64_at(buf, at)
        } else {
            u32_at(buf, at).map(|id| match id {
                u32::MAX => u64::MAX,
                id => u64::from(id),
            })
        }
    };
    let count = usize::try_from(u32_at(buf, count_at)?).ok()?;
    if !(1..=MAX_LAYERS).contains(&count) {
        return None;
    }
    let mut layers = Vec::with_capacity(count);
    for index in 0..count {
        let at = count_at + 4 + layout.record_len * index;
        let width = f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?);
        let material = id_at(at + layout.material_at)?;
        let deck = id_at(at + layout.deck_at)?;
        let function = u32_at(buf, at + layout.function_at)?;
        // A membrane has no width, and nor has a structural deck whose
        // profile is carried by the layer above it (RE-112).
        let widthless = matches!(function, 100 | 200);
        let width_ok =
            width.is_finite() && width < 10.0 && (width >= 1e-3 || (widthless && width == 0.0));
        let material = match material {
            u64::MAX => None,
            id => Some(u32::try_from(id).ok().filter(|id| materials.contains(id))?),
        };
        let deck_ok = deck == u64::MAX
            || u32::try_from(deck)
                .ok()
                .is_some_and(|id| declared.contains(&id));
        if !width_ok || !deck_ok || !matches!(function, 0..=5 | 100 | 200) {
            return None;
        }
        layers.push(CompoundLayer {
            width_feet: width,
            material,
            function,
        });
    }
    Some(layers)
}

/// Whether the layer count at `count_at` is framed as a type's layers are.
fn framed(buf: &[u8], count_at: usize, tag: [u8; 2]) -> bool {
    let tagged = count_at >= 6
        && buf.get(count_at - 6..count_at - 2) == Some(&[0xff; 4][..])
        && buf.get(count_at - 2..count_at) == Some(&tag[..]);
    if tagged {
        return true;
    }
    // The type's name (u32 k, k UTF-16 code units), u32 0.
    if count_at < 4 || u32_at(buf, count_at - 4) != Some(0) {
        return false;
    }
    (1..=MAX_NAME_UNITS).any(|k: usize| {
        let Some(start) = count_at.checked_sub(4 + 2 * k + 4) else {
            return false;
        };
        u32_at(buf, start) == Some(k as u32)
            && (0..k).all(|i| {
                buf.get(start + 4 + 2 * i..start + 6 + 2 * i)
                    .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
                    .is_some_and(|unit| unit >= 0x20)
            })
    })
}

/// The first framed layer list in `data`.
pub fn find_layers(
    data: &[u8],
    tag: [u8; 2],
    layout: LayerLayout,
    materials: &BTreeSet<u32>,
    declared: &BTreeSet<u32>,
) -> Option<Vec<CompoundLayer>> {
    (6..data.len().saturating_sub(4))
        .filter(|&at| framed(data, at, tag))
        .find_map(|at| layers_at(data, at, layout, materials, declared))
}

/// The layers of each type in `types`, by ElementId, from each type's own
/// data. A type whose copies disagree is dropped; empty for a release the
/// layout is not measured on.
pub fn scan_type_layers(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
    materials: &BTreeSet<u32>,
    declared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, Vec<CompoundLayer>>> {
    // Revit 2014 to 2023's data header frames a `u32` id (RE-111, RE-178).
    let header =
        crate::partition_names::element_data_layout(revit_version).map(|layout| layout.header);
    let (Some(header), Some(tag), Some(layout)) = (
        header,
        layer_frame_tag(revit_version),
        layer_layout(revit_version),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<Vec<CompoundLayer>>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let id = if layout.wide_ids {
                u64_at(buf, id_at).and_then(|id| u32::try_from(id).ok())
            } else {
                u32_at(buf, id_at)
            };
            let Some(id) = id.filter(|id| types.contains(id)) else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(LAYER_WINDOW))
                .min(buf.len());
            let Some(layers) = buf
                .get(id_at + 8..end)
                .and_then(|data| find_layers(data, tag, layout, materials, declared))
            else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(layers));
                }
                Some(held) => {
                    if held.as_ref() != Some(&layers) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, layers)| layers.map(|l| (id, l)))
        .collect())
}

/// What a wall's data says about its location line and its sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WallOrientation {
    /// Revit's location-line setting, 0 to 5. The stored line is the
    /// wall's centreline whatever it is (RE-54).
    pub location_line: u32,
    /// A word of 0 to 2. Every Snowdon Towers wall whose body is centred on
    /// its line at its type's thickness carries 1 (RE-54). Revit 2014 to
    /// 2020 do not write it, and it reads 0 there (RE-178, RE-179).
    pub word: u32,
    /// Set puts the wall's exterior to the right of its location line's
    /// direction; clear, to the left.
    pub flip: bool,
}

/// A wall's orientation, from its data. `None` without the anchor or with
/// values a wall does not take.
///
/// The flip is one byte. The byte after it is a second flag, 0 followed by
/// `00 00` or 1 followed by `ff ff` (what it sets is not measured): 35 of
/// the Revit 2026 house's 59 walls and 4 of Core Interior's 360 set it
/// (RE-126).
pub fn wall_orientation(data: &[u8]) -> Option<WallOrientation> {
    wall_orientation_with(data, true)
}

/// Whether a wall's data on `revit_version` holds
/// [`WallOrientation::word`]. Revit 2019 and 2020 write the flip right
/// after the location line: Autodesk's `rst_basic` wall 627064 holds `03 00
/// 00 00 · 00 01 ff ff` past the anchor on 2019 and 2020 and `03 00 00 00 ·
/// 01 00 00 00 · 00 01 ff ff` on 2021 to 2023 (RE-178). Revit 2014 to 2018
/// write it as 2019 does: `rac_basic` of 2017 holds its 2019 copy's bytes
/// past every wall's anchor (RE-179).
pub fn wall_orientation_has_word(revit_version: u32) -> bool {
    !(2014..=2020).contains(&revit_version)
}

/// [`wall_orientation`], for data with or without the word
/// ([`wall_orientation_has_word`]); without it, the word reads 0.
pub fn wall_orientation_with(data: &[u8], has_word: bool) -> Option<WallOrientation> {
    let at = memchr::memmem::find(data, &WALL_FLIP_ANCHOR)? + WALL_FLIP_ANCHOR.len();
    let location_line = u32_at(data, at)?;
    let (word, flip_at) = if has_word {
        (u32_at(data, at + 4)?, at + 8)
    } else {
        (0, at + 4)
    };
    let flip = match data.get(flip_at..flip_at + 4)? {
        [flip @ (0 | 1), 0, 0, 0] | [flip @ (0 | 1), 1, 0xff, 0xff] => *flip == 1,
        _ => return None,
    };
    (location_line <= 5 && word <= 2).then_some(WallOrientation {
        location_line,
        word,
        flip,
    })
}

/// Each wall's orientation, by ElementId, from its own data; a wall whose
/// copies disagree is dropped.
pub fn scan_wall_orientations(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, WallOrientation>> {
    let Some(layout) = crate::partition_names::element_data_layout(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let header = layout.header;
    let has_word = wall_orientation_has_word(revit_version);
    let mut found: BTreeMap<u32, Option<WallOrientation>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| walls.contains(id)) else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(LAYER_WINDOW))
                .min(buf.len());
            let Some(orientation) = buf
                .get(id_at + 8..end)
                .and_then(|data| wall_orientation_with(data, has_word))
            else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(orientation));
                }
                Some(held) => {
                    if *held != Some(orientation) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, orientation)| orientation.map(|o| (id, o)))
        .collect())
}

/// The one tag whose lists name walls, across every wall's copies; `None`
/// where there is none or more than one (RE-70).
fn join_list_tag(found: &BTreeMap<u32, Option<WallJoinLists>>) -> Option<u32> {
    let tags: BTreeSet<u32> = found
        .values()
        .flatten()
        .flatten()
        .map(|(tag, _)| *tag)
        .collect();
    let [tag] = tags.into_iter().collect::<Vec<_>>()[..] else {
        return None;
    };
    Some(tag)
}

/// The word that opens a wall's joined-wall list (RE-127).
pub const WALL_JOINED_LIST_WORD: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// A joined-wall list's tag is its document's join-list tag plus this:
/// `0x0b9f7f` beside `0x0b9f7d` on Core Interior, RE1 and the Revit 2026
/// house (RE-127).
pub const WALL_JOINED_LIST_TAG_STEP: u32 = 2;

/// Most `u32`s a joined-wall entry is taken to carry.
pub const MAX_WALL_JOINED_WORDS: usize = 256;

/// A wall's joined-wall entries: each wall it names, with the `u32`s its
/// entry carries.
pub type WallJoinedEntries = BTreeMap<u32, Vec<u32>>;

/// The entries of every joined-wall list in a wall's `data` (RE-127).
///
/// A list is `01 00 00 00 · u32 tag · 02 00 00 00`, a zero `u32` from Revit
/// 2025 ([`wall_join_count_offset`]), a `u32` count and that many entries
/// `u64 ElementId · u32 n · n × u32`, every ElementId a wall in `walls`. A
/// wall named in two lists with different words is left out.
pub fn wall_joined_entries(
    data: &[u8],
    count_at: usize,
    tag: u32,
    walls: &BTreeSet<u32>,
) -> WallJoinedEntries {
    let mut out: BTreeMap<u32, Option<Vec<u32>>> = BTreeMap::new();
    for at in memchr::memmem::find_iter(data, &WALL_JOINED_LIST_WORD) {
        if u32_at(data, at + 4) != Some(tag)
            || u32_at(data, at + 8) != Some(2)
            || (count_at > 12 && u32_at(data, at + 12) != Some(0))
        {
            continue;
        }
        let Some(count) = u32_at(data, at + count_at).map(|count| count as usize) else {
            continue;
        };
        if count == 0 || count > MAX_WALL_JOIN_ENTRIES {
            continue;
        }
        let mut cursor = at + count_at + 4;
        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let Some(id) = u64_at(data, cursor)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| walls.contains(id))
            else {
                break;
            };
            let Some(n) = u32_at(data, cursor + 8)
                .map(|n| n as usize)
                .filter(|n| *n <= MAX_WALL_JOINED_WORDS)
            else {
                break;
            };
            let Some(words) = (0..n)
                .map(|index| u32_at(data, cursor + 12 + 4 * index))
                .collect::<Option<Vec<u32>>>()
            else {
                break;
            };
            entries.push((id, words));
            cursor += 12 + 4 * n;
        }
        if entries.len() != count {
            continue;
        }
        for (id, words) in entries {
            match out.get_mut(&id) {
                None => {
                    out.insert(id, Some(words));
                }
                Some(held) => {
                    if held.as_ref() != Some(&words) {
                        *held = None;
                    }
                }
            }
        }
    }
    out.into_iter()
        .filter_map(|(id, words)| words.map(|w| (id, w)))
        .collect()
}

/// Each wall in `read`'s joined-wall entries, by ElementId (RE-127), in the
/// lists whose tag follows the document's join-list tag
/// ([`WALL_JOINED_LIST_TAG_STEP`]). A wall whose copies disagree is left
/// out, and so is every wall in a file whose join-list tag is not read.
pub fn scan_wall_joined_entries(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &BTreeSet<u32>,
    read: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, WallJoinedEntries>> {
    let (Some(header), Some(count_at)) = (
        crate::partition_names::element_data_header(revit_version),
        wall_join_count_offset(revit_version),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut lists: BTreeMap<u32, Option<WallJoinLists>> = BTreeMap::new();
    // Each copy of a wall's data: its partition and byte range.
    let mut copies: Vec<(u32, String, std::ops::Range<usize>)> = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| read.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(WALL_JOIN_WINDOW))
                .min(buf.len());
            let Some(data) = buf.get(id_at + 8..end) else {
                continue;
            };
            let found = wall_join_lists(data, count_at, walls);
            match lists.get_mut(&id) {
                None => {
                    lists.insert(id, Some(found));
                }
                Some(held) => {
                    if held.as_ref() != Some(&found) {
                        *held = None;
                    }
                }
            }
            copies.push((id, stream.clone(), id_at + 8..end));
        }
    }
    let Some(tag) = join_list_tag(&lists) else {
        return Ok(BTreeMap::new());
    };
    let tag = tag.wrapping_add(WALL_JOINED_LIST_TAG_STEP);
    let mut found: BTreeMap<u32, Option<WallJoinedEntries>> = BTreeMap::new();
    for (id, stream, range) in copies {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let Some(data) = inflated.bytes().get(range) else {
            continue;
        };
        let entries = wall_joined_entries(data, count_at, tag, walls);
        match found.get_mut(&id) {
            None => {
                found.insert(id, Some(entries));
            }
            Some(held) => {
                if held.as_ref() != Some(&entries) {
                    *held = None;
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, entries)| entries.map(|e| (id, e)))
        .collect())
}

/// Releases a wall type's face angles are read on (RE-86). Only Snowdon
/// Towers (Revit 2024) stores them; no 2025 file measured holds the record.
pub const WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// The bytes a wall type's three face angles follow (RE-86): `u32 2`, eight
/// zero bytes, an `f64` 0.7 and a count of 3. The word before them varies
/// by document.
pub const WALL_FACE_ANGLES_FRAME: [u8; 24] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, 0x66, 0x66, 0x66,
    0x66, 0x66, 0xe6, 0x3f, 0x03, 0x00, 0x00, 0x00,
];

/// The three `f64` a wall type's data stores after
/// [`WALL_FACE_ANGLES_FRAME`], radians. On Snowdon Towers all three are 0
/// on 40 of its 43 exported wall types. The three "Solar Wall" types store
/// 10 degrees in the middle one, and each of their 21 exported walls is
/// tapered: its exterior face leans 10.000 degrees from vertical, wider at
/// the base, and its interior face is vertical (RE-86). What the first and
/// third angle set is not measured.
///
/// `None` without the frame, with it more than once, or with an angle that
/// is neither 0 nor between 1e-9 radians and a right angle either way (the
/// frame's bytes also occur by chance in other elements' data).
pub fn wall_type_face_angles(data: &[u8]) -> Option<[f64; 3]> {
    let mut frames = memchr::memmem::find_iter(data, &WALL_FACE_ANGLES_FRAME);
    let at = frames.next()? + WALL_FACE_ANGLES_FRAME.len();
    if frames.next().is_some() {
        return None;
    }
    let angle = |index: usize| {
        data.get(at + 8 * index..at + 8 * index + 8)
            .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")))
            .filter(|value| {
                *value == 0.0 || (1e-9..std::f64::consts::FRAC_PI_2).contains(&value.abs())
            })
    };
    Some([angle(0)?, angle(1)?, angle(2)?])
}

/// Each type in `types` whose data stores [`wall_type_face_angles`], by
/// ElementId; a type whose copies disagree is dropped. Empty on a release
/// outside [`WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS`].
pub fn scan_wall_type_face_angles(
    rf: &mut RevitFile,
    revit_version: u32,
    types: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, [f64; 3]>> {
    let Some(header) = crate::partition_names::element_data_header(revit_version)
        .filter(|_| WALL_FACE_ANGLES_SUPPORTED_REVIT_VERSIONS.contains(&revit_version))
    else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<[f64; 3]>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| types.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(LAYER_WINDOW))
                .min(buf.len());
            let Some(angles) = buf.get(id_at + 8..end).and_then(wall_type_face_angles) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(angles));
                }
                Some(held) => {
                    if *held != Some(angles) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, angles)| angles.map(|a| (id, a)))
        .collect())
}

/// Where a join list's count sits past its opening word on
/// `revit_version`: Revit 2025 puts a zero `u32` before it (RE-70).
/// `None` where the layout is not measured.
pub fn wall_join_count_offset(revit_version: u32) -> Option<usize> {
    match revit_version {
        2024 => Some(12),
        2025..=2027 => Some(16),
        _ => None,
    }
}

/// The word that opens a wall's join list (RE-70).
pub const WALL_JOIN_LIST_WORD: [u8; 4] = [0x07, 0x00, 0x00, 0x00];

/// Bytes per join-list entry: `u32 · u64 ElementId · u32`.
pub const WALL_JOIN_ENTRY_LEN: usize = 16;

/// Most entries a join list is taken to have.
pub const MAX_WALL_JOIN_ENTRIES: usize = 64;

/// How far into a wall's data its join lists are looked for. They sit up
/// to 12 KB in on Snowdon Towers.
pub const WALL_JOIN_WINDOW: usize = 0x1_0000;

/// A wall's join lists, each as its tag and the ElementIds it names.
pub type WallJoinLists = Vec<(u32, Vec<u32>)>;

/// Every join list in a wall's `data` that names at least one wall and
/// only walls in `walls`, as its tag and the ElementIds it names (RE-70).
///
/// A list is `07 00 00 00 · u32 tag · 02 00 00 00`, a zero `u32` on Revit
/// 2025 ([`wall_join_count_offset`]), a `u32` count and that many
/// [`WALL_JOIN_ENTRY_LEN`]-byte entries whose `u64` is a wall's ElementId.
pub fn wall_join_lists(
    data: &[u8],
    count_at: usize,
    walls: &BTreeSet<u32>,
) -> Vec<(u32, Vec<u32>)> {
    let mut out = Vec::new();
    for at in memchr::memmem::find_iter(data, &WALL_JOIN_LIST_WORD) {
        let (Some(tag), Some(2), Some(count)) = (
            u32_at(data, at + 4),
            u32_at(data, at + 8),
            u32_at(data, at + count_at),
        ) else {
            continue;
        };
        let count = count as usize;
        if count == 0
            || count > MAX_WALL_JOIN_ENTRIES
            || (count_at > 12 && u32_at(data, at + 12) != Some(0))
        {
            continue;
        }
        let named: Option<Vec<u32>> = (0..count)
            .map(|index| {
                u64_at(data, at + count_at + 8 + index * WALL_JOIN_ENTRY_LEN)
                    .and_then(|id| u32::try_from(id).ok())
                    .filter(|id| walls.contains(id))
            })
            .collect();
        if let Some(named) = named {
            out.push((tag, named));
        }
    }
    out
}

/// The walls each wall in `read` names in its join lists, by ElementId
/// (RE-70). `walls` is every recovered wall; a list naming anything else is
/// not a join list.
///
/// The lists' tag is one value per document (`0x0b9f7d` on Snowdon Towers,
/// Core Interior and RE1, `0x039f3d` on the MIT tutorial house) that no
/// schema class carries, so it is read off the data: the one tag whose
/// lists name walls. A file where two tags do gives nothing, and so does a
/// wall whose copies disagree.
pub fn scan_wall_join_partners(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &BTreeSet<u32>,
    read: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BTreeSet<u32>>> {
    let (Some(header), Some(count_at)) = (
        crate::partition_names::element_data_header(revit_version),
        wall_join_count_offset(revit_version),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<WallJoinLists>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = u64_at(buf, id_at)
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| read.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(WALL_JOIN_WINDOW))
                .min(buf.len());
            let Some(data) = buf.get(id_at + 8..end) else {
                continue;
            };
            let lists = wall_join_lists(data, count_at, walls);
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(lists));
                }
                Some(held) => {
                    if held.as_ref() != Some(&lists) {
                        *held = None;
                    }
                }
            }
        }
    }
    let Some(tag) = join_list_tag(&found) else {
        return Ok(BTreeMap::new());
    };
    Ok(found
        .into_iter()
        .filter_map(|(id, lists)| {
            let named: BTreeSet<u32> = lists?
                .into_iter()
                .filter(|(list_tag, _)| *list_tag == tag)
                .flat_map(|(_, named)| named)
                .filter(|other| *other != id)
                .collect();
            Some((id, named))
        })
        .collect())
}
