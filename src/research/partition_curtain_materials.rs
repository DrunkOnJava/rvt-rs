//! A curtain wall mullion's and panel's material, from its type's data object
//! (RE-166).
//!
//! The data object (RE-153) of a mullion type, schema class
//! `SysMullionFamSym`, holds its material's ElementId as the `u64` 135 bytes
//! from the object's start: on every such object of Autodesk's 2024, 2025 and
//! 2026 sample projects (353 of 353 on each `rac_advanced`, 66 of 66 on
//! `rac_basic`, 1 of 1 on `rst_basic`) and of RE1 Architecture (10 of 10,
//! Revit 2025, "Aluminum 2" as Revit's export names it). Revit 2027 keeps it
//! there: once 2027's material names are read, each of the 353 types of the
//! 2027 `rac_advanced` holds at +135 the material its 2026 copy does (RE-179).
//! A panel type, `SysPanelFamSym`, holds a tag at +135 and its material's
//! ElementId right after it, at +139: RE1's one panel type, whose material is
//! the "Glass" of Revit's export, as a material followed a tag on Snowdon
//! Towers' panel types (RE-72). A value that is not one of the file's
//! materials is not read as one.
//!
//! The value is the type's `m_renderStyleId` (RE-201, #355), and an optional
//! field before it moves it: on Snowdon Towers (2025) 10 of the 13 exported
//! mullion types hold it at +133, and the scan misses them. Read by name from
//! the type's native record, it is Revit's material on all 17 exported
//! mullion and panel types there, 1,773 of 1,773 members and plates. The
//! native read is taken first; the offsets fill only the types it does not
//! read.
// Tracker: B66.

use crate::partition_room_parameters::verified_data_object;
use crate::{Result, RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// The type classes read, with the offset of the material from an object's
/// start.
const MATERIAL_AT: [(&str, usize); 2] = [("SysMullionFamSym", 135), ("SysPanelFamSym", 139)];

/// Revit releases whose mullion and panel types hold their material at +135
/// and +139.
pub fn supports_revit_version(revit_version: u32) -> bool {
    (2024..=2027).contains(&revit_version)
}

/// Each mullion and panel type's material, by the type's ElementId: its
/// native record's `m_renderStyleId` where that is one of `materials`, and
/// otherwise the ElementId among `materials` that its data object holds at
/// +135 (a mullion type) or +139 (a panel type).
pub fn scan_curtain_type_materials(
    rf: &mut RevitFile,
    revit_version: u32,
    materials: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, u32>> {
    if !supports_revit_version(revit_version) || materials.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut out = native_render_styles(rf, materials).unwrap_or_default();
    for (type_id, material) in scan_offsets(rf, materials)? {
        out.entry(type_id).or_insert(material);
    }
    Ok(out)
}

/// Each mullion and panel type's `m_renderStyleId`, by the type's ElementId,
/// where it is one of `materials`. Errs where the native record path does not
/// read the file's release.
fn native_render_styles(
    rf: &mut RevitFile,
    materials: &BTreeSet<u32>,
) -> anyhow::Result<BTreeMap<u32, u32>> {
    let mut out = BTreeMap::new();
    let options = native_document::Options {
        selected_classes: MATERIAL_AT
            .iter()
            .map(|(name, _)| name.to_string())
            .collect(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        else {
            return Ok(());
        };
        if !MATERIAL_AT.iter().any(|(name, _)| root.class_name == *name) {
            return Ok(());
        }
        let material = root
            .fields
            .get("m_renderStyleId")
            .and_then(|id| crate::native_metadata::identifier(id).ok())
            .and_then(|id| u32::try_from(id).ok());
        let type_id = u32::try_from(record.identity.element_id).ok();
        let material = material.filter(|material| materials.contains(material));
        if let (Some(type_id), Some(material)) = (type_id, material) {
            out.insert(type_id, material);
        }
        Ok(())
    })?;
    Ok(out)
}

/// Each mullion and panel type's material at +135 or +139 of its data
/// object, by the type's ElementId.
fn scan_offsets(rf: &mut RevitFile, materials: &BTreeSet<u32>) -> Result<BTreeMap<u32, u32>> {
    let mut out = BTreeMap::new();
    let classes = rf.memoised_schema_classes()?;
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
                let Some(value) = crate::le_bytes::u64_at(buf, start + at) else {
                    continue;
                };
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
