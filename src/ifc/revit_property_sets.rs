//! Revit's own property sets, as its exporter writes them when "Export Revit
//! property sets" is ticked (#35, RE-210): an element's named saved
//! parameters (RE-207, `research::native_element_parameters`) in one set per
//! parameter group, named for the group, and its type's in sets of the same
//! kind on its type object.
//!
//! Read in revit-ifc's `Release_25.x.x` branch (CLEANROOM.md source 5; read
//! and cited, no code copied):
//!
//! - `ParameterUtil.CacheParametersForElement` takes an element's parameters
//!   by parameter id, groups them by their group type id and leaves out the
//!   IFC group (`GroupTypeId.Ifc`); within a group it keeps one parameter per
//!   name with its spaces removed (`NamingUtil.RemoveSpaces`), the first.
//! - `PropertyUtil.CreateInternalRevitPropertySets` makes the sets of an
//!   element from its own parameters and, separately, those of its type from
//!   the type's, on the type object; a type with no type object gets its sets
//!   on its elements, named with the suffix `(Type)`. Each set is named by
//!   `LabelUtils.GetLabelForGroup`.
//! - A Yes/No is written as a Boolean, an integer of another spec as an
//!   integer (one with no spec, an enumeration, by its display text), a
//!   non-empty text as text, a real by its spec
//!   (`CreateRealPropertyByType`: a length, pipe or duct size as a length
//!   measure, an area, a volume, an angle as a plane angle, a number as a
//!   real, an air flow as a volumetric flow rate, an HVAC or piping
//!   pressure as a pressure, an apparent or electrical power as a power, an
//!   electrical potential as a voltage, a current as a current; each in SI
//!   units), and an ElementId by the display name of what it names: a
//!   material's name, a family type's as `<family> : <type>` (the type's
//!   name alone where the family has the same name).
//!
//! Only parameters the file names are written: built-in parameters have no
//! name here (#255). An enumeration and a real of a spec not listed above
//! are left out and counted.
//!
//! The group labels are Revit's English labels for the group type ids the
//! measured models use; a group not in [`GROUP_LABELS`] is left out.

use super::IfcModel;
use super::entities::{IfcEntity, Property, PropertySet, PropertyValue};
use crate::RevitFile;
use crate::research::native_element_parameters::{ElementParameter, ParameterLevel};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

/// The label of each parameter group, by its group type id without the
/// `autodesk.parameter.group:` prefix and version. A parameter with no group
/// is in `Other`. The IFC group (`ifc`) is not here: Revit writes no set of
/// it.
pub const GROUP_LABELS: &[(&str, &str)] = &[
    ("", "Other"),
    ("adskModelProperties", "Model Properties"),
    ("analysisResults", "Analysis Results"),
    ("constraints", "Constraints"),
    ("construction", "Construction"),
    ("data", "Data"),
    ("dimensions", "Dimensions"),
    ("electrical", "Electrical"),
    ("electricalEngineering", "Electrical Engineering"),
    ("electricalCircuiting", "Electrical - Circuiting"),
    ("electricalLighting", "Electrical - Lighting"),
    ("electricalLoads", "Electrical - Loads"),
    ("fireProtection", "Fire Protection"),
    ("general", "General"),
    ("graphics", "Graphics"),
    ("identityData", "Identity Data"),
    ("lightPhotometrics", "Photometrics"),
    ("materials", "Materials and Finishes"),
    ("mechanical", "Mechanical"),
    ("mechanicalAirflow", "Mechanical - Flow"),
    ("plumbing", "Plumbing"),
    ("structural", "Structural"),
    ("text", "Text"),
    ("visibility", "Visibility"),
];

/// The specs of reals Revit writes as a length measure, of those the measured
/// models use: length, pipe size and duct size.
const LENGTH_SPECS: &[&str] = &[
    "autodesk.spec.aec:length-",
    "autodesk.spec.aec.piping:pipeSize-",
    "autodesk.spec.aec.hvac:ductSize-",
];

/// The specs of reals Revit writes as a pressure, of those the measured
/// models use: HVAC and piping pressure.
const PRESSURE_SPECS: &[&str] = &[
    "autodesk.spec.aec.hvac:pressure-",
    "autodesk.spec.aec.piping:pressure-",
];

/// The specs of reals Revit writes as a power, of those the measured models
/// use: apparent power and electrical power.
const POWER_SPECS: &[&str] = &[
    "autodesk.spec.aec.electrical:apparentPower-",
    "autodesk.spec.aec.electrical:power-",
];

const METRES_PER_FOOT: f64 = 0.3048;
const SQUARE_METRES_PER_SQUARE_FOOT: f64 = METRES_PER_FOOT * METRES_PER_FOOT;
const CUBIC_METRES_PER_CUBIC_FOOT: f64 = SQUARE_METRES_PER_SQUARE_FOOT * METRES_PER_FOOT;

/// The suffix of a type's set written on its elements, where the type has no
/// type object (revit-ifc `Properties.Resources.PropertySetTypeSuffix`; not
/// translated: a German Revit 2022 export writes `ID-Daten(Type)`).
pub const TYPE_SUFFIX: &str = "(Type)";

/// What [`attach`] wrote.
#[derive(Debug, Default)]
pub struct Report {
    /// Elements given at least one property.
    pub elements: usize,
    /// Type objects' types given at least one property.
    pub types: usize,
    /// Elements given their type's sets, named with [`TYPE_SUFFIX`], their
    /// type having no type object.
    pub untyped_elements: usize,
    /// Properties written, on elements and types.
    pub properties: usize,
    /// Named parameters left out: no value Revit would write, or a kind
    /// not written yet (see the module doc).
    pub left_out: usize,
}

/// The label of `group_type_id`'s set, `None` for a group not known.
pub fn group_label(group_type_id: Option<&str>) -> Option<&'static str> {
    let key = group_type_id
        .map(|id| {
            let id = id.strip_prefix("autodesk.parameter.group:").unwrap_or(id);
            id.rsplit_once('-').map_or(id, |(name, _)| name)
        })
        .unwrap_or("");
    GROUP_LABELS
        .iter()
        .find(|(id, _)| *id == key)
        .map(|(_, label)| *label)
}

/// `parameter` as the property Revit's exporter writes for it, `None` where
/// it writes none or the kind is not written yet. Public with
/// `unstable-research`, whose `ElementParameter` it takes.
#[cfg(feature = "unstable-research")]
pub fn property(parameter: &ElementParameter) -> Option<Property> {
    revit_property(parameter)
}

fn revit_property(parameter: &ElementParameter) -> Option<Property> {
    let name = parameter.name.clone()?;
    let spec = parameter.spec_type_id.as_deref().unwrap_or("");
    let spec_kind = |prefix: &str| spec.starts_with(prefix);
    let value = match parameter.storage_type.as_str() {
        "String" => match parameter.value.as_str() {
            Some(text) if !text.is_empty() => PropertyValue::Text(text.to_string()),
            _ => return None,
        },
        "Integer" => match &parameter.value {
            serde_json::Value::Bool(b) => PropertyValue::Boolean(*b),
            // No spec: an enumeration, written by its display text.
            _ if spec.is_empty() => return None,
            value => PropertyValue::Integer(value.as_i64()?),
        },
        "Double" => {
            let v = parameter.value.as_f64()?;
            if LENGTH_SPECS.iter().any(|prefix| spec_kind(prefix)) {
                PropertyValue::LengthFeet(v)
            } else if spec_kind("autodesk.spec.aec:area-") {
                PropertyValue::AreaSquareFeet(v)
            } else if spec_kind("autodesk.spec.aec:volume-") {
                PropertyValue::VolumeCubicFeet(v)
            } else if spec_kind("autodesk.spec.aec:angle-") {
                PropertyValue::AngleRadians(v)
            } else if spec_kind("autodesk.spec.aec:number-") {
                PropertyValue::Real(v)
            } else if spec_kind("autodesk.spec.aec.hvac:airFlow-") {
                // Revit's internal units: ft³/s, kg/(ft·s²), kg·ft²/s³ and
                // kg·ft²/(s³·A).
                PropertyValue::VolumetricFlowRate(v * CUBIC_METRES_PER_CUBIC_FOOT)
            } else if PRESSURE_SPECS.iter().any(|prefix| spec_kind(prefix)) {
                PropertyValue::Pressure(v / METRES_PER_FOOT)
            } else if POWER_SPECS.iter().any(|prefix| spec_kind(prefix)) {
                PropertyValue::Power(v * SQUARE_METRES_PER_SQUARE_FOOT)
            } else if spec_kind("autodesk.spec.aec.electrical:potential-") {
                PropertyValue::ElectricVoltage(v * SQUARE_METRES_PER_SQUARE_FOOT)
            } else if spec_kind("autodesk.spec.aec.electrical:current-") {
                // Revit's internal unit of current is the ampere.
                PropertyValue::ElectricCurrent(v)
            } else {
                return None;
            }
        }
        // A material's display name is its name, a family type's
        // `<family> : <type>`, or the type's name alone where the family has
        // the same name (as measured on Snowdon Towers Architectural; Revit
        // writes `Parameter.AsValueString`).
        "ElementId" => {
            let name = parameter.target_name.clone()?;
            PropertyValue::Label(match &parameter.target_family {
                Some(family) if *family != name => format!("{family} : {name}"),
                _ => name,
            })
        }
        _ => return None,
    };
    Some(Property { name, value })
}

/// `parameters` grouped into Revit's sets. In each group one parameter is
/// kept per name with its spaces removed, the one of the lowest parameter id,
/// whether or not it is written. Returns the sets and how many parameters
/// were left out.
fn grouped<'a>(
    parameters: impl Iterator<Item = &'a ElementParameter>,
) -> (BTreeMap<&'static str, Vec<Property>>, usize) {
    let mut parameters: Vec<&ElementParameter> = parameters.collect();
    parameters.sort_by_key(|p| p.parameter_id);
    let mut kept: BTreeSet<(&'static str, String)> = BTreeSet::new();
    let mut sets: BTreeMap<&'static str, Vec<Property>> = BTreeMap::new();
    let mut left_out = 0;
    for parameter in parameters {
        let Some(label) = group_label(parameter.group_type_id.as_deref()) else {
            left_out += 1;
            continue;
        };
        let name = parameter.name.as_deref().unwrap_or("").replace(' ', "");
        if !kept.insert((label, name)) {
            left_out += 1;
            continue;
        }
        match revit_property(parameter) {
            Some(property) => sets.entry(label).or_default().push(property),
            None => left_out += 1,
        }
    }
    (sets, left_out)
}

/// Give `model`'s building elements and their type objects Revit's own
/// property sets from the parameters `file` saves; an element whose type has
/// no type object takes its type's sets itself, named with [`TYPE_SUFFIX`]. A
/// set of the same name already written (`Other`, `Identity Data`,
/// `Phasing`) takes the properties it does not hold.
pub fn attach(file: &mut RevitFile, model: &mut IfcModel) -> Result<Report> {
    let mut by_id: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        if let Ok(id) = tag.parse::<u64>() {
            by_id.entry(id).or_default().push(index);
        }
    }
    let mut report = Report::default();
    let ids: BTreeSet<u64> = by_id.keys().copied().collect();
    let saved = crate::research::native_element_parameters::read(file, &ids)?;
    // Where each element's sets already are, by (element, set name).
    let mut held: BTreeMap<(usize, String), usize> = BTreeMap::new();
    for (position, entity) in model.entities.iter().enumerate() {
        if let IfcEntity::ElementPropertySet { element, set } = entity {
            held.entry((*element, set.name.clone())).or_insert(position);
        }
    }
    let mut typed: BTreeSet<u32> = BTreeSet::new();
    for (id, indices) in &by_id {
        let Some(parameters) = saved.elements.get(id) else {
            continue;
        };
        let named = parameters.iter().filter(|p| p.name.is_some());
        let (instance, left_out) = grouped(
            named
                .clone()
                .filter(|p| p.level == ParameterLevel::Instance),
        );
        report.left_out += left_out;
        if !instance.is_empty() {
            report.elements += 1;
        }
        for &index in indices {
            for (label, properties) in &instance {
                report.properties += properties.len();
                merge_element_set(&mut model.entities, &mut held, index, label, properties);
            }
        }
        // The type's parameters go on its type object, once per type, or,
        // with no type object, on the element.
        let Some(type_id) = u32::try_from(*id)
            .ok()
            .and_then(|id| model.element_type_ids.get(&id).copied())
        else {
            let (types, left_out) = grouped(named.filter(|p| p.level == ParameterLevel::Type));
            report.left_out += left_out;
            if types.is_empty() {
                continue;
            }
            report.untyped_elements += 1;
            for &index in indices {
                for (label, properties) in &types {
                    report.properties += properties.len();
                    let label = format!("{label}{TYPE_SUFFIX}");
                    merge_element_set(&mut model.entities, &mut held, index, &label, properties);
                }
            }
            continue;
        };
        if !typed.insert(type_id) {
            continue;
        }
        let (types, left_out) = grouped(named.filter(|p| {
            p.level == ParameterLevel::Type && p.source_element_id == i64::from(type_id)
        }));
        report.left_out += left_out;
        if types.is_empty() {
            continue;
        }
        report.types += 1;
        let sets = model.type_property_sets.entry(type_id).or_default();
        for (label, properties) in types {
            report.properties += properties.len();
            sets.push(PropertySet {
                name: label.to_string(),
                properties,
            });
        }
    }
    Ok(report)
}

/// Add `properties` to the set `label` of the element at `index`, after
/// what that set already holds; a set of its own where it has none.
fn merge_element_set(
    entities: &mut Vec<IfcEntity>,
    held: &mut BTreeMap<(usize, String), usize>,
    index: usize,
    label: &str,
    properties: &[Property],
) {
    if let Some(&position) = held.get(&(index, label.to_string())) {
        if let Some(IfcEntity::ElementPropertySet { set, .. }) = entities.get_mut(position) {
            for property in properties {
                if !set.properties.iter().any(|p| p.name == property.name) {
                    set.properties.push(property.clone());
                }
            }
        }
        return;
    }
    held.insert((index, label.to_string()), entities.len());
    entities.push(IfcEntity::ElementPropertySet {
        element: index,
        set: PropertySet {
            name: label.to_string(),
            properties: properties.to_vec(),
        },
    });
}
