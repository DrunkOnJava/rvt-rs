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
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
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
