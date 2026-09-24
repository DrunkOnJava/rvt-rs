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
pub const MATERIALS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

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
    memchr::memmem::find_iter(object, &entry).find_map(|hit| {
        let n = u32_at(object, hit + entry.len())?;
        utf16_name(object, hit + entry.len() + 4, n)
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
