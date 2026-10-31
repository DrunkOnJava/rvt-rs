//! An element's saved parameter values, named by the definitions the file
//! holds, with a family instance's type values after its own (#35, RE-207).
//!
//! Each value comes from the element's own native record
//! ([`crate::native_metadata::SavedMetadata`]): its typed parameter sets
//! (`m_pParamValueSet*`), the family parameter slots of its own
//! `FamilyParams` (`FamilyInstance.m_pInstParams`, `FamilySymbol.m_pParams`)
//! that a definition resolves, and the native fields validated as built-in
//! parameters. A parameter is named only by its definition's `m_caption`:
//! a family, shared, project or global parameter. Built-in parameters keep
//! their negative id and no name, as the built-in catalog is not shipped
//! (#255).
//!
//! A family instance's list is its own values, then its type's
//! (`m_masterSymbolId`) type parameters it does not hold itself. A wall,
//! floor, ceiling or roof gets its type's the same way, through the field
//! its record names its type in ([`SYSTEM_TYPE_LINKS`]). A family
//! parameter whose definition says it is an instance parameter
//! (`m_instanceParam`) is taken only from the instance: the type's slot for
//! it is the family's default, not the instance's value.
//!
//! Values are as saved, in Revit's internal units (feet, square and cubic
//! feet, radians), not evaluated: a formula's or a reporting parameter's
//! saved value is what is read. Lengths, areas, volumes and angles also get
//! the SI value. A Yes/No value is a boolean. An ElementId value that names a
//! material or a family type also gets that element's name.
//!
//! Revit 2015 to 2027, as [`crate::native_document::extract`] reads them (RE-280).

use crate::RevitFile;
use crate::native_document::{self, Options};
use crate::native_parameter_definitions::Definition;
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Where a parameter is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterKind {
    /// Revit's own: a negative BuiltInParameter id, not named here.
    BuiltIn,
    /// A loaded family's parameter (`ParamElemFamily`).
    Family,
    /// A shared parameter (`ParamElemExternal`).
    Shared,
    /// A global parameter (`ParamElemGlobal`).
    Global,
    /// A project parameter, or any other definition owner.
    Project,
    /// A positive id the file holds no definition for.
    Undefined,
}

/// Whether a value is the element's own or its type's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ParameterLevel {
    Instance,
    Type,
}

/// One saved parameter value of an element.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ElementParameter {
    /// The definition's caption; `None` for a built-in or undefined id.
    pub name: Option<String>,
    pub parameter_id: i64,
    pub kind: ParameterKind,
    /// `String`, `Integer`, `Double` or `ElementId`.
    pub storage_type: String,
    pub spec_type_id: Option<String>,
    pub group_type_id: Option<String>,
    /// The saved value, in internal units; a Yes/No value as a boolean.
    pub value: Value,
    pub si_value: Option<f64>,
    pub si_unit: Option<&'static str>,
    /// The name of the material or family type an ElementId value names.
    pub target_name: Option<String>,
    /// The family of the family type an ElementId value names.
    pub target_family: Option<String>,
    pub level: ParameterLevel,
    /// The record the value was read from: the element, or its type.
    pub source_element_id: i64,
    pub source_class: String,
}

/// [`read`]'s result.
#[derive(Debug, Default)]
pub struct ElementParameters {
    /// The requested elements' parameters, by element id.
    pub elements: BTreeMap<u64, Vec<ElementParameter>>,
    /// Family slots that no definition resolves (mostly built-ins, whose
    /// storage type is not known), counted for each element listed: its
    /// own and its type's.
    pub unresolved_family_slots: usize,
    /// Instance parameters a family instance holds no value for, whose
    /// type holds the family's default, counted for each element listed.
    pub instance_parameters_without_value: usize,
}

impl ElementParameters {
    /// An element's parameters as the JSON entries `rvt-elements` and
    /// Python's `decoded_elements()` list, none for an id not read.
    pub fn json_entries(&self, id: Option<u32>) -> Vec<Value> {
        id.and_then(|id| self.elements.get(&u64::from(id)))
            .into_iter()
            .flatten()
            .filter_map(|parameter| serde_json::to_value(parameter).ok())
            .collect()
    }
}

/// What one record contributes.
#[derive(Default)]
struct RecordParameters {
    class_name: String,
    own: Vec<ElementParameter>,
    /// Family parameter ids the record's slots hold that are instance
    /// parameters (on a type: defaults).
    instance_defaults: BTreeSet<i64>,
    unresolved_slots: usize,
    master_symbol: Option<i64>,
}

/// The system-family element records that name their type, as (element
/// record class, the field naming the type, the type record's class).
pub const SYSTEM_TYPE_LINKS: [(&str, &str, &str); 7] = [
    ("SWall", "m_WallAttributesId", "BasicWallType"),
    ("ArcWall", "m_WallAttributesId", "BasicWallType"),
    ("FaceWall", "m_typeId", "BasicWallType"),
    ("Floor", "m_floorAttributesId", "FloorAttributes"),
    ("Ceiling", "m_ceilingAttributesId", "CompoundCeilingType"),
    ("ProfileRoof", "m_roofAttributesId", "RoofAttributes"),
    ("Roof", "m_roofAttributesId", "RoofAttributes"),
];

/// Read the saved parameters of `ids` (ElemTable element ids). Elements
/// with no native record, or whose record does not decode, get none, as
/// does every element of a file from before Revit 2015.
pub fn read(file: &mut RevitFile, ids: &BTreeSet<u64>) -> Result<ElementParameters> {
    read_defined(file, ids, None)
}

/// [`read`], reading nothing where `filter` accepts the name of none of the
/// file's parameter definitions ([`native_document::Options::definition_filter`]).
pub fn read_defined(
    file: &mut RevitFile,
    ids: &BTreeSet<u64>,
    filter: Option<fn(&str) -> bool>,
) -> Result<ElementParameters> {
    read_selected(file, ids, filter, true)
}

/// [`read_defined`] reading the records of `ids` alone: no type, family or
/// material record is selected, so an element gets no type value and an
/// ElementId value no target name. For elements with no type, such as Levels.
pub fn read_own_defined(
    file: &mut RevitFile,
    ids: &BTreeSet<u64>,
    filter: Option<fn(&str) -> bool>,
) -> Result<ElementParameters> {
    read_selected(file, ids, filter, false)
}

fn read_selected(
    file: &mut RevitFile,
    ids: &BTreeSet<u64>,
    filter: Option<fn(&str) -> bool>,
    types: bool,
) -> Result<ElementParameters> {
    if !native_document::NATIVE_RELEASES.contains(&file.layout_release()?) || ids.is_empty() {
        return Ok(ElementParameters::default());
    }
    let selected_classes = if types {
        ["FamilySymbol", "Family", "MaterialElem"]
            .into_iter()
            .chain(SYSTEM_TYPE_LINKS.iter().map(|link| link.2))
            .map(String::from)
            .collect()
    } else {
        BTreeSet::new()
    };
    let options = Options {
        selected_ids: ids.clone(),
        selected_classes,
        releases: native_document::NATIVE_RELEASES,
        definition_filter: filter,
        ..Default::default()
    };
    let mut records: BTreeMap<i64, RecordParameters> = BTreeMap::new();
    let mut materials: BTreeMap<i64, String> = BTreeMap::new();
    let mut families: BTreeMap<i64, String> = BTreeMap::new();
    let mut symbols: BTreeMap<i64, (Option<i64>, String)> = BTreeMap::new();
    native_document::extract(file, &options, |record| {
        let (Some(graph), Some(metadata)) = (&record.graph, &record.saved_metadata) else {
            return Ok(());
        };
        let Ok(id) = i64::try_from(record.identity.element_id) else {
            return Ok(());
        };
        let root = &graph.objects[0];
        let string_field = |class: &str, field: &str| {
            graph
                .objects
                .iter()
                .find(|object| object.class_name == class)
                .and_then(|object| object.fields.get(field))
                .and_then(Value::as_str)
                .map(str::to_string)
        };
        match root.class_name.as_str() {
            "MaterialElem" => {
                if let Some(name) = string_field("Material", "m_name") {
                    materials.insert(id, name);
                }
            }
            "Family" => {
                if let Some(name) = string_field("Family", "m_name") {
                    families.insert(id, name);
                }
            }
            "FamilySymbol" => {
                let family = root
                    .fields
                    .get("m_familyId")
                    .and_then(|value| crate::native_metadata::identifier(value).ok());
                if let Some(name) = string_field("SymbolInfo", "m_name") {
                    symbols.insert(id, (family, name));
                }
            }
            _ => {}
        }
        let is_type = root.class_name == "FamilySymbol"
            || SYSTEM_TYPE_LINKS
                .iter()
                .any(|link| link.2 == root.class_name);
        if !ids.contains(&record.identity.element_id) && !is_type {
            return Ok(());
        }
        let definitions: BTreeMap<i64, &Definition> = metadata
            .parameter_definitions
            .iter()
            .map(|definition| (definition.parameter_id, definition))
            .collect();
        let class_name = root.class_name.clone();
        let entry = |parameter_id: i64, storage_type: &str, raw: &Value| {
            parameter(
                parameter_id,
                storage_type,
                raw,
                definitions.get(&parameter_id).copied(),
                id,
                &class_name,
            )
        };
        let mut own = Vec::new();
        for set in &metadata.parameter_sets {
            for value in &set.parameters {
                own.push(entry(
                    value.serialized_parameter_id,
                    &value.storage_type,
                    &value.raw_value,
                ));
            }
        }
        let mut instance_defaults = BTreeSet::new();
        for value in &metadata.resolved_family_parameters {
            let is_instance = definitions
                .get(&value.parameter_id)
                .and_then(|definition| definition.owner_fields.get("m_instanceParam"))
                .and_then(Value::as_bool)
                == Some(true);
            if is_instance {
                instance_defaults.insert(value.parameter_id);
            }
            own.push(entry(
                value.parameter_id,
                &value.storage_type,
                &value.raw_value,
            ));
        }
        for value in &metadata.raw_field_parameters {
            own.push(entry(
                value.parameter_id,
                &value.storage_type,
                &value.raw_value,
            ));
        }
        // One value per parameter: the first source read keeps it.
        let mut seen = BTreeSet::new();
        own.retain(|value| seen.insert(value.parameter_id));
        // The owner's own FamilyParams, as `project_with_definitions` picks
        // it; a Family record's graph also holds its types' slots.
        let owner_object = match class_name.as_str() {
            "FamilySymbol" => Some("m_pParams"),
            "FamilyInstance" => Some("m_pInstParams"),
            _ => None,
        }
        .and_then(|field| root.fields.get(field))
        .and_then(|pointer| pointer["offset"].as_u64())
        .and_then(|offset| {
            graph
                .edges
                .iter()
                .find(|edge| edge.source_object_index == 0 && edge.pointer_offset as u64 == offset)
        })
        .map(|edge| edge.target_object_index);
        let owner_slots = metadata
            .family_parameter_slots
            .iter()
            .filter(|slot| Some(slot.source_object) == owner_object)
            .count();
        records.insert(
            id,
            RecordParameters {
                class_name: class_name.clone(),
                own,
                instance_defaults,
                unresolved_slots: owner_slots
                    .saturating_sub(metadata.resolved_family_parameters.len()),
                master_symbol: std::iter::once(("FamilyInstance", "m_masterSymbolId"))
                    .chain(SYSTEM_TYPE_LINKS.iter().map(|link| (link.0, link.1)))
                    .find(|link| link.0 == class_name)
                    .and_then(|link| root.fields.get(link.1))
                    .and_then(|value| crate::native_metadata::identifier(value).ok()),
            },
        );
        Ok(())
    })?;
    let mut result = ElementParameters::default();
    for &element_id in ids {
        let Some(record) = i64::try_from(element_id)
            .ok()
            .and_then(|id| records.get(&id))
        else {
            continue;
        };
        let mut values = record.own.clone();
        result.unresolved_family_slots += record.unresolved_slots;
        if let Some(symbol) = record.master_symbol.and_then(|id| records.get(&id)) {
            result.unresolved_family_slots += symbol.unresolved_slots;
            let held: BTreeSet<i64> = values.iter().map(|value| value.parameter_id).collect();
            for value in &symbol.own {
                if held.contains(&value.parameter_id) {
                    continue;
                }
                if symbol.instance_defaults.contains(&value.parameter_id) {
                    result.instance_parameters_without_value += 1;
                    continue;
                }
                let mut value = value.clone();
                value.level = ParameterLevel::Type;
                values.push(value);
            }
        }
        if record.class_name == "FamilySymbol" {
            for value in &mut values {
                value.level = ParameterLevel::Type;
            }
        }
        for value in &mut values {
            let Some(target) = value
                .value
                .as_i64()
                .filter(|_| value.storage_type == "ElementId")
            else {
                continue;
            };
            if let Some(name) = materials.get(&target) {
                value.target_name = Some(name.clone());
            } else if let Some((family, name)) = symbols.get(&target) {
                value.target_name = Some(name.clone());
                value.target_family = family.and_then(|family| families.get(&family)).cloned();
            }
        }
        result.elements.insert(element_id, values);
    }
    Ok(result)
}

fn parameter(
    parameter_id: i64,
    storage_type: &str,
    raw: &Value,
    definition: Option<&Definition>,
    source_element_id: i64,
    source_class: &str,
) -> ElementParameter {
    let kind = match definition.map(|definition| definition.owner_class.as_str()) {
        _ if parameter_id < 0 => ParameterKind::BuiltIn,
        None => ParameterKind::Undefined,
        Some("ParamElemFamily") => ParameterKind::Family,
        Some("ParamElemExternal") => ParameterKind::Shared,
        Some("ParamElemGlobal") => ParameterKind::Global,
        Some(_) => ParameterKind::Project,
    };
    let spec = definition.and_then(|definition| definition.spec_type_id.clone());
    let value = match (spec.as_deref(), raw.as_i64()) {
        (Some(spec), Some(n)) if spec.starts_with("autodesk.spec:spec.bool-") => {
            Value::Bool(n != 0)
        }
        _ => raw.clone(),
    };
    let (si_value, si_unit) = match (spec.as_deref(), raw.as_f64()) {
        (Some(spec), Some(n)) if storage_type == "Double" => si(spec, n),
        _ => (None, None),
    };
    ElementParameter {
        name: definition.map(|definition| definition.caption.clone()),
        parameter_id,
        kind,
        storage_type: storage_type.to_string(),
        spec_type_id: spec,
        group_type_id: definition
            .and_then(|definition| definition.group_type_id.clone())
            .filter(|group| !group.is_empty()),
        value,
        si_value,
        si_unit,
        target_name: None,
        target_family: None,
        level: ParameterLevel::Instance,
        source_element_id,
        source_class: source_class.to_string(),
    }
}

/// A value in Revit's internal units as SI, for the specs whose internal
/// unit is fixed: feet, square feet, cubic feet and radians.
fn si(spec: &str, value: f64) -> (Option<f64>, Option<&'static str>) {
    const FOOT: f64 = 0.3048;
    let Some(kind) = spec
        .strip_prefix("autodesk.spec.aec:")
        .and_then(|rest| rest.split('-').next())
    else {
        return (None, None);
    };
    match kind {
        "length" => (Some(value * FOOT), Some("m")),
        "area" => (Some(value * FOOT * FOOT), Some("m2")),
        "volume" => (Some(value * FOOT * FOOT * FOOT), Some("m3")),
        "angle" => (Some(value), Some("rad")),
        _ => (None, None),
    }
}
