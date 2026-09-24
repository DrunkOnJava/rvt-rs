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
//! The count is framed one of two ways ([`find_layers`]):
//! - `ff ff ff ff` and a per-release tag, `0x10a6` on Revit 2024 and
//!   `0x110e` on Revit 2025 ([`layer_frame_tag`]). On 2024 this follows
//!   the type's name directly.
//! - the type's name (`u32 k`, `k` UTF-16 code units), then `u32 0`:
//!   Revit 2025 floors and ceilings, whose types on RE1 are named "-", and
//!   Revit 2024 roofs (RE-56).
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
pub const COMPOUND_STRUCTURE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

/// Releases a wall's location line (RE-49's bounded line) is read on. On
/// 2025 it is measured by the same house saved in 2024 and 2025, whose 50
/// walls read the same line, orientation and layers from both (RE-55).
pub const WALL_LINE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

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

/// Bytes per layer record.
pub const LAYER_RECORD_LEN: usize = 37;

/// Most layers a type is taken to have.
pub const MAX_LAYERS: usize = 32;

/// How far into a type's data the layers are looked for.
pub const LAYER_WINDOW: usize = 0x2000;

/// Longest type name, in UTF-16 code units, a name-framed layer count is
/// looked for behind.
pub const MAX_NAME_UNITS: usize = 256;

/// The bytes a wall's location line, a word and its flip flag follow.
pub const WALL_FLIP_ANCHOR: [u8; 8] = [0xff, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00];

/// The tag framing a type's layer count on `revit_version`.
pub fn layer_frame_tag(revit_version: u32) -> Option<[u8; 2]> {
    match revit_version {
        2024 => Some([0xa6, 0x10]),
        2025 => Some([0x0e, 0x11]),
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
/// only on a membrane), a material in `materials` or by category, a deck
/// profile that is none or a declared id, and a known function.
pub fn layers_at(
    buf: &[u8],
    count_at: usize,
    materials: &BTreeSet<u32>,
    declared: &BTreeSet<u32>,
) -> Option<Vec<CompoundLayer>> {
    let count = usize::try_from(u32_at(buf, count_at)?).ok()?;
    if !(1..=MAX_LAYERS).contains(&count) {
        return None;
    }
    let mut layers = Vec::with_capacity(count);
    for index in 0..count {
        let at = count_at + 4 + LAYER_RECORD_LEN * index;
        let width = f64::from_le_bytes(buf.get(at..at + 8)?.try_into().ok()?);
        let material = u64_at(buf, at + 8)?;
        let deck = u64_at(buf, at + 16)?;
        let function = u32_at(buf, at + 24)?;
        let membrane = function == 100;
        let width_ok =
            width.is_finite() && width < 10.0 && (width >= 1e-3 || (membrane && width == 0.0));
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
    materials: &BTreeSet<u32>,
    declared: &BTreeSet<u32>,
) -> Option<Vec<CompoundLayer>> {
    (6..data.len().saturating_sub(4))
        .filter(|&at| framed(data, at, tag))
        .find_map(|at| layers_at(data, at, materials, declared))
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
    let (Some(header), Some(tag)) = (
        crate::partition_names::element_data_header(revit_version),
        layer_frame_tag(revit_version),
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
            let Some(layers) = buf
                .get(id_at + 8..end)
                .and_then(|data| find_layers(data, tag, materials, declared))
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
    /// its line at its type's thickness carries 1 (RE-54).
    pub word: u32,
    /// Set puts the wall's exterior to the right of its location line's
    /// direction; clear, to the left.
    pub flip: bool,
}

/// A wall's orientation, from its data. `None` without the anchor or with
/// values a wall does not take.
pub fn wall_orientation(data: &[u8]) -> Option<WallOrientation> {
    let at = memchr::memmem::find(data, &WALL_FLIP_ANCHOR)? + WALL_FLIP_ANCHOR.len();
    let location_line = u32_at(data, at)?;
    let word = u32_at(data, at + 4)?;
    let flip = match u32_at(data, at + 8)? {
        0 => false,
        1 => true,
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
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<WallOrientation>> = BTreeMap::new();
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
                .filter(|id| walls.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(LAYER_WINDOW))
                .min(buf.len());
            let Some(orientation) = buf.get(id_at + 8..end).and_then(wall_orientation) else {
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

/// Where a join list's count sits past its opening word on
/// `revit_version`: Revit 2025 puts a zero `u32` before it (RE-70).
/// `None` where the layout is not measured.
pub fn wall_join_count_offset(revit_version: u32) -> Option<usize> {
    match revit_version {
        2024 => Some(12),
        2025 => Some(16),
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
    let tags: BTreeSet<u32> = found
        .values()
        .flatten()
        .flatten()
        .map(|(tag, _)| *tag)
        .collect();
    let [tag] = tags.into_iter().collect::<Vec<_>>()[..] else {
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
