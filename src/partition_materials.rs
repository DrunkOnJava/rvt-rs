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
//! teste_export_2025. Names are not read here: the name field is not yet
//! identified on materials that families bring.

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
    if !(0.0..=1.0).contains(&transparency) || !(0.0..=1.0).contains(&second) {
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
