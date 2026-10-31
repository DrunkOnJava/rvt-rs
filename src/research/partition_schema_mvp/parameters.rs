//! Named parameters, `IsExternal`, phases created and serial numbers of record-backed elements.

use super::*;

pub(super) fn attach_type_text_parameters(
    parameters: &BTreeMap<u32, BTreeMap<&'static str, String>>,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        let type_id = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        });
        let Some(values) = type_id.and_then(|id| parameters.get(&id)) else {
            continue;
        };
        for (name, value) in values {
            element.fields.push((
                format!("{TYPE_PARAMETER_FIELD_PREFIX}{name}"),
                InstanceField::String(value.clone()),
            ));
        }
    }
}

/// The `IsExternal` Revit's exporter calculates for `element` (RE-193),
/// following revit-ifc's `CategoryUtil.IsElementExternal` (CLEANROOM.md
/// source 5): a parameter named [`IS_EXTERNAL_PARAMETER_NAME`] on the
/// element, then on its type; its type's Function, external unless interior
/// (0); a roof is external; otherwise not external, where `last_step`
/// allows it. `None` for the classes whose value cannot be calculated from
/// what is read, and where the last step is not allowed.
pub(super) fn calculated_is_external(element: &DecodedElement, last_step: bool) -> Option<bool> {
    let flag = |wanted: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Bool(flag) if name == wanted => Some(*flag),
            _ => None,
        })
    };
    if let Some(value) = flag(&format!(
        "{NAMED_PARAMETER_FIELD_PREFIX}{IS_EXTERNAL_PARAMETER_NAME}"
    ))
    .or_else(|| {
        flag(&format!(
            "{TYPE_NAMED_PARAMETER_FIELD_PREFIX}{IS_EXTERNAL_PARAMETER_NAME}"
        ))
    }) {
        return Some(value);
    }
    let function = element.fields.iter().find_map(|(name, value)| match value {
        InstanceField::Integer { value, .. } if name == TYPE_FUNCTION_FIELD => Some(*value),
        _ => None,
    });
    if let Some(function) = function {
        return Some(function != 0);
    }
    if element.class == "Roof" {
        return Some(true);
    }
    let class = element.class.as_str();
    if !last_step
        || IS_EXTERNAL_FUNCTION_CLASSES.contains(&class)
        || IS_EXTERNAL_UNKNOWN_CLASSES.contains(&class)
    {
        return None;
    }
    Some(false)
}

/// Give each element of `groups` the Yes/No parameters named `IsExternal` or
/// `<property set>.IsExternal` that it carries ([`NAMED_PARAMETER_FIELD_PREFIX`];
/// for the plain name also its type's, [`TYPE_NAMED_PARAMETER_FIELD_PREFIX`]),
/// then the value Revit's exporter calculates ([`IS_EXTERNAL_FIELD`]). Revit's
/// exporter reads `Pset_WallCommon.IsExternal` on a wall before anything else,
/// and the BIM4LCA office defines one such parameter for each of twelve
/// property sets (RE-193).
pub(super) fn attach_is_external(
    rf: &mut RevitFile,
    revit_version: u32,
    mut groups: [&mut Vec<DecodedElement>; 8],
) {
    use crate::partition_room_parameters as prp;
    let last_step = IS_EXTERNAL_LAST_STEP_REVIT_VERSIONS.contains(&revit_version);
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let parameters: Vec<(String, u32)> = prp::find_parameter_definitions(rf, revit_version)
        .into_iter()
        .filter(|(name, _)| {
            name == IS_EXTERNAL_PARAMETER_NAME
                || name.ends_with(&format!(".{IS_EXTERNAL_PARAMETER_NAME}"))
        })
        .collect();
    if !parameters.is_empty() {
        let owners: BTreeSet<u32> = groups
            .iter()
            .flat_map(|elements| elements.iter())
            .flat_map(|element| element.id.into_iter().chain(type_of(element)))
            .collect();
        let ids: Vec<i64> = parameters.iter().map(|(_, id)| i64::from(*id)).collect();
        let mut found = prp::scan_integer_parameters(rf, revit_version, &owners, &ids);
        let values: Vec<(String, BTreeMap<u32, u32>)> = parameters
            .into_iter()
            .map(|(name, id)| {
                let values = found.remove(&i64::from(id)).unwrap_or_default();
                (name, values)
            })
            .collect();
        for element in groups.iter_mut().flat_map(|elements| elements.iter_mut()) {
            let type_id = type_of(element);
            for (name, found) in &values {
                if let Some(&value) = element.id.and_then(|id| found.get(&id)) {
                    element.fields.push((
                        format!("{NAMED_PARAMETER_FIELD_PREFIX}{name}"),
                        InstanceField::Bool(value != 0),
                    ));
                }
                if let (true, Some(&value)) = (
                    name == IS_EXTERNAL_PARAMETER_NAME,
                    type_id.and_then(|id| found.get(&id)),
                ) {
                    element.fields.push((
                        format!("{TYPE_NAMED_PARAMETER_FIELD_PREFIX}{name}"),
                        InstanceField::Bool(value != 0),
                    ));
                }
            }
        }
    }
    attach_calculated_is_external(
        groups.into_iter().flat_map(|elements| elements.iter_mut()),
        last_step,
    );
}

/// Whether `value` is a Yes/No parameter's: a boolean, or a 0 or 1.
fn yes_no(value: &serde_json::Value) -> Option<bool> {
    match value {
        serde_json::Value::Bool(flag) => Some(*flag),
        other => match other.as_i64() {
            Some(0) => Some(false),
            Some(1) => Some(true),
            _ => None,
        },
    }
}

/// The field a parameter's saved `value` gives a property of IFC4 type
/// `kind`, as [`attach_named_parameters`] reads it from the partition data.
fn named_field(
    kind: crate::ifc::standard_property_sets::StandardPropertyKind,
    value: &serde_json::Value,
) -> Option<InstanceField> {
    use crate::ifc::standard_property_sets::StandardPropertyKind as Kind;
    match kind {
        Kind::Boolean => yes_no(value).map(InstanceField::Bool),
        Kind::Count | Kind::Integer => {
            let count = value.as_i64().filter(|n| i32::try_from(*n).is_ok())?;
            Some(InstanceField::Integer {
                value: count,
                signed: false,
                size: 4,
            })
        }
        Kind::Identifier | Kind::Label | Kind::Text => {
            let text = value.as_str().filter(|text| !text.is_empty())?;
            Some(InstanceField::String(text.to_string()))
        }
        Kind::Length | Kind::PositiveLength | Kind::ThermalTransmittance => {
            let number = value.as_f64()?;
            if kind == Kind::PositiveLength && number <= 0.0 {
                return None;
            }
            (number == 0.0 || number.is_normal()).then_some(InstanceField::Float {
                value: number,
                size: 8,
            })
        }
    }
}

/// Give each of `elements` the values of the parameters it holds that are
/// named `IsExternal` or `<property set>.IsExternal` (on the element and, for
/// the plain name, its type) and of those named `<property set>.<property>`,
/// or for the property alone, for a property of a standard set, as the
/// fields [`attach_is_external`] and [`attach_named_parameters`] give them
/// from the partition data on 2024 and later, here read from the saved
/// parameters on the releases in [`NATIVE_NAMED_PARAMETER_REVIT_VERSIONS`]
/// (RE-283). A qualified name is read before a bare one; a bare name
/// fills the property of each standard set that defines it.
pub(super) fn attach_native_named_parameters<'a>(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: impl IntoIterator<Item = &'a mut DecodedElement>,
) {
    use crate::ifc::standard_property_sets::{self as sps, StandardPropertyKind as Kind};
    use crate::native_element_parameters::{ParameterLevel, read_defined};
    if !NATIVE_NAMED_PARAMETER_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let mut elements: Vec<&mut DecodedElement> = elements.into_iter().collect();
    let ids: BTreeSet<u64> = elements
        .iter()
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(saved) = read_defined(rf, &ids, Some(names_a_property)) else {
        return;
    };
    let is_external_suffix = format!(".{IS_EXTERNAL_PARAMETER_NAME}");
    for element in elements.iter_mut() {
        let Some(parameters) = element.id.and_then(|id| saved.elements.get(&u64::from(id))) else {
            continue;
        };
        let mut qualified: Vec<(String, Kind, &serde_json::Value)> = Vec::new();
        let mut bare: Vec<(String, Kind, &serde_json::Value)> = Vec::new();
        let mut flags: Vec<(String, bool)> = Vec::new();
        for parameter in parameters {
            let Some(name) = parameter.name.as_deref() else {
                continue;
            };
            if name == IS_EXTERNAL_PARAMETER_NAME || name.ends_with(&is_external_suffix) {
                let Some(flag) = yes_no(&parameter.value) else {
                    continue;
                };
                match parameter.level {
                    ParameterLevel::Instance => {
                        flags.push((format!("{NAMED_PARAMETER_FIELD_PREFIX}{name}"), flag));
                    }
                    ParameterLevel::Type if name == IS_EXTERNAL_PARAMETER_NAME => {
                        flags.push((format!("{TYPE_NAMED_PARAMETER_FIELD_PREFIX}{name}"), flag));
                    }
                    ParameterLevel::Type => {}
                }
                continue;
            }
            if parameter.level != ParameterLevel::Instance {
                continue;
            }
            if let Some((set, property)) = name.split_once('.') {
                if let Some(kind) = sps::property_kind(set, property) {
                    qualified.push((name.to_string(), kind, &parameter.value));
                }
                continue;
            }
            for set in sps::STANDARD_PROPERTY_SETS {
                if let Some(kind) = set.property_kind(name) {
                    bare.push((format!("{}.{name}", set.name), kind, &parameter.value));
                }
            }
        }
        for (field_name, flag) in flags {
            if !element.fields.iter().any(|(held, _)| *held == field_name) {
                element.fields.push((field_name, InstanceField::Bool(flag)));
            }
        }
        for (name, kind, value) in qualified.into_iter().chain(bare) {
            if REVIT_OMITTED_PARAMETERS.contains(&name.as_str()) {
                continue;
            }
            let field_name = format!("{NAMED_PARAMETER_FIELD_PREFIX}{name}");
            if element.fields.iter().any(|(held, _)| *held == field_name) {
                continue;
            }
            if let Some(field) = named_field(kind, value) {
                element.fields.push((field_name, field));
            }
        }
    }
}

/// Whether a parameter named `name` is one [`attach_native_named_parameters`]
/// reads: `IsExternal`, `<set>.IsExternal`, `<set>.<property>` of a standard
/// set, or a property some standard set defines.
fn names_a_property(name: &str) -> bool {
    use crate::ifc::standard_property_sets as sps;
    if name == IS_EXTERNAL_PARAMETER_NAME {
        return true;
    }
    match name.split_once('.') {
        Some((set, property)) => {
            property == IS_EXTERNAL_PARAMETER_NAME || sps::property_kind(set, property).is_some()
        }
        None => sps::STANDARD_PROPERTY_SETS
            .iter()
            .any(|set| set.property_kind(name).is_some()),
    }
}

/// Give each of `elements` the `IsExternal` revit-ifc calculates for it
/// ([`calculated_is_external`], [`IS_EXTERNAL_FIELD`]) on the releases that
/// read elements from their element records (RE-283): where a
/// parameter named for it says (RE-193), a wall's type Function says, or, for
/// the classes in [`IS_EXTERNAL_LAST_STEP_RECORD_CLASSES`], the calculation's
/// last step, not external. Any other element gets none.
pub(super) fn attach_record_is_external<'a>(
    elements: impl IntoIterator<Item = &'a mut DecodedElement>,
) {
    let suffix = format!(".{IS_EXTERNAL_PARAMETER_NAME}");
    for element in elements {
        let named = element.fields.iter().any(|(name, _)| {
            name.strip_prefix(NAMED_PARAMETER_FIELD_PREFIX)
                .or_else(|| name.strip_prefix(TYPE_NAMED_PARAMETER_FIELD_PREFIX))
                .is_some_and(|rest| rest == IS_EXTERNAL_PARAMETER_NAME || rest.ends_with(&suffix))
        });
        let last_step = IS_EXTERNAL_LAST_STEP_RECORD_CLASSES.contains(&element.class.as_str());
        if !(named || last_step || element.class == "Wall") {
            continue;
        }
        if let Some(external) = calculated_is_external(element, last_step) {
            element
                .fields
                .push((IS_EXTERNAL_FIELD.into(), InstanceField::Bool(external)));
        }
    }
}

/// Give each of `elements` the value [`calculated_is_external`] finds for it
/// ([`IS_EXTERNAL_FIELD`]).
pub(super) fn attach_calculated_is_external<'a>(
    elements: impl IntoIterator<Item = &'a mut DecodedElement>,
    last_step: bool,
) {
    for element in elements {
        if let Some(external) = calculated_is_external(element, last_step) {
            element
                .fields
                .push((IS_EXTERNAL_FIELD.into(), InstanceField::Bool(external)));
        }
    }
}

pub(super) fn attach_named_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 8],
) {
    use crate::ifc::standard_property_sets::{self as sps, StandardPropertyKind as Kind};
    use crate::partition_room_parameters::{self as prp, EntryKind, EntryValue};
    let definitions = prp::find_parameter_definitions(rf, revit_version);
    // Qualified names first: an element's bare-named value only fills a
    // property its qualified one leaves empty.
    let mut parameters: Vec<(String, i64, Kind)> = definitions
        .iter()
        .filter_map(|(name, id)| {
            let (set, property) = name.split_once('.')?;
            let kind = sps::property_kind(set, property)
                .filter(|_| property != IS_EXTERNAL_PARAMETER_NAME)?;
            Some((name.clone(), i64::from(*id), kind))
        })
        .collect();
    for (name, id) in &definitions {
        if name.contains('.') || name == IS_EXTERNAL_PARAMETER_NAME {
            continue;
        }
        for set in sps::STANDARD_PROPERTY_SETS {
            if let Some(kind) = set.property_kind(name) {
                parameters.push((format!("{}.{name}", set.name), i64::from(*id), kind));
            }
        }
    }
    parameters.retain(|(name, _, _)| !REVIT_OMITTED_PARAMETERS.contains(&name.as_str()));
    if parameters.is_empty() {
        return;
    }
    let entry_kind = |kind: Kind| match kind {
        Kind::Boolean | Kind::Count | Kind::Integer => EntryKind::Word,
        Kind::Identifier | Kind::Label | Kind::Text => EntryKind::Text,
        Kind::Length | Kind::PositiveLength | Kind::ThermalTransmittance => EntryKind::Real,
    };
    // A bare name several sets hold is one parameter: scan it once.
    let mut scanned = BTreeSet::new();
    let wanted: Vec<(i64, EntryKind)> = parameters
        .iter()
        .filter(|(_, id, _)| scanned.insert(*id))
        .map(|(_, id, kind)| (*id, entry_kind(*kind)))
        .collect();
    let owners: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter())
        .filter_map(|element| element.id)
        .collect();
    let found = prp::scan_parameter_entries(rf, revit_version, &owners, &wanted);
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some(id) = element.id else {
            continue;
        };
        for (name, parameter, kind) in &parameters {
            let Some(value) = found.get(parameter).and_then(|values| values.get(&id)) else {
                continue;
            };
            let field_name = format!("{NAMED_PARAMETER_FIELD_PREFIX}{name}");
            if element.fields.iter().any(|(held, _)| *held == field_name) {
                continue;
            }
            let field = match (kind, value) {
                (Kind::Boolean, EntryValue::Word(word @ (0 | 1))) => {
                    InstanceField::Bool(*word == 1)
                }
                // How a negative integer is stored is not measured.
                (Kind::Count | Kind::Integer, EntryValue::Word(count))
                    if i32::try_from(*count).is_ok() =>
                {
                    InstanceField::Integer {
                        value: i64::from(*count),
                        signed: false,
                        size: 4,
                    }
                }
                (_, EntryValue::Text(text)) if !text.is_empty() => {
                    InstanceField::String(text.clone())
                }
                (Kind::PositiveLength, EntryValue::Real(length)) if *length <= 0.0 => continue,
                (_, EntryValue::Real(value)) if *value == 0.0 || value.is_normal() => {
                    InstanceField::Float {
                        value: *value,
                        size: 8,
                    }
                }
                _ => continue,
            };
            element.fields.push((field_name, field));
        }
    }
}

/// Give each element the one phase its record's reference list names
/// ([`crate::partition_phases::phase_created`]).
pub(super) fn attach_phases_created(rf: &mut RevitFile, groups: [&mut Vec<DecodedElement>; 6]) {
    let phases = crate::partition_phases::phase_names(rf).unwrap_or_default();
    if phases.is_empty() {
        return;
    }
    for element in groups.into_iter().flat_map(|group| group.iter_mut()) {
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        if let Some((_, name)) = crate::partition_phases::phase_created(&references, &phases) {
            element.fields.push((
                PHASE_CREATED_FIELD.into(),
                InstanceField::String(name.into()),
            ));
        }
    }
}

/// Give each element of `groups` its value of the shared parameter `Serial
/// Number`: the text entry with the parameter's ElementId in the element's
/// own data object (RE-156). A file with no such parameter changes nothing.
pub(super) fn attach_serial_numbers(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 6],
) {
    use crate::partition_room_parameters as prp;
    let definitions = prp::find_parameter_definitions(rf, revit_version);
    let Some(&parameter) = definitions.get(SERIAL_NUMBER_PARAMETER_NAME) else {
        return;
    };
    let ids: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter().filter_map(|element| element.id))
        .collect();
    let values = prp::scan_text_parameter(rf, revit_version, &ids, i64::from(parameter));
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        if let Some(value) = element.id.and_then(|id| values.get(&id)) {
            element.fields.push((
                SERIAL_NUMBER_FIELD.into(),
                InstanceField::String(value.clone()),
            ));
        }
    }
}
