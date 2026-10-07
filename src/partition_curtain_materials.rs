//! A curtain wall mullion's and panel's material, from its type's data object
//! (RE-166, B66).
//!
//! The data object (RE-153) of a mullion type, schema class
//! `SysMullionFamSym`, holds its material's ElementId as the `u64` 135 bytes
//! from the object's start: on every such object of Autodesk's 2024, 2025 and
//! 2026 sample projects (353 of 353 on each `rac_advanced`, 66 of 66 on
//! `rac_basic`, 1 of 1 on `rst_basic`) and of RE1 Architecture (10 of 10,
//! Revit 2025, "Aluminum 2" as Revit's export names it). Revit 2027 keeps it
//! there: once 2027's material names are read, each of the 353 types of the
//! 2027 `rac_advanced` holds at +135 the material its 2026 copy does (RE-177).
//! A panel type, `SysPanelFamSym`, holds a tag at +135 and its material's
//! ElementId right after it, at +139: RE1's one panel type, whose material is
//! the "Glass" of Revit's export, as a material followed a tag on Snowdon
//! Towers' panel types (RE-72). A value that is not one of the file's
//! materials is not read as one.

use crate::partition_room_parameters::verified_data_object;
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// The type classes read, with the offset of the material from an object's
/// start.
const MATERIAL_AT: [(&str, usize); 2] = [("SysMullionFamSym", 135), ("SysPanelFamSym", 139)];

/// Revit releases whose mullion and panel types hold their material at +135
/// and +139.
pub fn supports_revit_version(revit_version: u32) -> bool {
    (2024..=2027).contains(&revit_version)
}

/// Each mullion and panel type's material, by the type's ElementId: the
/// ElementId among `materials` that its data object holds at +135 (a
/// mullion type) or +139 (a panel type).
pub fn scan_curtain_type_materials(
    rf: &mut RevitFile,
    revit_version: u32,
    materials: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, u32>> {
    let mut out = BTreeMap::new();
    if !supports_revit_version(revit_version) || materials.is_empty() {
        return Ok(out);
    }
    let classes = rf.schema_classes()?;
    let tags: Vec<(u32, usize)> = MATERIAL_AT
        .iter()
        .filter_map(|(name, at)| {
            let class = classes.classes.iter().find(|class| class.name == *name)?;
            Some((u32::from(class.tag), *at))
        })
        .collect();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for &(tag, at) in &tags {
            // The class word ends a data object's 20-byte header.
            let word = (0xffff_0000u32 | tag).to_le_bytes();
            for hit in memchr::memmem::find_iter(buf, &word) {
                let Some(start) = hit.checked_sub(16) else {
                    continue;
                };
                let Some(object) = verified_data_object(buf, start) else {
                    continue;
                };
                if object.class & 0xffff != tag || object.end < start + at + 8 {
                    continue;
                }
                let value = u64::from_le_bytes(
                    buf[start + at..start + at + 8].try_into().expect("8 bytes"),
                );
                match u32::try_from(value) {
                    Ok(material) if materials.contains(&material) => {
                        out.insert(object.element_id, material);
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(out)
}
