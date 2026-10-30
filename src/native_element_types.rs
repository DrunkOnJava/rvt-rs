//! The type a pipe, wall sweep or ramp names in its own native record
//! (RE-240).
//!
//! A pipe's `RbsPipeCurve` record names its type in `m_idType`, a wall
//! sweep's `WallSweep` record in `m_typeId` and a ramp's `Ramp` record in
//! `m_rampAttrId`; the type's record, an `RbsPipeType`, `CorniceAttr` or
//! `RampAttributes`, holds its name in `SymbolInfo.m_name`. On both BIM4LCA
//! office files (Revit 2024) that is Revit's type for all 12 pipes, 9 wall
//! sweeps and 5 ramps, which neither RE-130's stored layout nor RE-69's
//! reference list types.

use crate::native_json::{agree, element_id};
use crate::{RevitFile, native_document};
use std::collections::BTreeMap;

/// For each decoded class read here: the element's record class, the member
/// naming its type, and the type's record class.
const KINDS: [(&str, &str, &str, &str); 3] = [
    ("Pipe", "RbsPipeCurve", "m_idType", "RbsPipeType"),
    ("WallSweep", "WallSweep", "m_typeId", "CorniceAttr"),
    ("Ramp", "Ramp", "m_rampAttrId", "RampAttributes"),
];

/// Whether [`system_type_names`] reads elements of decoded class `class`.
pub fn reads_class(class: &str) -> bool {
    KINDS.iter().any(|kind| kind.0 == class)
}

/// The type id and name of each of `elements` (ElementId to decoded class)
/// whose record is of its class's record class and names a type whose
/// record is of the matching type class and has a name, read in one pass.
/// An element or type whose copies disagree is left out.
pub fn system_type_names(
    rf: &mut RevitFile,
    elements: &BTreeMap<u64, String>,
) -> anyhow::Result<BTreeMap<u64, (u64, String)>> {
    if elements.is_empty() {
        return Ok(BTreeMap::new());
    }
    let options = native_document::Options {
        selected_ids: elements.keys().copied().collect(),
        selected_classes: KINDS.iter().map(|kind| kind.3.to_string()).collect(),
        ..native_document::Options::default()
    };
    let mut types: BTreeMap<u64, Option<(u64, &'static str)>> = BTreeMap::new();
    let mut names: BTreeMap<u64, Option<(String, &'static str)>> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if let Some(kind) = KINDS.iter().find(|kind| kind.3 == class) {
            let name = graph
                .objects
                .iter()
                .find(|object| object.class_name == "SymbolInfo")
                .and_then(|object| object.fields.get("m_name"))
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .map(|name| (name.to_string(), kind.3));
            agree(&mut names, id, name);
        }
        let Some(kind) = elements.get(&id).and_then(|decoded| {
            KINDS
                .iter()
                .find(|kind| kind.0 == decoded.as_str() && kind.1 == class)
        }) else {
            return Ok(());
        };
        let named = graph
            .objects
            .iter()
            .filter(|object| object.class_name == kind.1)
            .find_map(|object| object.fields.get(kind.2).and_then(element_id))
            .map(|type_id| (type_id, kind.3));
        agree(&mut types, id, named);
        Ok(())
    })?;
    Ok(types
        .into_iter()
        .filter_map(|(element, named)| {
            let (type_id, type_class) = named?;
            let (name, class) = names.get(&type_id)?.as_ref()?;
            (*class == type_class).then(|| (element, (type_id, name.clone())))
        })
        .collect())
}
