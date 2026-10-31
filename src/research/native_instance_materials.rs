//! The materials a family instance's own parameters give it, from its native
//! record (RE-200, #355).
//!
//! A family instance's record holds its instance parameter values in the
//! `FamilyParams` object its `m_pInstParams` points to: one `m_params` entry
//! per parameter, whose `m_elemId` is the value of an ElementId parameter. A
//! material parameter's value is a material's ElementId. Geometry bound to
//! such a parameter has no fixed material in its type's map (RE-82: the
//! value is unset), so the instance's value is its material. On the R2025
//! models, the window trims and muntins of `12_Rooms_Door_Windows.rvt` to
//! `17_Rooms_Stairs_Spiral.rvt` have only unset values in their types' maps,
//! and their instance parameters name Revit's set on every one (RE-200).

use crate::{RevitFile, native_document, native_metadata::identifier};
use std::collections::{BTreeMap, BTreeSet};

/// The materials among the instance parameter values of each of
/// `instances`, by the instance's ElementId, in the order its parameters
/// hold them, each once. An ElementId that is not one of `materials` is not
/// a material value (a parameter can name another element). An instance
/// with no material value is left out. Errs where the native record path
/// does not read the file's release.
pub fn instance_parameter_materials(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
    materials: &BTreeSet<u32>,
) -> anyhow::Result<BTreeMap<u64, Vec<u32>>> {
    let mut out: BTreeMap<u64, Vec<u32>> = BTreeMap::new();
    if instances.is_empty() {
        return Ok(out);
    }
    let options = native_document::Options {
        selected_ids: instances.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        if root.class_name != "FamilyInstance" {
            return Ok(());
        }
        // Only the object the root's own `m_pInstParams` points to: the
        // record's graph can hold other parameter objects.
        let Some(offset) = root
            .fields
            .get("m_pInstParams")
            .and_then(|pointer| pointer["offset"].as_u64())
        else {
            return Ok(());
        };
        let Some(target) = graph
            .edges
            .iter()
            .find(|edge| edge.source_object_index == 0 && edge.pointer_offset as u64 == offset)
            .and_then(|edge| graph.objects.get(edge.target_object_index))
        else {
            return Ok(());
        };
        if target.class_name != "FamilyParams" {
            return Ok(());
        }
        let Some(params) = target.fields.get("m_params").and_then(|v| v.as_array()) else {
            return Ok(());
        };
        let mut found: Vec<u32> = Vec::new();
        for value in params {
            let Some(id) = value
                .get("m_elemId")
                .and_then(|id| identifier(id).ok())
                .and_then(|id| u32::try_from(id).ok())
            else {
                continue;
            };
            if materials.contains(&id) && !found.contains(&id) {
                found.push(id);
            }
        }
        if !found.is_empty() {
            out.insert(record.identity.element_id, found);
        }
        Ok(())
    })?;
    Ok(out)
}
