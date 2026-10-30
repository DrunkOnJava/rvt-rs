//! What a storey's Level gives beyond its name and elevation (W7, RE-275).
//!
//! Revit's IFC4 export writes `Pset_BuildingStoreyCommon.Reference` on each
//! storey: the name of the Level's type (revit-ifc's `ReferenceCalculator`,
//! CLEANROOM.md source 5). The Level's native record names its type in
//! `m_attrId`, and the `LevelAttributes` record that id names holds the
//! name in `SymbolInfo.m_name`: `Korkomerkintä` on each of the BIM4LCA
//! office's 10 Levels, as built and at building permit, and `Level 1` on the
//! R2025 models' (RE-275).

use super::entities::StoreyProperties;
use crate::RevitFile;
use std::collections::BTreeMap;

/// The properties of each of `levels` (a storey's index and its Level's
/// ElementId), by storey index. A Level whose record names no type with a
/// name has no entry; a file whose native records are not read gives none.
pub(super) fn read(
    rf: &mut RevitFile,
    levels: &[(usize, u32)],
) -> BTreeMap<usize, StoreyProperties> {
    let mut out = BTreeMap::new();
    if levels.is_empty() {
        return out;
    }
    let classes: BTreeMap<u64, String> = levels
        .iter()
        .map(|(_, id)| (u64::from(*id), "Level".to_string()))
        .collect();
    let names = crate::native_element_types::system_type_names(rf, &classes).unwrap_or_default();
    for (index, id) in levels {
        if let Some((_, name)) = names.get(&u64::from(*id)) {
            out.insert(
                *index,
                StoreyProperties {
                    reference: Some(name.clone()),
                },
            );
        }
    }
    out
}
