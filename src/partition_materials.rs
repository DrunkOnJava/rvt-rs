//! Material shading colours and transparency (RE-53).
//!
//! A Revit material is serialised as an object `01 00 00 00 · u64 id` whose
//! class tag sits [`MATERIAL_TAG_OFFSET`] bytes past the id: `0x0a28` on
//! Revit 2024 and `0x0a6b` on Revit 2025 ([`material_object_tag`]). Some of
//! these objects open with the element-data header
//! ([`crate::partition_names::element_data_header`]), which ends in the same
//! `01 00 00 00 · u64 id`; others, the materials families bring with them,
//! sit inside another element's data. Found by the tag, they number exactly
//! the `OST_Materials` records on every file measured.
//!
//! The first frame of this shape after the id holds the material's shading
//! ([`appearance_at`]):
//!
//! ```text
//! ff ff ff ff ff ff ff ff
//! f32   transparency, 0 to 1
//! f32   0.5 on every material measured
//! 4 ×   u32 COLORREF   pattern colours
//! u32   COLORREF       shading colour, r g b 00
//! u32   shininess, 64 by default
//! ```
//!
//! Colour and transparency equal the `IfcSurfaceStyleRendering` Revit's own
//! IFC4 export gives the same material on 123 of 123 materials across
//! Snowdon Towers, Core Interior, RE1 Architecture, Projeto1 and
//! teste_export_2025.
//!
//! Names (RE-58, [`name_at`]) come from the same object: its own name field
//! after the class tag, else a framed string or a name parameter entry
//! further on. Not every material's name is found; the 2025 layout that
//! keeps it further out is not read yet.

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases these layouts are measured on.
pub const MATERIALS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Offset past a material's ElementId of its object's class tag.
pub const MATERIAL_TAG_OFFSET: usize = 0x47;

/// How far past the id the shading frame is looked for.
pub const APPEARANCE_WINDOW: usize = 0x2000;

/// The material object class tag of `revit_version`, or `None` where it is
/// not measured.
pub fn material_object_tag(revit_version: u32) -> Option<[u8; 2]> {
    match revit_version {
        2024 => Some([0x28, 0x0a]),
        2025 => Some([0x6b, 0x0a]),
        2026 => Some([0x9c, 0x0a]),
        2027 => Some([0xcc, 0x0a]),
        _ => None,
    }
}

/// A material's shading as Revit's shaded views and its IFC export use it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialAppearance {
    /// Shading colour, sRGB.
    pub rgb: [u8; 3],
    /// 0 opaque to 1 fully transparent.
    pub transparency: f32,
    /// Revit's shininess, 0 to 128.
    pub shininess: u32,
}

impl MaterialAppearance {
    /// The colour packed as `0x00BBGGRR`, as [`crate::ifc::MaterialInfo`]
    /// carries it.
    pub fn color_packed(&self) -> u32 {
        u32::from(self.rgb[0]) | (u32::from(self.rgb[1]) << 8) | (u32::from(self.rgb[2]) << 16)
    }
}

fn f32_at(buf: &[u8], at: usize) -> Option<f32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| f32::from_le_bytes(s.try_into().expect("4 bytes")))
}

/// The shading frame whose colour starts at `at`.
fn frame_at(buf: &[u8], at: usize) -> Option<MaterialAppearance> {
    let start = at.checked_sub(32)?;
    if buf.get(start..start + 8)? != [0xff; 8] {
        return None;
    }
    let transparency = f32_at(buf, at - 24)?;
    let second = f32_at(buf, at - 20)?;
    // A run of `ff` longer than 8 also frames a read a few bytes early,
    // whose floats are subnormal: Core Interior's Glass reads 2.35e-38 and
    // black there, 0.75 and blue three bytes on.
    let plausible = |v: f32| (0.0..=1.0).contains(&v) && (v == 0.0 || v.is_normal());
    if !plausible(transparency) || !plausible(second) {
        return None;
    }
    // Each COLORREF's high byte is zero.
    for high in [at - 13, at - 9, at - 5, at - 1, at + 3] {
        if *buf.get(high)? != 0 {
            return None;
        }
    }
    let shininess = u32::from_le_bytes(buf.get(at + 4..at + 8)?.try_into().ok()?);
    Some(MaterialAppearance {
        rgb: [buf[at], buf[at + 1], buf[at + 2]],
        transparency,
        shininess,
    })
}

/// The Revit 2023 shading frame whose colour starts at `at` (RE-116): the
/// transparency and 0.5 as on 2024, then four pattern slots of `u32`
/// ElementId (`ff` × 4 when unset) and `u32` COLORREF, eight bytes each
/// where 2024 keeps four bare COLORREFs, then the shading colour and the
/// shininess. A material that opens its own data frames it with `ff ff ff
/// ff 9b 0b`; a family's own material does not, so the frame is known by
/// its 0.5 and its slots instead.
fn frame_at_2023(buf: &[u8], at: usize) -> Option<MaterialAppearance> {
    let transparency = f32_at(buf, at.checked_sub(40)?)?;
    let second = f32_at(buf, at - 36)?;
    let plausible = |v: f32| (0.0..=1.0).contains(&v) && (v == 0.0 || v.is_normal());
    if !plausible(transparency) || second != 0.5 {
        return None;
    }
    for slot in [at - 32, at - 24, at - 16, at - 8] {
        let id = u32_at(buf, slot)?;
        if id != u32::MAX && id >= 0x0100_0000 {
            return None;
        }
    }
    for high in [at - 25, at - 17, at - 9, at - 1, at + 3] {
        if *buf.get(high)? != 0 {
            return None;
        }
    }
    let shininess = u32::from_le_bytes(buf.get(at + 4..at + 8)?.try_into().ok()?);
    Some(MaterialAppearance {
        rgb: [buf[at], buf[at + 1], buf[at + 2]],
        transparency,
        shininess,
    })
}

/// The first shading frame after the material ElementId at `id_at`.
pub fn appearance_at(buf: &[u8], id_at: usize) -> Option<MaterialAppearance> {
    let from = id_at.checked_add(8 + 32)?;
    let to = id_at
        .saturating_add(APPEARANCE_WINDOW)
        .min(buf.len().saturating_sub(8));
    (from..to).find_map(|at| frame_at(buf, at))
}

/// Every material's shading, by ElementId, read from every partition. Only
/// ids in `declared` count; an id whose copies disagree is dropped. Empty
/// for a release the layout is not measured on.
pub fn scan_material_appearances(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, MaterialAppearance>> {
    if revit_version == 2023 {
        return Ok(scan_material_appearances_2023(rf, declared));
    }
    let Some(tag) = material_object_tag(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<MaterialAppearance>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let id_at = at + 4;
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| declared.contains(id))
            else {
                continue;
            };
            let tag_at = id_at + MATERIAL_TAG_OFFSET;
            if buf.get(tag_at..tag_at + 2) != Some(&tag[..]) {
                continue;
            }
            let Some(appearance) = appearance_at(buf, id_at) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(appearance));
                }
                Some(held) => {
                    if held.as_ref() != Some(&appearance) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, appearance)| appearance.map(|a| (id, a)))
        .collect())
}

/// How far past a material's ElementId its name is looked for.
pub const NAME_WINDOW: usize = 0x1000;

/// The BuiltInParameter some materials keep their name under: the entry
/// `i64 -1001203 · u32 0 · u32 n · UTF-16 × n`.
pub const NAME_PARAMETER: i64 = -1_001_203;

/// The tag of the framed string (`ff ff ff ff · tag · u32 n · UTF-16 × n`)
/// that holds a material's name on `revit_version`.
pub fn material_name_frame_tag(revit_version: u32) -> Option<[u8; 2]> {
    match revit_version {
        2024 => Some([0x49, 0x01]),
        2025 => Some([0x5c, 0x01]),
        2026 => Some([0x64, 0x01]),
        2027 => Some([0x68, 0x01]),
        _ => None,
    }
}

/// The `n` UTF-16 code units at `at`, when they are a name: 1 to 256 of
/// them, none a control character or a noncharacter, and valid UTF-16.
fn utf16_name(buf: &[u8], at: usize, n: u32) -> Option<String> {
    let n = usize::try_from(n).ok().filter(|n| (1..=256).contains(n))?;
    let bytes = buf.get(at..at.checked_add(2 * n)?)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|unit| u16::from_le_bytes([unit[0], unit[1]]))
        .collect();
    if units.iter().any(|u| !(0x20..0xfffe).contains(u)) {
        return None;
    }
    String::from_utf16(&units).ok()
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| i64::from_le_bytes(s.try_into().expect("8 bytes")))
}

fn is_builtin_parameter(id: i64) -> bool {
    (-2_000_000..=-1_000_000).contains(&id)
}

/// The tag after `ff ff ff ff` that ends a material's own name on
/// `revit_version`.
pub fn material_name_end_tag(revit_version: u32) -> Option<[u8; 2]> {
    match revit_version {
        2024 => Some([0x17, 0x0c]),
        2025 => Some([0x6b, 0x0c]),
        2026 => Some([0xac, 0x0c]),
        2027 => Some([0xe0, 0x0c]),
        _ => None,
    }
}

/// The name that ends at the first `ff ff ff ff` and
/// [`material_name_end_tag`] past the class tag: the one `u32 n · UTF-16 ×
/// n` whose last unit sits right before it. Revit ends a material's own name
/// this way wherever that name sits, past any parameter entries the field
/// holds first.
fn terminated_name(object: &[u8], revit_version: u32) -> Option<String> {
    let end_tag = material_name_end_tag(revit_version)?;
    let terminator = [0xff, 0xff, 0xff, 0xff, end_tag[0], end_tag[1]];
    let from = MATERIAL_TAG_OFFSET + 2;
    let end = from + memchr::memmem::find(object.get(from..)?, &terminator)?;
    let mut names = (1..=256u32).filter_map(|n| {
        let at = end
            .checked_sub(4 + 2 * n as usize)
            .filter(|&at| at >= from)?;
        (u32_at(object, at)? == n)
            .then(|| utf16_name(object, at + 4, n))
            .flatten()
    });
    let name = names.next()?;
    names.next().is_none().then_some(name)
}

/// A material's name from its object at the ElementId `id_at` (RE-58), the
/// first of:
/// - its own name field: after the class tag at [`MATERIAL_TAG_OFFSET`], two
///   to four zero `u32`s, then `u32 n` and the name. A slot followed by a
///   BuiltInParameter id is a parameter entry, not a name;
/// - the name ended by `ff ff ff ff` and [`material_name_end_tag`], found
///   from that end where parameter entries of several kinds come first;
/// - the first string framed with [`material_name_frame_tag`];
/// - the first [`NAME_PARAMETER`] entry.
pub fn name_at(buf: &[u8], id_at: usize, revit_version: u32) -> Option<String> {
    let end = id_at.saturating_add(NAME_WINDOW).min(buf.len());
    let object = buf.get(id_at..end)?;
    let mut at = MATERIAL_TAG_OFFSET + 2;
    let mut zeros = 0;
    while zeros < 4 && u32_at(object, at) == Some(0) {
        at += 4;
        zeros += 1;
    }
    if zeros >= 2 {
        if let Some(n) = u32_at(object, at) {
            if !i64_at(object, at + 4).is_some_and(is_builtin_parameter) {
                if let Some(name) = utf16_name(object, at + 4, n) {
                    return Some(name);
                }
            }
        }
    }
    if let Some(name) = terminated_name(object, revit_version) {
        return Some(name);
    }
    if let Some(tag) = material_name_frame_tag(revit_version) {
        let frame = [0xff, 0xff, 0xff, 0xff, tag[0], tag[1]];
        let framed = memchr::memmem::find_iter(object, &frame).find_map(|hit| {
            let n = u32_at(object, hit + frame.len())?;
            utf16_name(object, hit + frame.len() + 4, n)
        });
        if framed.is_some() {
            return framed;
        }
    }
    let mut entry = NAME_PARAMETER.to_le_bytes().to_vec();
    entry.extend_from_slice(&[0, 0, 0, 0]);
    let parameter = memchr::memmem::find_iter(object, &entry).find_map(|hit| {
        let n = u32_at(object, hit + entry.len())?;
        utf16_name(object, hit + entry.len() + 4, n)
    });
    parameter.or_else(|| closed_name(object))
}

/// Last resort (RE-125): the first string in the object that is closed by
/// eight zero bytes, a `u64` ElementId and `0xff`×8. A material whose own
/// name field is a run of shared-parameter entries (a manufacturer's product,
/// supplier, standard, URL) keeps its name there, after the entries. On the
/// 2026 flowbim.ee house it names the 32 materials no other rule reads, every
/// one a material Revit's export names; it is tried last because on nine
/// other objects it lands on an earlier string.
fn closed_name(object: &[u8]) -> Option<String> {
    (MATERIAL_TAG_OFFSET + 2..object.len().saturating_sub(4)).find_map(|at| {
        let n = u32_at(object, at)?;
        let name = utf16_name(object, at + 4, n)?;
        let after = at + 4 + 2 * usize::try_from(n).ok()?;
        let id = u64::from_le_bytes(object.get(after + 8..after + 16)?.try_into().ok()?);
        (object.get(after..after + 8) == Some(&[0u8; 8][..])
            && object.get(after + 16..after + 24) == Some(&[0xffu8; 8][..])
            && u32::try_from(id).is_ok_and(|id| id > 0))
        .then_some(name)
    })
}

/// Every material's name, by ElementId, read from every partition (RE-58).
/// Only ids in `declared` count. An id whose copies disagree is dropped,
/// and so is a name read for two or more materials: Revit keeps material
/// names unique, so one of them is misread. Empty for a release the layout
/// is not measured on.
pub fn scan_material_names(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, String>> {
    let (Some(tag), Some(_)) = (
        material_object_tag(revit_version),
        material_name_frame_tag(revit_version),
    ) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &[1u8, 0, 0, 0]) {
            let id_at = at + 4;
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| declared.contains(id))
            else {
                continue;
            };
            let tag_at = id_at + MATERIAL_TAG_OFFSET;
            if buf.get(tag_at..tag_at + 2) != Some(&tag[..]) {
                continue;
            }
            let Some(name) = name_at(buf, id_at, revit_version) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(name));
                }
                Some(held) => {
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                }
            }
        }
    }
    let names: BTreeMap<u32, String> = found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect();
    let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
    for name in names.values() {
        *uses.entry(name.as_str()).or_default() += 1;
    }
    let shared: BTreeSet<String> = uses
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name.to_string())
        .collect();
    Ok(names
        .into_iter()
        .filter(|(_, name)| !shared.contains(name))
        .collect())
}

/// Releases a category's material (RE-91) is read on. Measured on Revit
/// 2024 files and, in [`CATEGORY_MATERIAL_FRAME_2023`]'s layout, on Revit
/// 2023 (RE-115); RE1's 2025 files hold no entry in either layout.
pub const CATEGORY_MATERIAL_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2023, 2024];

/// A category's object-styles entry on Revit 2023 (RE-115), after its
/// `BuiltInCategory` (`i32`): an unset `u32`, a `u32` that is 1 or 2 (the
/// entry is held twice), `u32 1`, `ff ff ff ff 3f 01`, eight bytes, then
/// [`CATEGORY_MATERIAL_MARK_2023`] and the material (`u32`, `ff` × 4 when
/// unset). This frame is the bytes from the unset `u32` to the `3f 01`,
/// with the 1-or-2 word skipped.
pub const CATEGORY_MATERIAL_FRAME_2023: [u8; 4] = [0xff, 0xff, 0xff, 0xff];
/// See [`CATEGORY_MATERIAL_FRAME_2023`]: `i32 -3000010`, 26 bytes past the
/// category, right before the material.
pub const CATEGORY_MATERIAL_MARK_2023: [u8; 4] = [0x36, 0x39, 0xd2, 0xff];

/// The bytes a category's entry in the document's object styles holds
/// after its `BuiltInCategory` (`i64`), before its material (`u64`): an
/// unset `u64`, `u32 1` and another unset `u64` (RE-91).
pub const CATEGORY_MATERIAL_FRAME: [u8; 20] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01, 0x00, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
    0xff, 0xff, 0xff, 0xff,
];

/// The material the document's object styles give `category` (RE-91): the
/// `u64` after its `BuiltInCategory` and [`CATEGORY_MATERIAL_FRAME`]. On
/// Core Interior the Walls entry holds 87, "Default Wall", and Roofs 88,
/// "Default Roof", and every other category is unset; Revit's IFC4 export
/// writes "Default Wall" for each of Core Interior's 356 walls whose layer
/// takes its category's material.
///
/// On Revit 2023 the entry is [`CATEGORY_MATERIAL_FRAME_2023`]'s: on two
/// projects the Walls, Floors and Roofs entries hold the materials Revit's
/// own export writes for their layers that take their category's material
/// (RE-115).
///
/// `None` on a release outside [`CATEGORY_MATERIAL_SUPPORTED_REVIT_VERSIONS`],
/// without the entry, where it is unset (0 or all ones), or where copies
/// disagree.
pub fn scan_category_material(
    rf: &mut RevitFile,
    revit_version: u32,
    category: i64,
) -> Result<Option<u32>> {
    if !CATEGORY_MATERIAL_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Ok(None);
    }
    if revit_version == 2023 {
        return scan_category_material_2023(rf, category);
    }
    let mut pattern = category.to_le_bytes().to_vec();
    pattern.extend_from_slice(&CATEGORY_MATERIAL_FRAME);
    let mut found: BTreeSet<u64> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &pattern) {
            if let Some(material) = buf
                .get(at + pattern.len()..at + pattern.len() + 8)
                .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
            {
                found.insert(material);
            }
        }
    }
    let mut values = found.into_iter();
    let (Some(material), None) = (values.next(), values.next()) else {
        return Ok(None);
    };
    Ok(u32::try_from(material)
        .ok()
        .filter(|&id| id != 0 && id != u32::MAX))
}

/// A Revit 2023 material's class tag (RE-113), [`MATERIAL_TAG_OFFSET_2023`]
/// past its `u32` ElementId, behind `00 00 00 · ff ff ff ff`: the place
/// 2024's `0x0a28` has past its `u64` id ([`MATERIAL_TAG_OFFSET`]).
pub const MATERIAL_TAG_2023: [u8; 2] = [0xfb, 0x09];
/// See [`MATERIAL_TAG_2023`].
pub const MATERIAL_TAG_OFFSET_2023: usize = 0x27;

/// Every Revit 2023 material, by ElementId, with its name where it is read
/// (RE-113). A material is an object `01 00 00 00 · u32 id` carrying
/// [`MATERIAL_TAG_2023`], whether it opens an element's data or sits inside
/// another's (a family's own materials); only ids in `declared` count. Its
/// name is its own name field, as RE-58 reads a 2024 one: right after the
/// tag, two to four zero `u32`s, `u32 n` and the name, where no
/// BuiltInParameter id follows the zeros (Revit_IFC5_Einhoven); else the
/// one ending right before the first `ff ff ff ff eb 0b` past the tag, as
/// RE-58 ends a 2024 name before `ff ff ff ff 17 0c` (RE-116);
/// else its first [`NAME_PARAMETER`] entry, `i32 -1001203 · u32 0 · u32 n ·
/// UTF-16 × n`, whose value is a name; else the one its element data gives
/// it ([`crate::partition_names::find_element_data_names_2023`]), which on
/// some materials is an appearance asset's file name instead. An id whose
/// copies disagree has no name, and nor has a name read for two or more
/// materials.
pub fn scan_materials_2023(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
) -> (BTreeSet<u32>, BTreeMap<u32, String>) {
    const PREFIX: [u8; 7] = [0, 0, 0, 0xff, 0xff, 0xff, 0xff];
    let mut entry = i32::try_from(NAME_PARAMETER)
        .expect("a 32-bit BuiltInParameter")
        .to_le_bytes()
        .to_vec();
    entry.extend_from_slice(&[0, 0, 0, 0]);
    let mut materials = BTreeSet::new();
    let mut found: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for tag_at in memchr::memmem::find_iter(buf, &MATERIAL_TAG_2023) {
            let Some(id_at) = tag_at.checked_sub(MATERIAL_TAG_OFFSET_2023) else {
                continue;
            };
            if id_at < 4
                || buf.get(id_at - 4..id_at) != Some(&[1, 0, 0, 0][..])
                || buf.get(tag_at - PREFIX.len()..tag_at) != Some(&PREFIX[..])
            {
                continue;
            }
            let Some(id) = u32_at(buf, id_at).filter(|id| declared.contains(id)) else {
                continue;
            };
            materials.insert(id);
            let end = id_at.saturating_add(NAME_WINDOW).min(buf.len());
            let object = &buf[tag_at + 2..end];
            let name = own_name_field(object)
                .or_else(|| terminated_name_2023(object))
                .or_else(|| {
                    memchr::memmem::find_iter(object, &entry).find_map(|hit| {
                        let n = u32_at(object, hit + entry.len())?;
                        utf16_name(object, hit + entry.len() + 4, n)
                    })
                });
            let Some(name) = name else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(name));
                }
                Some(held) => {
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                }
            }
        }
    }
    let mut names: BTreeMap<u32, String> = found
        .into_iter()
        .filter_map(|(id, name)| name.map(|n| (id, n)))
        .collect();
    let unnamed: BTreeSet<u32> = materials
        .iter()
        .copied()
        .filter(|id| !names.contains_key(id))
        .collect();
    for stream in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&stream) {
            for (id, name) in
                crate::partition_names::find_element_data_names_2023(inflated.bytes(), &unnamed)
            {
                names.entry(id).or_insert(name);
            }
        }
    }
    let mut uses: BTreeMap<&str, usize> = BTreeMap::new();
    for name in names.values() {
        *uses.entry(name.as_str()).or_default() += 1;
    }
    let shared: BTreeSet<String> = uses
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(name, _)| name.to_string())
        .collect();
    let names = names
        .into_iter()
        .filter(|(_, name)| !shared.contains(name))
        .collect();
    (materials, names)
}

fn scan_category_material_2023(rf: &mut RevitFile, category: i64) -> Result<Option<u32>> {
    let Ok(category) = i32::try_from(category) else {
        return Ok(None);
    };
    let mut pattern = category.to_le_bytes().to_vec();
    pattern.extend_from_slice(&CATEGORY_MATERIAL_FRAME_2023);
    let mut found: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &pattern) {
            let entry = |offset: usize, len: usize| buf.get(at + offset..at + offset + len);
            let word = entry(8, 4).map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")));
            if !matches!(word, Some(1 | 2))
                || entry(12, 4) != Some(&[1, 0, 0, 0][..])
                || entry(16, 6) != Some(&[0xff, 0xff, 0xff, 0xff, 0x3f, 0x01][..])
                || entry(30, 4) != Some(&CATEGORY_MATERIAL_MARK_2023[..])
            {
                continue;
            }
            if let Some(material) = u32_at(buf, at + 34) {
                found.insert(material);
            }
        }
    }
    let mut values = found.into_iter();
    let (Some(material), None) = (values.next(), values.next()) else {
        return Ok(None);
    };
    Ok(Some(material).filter(|&id| id != 0 && id != u32::MAX))
}

/// [`scan_material_appearances`] on Revit 2023 (RE-116): each material
/// object of [`scan_materials_2023`]'s shape, and its first
/// [`frame_at_2023`] frame.
fn scan_material_appearances_2023(
    rf: &mut RevitFile,
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, MaterialAppearance> {
    const PREFIX: [u8; 7] = [0, 0, 0, 0xff, 0xff, 0xff, 0xff];
    let mut found: BTreeMap<u32, Option<MaterialAppearance>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for tag_at in memchr::memmem::find_iter(buf, &MATERIAL_TAG_2023) {
            let Some(id_at) = tag_at.checked_sub(MATERIAL_TAG_OFFSET_2023) else {
                continue;
            };
            if id_at < 4
                || buf.get(id_at - 4..id_at) != Some(&[1, 0, 0, 0][..])
                || buf.get(tag_at - PREFIX.len()..tag_at) != Some(&PREFIX[..])
            {
                continue;
            }
            let Some(id) = u32_at(buf, id_at).filter(|id| declared.contains(id)) else {
                continue;
            };
            let to = id_at
                .saturating_add(APPEARANCE_WINDOW)
                .min(buf.len().saturating_sub(8));
            let Some(appearance) = (tag_at + 2 + 40..to).find_map(|at| frame_at_2023(buf, at))
            else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(appearance));
                }
                Some(held) => {
                    if held.as_ref() != Some(&appearance) {
                        *held = None;
                    }
                }
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, appearance)| appearance.map(|a| (id, a)))
        .collect()
}

/// The name `u32 n · UTF-16 × n` ending right before the first `ff ff ff ff
/// eb 0b` in `object` (RE-116), where exactly one length fits.
fn terminated_name_2023(object: &[u8]) -> Option<String> {
    const TERMINATOR: [u8; 6] = [0xff, 0xff, 0xff, 0xff, 0xeb, 0x0b];
    let end = memchr::memmem::find(object, &TERMINATOR)?;
    let mut names = (1..=256u32).filter_map(|n| {
        let at = end.checked_sub(4 + 2 * n as usize)?;
        (u32_at(object, at)? == n)
            .then(|| utf16_name(object, at + 4, n))
            .flatten()
    });
    let name = names.next()?;
    names.next().is_none().then_some(name)
}

/// The name `u32 n · UTF-16 × n` after two to four zero `u32`s at the start
/// of `object`, the bytes right after a 2023 material's class tag (RE-58's
/// own name field), unless a BuiltInParameter id follows the zeros.
fn own_name_field(object: &[u8]) -> Option<String> {
    let mut at = 0;
    let mut zeros = 0;
    while zeros < 4 && u32_at(object, at) == Some(0) {
        at += 4;
        zeros += 1;
    }
    if zeros < 2 {
        return None;
    }
    let n = u32_at(object, at)?;
    // A slot followed by a BuiltInParameter id is a parameter entry.
    let parameter = i32::from_le_bytes(object.get(at + 4..at + 8)?.try_into().ok()?);
    if is_builtin_parameter(i64::from(parameter)) {
        return None;
    }
    // Where the field holds something else, two zeros can still lead to a
    // "name" of private-use or specials-block units; none is a name
    // (RE-109 drops them from 2023 name entries the same way).
    utf16_name(object, at + 4, n).filter(|name| {
        !name
            .chars()
            .any(|c| matches!(c, '\u{e000}'..='\u{f8ff}' | '\u{fff0}'..='\u{ffff}'))
    })
}
