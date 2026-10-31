//! Properties written where Revit's own exporter writes them and IFC4's
//! definitions do not (#35, W7), for `rvt-ifc --revit-set-placement`.
//!
//! By default rvt-rs writes each standard property in the set
//! buildingSMART's IFC4 definitions put it in (RE-194,
//! [`super::standard_property_sets`]). Revit's IFC4 export also writes, as
//! revit-ifc's property set definitions have it (CLEANROOM.md source 5; read
//! and cited, no code copied):
//!
//! - every standard set whose entity's name begins with the element's
//!   entity's name, as revit-ifc matches a set's entities by the start of
//!   the name: `Pset_StairFlightCommon` on a stair (`IfcStairFlight` begins
//!   `IfcStair`), `Pset_SpaceHeaterTypeCommon` on a space,
//!   `Pset_RampFlightCommon` on a ramp. Only its `Reference` is written, the
//!   element's type name, which Revit's export gives every set the same:
//!   its other properties need not equal the element's in its own sets (on
//!   the BIM4LCA office a stair's `Pset_StairFlightCommon.NumberOfTreads`
//!   differs from its `Pset_StairCommon.NumberOfTreads` on 12 of 14);
//! - a project or shared parameter named `<set>.<property>` (RE-194) whose
//!   set applies to the element but whose property buildingSMART's
//!   definition of the set does not have, in that set: a text as `IfcText`,
//!   as Revit's export writes it; and one named for a set matched by name
//!   and `Reference`, which comes before the copied `Reference`. The BIM4LCA office's rooms hold
//!   `Pset_SpaceCommon.FloorCovering`, `WallCovering` and `CeilingCovering`,
//!   which IFC4 moved to `Pset_SpaceCoveringRequirements` (RE-235). A value
//!   of another kind is left out and counted.
//!
//! What a project's own user-defined property set file adds to Revit's
//! export is not written (RE-194). Measured on the BIM4LCA office (RE-235).

use super::IfcModel;
use super::entities::{IfcEntity, Property, PropertySet, PropertyValue};
use super::standard_property_sets::{STANDARD_PROPERTY_SETS, StandardPropertySet};
use crate::RevitFile;
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// What [`attach`] wrote.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct Report {
    /// Elements given at least one property.
    pub elements: usize,
    /// Properties written.
    pub properties: usize,
    /// Parameters named for a set and a property its definition lacks
    /// whose value is not a text, left out.
    pub left_out: usize,
}

/// Whether `set` is one revit-ifc matches to an element written as
/// `ifc_type` by the start of one of its entities' names, and IFC4 does not
/// apply to it.
fn matched_by_name(
    set: &StandardPropertySet,
    ifc_type: &str,
    predefined_type: Option<&str>,
) -> bool {
    !set.applies_to(ifc_type, predefined_type)
        && set.applies.iter().any(|(entity, scoped)| {
            scoped.is_none() && *entity != ifc_type && entity.starts_with(ifc_type)
        })
}

/// A parameter's set, its property and the value type it is written as.
type Placement<'a> = (&'static str, &'a str, fn(String) -> PropertyValue);

/// Where a text parameter named `<set>.<property>` goes on an element
/// written as `ifc_type` with `predefined_type`: in a set that applies to it
/// whose definition lacks the property, as `IfcText`; or, for `Reference`,
/// in a set matched to it by name, as `IfcIdentifier`, before the
/// `Reference` its other sets hold, as revit-ifc reads `<set>.<property>`
/// first (RE-194).
fn parameter_placement<'a>(
    name: &'a str,
    ifc_type: &str,
    predefined_type: Option<&str>,
) -> Option<Placement<'a>> {
    let (set_name, property) = name.split_once('.')?;
    if property.is_empty() || property.contains('.') {
        return None;
    }
    let set = STANDARD_PROPERTY_SETS
        .iter()
        .find(|set| set.name == set_name)?;
    if set.applies_to(ifc_type, predefined_type) && !set.defines(property) {
        return Some((set.name, property, PropertyValue::Text));
    }
    (property == "Reference" && matched_by_name(set, ifc_type, predefined_type)).then_some((
        set.name,
        property,
        PropertyValue::Identifier,
    ))
}

/// The sets and properties Revit's export writes on an element written as
/// `ifc_type` with `predefined_type`, beyond IFC4's: each set, with the
/// names of the properties it may take.
fn revit_placements(
    ifc_type: &str,
    predefined_type: Option<&str>,
) -> Vec<(&'static str, Vec<&'static str>)> {
    STANDARD_PROPERTY_SETS
        .iter()
        .filter(|set| matched_by_name(set, ifc_type, predefined_type))
        .map(|set| {
            (
                set.name,
                set.properties
                    .iter()
                    .map(|(name, _)| *name)
                    .filter(|name| *name == "Reference")
                    .collect(),
            )
        })
        .collect()
}

/// The properties of a stair type's `Pset_StairCommon` that Revit's export
/// repeats in the `Pset_StairFlightCommon` of the same type object (RE-286).
const STAIR_TYPE_DIMENSIONS: [&str; 3] = ["NosingLength", "NumberOfRiser", "NumberOfTreads"];

/// Give each stair type object that holds the stair dimensions of
/// `Pset_StairCommon` the same in `Pset_StairFlightCommon`, which revit-ifc
/// matches to it by the start of the flight entity's name (RE-286). Returns
/// the properties written.
fn add_stair_type_flight_sets(model: &mut IfcModel) -> usize {
    let mut added = 0;
    for by_entity in model.type_entity_sets.values_mut() {
        let Some(sets) = by_entity.get_mut("IFCSTAIRTYPE") else {
            continue;
        };
        let Some(common) = sets.iter().find(|set| set.name == "Pset_StairCommon") else {
            continue;
        };
        let properties: Vec<Property> = common
            .properties
            .iter()
            .filter(|property| STAIR_TYPE_DIMENSIONS.contains(&property.name.as_str()))
            .cloned()
            .collect();
        if properties.is_empty() {
            continue;
        }
        added += properties.len();
        sets.push(PropertySet {
            name: "Pset_StairFlightCommon".into(),
            properties,
        });
    }
    added
}

/// Add to each building element of `model` the sets and properties Revit's
/// export writes beyond IFC4's definitions (see the module doc), merged
/// into a set of the same name the element has: a parameter's text where a
/// parameter gives it, otherwise the value the element already has under
/// the property's name in another set.
pub fn attach(file: &mut RevitFile, model: &mut IfcModel) -> Result<Report> {
    // Every piece an element is written as carries its ElementId.
    let mut by_id: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                by_id.entry(id).or_default().push(index);
            }
        }
    }
    let ids: BTreeSet<u64> = by_id.keys().copied().collect();
    let saved = crate::research::native_element_parameters::read(file, &ids)?;
    let mut held: BTreeMap<(usize, String), usize> = BTreeMap::new();
    let mut values: BTreeMap<usize, BTreeMap<String, Property>> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        match entity {
            IfcEntity::BuildingElement {
                property_set: Some(set),
                ..
            } => {
                let found = values.entry(position).or_default();
                for property in &set.properties {
                    found
                        .entry(property.name.clone())
                        .or_insert_with(|| property.clone());
                }
            }
            IfcEntity::ElementPropertySet { element, set } => {
                held.entry((*element, set.name.clone())).or_insert(position);
                let found = values.entry(*element).or_default();
                for property in &set.properties {
                    found
                        .entry(property.name.clone())
                        .or_insert_with(|| property.clone());
                }
            }
            _ => {}
        }
    }
    let mut additions: Vec<(usize, &'static str, Vec<Property>)> = Vec::new();
    let mut left_out = 0;
    for (id, indices) in &by_id {
        let Some(parameters) = saved.elements.get(id) else {
            continue;
        };
        for (piece, &index) in indices.iter().enumerate() {
            let Some(IfcEntity::BuildingElement {
                ifc_type,
                predefined_type,
                ..
            }) = model.entities.get(index)
            else {
                continue;
            };
            let mut seen: BTreeSet<&str> = BTreeSet::new();
            for parameter in parameters {
                let Some(name) = parameter.name.as_deref() else {
                    continue;
                };
                let Some((set, property, kind)) =
                    parameter_placement(name, ifc_type, predefined_type.as_deref())
                else {
                    continue;
                };
                if !seen.insert(name) {
                    continue;
                }
                match parameter.value.as_str().filter(|text| !text.is_empty()) {
                    Some(text) => additions.push((
                        index,
                        set,
                        vec![Property {
                            name: property.to_string(),
                            value: kind(text.to_string()),
                        }],
                    )),
                    None if piece == 0 => left_out += 1,
                    None => {}
                }
            }
        }
    }
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            ifc_type,
            predefined_type,
            ..
        } = entity
        else {
            continue;
        };
        let Some(found) = values.get(&index) else {
            continue;
        };
        for (set, names) in revit_placements(ifc_type, predefined_type.as_deref()) {
            let properties: Vec<Property> = names
                .iter()
                .filter_map(|name| found.get(*name).cloned())
                .collect();
            if !properties.is_empty() {
                additions.push((index, set, properties));
            }
        }
    }
    additions.extend(stair_flight_additions(&model.entities));
    let mut report = Report {
        left_out,
        ..Report::default()
    };
    let mut touched: BTreeSet<usize> = BTreeSet::new();
    for (index, set, properties) in additions {
        let mut added = 0;
        match held.get(&(index, set.to_string())) {
            Some(&position) => {
                if let Some(IfcEntity::ElementPropertySet { set: held_set, .. }) =
                    model.entities.get_mut(position)
                {
                    for property in properties {
                        if !held_set.properties.iter().any(|p| p.name == property.name) {
                            held_set.properties.push(property);
                            added += 1;
                        }
                    }
                }
            }
            None => {
                added = properties.len();
                held.insert((index, set.to_string()), model.entities.len());
                model.entities.push(IfcEntity::ElementPropertySet {
                    element: index,
                    set: PropertySet {
                        name: set.to_string(),
                        properties,
                    },
                });
            }
        }
        if added > 0 {
            touched.insert(index);
            report.properties += added;
        }
    }
    report.properties += add_stair_type_flight_sets(model);
    report.elements = touched.len();
    Ok(report)
}

/// The counts of `Pset_StairFlightCommon` a stair holds as the sum over its
/// flights.
const STAIR_FLIGHT_COUNTS: [&str; 2] = ["NumberOfRiser", "NumberOfTreads"];

/// The lengths of `Pset_StairFlightCommon` a stair holds as its flights'
/// common value.
const STAIR_FLIGHT_LENGTHS: [&str; 5] = [
    "RiserHeight",
    "TreadLength",
    "NosingLength",
    "TreadLengthAtOffset",
    "TreadLengthAtInnerSide",
];

/// Whether two lengths in feet are one value, within the rounding of the
/// flights' own computations.
fn same_length(a: &PropertyValue, b: &PropertyValue) -> bool {
    match (a, b) {
        (PropertyValue::PositiveLengthFeet(a), PropertyValue::PositiveLengthFeet(b))
        | (PropertyValue::LengthFeet(a), PropertyValue::LengthFeet(b)) => {
            (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
        }
        _ => false,
    }
}

/// The `Pset_StairFlightCommon` values Revit's export gives a stair, beside
/// the flights' own (RE-287): `NumberOfRiser` and `NumberOfTreads` are the sums
/// over the stair's flights, and each length is the flights' value, which
/// their calculators take from the stair itself. Measured on the BIM4LCA
/// office, as built and at permit: all 14 stairs with flights (one to three).
/// A property is written only where every flight holds it and, for a length,
/// all agree.
fn stair_flight_additions(entities: &[IfcEntity]) -> Vec<(usize, &'static str, Vec<Property>)> {
    const SET: &str = "Pset_StairFlightCommon";
    let is_entity = |index: usize, wanted: &str| {
        matches!(
            entities.get(index),
            Some(IfcEntity::BuildingElement { ifc_type, .. }) if ifc_type == wanted
        )
    };
    let mut flights: BTreeMap<usize, &PropertySet> = BTreeMap::new();
    for entity in entities {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            if set.name == SET && is_entity(*element, "IFCSTAIRFLIGHT") {
                flights.entry(*element).or_insert(set);
            }
        }
    }
    let mut additions = Vec::new();
    for entity in entities {
        let IfcEntity::Aggregate { whole, parts } = entity else {
            continue;
        };
        if !is_entity(*whole, "IFCSTAIR") {
            continue;
        }
        let runs: Vec<&PropertySet> = parts
            .iter()
            .filter(|part| is_entity(**part, "IFCSTAIRFLIGHT"))
            .map(|part| flights.get(part).copied())
            .collect::<Option<_>>()
            .unwrap_or_default();
        if runs.is_empty() {
            continue;
        }
        let mut properties = Vec::new();
        for name in STAIR_FLIGHT_COUNTS {
            let counts: Option<Vec<i64>> = runs
                .iter()
                .map(|set| {
                    set.properties
                        .iter()
                        .find_map(|property| match &property.value {
                            PropertyValue::CountValue(count) if property.name == name => {
                                Some(*count)
                            }
                            _ => None,
                        })
                })
                .collect();
            if let Some(counts) = counts {
                properties.push(Property {
                    name: name.into(),
                    value: PropertyValue::CountValue(counts.iter().sum()),
                });
            }
        }
        for name in STAIR_FLIGHT_LENGTHS {
            let found: Option<Vec<&PropertyValue>> = runs
                .iter()
                .map(|set| {
                    set.properties
                        .iter()
                        .find(|property| property.name == name)
                        .map(|property| &property.value)
                })
                .collect();
            let Some(found) = found else {
                continue;
            };
            let first = found[0];
            if found.iter().all(|value| same_length(first, value)) {
                properties.push(Property {
                    name: name.into(),
                    value: first.clone(),
                });
            }
        }
        if !properties.is_empty() {
            additions.push((*whole, SET, properties));
        }
    }
    additions
}
