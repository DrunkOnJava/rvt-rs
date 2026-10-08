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
//!
//! On Revit 2023 (RE-113) the block is `[owner u32][28 x 0xff][3 x 0x00]`,
//! a map entry is `u32 key · u32 material`, and only the block's first map
//! is the type's ([`type_material_names_2023`]).

use crate::RevitFile;
use crate::partition_element_records::bbox_marker;
use std::collections::{BTreeMap, BTreeSet};

/// Largest map entry count read.
const MAX_ENTRIES: usize = 64;
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

/// [`VALUE_BLOCK_MARK`] on Revit 2023, where the owner is a `u32`.
const VALUE_BLOCK_MARK_2023: [u8; 31] = {
    let mut mark = [0xffu8; 31];
    mark[28] = 0;
    mark[29] = 0;
    mark[30] = 0;
    mark
};

/// How a release frames value blocks and their maps.
#[derive(Clone, Copy)]
struct BlockLayout {
    mark: &'static [u8],
    /// Bytes of the owner id before the mark, and of a map value.
    id_len: usize,
    /// Read only the block's first map. A 2023 type's data runs on past
    /// its map into single-entry lists of other materials (RE-113).
    first_map_only: bool,
}

const BLOCK_LAYOUT: BlockLayout = BlockLayout {
    mark: &VALUE_BLOCK_MARK,
    id_len: 8,
    first_map_only: false,
};

const BLOCK_LAYOUT_2023: BlockLayout = BlockLayout {
    mark: &VALUE_BLOCK_MARK_2023,
    id_len: 4,
    first_map_only: true,
};

fn id_at(buf: &[u8], at: usize, len: usize) -> Option<u32> {
    let bytes = buf.get(at..at.checked_add(len)?)?;
    match len {
        4 => Some(u32::from_le_bytes(bytes.try_into().ok()?)),
        _ => u32::try_from(u64::from_le_bytes(bytes.try_into().ok()?)).ok(),
    }
}

/// Every owner's map materials in one inflated partition, in the order they
/// first appear, by the declared ElementId of the value block they are in.
/// An owner whose value block holds no map is present with no materials.
pub fn scan_partition(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    materials: &BTreeSet<u32>,
) -> BTreeMap<u32, Vec<u32>> {
    scan_partition_with(buf, declared_ids, materials, BLOCK_LAYOUT)
}

fn scan_partition_with(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    materials: &BTreeSet<u32>,
    layout: BlockLayout,
) -> BTreeMap<u32, Vec<u32>> {
    let blocks: Vec<(usize, u32)> = memchr::memmem::find_iter(buf, layout.mark)
        .filter(|&at| at >= layout.id_len && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = id_at(buf, at - layout.id_len, layout.id_len)?;
            declared_ids.contains(&id).then_some((at, id))
        })
        .collect();
    let mut out: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    let mut partial: BTreeSet<u32> = BTreeSet::new();
    for (index, &(start, owner)) in blocks.iter().enumerate() {
        let end = blocks
            .get(index + 1)
            .map_or(buf.len(), |next| next.0.saturating_sub(layout.id_len))
            .min(start + MAX_BLOCK_LEN)
            .min(buf.len());
        let list = out.entry(owner).or_default();
        let mut at = start + layout.mark.len();
        while at + 4 <= end {
            if let Some(found) = map_at(buf, at, end, materials, layout.id_len) {
                if found.partial {
                    partial.insert(owner);
                } else {
                    for material in found.materials {
                        if !list.contains(&material) {
                            list.push(material);
                        }
                    }
                }
                if layout.first_map_only {
                    break;
                }
                at = found.end;
            } else {
                at += 1;
            }
        }
    }
    // A type whose only maps are partial draws geometry in materials not
    // read here: leave it out rather than report it as drawing none (RE-84).
    out.retain(|owner, list| !list.is_empty() || !partial.contains(owner));
    out
}

struct MaterialMap {
    materials: Vec<u32>,
    end: usize,
    /// Some entries are unset. Such a map shows the type draws geometry,
    /// but its materials are not the element's: taken as its set, it gave
    /// 46 more wrong sets than right ones on Snowdon Towers (RE-124).
    partial: bool,
}

/// The map at `at`, when every one of its values is a material.
fn map_at(
    buf: &[u8],
    at: usize,
    end: usize,
    materials: &BTreeSet<u32>,
    id_len: usize,
) -> Option<MaterialMap> {
    let count = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=MAX_ENTRIES).contains(&count) {
        return None;
    }
    let entry_len = 4 + id_len;
    let map_end = at + 4 + count * entry_len;
    if map_end > end {
        return None;
    }
    let mut found = Vec::with_capacity(count);
    let mut unset = 0;
    for entry in 0..count {
        let value_at = at + 4 + entry * entry_len + 4;
        // An unset value (all `0xff`) is a part with no material of its
        // own; a 2026 manufacturer door's map holds several (RE-124).
        if buf
            .get(value_at..value_at + id_len)?
            .iter()
            .all(|&b| b == 0xff)
        {
            unset += 1;
            continue;
        }
        let material = id_at(buf, value_at, id_len).filter(|m| materials.contains(m))?;
        found.push(material);
    }
    if found.is_empty() {
        return None;
    }
    Some(MaterialMap {
        materials: found,
        end: map_end,
        partial: unset > 0,
    })
}

/// Every type's map materials in the file, by owner ElementId, with the
/// material names they resolve to (RE-58). A type whose value block holds no
/// map is present with no names (RE-84). Empty for a release whose
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
    // Without the file's materials no map can be recognised, and every type
    // would read as drawing no geometry (RE-84): fail closed instead.
    if names.is_empty() {
        return BTreeMap::new();
    }
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
        .filter_map(|(owner, ids)| {
            let resolved: Vec<String> =
                ids.iter().filter_map(|id| names.get(id).cloned()).collect();
            // A map whose names are not read is not "no materials".
            (ids.is_empty() || !resolved.is_empty()).then_some((owner, resolved))
        })
        .collect()
}

/// The name Revit's own IFC export gives geometry drawn with no material of
/// its own.
pub const UNNAMED_MATERIAL: &str = "<Unnamed>";

/// Largest geometry tag accepted as a key of an all-unset map.
const MAX_GEOMETRY_TAG: u32 = 0xffff;

/// Whether the block `buf[start..end]` opens with a geometry-material map
/// whose every value is unset. The first map read in the block decides:
/// `u32 n` in 1..=[`MAX_ENTRIES`] then `n` entries of `u32 key · u64 value`,
/// with distinct keys no larger than [`MAX_GEOMETRY_TAG`] (the type's
/// geometry tags, RE-149), and every value either unset (`0xff` x 8) or a
/// material of the file. A map with a material in it answers no (a partial
/// map is the type's, RE-124); one with only unset values answers yes. Such
/// a type draws its geometry with no material: Revit's export gives its
/// instances [`UNNAMED_MATERIAL`].
fn opens_unset_map(buf: &[u8], start: usize, end: usize, materials: &BTreeSet<u32>) -> bool {
    let mut at = start;
    while at + 4 <= end {
        let count = u32::from_le_bytes(buf[at..at + 4].try_into().expect("4 bytes")) as usize;
        let map_end = at + 4 + count * 12;
        if (1..=MAX_ENTRIES).contains(&count) && map_end <= end {
            let mut keys = Vec::with_capacity(count);
            let (mut unset, mut material) = (0usize, 0usize);
            for entry in 0..count {
                let e = at + 4 + entry * 12;
                keys.push(u32::from_le_bytes(
                    buf[e..e + 4].try_into().expect("4 bytes"),
                ));
                let value = &buf[e + 4..e + 12];
                if value.iter().all(|&b| b == 0xff) {
                    unset += 1;
                } else if id_at(value, 0, 8).is_some_and(|id| materials.contains(&id)) {
                    material += 1;
                }
            }
            let small = keys.iter().all(|&k| k <= MAX_GEOMETRY_TAG);
            keys.sort_unstable();
            keys.dedup();
            if small && keys.len() == count && unset + material == count {
                return material == 0;
            }
        }
        at += 1;
    }
    false
}

/// The family types whose value block holds no geometry-material map with a
/// material but opens with one whose every value is unset (RE-149, #355):
/// types that draw their geometry with no material of their own. On the
/// reference models these are the column type of `2024_Core_Interior.rvt`
/// and the RE1 fittings, terminals and fixtures, whose instances Revit's
/// export gives [`UNNAMED_MATERIAL`]. Empty where [`type_material_names`]
/// reads nothing.
pub fn unset_material_types(rf: &mut RevitFile, revit_version: u32) -> BTreeSet<u32> {
    let with_materials = type_material_names(rf, revit_version);
    if with_materials.is_empty() {
        return BTreeSet::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeSet::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let Ok(names) = crate::partition_materials::scan_material_names(rf, revit_version, &declared)
    else {
        return BTreeSet::new();
    };
    let materials: BTreeSet<u32> = names.keys().copied().collect();
    let mut out = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let blocks: Vec<(usize, u32)> = memchr::memmem::find_iter(buf, BLOCK_LAYOUT.mark)
            .filter(|&at| at >= BLOCK_LAYOUT.id_len && buf[at - 1] != 0xff)
            .filter_map(|at| {
                let id = id_at(buf, at - BLOCK_LAYOUT.id_len, BLOCK_LAYOUT.id_len)?;
                declared.contains(&id).then_some((at, id))
            })
            .collect();
        for (index, &(start, owner)) in blocks.iter().enumerate() {
            if with_materials
                .get(&owner)
                .is_some_and(|names| !names.is_empty())
            {
                continue;
            }
            let end = blocks
                .get(index + 1)
                .map_or(buf.len(), |next| next.0.saturating_sub(BLOCK_LAYOUT.id_len))
                .min(start + MAX_BLOCK_LEN)
                .min(buf.len());
            if opens_unset_map(buf, start + BLOCK_LAYOUT.mark.len(), end, &materials) {
                out.insert(owner);
            }
        }
    }
    out
}

/// [`type_material_names`] on Revit 2023 (RE-113). A map value counts as a
/// material when it is one of [`crate::partition_materials::scan_materials_2023`];
/// a type whose map names a material with no name read gets none. Only
/// owners whose block holds a map are returned: a map missed here must not
/// read as a type that draws no geometry (RE-84).
pub fn type_material_names_2023(rf: &mut RevitFile) -> BTreeMap<u32, Vec<String>> {
    type_material_names_32(rf, crate::partition_element_records_2023::REVIT_2023)
}

/// [`type_material_names_2023`] on any 32-bit release, Revit 2014 to 2023,
/// whose materials are [`crate::partition_materials::scan_materials_32`]'s
/// (RE-178). Empty on any other release.
pub fn type_material_names_32(
    rf: &mut RevitFile,
    revit_version: u32,
) -> BTreeMap<u32, Vec<String>> {
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&records);
    let (tagged, names) =
        crate::partition_materials::scan_materials_32(rf, revit_version, &declared);
    let streams = rf.partition_stream_names();
    let mut by_owner: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for stream in &streams {
        let Ok(inflated) = rf.inflated_partition(stream) else {
            continue;
        };
        for (owner, found) in
            scan_partition_with(inflated.bytes(), &declared, &tagged, BLOCK_LAYOUT_2023)
        {
            let list = by_owner.entry(owner).or_default();
            for material in found {
                if !list.contains(&material) {
                    list.push(material);
                }
            }
        }
    }
    // A map with a material whose name is not read gives the type nothing:
    // a partial set would be a wrong one.
    by_owner
        .into_iter()
        .filter(|(_, ids)| !ids.is_empty())
        .filter_map(|(owner, ids)| {
            let resolved: Option<Vec<String>> =
                ids.iter().map(|id| names.get(id).cloned()).collect();
            resolved.map(|names| (owner, names))
        })
        .collect()
}
