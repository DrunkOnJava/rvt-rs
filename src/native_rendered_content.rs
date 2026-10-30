//! Family types whose family is rendered content, from the native records
//! of the type and its family (RE-202, #355).
//!
//! A family's record holds `Family.m_eRenderModelType`, and a type's names
//! its family in `m_familyId`. Revit's export writes exactly one material,
//! `<Unnamed>`, for every instance of a family whose value is 4, whatever
//! its type's geometry maps: on Snowdon Towers the 17 "Tree - Honey-Locust",
//! 100 RPC Shrub, 6 RPC Female and 4 RPC Male, 127 of 127. No family of
//! Core Interior or RE1 has the value.

use crate::native_json::element_id;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// `Family.m_eRenderModelType` of rendered content.
pub const RENDERED_CONTENT: i64 = 4;

/// Those of `types` whose family's `m_eRenderModelType` is
/// [`RENDERED_CONTENT`]. Errs where the native record path does not read the
/// file's release.
pub fn rendered_content_types(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    if types.is_empty() {
        return Ok(BTreeSet::new());
    }
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: types.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        if let Some(family) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .and_then(|root| root.fields.get("m_familyId"))
            .and_then(element_id)
        {
            family_of.insert(record.identity.element_id, family);
        }
        Ok(())
    })?;
    let mut rendered: BTreeSet<u64> = BTreeSet::new();
    let options = native_document::Options {
        selected_ids: family_of.values().copied().collect(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let render_type = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
            .filter(|root| root.class_name == "Family")
            .and_then(|root| root.fields.get("m_eRenderModelType"))
            .and_then(serde_json::Value::as_i64);
        if render_type == Some(RENDERED_CONTENT) {
            rendered.insert(record.identity.element_id);
        }
        Ok(())
    })?;
    Ok(family_of
        .into_iter()
        .filter(|(_, family)| rendered.contains(family))
        .map(|(type_id, _)| type_id)
        .collect())
}
