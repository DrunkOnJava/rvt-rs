//! The materials a family type's geometry is drawn in, from the type's
//! native record (RE-203, #355).
//!
//! RE-82 reads a type's geometry-material map by scanning its value block
//! for a counted map of material ElementIds. The map is the type's
//! `FamilySymbol.m_geomTag2MaterialId`: its keys are geometry tags, its
//! values material ElementIds, −1 where the type fixes no material (RE-149).
//! Where the type nests a component Revit exports as an element of its own,
//! the scan also picks up the component's materials: Snowdon Towers' 21
//! "Counter Top w Sink Hole" take their sink's three appliance materials as
//! well as "Counter Top". Read by name, the map holds "Counter Top" alone,
//! Revit's set (RE-203). Geometry mapped to −1 is exported as one more
//! material, `<Unnamed>` (RE-149).

use crate::native_json::element_id;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// A type's geometry materials: the materials its map names, in map order,
/// each once, and whether it also maps geometry to no material (−1).
#[derive(Clone, Debug, Default)]
pub struct GeometryMaterials {
    pub materials: Vec<u32>,
    pub unset: bool,
    /// The type's family (`m_familyId`).
    pub family: Option<u64>,
}

/// The materials each of `types` maps its geometry to, by the type's
/// ElementId: the values of its `m_geomTag2MaterialId` that are among
/// `materials`, and whether any value is −1. A type whose map names none of
/// them is left out. Errs where the native record path does not read the
/// file's release.
pub fn geometry_materials(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
    materials: &BTreeSet<u32>,
) -> anyhow::Result<BTreeMap<u64, GeometryMaterials>> {
    let mut out = BTreeMap::new();
    if types.is_empty() {
        return Ok(out);
    }
    let options = native_document::Options {
        selected_ids: types.clone(),
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
        let Some(map) = root
            .fields
            .get("m_geomTag2MaterialId")
            .and_then(serde_json::Value::as_array)
        else {
            return Ok(());
        };
        let mut found = GeometryMaterials {
            family: root.fields.get("m_familyId").and_then(element_id),
            ..GeometryMaterials::default()
        };
        for entry in map {
            let Some(id) = entry
                .pointer("/second/m_id/m_id64")
                .and_then(serde_json::Value::as_i64)
            else {
                continue;
            };
            if id == -1 {
                found.unset = true;
            } else if let Some(id) = u32::try_from(id)
                .ok()
                .filter(|id| materials.contains(id) && !found.materials.contains(id))
            {
                found.materials.push(id);
            }
        }
        if !found.materials.is_empty() {
            out.insert(record.identity.element_id, found);
        }
        Ok(())
    })?;
    Ok(out)
}
