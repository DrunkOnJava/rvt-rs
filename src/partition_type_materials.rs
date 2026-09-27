//! The materials a family type's geometry uses (#355, RE-82).
//!
//! A family type's parameter value block (RE-77: `[owner u64][56 x 0xff]
//! [3 x 0x00]`) holds a counted map `u32 n · n × (u32 key · u64 material)`
//! whose every value is a material ElementId: the materials its geometry is
//! drawn in. Materials a type only names in a parameter (a window's
//! alternative cladding, a light's alternative finish) are not in it.
//! Revit's own IFC export gives a family instance exactly its type's map
//! materials on 1,331 of 1,371 Snowdon Towers elements (2024), 43 of 43 on
//! RE1 Architecture (2025) and 132 of 138 on Core Interior (2024)
//! (`reports/element-framing/RE-82-type-material-maps.md`). The rest are
//! families whose map also covers a nested component Revit exports as its
//! own element (a counter top's appliance) or whose material Revit writes
//! unnamed (planting).

use crate::RevitFile;
use crate::partition_element_records::bbox_marker;
use std::collections::{BTreeMap, BTreeSet};

/// Largest map entry count read.
const MAX_ENTRIES: usize = 64;
/// Bytes of one map entry: `u32` key and `u64` material ElementId.
const ENTRY_LEN: usize = 12;
/// Bytes read past a value block's start when its next block is far away.
const MAX_BLOCK_LEN: usize = 200_000;

/// The bytes that follow an ElementId to open its value block.
const VALUE_BLOCK_MARK: [u8; 59] = {
    let mut mark = [0xffu8; 59];
    mark[56] = 0;
    mark[57] = 0;
    mark[58] = 0;
    mark
};

/// Every owner's map materials in one inflated partition, in the order they
/// first appear, by the declared ElementId of the value block they are in.
pub fn scan_partition(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    materials: &BTreeSet<u32>,
) -> BTreeMap<u32, Vec<u32>> {
    let blocks: Vec<(usize, u32)> = memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
        .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = u64::from_le_bytes(buf.get(at - 8..at)?.try_into().ok()?);
            let id = u32::try_from(id).ok()?;
            declared_ids.contains(&id).then_some((at, id))
        })
        .collect();
    let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (index, &(start, owner)) in blocks.iter().enumerate() {
        let end = blocks
            .get(index + 1)
            .map_or(buf.len(), |next| next.0.saturating_sub(8))
            .min(start + MAX_BLOCK_LEN)
            .min(buf.len());
        let mut at = start + VALUE_BLOCK_MARK.len();
        while at + 4 <= end {
            if let Some(found) = map_at(buf, at, end, materials) {
                let list = out.entry(owner).or_default();
                for material in found.materials {
                    if !list.contains(&material) {
                        list.push(material);
                    }
                }
                at = found.end;
            } else {
                at += 1;
            }
        }
    }
    out
}

struct MaterialMap {
    materials: Vec<u32>,
    end: usize,
}

/// The map at `at`, when every one of its values is a material.
fn map_at(buf: &[u8], at: usize, end: usize, materials: &BTreeSet<u32>) -> Option<MaterialMap> {
    let count = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=MAX_ENTRIES).contains(&count) {
        return None;
    }
    let map_end = at + 4 + count * ENTRY_LEN;
    if map_end > end {
        return None;
    }
    let mut found = Vec::with_capacity(count);
    for entry in 0..count {
        let value_at = at + 4 + entry * ENTRY_LEN + 4;
        let value = u64::from_le_bytes(buf.get(value_at..value_at + 8)?.try_into().ok()?);
        let material = u32::try_from(value)
            .ok()
            .filter(|m| materials.contains(m))?;
        found.push(material);
    }
    Some(MaterialMap {
        materials: found,
        end: map_end,
    })
}

/// Every type's map materials in the file, by owner ElementId, with the
/// material names they resolve to (RE-58). Empty for a release whose
/// element records are not decoded, or whose material names are not read.
pub fn type_material_names(rf: &mut RevitFile, revit_version: u32) -> BTreeMap<u32, Vec<String>> {
    if bbox_marker(revit_version).is_none() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let Ok(names) = crate::partition_materials::scan_material_names(rf, revit_version, &declared)
    else {
        return BTreeMap::new();
    };
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let mut by_owner: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, found) in scan_partition(inflated.bytes(), &declared, &materials) {
            let list = by_owner.entry(owner).or_default();
            for material in found {
                if !list.contains(&material) {
                    list.push(material);
                }
            }
        }
    }
    by_owner
        .into_iter()
        .map(|(owner, ids)| {
            let resolved = ids.iter().filter_map(|id| names.get(id).cloned()).collect();
            (owner, resolved)
        })
        .collect()
}
