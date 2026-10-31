//! What a storey's Level gives beyond its name and elevation (W7, RE-275).
//!
//! Revit's IFC4 export writes `Pset_BuildingStoreyCommon` on each storey with
//! two properties besides `AboveGround`, both read here from the Level's own
//! native record:
//!
//! - `Reference`, the name of the Level's type (revit-ifc's
//!   `ReferenceCalculator`, CLEANROOM.md source 5): the `LevelAttributes`
//!   record the Level names in `m_attrId` holds it in `SymbolInfo.m_name`,
//!   `Korkomerkintä` on each of the BIM4LCA office's 10 Levels, as built and
//!   at building permit, and `Level 1` on the R2025 models';
//! - `EntranceLevel`, which revit-ifc's definition of the set gives no
//!   calculator, so it is the value of a yes/no parameter named
//!   `Pset_BuildingStoreyCommon.EntranceLevel` or `EntranceLevel` on the
//!   Level, where the Level holds one: the office's Level_01 yes and 8 more
//!   Levels no, its Sea Level none and so no property.

use super::entities::StoreyProperties;
use crate::RevitFile;
use crate::research::native_element_parameters::{self, ParameterLevel};
use std::collections::{BTreeMap, BTreeSet};

/// The parameter names revit-ifc looks `EntranceLevel` up by, the qualified
/// name first.
const ENTRANCE_LEVEL_PARAMETERS: [&str; 2] =
    ["Pset_BuildingStoreyCommon.EntranceLevel", "EntranceLevel"];

fn is_entrance_level_name(name: &str) -> bool {
    ENTRANCE_LEVEL_PARAMETERS.contains(&name)
}

/// The properties of each of `levels` (a storey's index and its Level's
/// ElementId), by storey index. A Level whose record gives neither has no
/// entry; a file whose native records are not read gives none.
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
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    // A Level has no type, so its own record is all that is read, and only
    // where the file defines a parameter by one of these names.
    let parameters =
        native_element_parameters::read_own_defined(rf, &ids, Some(is_entrance_level_name))
            .unwrap_or_default();
    for (index, id) in levels {
        let id = u64::from(*id);
        let reference = names.get(&id).map(|(_, name)| name.clone());
        let entrance_level = parameters.elements.get(&id).and_then(|list| {
            ENTRANCE_LEVEL_PARAMETERS.iter().find_map(|wanted| {
                list.iter().find_map(|parameter| {
                    if parameter.name.as_deref() == Some(*wanted)
                        && parameter.level == ParameterLevel::Instance
                    {
                        parameter.value.as_bool()
                    } else {
                        None
                    }
                })
            })
        });
        if reference.is_some() || entrance_level.is_some() {
            out.insert(
                *index,
                StoreyProperties {
                    reference,
                    entrance_level,
                },
            );
        }
    }
    out
}
