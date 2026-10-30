//! A wall's type Function, from the native records of the wall and its
//! type (RE-185, RE-193).
//!
//! A wall's native record names its type in `m_WallAttributesId`, and the
//! type's record holds its Function as `m_function`, the enum Revit's API
//! calls `WallFunction`: 0 interior, 1 exterior, then foundation, retaining,
//! soffit and core shaft. No wall type holds a `FUNCTION_PARAM` entry in its
//! data object (RE-185), so a wall's Function is read here. It is Revit's
//! `IsExternal` on RE1 Architecture's 8 walls and on the 117 walls of the
//! R2025 models (RE-185), and on the 2 walls of the BIM4LCA office no
//! parameter decides (RE-193).

use crate::native_json::identifier_i64;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// The type Function (`m_function`) of each of `walls`, by the wall's
/// ElementId: the wall's record names its type, and the type's record holds
/// the Function. A wall whose type or Function is not found is left out.
/// Read on Revit 2015 to 2027, the type's id in either shape of `Identifier`
/// (RE-283). Errs where the native record path does not read the file's
/// release.
pub fn wall_type_functions(
    rf: &mut RevitFile,
    walls: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, i64>> {
    let mut wall_type: BTreeMap<u64, u64> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: walls.clone(),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(type_id) = graph.objects.iter().find_map(|object| {
            let id = identifier_i64(object.fields.get("m_WallAttributesId")?)?;
            u64::try_from(id).ok()
        }) {
            wall_type.insert(record.identity.element_id, type_id);
        }
        Ok(())
    })?;
    let mut function: BTreeMap<u64, i64> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: wall_type.values().copied().collect(),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(value) = graph.objects.iter().find_map(|object| {
            object
                .fields
                .get("m_function")
                .and_then(serde_json::Value::as_i64)
        }) {
            function.insert(record.identity.element_id, value);
        }
        Ok(())
    })?;
    Ok(wall_type
        .into_iter()
        .filter_map(|(wall, type_id)| Some((wall, *function.get(&type_id)?)))
        .collect())
}
