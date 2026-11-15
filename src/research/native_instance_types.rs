//! The type of a Revit 2015 to 2023 family instance, read from its own native
//! record and the records of the symbol and family it names (RE-477, #1603;
//! RE-475 found where the type is).
//!
//! The scan's reference-list join (`type_and_family_names_2023`) names a type
//! only when the list holds a symbol record with a name entry. On the spent
//! Revit 2018 project 68 of the 70 windows name no such symbol: the one named id
//! only window records hold is the type's master symbol, not Revit's type, and
//! Revit's own type id is in the list as an id with neither a record nor a
//! name. The native record names all of it by member name:
//! - Revit's type id is the `m_symbolId` of the instance's `InstanceInfo` (the
//!   Tag of the type in Revit's export, 70 of 70 windows), or the master symbol
//!   where the instance is a member of a model group (`ElementGroupMembership`),
//!   whose symbol is then a copy of the master made for that group's instance
//!   (the Tag of the type in Revit's export, 2 of 2 doors);
//! - the type's name is the `SymbolInfo.m_name` of the symbol the instance's
//!   `m_masterSymbolId` names (equal to Revit's on 70 of 70; the `SymbolInfo` of
//!   the symbol at `m_symbolId` holds none);
//! - the family is the `Family` record the master symbol's `m_familyId` names,
//!   and its name `Family.m_name`.
//!
//! An ElementId is read in either shape the schema saves it, `{"m_id":{"m_id64":n}}`
//! from Revit 2024 and `{"m_id":{"m_id":n}}` before (RE-280).

use crate::graph_value::Fv;
use crate::native_json::identifier_u64;
use crate::native_parameters::ObjectGraph;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// A family instance's type as Revit's export names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceType {
    /// Revit's ElementId for the type: the `m_symbolId` of the instance's
    /// `InstanceInfo`, or its master symbol's where the instance is a member
    /// of a model group.
    pub type_id: u64,
    /// The type's name: the `SymbolInfo.m_name` of the instance's master symbol.
    pub type_name: String,
    /// The family's name: `Family.m_name` of the family the master symbol names.
    pub family_name: String,
}

/// The non-empty string `field` of the first object of `class` in `graph`.
fn text(graph: &ObjectGraph, class: &str, field: &str) -> Option<String> {
    graph
        .objects
        .iter()
        .find(|object| object.class_name == class)
        .and_then(|object| object.fields.get(field))
        .and_then(Fv::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

/// Whether `graph` is the record of a member of a model group: its
/// `ElementGroupMembership` names a nonzero `m_groupId`.
fn in_group(graph: &ObjectGraph) -> bool {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "ElementGroupMembership")
        .filter_map(|object| object.fields.get("m_groupId"))
        .filter_map(identifier_u64)
        .any(|group| group != 0)
}

/// The nonzero ElementId in `field` of the root object of `graph`.
fn root_id(graph: &ObjectGraph, field: &str) -> Option<u64> {
    graph
        .objects
        .first()
        .and_then(|root| root.fields.get(field))
        .and_then(identifier_u64)
        .filter(|id| *id != 0)
}

/// The type of each of `instances` (family instances' ElementIds) that its
/// native record, its master symbol's and that symbol's family's name in full.
/// An instance whose record is not a `FamilyInstance`, that names no symbol or
/// no master symbol, whose master symbol is not a `FamilySymbol` with a name
/// and a family, or whose family is not a `Family` with a name, is left out.
/// So is a member of a model group whose `m_symbolId` is neither its master
/// symbol nor a symbol whose `m_masterId` names it: the master is then a
/// guess. Each record is the current copy the native record path routes to.
/// One pass reads the instances, every symbol and every family, since which
/// symbols they name is known only once they are read. Errs where the native
/// record path does not read the file's release.
pub fn instance_types(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, InstanceType>> {
    if instances.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let mut grouped: BTreeSet<u64> = BTreeSet::new();
    let mut masters: BTreeMap<u64, u64> = BTreeMap::new();
    let mut symbol_masters: BTreeMap<u64, u64> = BTreeMap::new();
    let mut symbol_names: BTreeMap<u64, String> = BTreeMap::new();
    let mut symbol_families: BTreeMap<u64, u64> = BTreeMap::new();
    let mut family_names: BTreeMap<u64, String> = BTreeMap::new();
    let options = native_document::Options {
        selected_ids: instances.clone(),
        selected_classes: BTreeSet::from(["FamilySymbol".to_string(), "Family".to_string()]),
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match record.class_name.as_deref() {
            Some("FamilySymbol") => {
                if let Some(master) = root_id(graph, "m_masterId") {
                    symbol_masters.insert(id, master);
                }
                if let (Some(name), Some(family)) = (
                    text(graph, "SymbolInfo", "m_name"),
                    root_id(graph, "m_familyId"),
                ) {
                    symbol_names.insert(id, name);
                    symbol_families.insert(id, family);
                }
            }
            Some("Family") => {
                if let Some(name) = text(graph, "Family", "m_name") {
                    family_names.insert(id, name);
                }
            }
            Some("FamilyInstance") if instances.contains(&id) => {
                let symbol = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "InstanceInfo")
                    .and_then(|info| info.fields.get("m_symbolId"))
                    .and_then(identifier_u64)
                    .filter(|symbol| *symbol != 0);
                if let Some((symbol, master)) = symbol.zip(root_id(graph, "m_masterSymbolId")) {
                    drawn_by.insert(id, symbol);
                    masters.insert(id, master);
                    if in_group(graph) {
                        grouped.insert(id);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(masters
        .iter()
        .filter_map(|(&instance, master)| {
            let drawn = *drawn_by.get(&instance)?;
            let type_id = if grouped.contains(&instance) {
                // The symbol that draws a group's instance is a copy of the
                // master made for it; anything else is not measured.
                (drawn == *master || symbol_masters.get(&drawn) == Some(master))
                    .then_some(*master)?
            } else {
                drawn
            };
            let type_name = symbol_names.get(master)?.clone();
            let family_name = family_names.get(symbol_families.get(master)?)?.clone();
            Some((
                instance,
                InstanceType {
                    type_id,
                    type_name,
                    family_name,
                },
            ))
        })
        .collect())
}
