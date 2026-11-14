//! Ducts, pipes, fittings, connectors and MEP systems.

use super::*;

/// Give each pipe without a type the one id in its record's reference
/// list that carries a pipe or duct type name
/// ([`crate::partition_names::find_mep_curve_type_names`], RE-130). Pipe
/// and duct types have no name entry, so
/// [`crate::partition_names::resolve_type`] finds none for them. Ducts are
/// left alone: their system family follows their type's shape, which is not
/// read.
pub(super) fn attach_pipe_type_names(rf: &mut RevitFile, elements: &mut [DecodedElement]) {
    let has_type = |element: &DecodedElement| {
        element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_NAME_FIELD)
    };
    if !elements
        .iter()
        .any(|element| element.class == "Pipe" && !has_type(element))
    {
        return;
    }
    let mut pending: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if element.class != "Pipe" || has_type(element) {
            continue;
        }
        let (Some(own), Some((references, _))) = (element.id, record_references(rf, element))
        else {
            continue;
        };
        // The list's first slot is a constant 3, not a reference (#228); the
        // name pattern can match after it by chance.
        let references: BTreeSet<u32> = references
            .iter()
            .skip(1)
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|&id| id != own)
            .collect();
        pending.push((index, references));
    }
    if pending.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = pending
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in
            crate::partition_names::find_mep_curve_type_names(inflated.bytes(), &wanted)
        {
            match names.get_mut(&id) {
                None => {
                    names.insert(id, Some(name));
                }
                Some(held) => {
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                }
            }
        }
    }
    for (index, references) in pending {
        let mut typed = references
            .iter()
            .filter_map(|id| Some((*id, names.get(id)?.clone()?)));
        let (Some((type_id, type_name)), None) = (typed.next(), typed.next()) else {
            continue;
        };
        let element = &mut elements[index];
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element
            .fields
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(type_name)));
    }
}

/// A duct's or pipe's width and height, feet (RE-134), when read.
pub fn curve_size_from_fields(fields: &[(String, InstanceField)]) -> Option<(f64, f64)> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    Some((field(CURVE_SIZE_FIELDS[0])?, field(CURVE_SIZE_FIELDS[1])?))
}

/// Give each pipe fitting its nominal diameter
/// ([`crate::partition_fitting_sizes`], RE-165). A fitting whose connectors
/// disagree, or whose object is not found, gets nothing.
pub(super) fn attach_fitting_nominal_diameters(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_fitting_sizes as pfs;
    let fittings: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "PipeFitting")
        .filter_map(|element| element.id)
        .collect();
    let diameters =
        pfs::scan_fitting_nominal_diameters(rf, revit_version, &fittings).unwrap_or_default();
    for element in products.iter_mut() {
        if element.class != "PipeFitting" {
            continue;
        }
        if let Some(&diameter) = element.id.and_then(|id| diameters.get(&id)) {
            element.fields.push((
                FITTING_NOMINAL_DIAMETER_FIELD.into(),
                InstanceField::Float {
                    value: diameter,
                    size: 8,
                },
            ));
        }
    }
}

/// The MEP systems an element is a member of, by ElementId, with their names
/// (RE-162).
pub fn mep_systems_from_fields(fields: &[(String, InstanceField)]) -> Vec<(u32, Option<String>)> {
    fields
        .iter()
        .filter_map(|(name, value)| {
            let id = name.strip_prefix(MEP_SYSTEM_FIELD_PREFIX)?.parse().ok()?;
            let InstanceField::String(system_name) = value else {
                return None;
            };
            Some((id, Some(system_name.clone()).filter(|n| !n.is_empty())))
        })
        .collect()
}

/// Give each element of `groups` the MEP systems it is a member of
/// ([`crate::partition_mep_systems`], RE-162).
pub(super) fn attach_mep_systems(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 6],
) {
    use crate::partition_mep_systems as pms;
    if !pms::supports_revit_version(revit_version) {
        return;
    }
    let ids: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter().filter_map(|element| element.id))
        .collect();
    let systems = pms::scan_mep_systems(rf, revit_version, &ids).unwrap_or_default();
    if systems.is_empty() {
        return;
    }
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        let Some(id) = element.id else {
            continue;
        };
        for system in systems.iter().filter(|system| system.members.contains(&id)) {
            element.fields.push((
                format!("{MEP_SYSTEM_FIELD_PREFIX}{}", system.id),
                InstanceField::String(system.name.clone().unwrap_or_default()),
            ));
        }
    }
}

/// Give each duct and pipe the width and height its curve object holds, and
/// each one still without a type (a duct, or a pipe RE-130's reference list
/// does not type) its curve's type and that type's name
/// ([`crate::partition_curve_fields`], RE-134;
/// [`crate::partition_names::find_mep_curve_type_names`], RE-130). An element
/// whose anchor is not found gets nothing.
pub(super) fn attach_curve_fields(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_curve_fields as pcf;
    if !pcf::supports_revit_version(revit_version) {
        return;
    }
    let is_curve = |element: &DecodedElement| matches!(element.class.as_str(), "Duct" | "Pipe");
    let mut by_stream: BTreeMap<String, Vec<(u32, [f64; 6])>> = BTreeMap::new();
    for element in products.iter().filter(|element| is_curve(element)) {
        if let (Some(id), Some(bbox), Some(stream)) = (
            element.id,
            element_record_bbox(element),
            element_source_stream(element),
        ) {
            by_stream.entry(stream).or_default().push((id, bbox));
        }
    }
    let Ok(fields) = pcf::scan_curve_fields(rf, revit_version, &by_stream) else {
        return;
    };
    if fields.is_empty() {
        return;
    }
    let pipes: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Pipe")
        .filter_map(|element| element.id)
        .collect();
    let diameters = pcf::scan_pipe_diameters(rf, revit_version, &pipes).unwrap_or_default();
    let has = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().any(|(name, _)| name == wanted)
    };
    let wanted: BTreeSet<u32> = products
        .iter()
        .filter(|element| is_curve(element) && !has(element, TYPE_NAME_FIELD))
        .filter_map(|element| Some(fields.get(&element.id?)?.type_id))
        .collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    if !wanted.is_empty() {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            for (id, name) in
                crate::partition_names::find_mep_curve_type_names(inflated.bytes(), &wanted)
            {
                match names.get_mut(&id) {
                    None => {
                        names.insert(id, Some(name));
                    }
                    Some(held) => {
                        if held.as_deref() != Some(name.as_str()) {
                            *held = None;
                        }
                    }
                }
            }
        }
    }
    for element in products.iter_mut().filter(|element| is_curve(element)) {
        let Some(curve) = element.id.and_then(|id| fields.get(&id)).copied() else {
            continue;
        };
        // A duct its own record sized and drew (RE-371) keeps its values.
        let native = has(element, CURVE_SIZE_FIELDS[0]);
        if !native {
            for (name, value) in CURVE_SIZE_FIELDS
                .iter()
                .zip([curve.width_feet, curve.height_feet])
            {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
        // A pipe's inner diameter (RE-157).
        if let Some(&(inner, _)) = element.id.and_then(|id| diameters.get(&id)) {
            element.fields.push((
                PIPE_INNER_DIAMETER_FIELD.into(),
                InstanceField::Float {
                    value: inner,
                    size: 8,
                },
            ));
        }
        // A duct along one of the model's axes: its box is its section by its
        // length (RE-134: 25 of 25 RE1 ducts).
        if element.class == "Duct" && !has(element, DUCT_LENGTH_FIELD) {
            if let Some(length) = element_record_bbox(element)
                .and_then(|bbox| curve.length_in_box(bbox, CURVE_BOX_TOLERANCE_FEET))
            {
                element.fields.push((
                    DUCT_LENGTH_FIELD.into(),
                    InstanceField::Float {
                        value: length,
                        size: 8,
                    },
                ));
            }
        }
        if !is_curve(element) || has(element, TYPE_NAME_FIELD) {
            continue;
        }
        let Some(Some(type_name)) = names.get(&curve.type_id) else {
            continue;
        };
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: curve.type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String(type_name.clone()),
        ));
    }
    attach_duct_system_families(rf, products);
}

/// Give each typed duct the system family its type's shape names
/// (`Rectangular Duct`, `Round Duct`, `Oval Duct`,
/// [`crate::native_duct_shapes`]), so it is named
/// `Family:Type:ElementId` as Revit names it. A file whose release the native
/// record path does not read leaves its ducts without one.
pub(super) fn attach_duct_system_families(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let has = |element: &DecodedElement, wanted: &str| {
        element.fields.iter().any(|(name, _)| name == wanted)
    };
    let types: BTreeSet<u64> = products
        .iter()
        .filter(|element| {
            element.class == "Duct"
                && has(element, TYPE_NAME_FIELD)
                && !has(element, FAMILY_NAME_FIELD)
        })
        .filter_map(type_of)
        .map(u64::from)
        .collect();
    if types.is_empty() {
        return;
    }
    let Ok(shapes) = crate::native_duct_shapes::duct_type_shapes(rf, &types) else {
        return;
    };
    let locale = rf.basic_file_info().ok().and_then(|info| info.locale);
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Duct")
    {
        if has(element, FAMILY_NAME_FIELD) {
            continue;
        }
        let Some(shape) = type_of(element).and_then(|id| shapes.get(&u64::from(id))) else {
            continue;
        };
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(
                localized_system_family(shape.system_family(), locale.as_deref()).into(),
            ),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
        ));
        element.fields.push((
            DUCT_SHAPE_FIELD.into(),
            InstanceField::String(shape.ifc_shape().into()),
        ));
    }
}

/// The cylinder the partition MVP gave a pipe, when its connector entries
/// and record box made one (RE-131).
pub fn pipe_body_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_pipe_axes::PipeBody> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some(crate::partition_pipe_axes::PipeBody {
        start: point(PIPE_AXIS_START_FIELDS)?,
        end: point(PIPE_AXIS_END_FIELDS)?,
        radius_feet: field(PIPE_RADIUS_FIELD)?,
    })
}

/// Give each pipe the cylinder its connector entries and its record box make
/// ([`crate::partition_pipe_axes`], RE-131), and a pipe they do not place, or
/// every pipe of a release whose entries are not measured, the one its own
/// record gives ([`attach_native_pipes`], RE-370). A pipe neither places keeps
/// its box.
pub(super) fn attach_pipe_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_pipe_axes as ppa;
    if !ppa::supports_revit_version(revit_version) {
        // A release whose connector entries are not measured: every pipe from
        // its own record (RE-370).
        attach_native_pipes(rf, revit_version, products);
        return;
    }
    let mut by_stream: BTreeMap<String, Vec<(u32, [f64; 6])>> = BTreeMap::new();
    for element in products.iter().filter(|element| element.class == "Pipe") {
        if let (Some(id), Some(bbox), Some(stream)) = (
            element.id,
            element_record_bbox(element),
            element_source_stream(element),
        ) {
            by_stream.entry(stream).or_default().push((id, bbox));
        }
    }
    if by_stream.is_empty() {
        return;
    }
    let bodies = ppa::scan_pipe_bodies(rf, revit_version, &by_stream).unwrap_or_default();
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Pipe")
    {
        let Some(body) = element.id.and_then(|id| bodies.get(&id)) else {
            continue;
        };
        for (names, point) in [
            (PIPE_AXIS_START_FIELDS, body.start),
            (PIPE_AXIS_END_FIELDS, body.end),
        ] {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
        element.fields.push((
            PIPE_RADIUS_FIELD.into(),
            InstanceField::Float {
                value: body.radius_feet,
                size: 8,
            },
        ));
    }
    // The pipes the connector entries do not place, from their own record
    // (RE-370), before their ends are keyed.
    attach_native_pipes(rf, revit_version, products);
    // #847: the keys at each end say which of the pipe's connectors it is.
    let mut ends = ppa::CurveEnds::new();
    for element in products.iter().filter(|element| element.class == "Pipe") {
        if let (Some(id), Some(stream), Some(body)) = (
            element.id,
            element_source_stream(element),
            pipe_body_from_fields(&element.fields),
        ) {
            ends.entry(stream)
                .or_default()
                .push((id, [body.start, body.end]));
        }
    }
    let Ok(keys) = ppa::scan_pipe_end_keys(rf, revit_version, &ends) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Pipe")
    {
        // A pipe its own record keyed (RE-370) keeps those keys.
        if element
            .fields
            .iter()
            .any(|(name, _)| name == PIPE_END_KEYS_FIELDS[0])
        {
            continue;
        }
        let Some(end_keys) = element.id.and_then(|id| keys.get(&id)) else {
            continue;
        };
        for (name, keys) in PIPE_END_KEYS_FIELDS.iter().zip(end_keys) {
            element.fields.push(((*name).into(), end_keys_field(keys)));
        }
    }
}

/// A duct's two ends ([`DUCT_ENDS_FIELDS`], #847), where its box gave them.
pub fn duct_ends_from_fields(fields: &[(String, InstanceField)]) -> Option<[[f64; 3]; 2]> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some([point(DUCT_ENDS_FIELDS[0])?, point(DUCT_ENDS_FIELDS[1])?])
}

/// The keys of the connector entries at a duct's two ends
/// ([`DUCT_END_KEYS_FIELDS`], #847).
pub fn duct_end_keys_from_fields(
    fields: &[(String, InstanceField)],
) -> [Vec<crate::partition_pipe_axes::EntryKey>; 2] {
    end_keys_from_fields(fields, DUCT_END_KEYS_FIELDS)
}

/// The keys of the connector entries at a pipe's first and second end
/// ([`PIPE_END_KEYS_FIELDS`], #847).
pub fn pipe_end_keys_from_fields(
    fields: &[(String, InstanceField)],
) -> [Vec<crate::partition_pipe_axes::EntryKey>; 2] {
    end_keys_from_fields(fields, PIPE_END_KEYS_FIELDS)
}

/// A vector-of-pairs keys field ([`PIPE_END_KEYS_FIELDS`]) for each of
/// `names`.
pub(super) fn end_keys_from_fields(
    fields: &[(String, InstanceField)],
    names: [&str; 2],
) -> [Vec<crate::partition_pipe_axes::EntryKey>; 2] {
    let keys = |wanted: &str| {
        fields
            .iter()
            .find_map(|(name, value)| match value {
                InstanceField::Vector(pairs) if name == wanted => Some(
                    pairs
                        .iter()
                        .filter_map(|pair| match pair {
                            InstanceField::Vector(parts) => match parts.as_slice() {
                                [
                                    InstanceField::Integer { value: element, .. },
                                    InstanceField::Integer { value: index, .. },
                                ] => Some((
                                    u64::try_from(*element).ok()?,
                                    u32::try_from(*index).ok()?,
                                )),
                                _ => None,
                            },
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            })
            .unwrap_or_default()
    };
    [keys(names[0]), keys(names[1])]
}

/// The keys field values for `keys` ([`PIPE_END_KEYS_FIELDS`]).
pub(super) fn end_keys_field(keys: &[crate::partition_pipe_axes::EntryKey]) -> InstanceField {
    InstanceField::Vector(
        keys.iter()
            .map(|(element, index)| {
                InstanceField::Vector(vec![
                    InstanceField::Integer {
                        value: i64::try_from(*element).unwrap_or(i64::MAX),
                        signed: true,
                        size: 8,
                    },
                    InstanceField::Integer {
                        value: i64::from(*index),
                        signed: false,
                        size: 4,
                    },
                ])
            })
            .collect(),
    )
}

/// Give each duct along one of the model's axes its two ends, the centres of
/// its record box's faces across its length (RE-134), and the keys of the
/// connector entries there (#847), which say which of its connectors each
/// end is, as for a pipe. A duct whose box leaves no single length axis is
/// given neither.
pub(super) fn attach_duct_end_keys(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_pipe_axes as ppa;
    if !ppa::supports_revit_version(revit_version) {
        return;
    }
    let mut ends = ppa::CurveEnds::new();
    // A duct its own record drew along its line (RE-371) has its ends and
    // keys already.
    let drawn = |element: &DecodedElement| {
        element
            .fields
            .iter()
            .any(|(name, _)| name == DUCT_ENDS_FIELDS[0][0])
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Duct" && !drawn(element))
    {
        let (Some(id), Some(bbox), Some((width, height)), Some(stream)) = (
            element.id,
            element_record_bbox(element),
            curve_size_from_fields(&element.fields),
            element_source_stream(element),
        ) else {
            continue;
        };
        let dims = [bbox[3] - bbox[0], bbox[4] - bbox[1], bbox[5] - bbox[2]];
        let near = |a: f64, b: f64| (a - b).abs() <= CURVE_BOX_TOLERANCE_FEET;
        let axes: Vec<usize> = (0..3)
            .filter(|&axis| {
                let (a, b) = (dims[(axis + 1) % 3], dims[(axis + 2) % 3]);
                (near(a, width) && near(b, height)) || (near(a, height) && near(b, width))
            })
            .collect();
        let [axis] = axes.as_slice() else {
            continue;
        };
        let centre = [
            (bbox[0] + bbox[3]) / 2.0,
            (bbox[1] + bbox[4]) / 2.0,
            (bbox[2] + bbox[5]) / 2.0,
        ];
        let mut start = centre;
        let mut end = centre;
        start[*axis] = bbox[*axis];
        end[*axis] = bbox[*axis + 3];
        for (names, point) in DUCT_ENDS_FIELDS.iter().zip([start, end]) {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
        ends.entry(stream).or_default().push((id, [start, end]));
    }
    let Ok(keys) = ppa::scan_pipe_end_keys(rf, revit_version, &ends) else {
        return;
    };
    for element in products.iter_mut().filter(|element| {
        element.class == "Duct"
            && !element
                .fields
                .iter()
                .any(|(name, _)| name == DUCT_END_KEYS_FIELDS[0])
    }) {
        let Some(end_keys) = element.id.and_then(|id| keys.get(&id)) else {
            continue;
        };
        for (name, keys) in DUCT_END_KEYS_FIELDS.iter().zip(end_keys) {
            element.fields.push(((*name).into(), end_keys_field(keys)));
        }
    }
}

/// The field naming the element joined at connector `connector` of a duct,
/// pipe or fitting (RE-138, RE-141).
pub fn connector_element_field(connector: u32) -> String {
    format!("m_connector_{connector}_element")
}

/// The field naming that element's connector index (RE-138, RE-141).
pub fn connector_index_field(connector: u32) -> String {
    format!("m_connector_{connector}_index")
}

/// The joins of an element: its connector, then the element and connector
/// index it is joined to (RE-138, RE-141).
pub type ConnectorJoins = Vec<(u32, u32, u32)>;

/// The joins of a duct, pipe or fitting, from its fields (RE-138, RE-141).
pub fn connector_joins_from_fields(fields: &[(String, InstanceField)]) -> ConnectorJoins {
    let mut joins = ConnectorJoins::new();
    for (name, value) in fields {
        let InstanceField::ElementId { id: other, .. } = value else {
            continue;
        };
        let Some(connector) = name
            .strip_prefix("m_connector_")
            .and_then(|rest| rest.strip_suffix("_element"))
            .and_then(|number| number.parse::<u32>().ok())
        else {
            continue;
        };
        let index_name = connector_index_field(connector);
        let other_index = fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if *name == index_name => {
                u32::try_from(*value).ok()
            }
            _ => None,
        });
        if let Some(other_index) = other_index {
            joins.push((connector, *other, other_index));
        }
    }
    joins
}

/// Give each element the element and connector joined at each of its
/// connectors. Where the native record path reads the file, the joins are
/// the ones its family instances' connectors list
/// ([`crate::native_connectors::family_instance_joins`], RE-171); elsewhere,
/// on the release the list form is measured on, the lists after a duct's or
/// pipe's anchor and those with a fitting first
/// ([`crate::partition_connector_pairs`], RE-138, RE-141). A connector with no
/// join read is left without the fields.
pub(super) fn attach_connector_pairs(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_connector_pairs as pcp;
    // Only the family instances of the MEP categories are read: decoding
    // every family instance cost a second on Core Interior's furniture.
    let instances: BTreeSet<u64> = products
        .iter()
        .filter(|element| JOINED_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| element.id)
        .map(u64::from)
        .collect();
    if instances.is_empty() {
        return;
    }
    let pairs = match crate::native_connectors::family_instance_joins(rf, &instances) {
        Ok(pairs) => pairs,
        Err(_) if pcp::supports_revit_version(revit_version) => {
            let ids_of = |classes: &[&str]| -> BTreeSet<u64> {
                products
                    .iter()
                    .filter(|element| classes.contains(&element.class.as_str()))
                    .filter_map(|element| element.id)
                    .map(u64::from)
                    .collect()
            };
            let everything = products
                .iter()
                .filter_map(|element| element.id)
                .map(u64::from)
                .collect();
            let mut pairs =
                pcp::scan_connector_pairs(rf, &ids_of(&["Duct", "Pipe"])).unwrap_or_default();
            pairs.extend(
                pcp::scan_fitting_pairs(rf, &ids_of(&["DuctFitting", "PipeFitting"]), &everything)
                    .unwrap_or_default(),
            );
            pairs
        }
        Err(_) => return,
    };
    let mut by_element: BTreeMap<u64, Vec<&pcp::ConnectorPair>> = BTreeMap::new();
    for pair in &pairs {
        by_element.entry(pair.element).or_default().push(pair);
    }
    for element in products.iter_mut() {
        let Some(own) = element.id.and_then(|id| by_element.get(&u64::from(id))) else {
            continue;
        };
        let mut joined: BTreeSet<u32> = BTreeSet::new();
        for pair in own {
            let Ok(other) = u32::try_from(pair.other) else {
                continue;
            };
            if !joined.insert(pair.index) {
                continue;
            }
            element.fields.push((
                connector_element_field(pair.index),
                InstanceField::ElementId { tag: 0, id: other },
            ));
            element.fields.push((
                connector_index_field(pair.index),
                InstanceField::Integer {
                    value: i64::from(pair.other_index),
                    signed: false,
                    size: 4,
                },
            ));
        }
    }
}

/// The fields [`attach_native_pipes`] reads from a pipe's `RbsPipeCurve` record
/// and its type's `RbsPipeType` record (`graph_fields`, #1335).
fn pipe_read_fields(class: &str) -> Option<&'static [&'static str]> {
    Some(match class {
        "RbsPipeCurve" => &[
            "m_id",
            "m_idType",
            "m_origin",
            "m_dirVec",
            "m_endParams",
            "m_dOuterDiameter",
            "m_nIndex",
            "m_arrRefs",
            "m_pConnector",
            "m_paramOnCurve",
            "m_eDirection",
        ],
        "RbsPipeType" => &["m_id", "m_name"],
        _ => &["m_id"],
    })
}

/// What a pipe's own record holds (RE-370): its type, the two ends of its
/// one `GLine`, its outer diameter, feet, the index of the connector at
/// each end, and each connector's calculated direction.
#[derive(Debug, Clone, PartialEq)]
struct NativePipe {
    type_id: Option<u32>,
    ends: Option<[[f64; 3]; 2]>,
    outer_diameter: Option<f64>,
    connectors: Option<[u32; 2]>,
    directions: BTreeMap<u32, i64>,
    /// Each connector's first join to another element (RE-371).
    joins: Vec<(u32, u32, u32)>,
}

/// The one object of `class` in a record's graph, or `None` where it holds
/// none or more than one.
fn only_object<'a>(
    graph: &'a crate::native_parameters::ObjectGraph,
    class: &str,
) -> Option<&'a crate::native_parameters::GraphObject> {
    let mut found = graph
        .objects
        .iter()
        .filter(move |object| object.class_name == class);
    match (found.next(), found.next()) {
        (Some(object), None) => Some(object),
        _ => None,
    }
}

/// The two ends of a duct's or pipe's one `GLine`, from its first end
/// parameter to its second (RE-370), model feet.
fn line_ends(graph: &crate::native_parameters::ObjectGraph) -> Option<[[f64; 3]; 2]> {
    use crate::native_json::finite_triple;
    let line = only_object(graph, "GLine")?;
    let origin = finite_triple(line.fields.get("m_origin")?)?;
    let direction = finite_triple(line.fields.get("m_dirVec")?)?;
    let [start, end] = line
        .fields
        .get("m_endParams")?
        .as_array()?
        .iter()
        .map(crate::graph_value::Fv::as_f64)
        .collect::<Option<Vec<f64>>>()?[..]
    else {
        return None;
    };
    let at = |t: f64| std::array::from_fn(|axis| origin[axis] + direction[axis] * t);
    let ends = [at(start), at(end)];
    ends.iter()
        .flatten()
        .all(|value| value.is_finite())
        .then_some(ends)
}

/// Which connector sits at each end of a duct's or pipe's line (RE-370): a
/// `SegmentConnectorPosition` names its `Connector` by pointer and sits at
/// parameter 0 (the line's start) or 1 (its end). `None` unless each end
/// has exactly one, and they differ. With `taps`, a connector between the
/// ends (a duct's tap, RE-371) is passed over; without, it gives `None`, as
/// for a pipe.
fn end_connectors(graph: &crate::native_parameters::ObjectGraph, taps: bool) -> Option<[u32; 2]> {
    let mut at_ends: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    for position in graph
        .objects
        .iter()
        .filter(|object| object.class_name == "SegmentConnectorPosition")
    {
        let end = match position
            .fields
            .get("m_paramOnCurve")
            .and_then(crate::graph_value::Fv::as_f64)
        {
            Some(0.0) => 0,
            Some(1.0) => 1,
            Some(between) if taps && between > 0.0 && between < 1.0 => continue,
            _ => return None,
        };
        let index = position
            .fields
            .pointer("/m_pConnector/pointer_token")
            .and_then(crate::graph_value::Fv::as_u64)
            .and_then(|token| {
                let mut found = graph.objects.iter().filter(|object| {
                    object.class_name == "Connector" && u64::from(object.token) == token
                });
                match (found.next(), found.next()) {
                    (Some(connector), None) => connector.fields.get("m_nIndex")?.as_u64(),
                    _ => None,
                }
            })
            .and_then(|index| u32::try_from(index).ok())?;
        at_ends[end].push(index);
    }
    match (at_ends[0].as_slice(), at_ends[1].as_slice()) {
        ([start], [end]) if start != end => Some([*start, *end]),
        _ => None,
    }
}

fn native_pipe(graph: &crate::native_parameters::ObjectGraph, own: u64) -> NativePipe {
    use crate::native_json::identifier_i64;
    let positive = |class: &str, field: &str| {
        only_object(graph, class)?
            .fields
            .get(field)?
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
    };
    let type_id = only_object(graph, "RbsPipeCurve")
        .and_then(|curve| curve.fields.get("m_idType"))
        .and_then(identifier_i64)
        .and_then(|id| u32::try_from(id).ok());
    NativePipe {
        type_id,
        // The centreline is the pipe's one line.
        ends: line_ends(graph),
        outer_diameter: positive("PipeDomainDesignPropertyManager", "m_dOuterDiameter"),
        connectors: end_connectors(graph, false),
        directions: pipe_directions(graph),
        joins: segment_joins(graph, own),
    }
}

/// Each connector's calculated direction in a pipe's record (RE-212): the
/// `m_eDirection` of the `SegmentConnectorCalculation` whose `m_pConnector`
/// names the connector, by the connector's `m_nIndex`. A connector with two
/// calculations that disagree is left out.
fn pipe_directions(graph: &crate::native_parameters::ObjectGraph) -> BTreeMap<u32, i64> {
    let index_of_token: BTreeMap<u64, u32> = graph
        .objects
        .iter()
        .filter(|object| object.class_name == "Connector")
        .filter_map(|connector| {
            let index = connector.fields.get("m_nIndex")?.as_u64()?;
            Some((u64::from(connector.token), u32::try_from(index).ok()?))
        })
        .collect();
    let mut directions: BTreeMap<u32, Option<i64>> = BTreeMap::new();
    for calculation in graph
        .objects
        .iter()
        .filter(|object| object.class_name == "SegmentConnectorCalculation")
    {
        let Some((index, direction)) = calculation
            .fields
            .pointer("/m_pConnector/pointer_token")
            .and_then(crate::graph_value::Fv::as_u64)
            .and_then(|token| index_of_token.get(&token))
            .zip(
                calculation
                    .fields
                    .get("m_eDirection")
                    .and_then(crate::graph_value::Fv::as_i64),
            )
        else {
            continue;
        };
        let held = directions.entry(*index).or_insert(Some(direction));
        if *held != Some(direction) {
            *held = None;
        }
    }
    directions
        .into_iter()
        .filter_map(|(index, direction)| Some((index, direction?)))
        .collect()
}

/// The calculated direction of each of a pipe's connectors its record gave
/// ([`PIPE_FLOW_DIRECTIONS_FIELD`], RE-370), by connector index.
pub fn pipe_flow_directions_from_fields(fields: &[(String, InstanceField)]) -> BTreeMap<u32, i64> {
    fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(pairs) if name == PIPE_FLOW_DIRECTIONS_FIELD => Some(
                pairs
                    .iter()
                    .filter_map(|pair| match pair {
                        InstanceField::Vector(parts) => match parts.as_slice() {
                            [
                                InstanceField::Integer { value: index, .. },
                                InstanceField::Integer {
                                    value: direction, ..
                                },
                            ] => Some((u32::try_from(*index).ok()?, *direction)),
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// Give each pipe still without a type or a body what its own
/// `RbsPipeCurve` record names (RE-370), on the releases the native record
/// path reads (`NATIVE_PIPE_REVIT_VERSIONS`): its type (`m_idType`, an
/// `RbsPipeType` whose `SymbolInfo.m_name` is its name, as RE-240 reads it on
/// 2024), and the cylinder it is drawn as, its one `GLine` between its two end
/// parameters and half its `m_dOuterDiameter`, so its quantities (RE-247) and
/// a vertical pipe's invert (RE-157) are written as for a pipe RE-131 places.
/// On 2024 to 2027 it runs from [`attach_pipe_axes`], before RE-130's and
/// RE-134's type names, so every pipe takes its type from its record, as
/// RE-240 measured against Revit's export; there it draws the pipes RE-131's
/// connector entries do not place, on 2026 and 2027 too since RE-346
/// widened those entries to them. On 2015 to 2023 it types and draws every pipe. The cylinder is held to RE-131's own test: its record box must
/// give the same radius and be the box the cylinder fills
/// ([`crate::partition_pipe_axes::pipe_body`]). Its inner diameter is not
/// given: a level pipe's invert, which needs it, comes from the curve fields,
/// on 2025, the release RE-157 measured it on, and on 2026 and 2027 since
/// RE-346. A pipe whose copies disagree, or whose
/// record holds no line, more than one or no positive outer diameter, keeps
/// its box.
pub(super) fn attach_native_pipes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_pipe_axes as ppa;
    if !NATIVE_PIPE_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let wanting = |element: &DecodedElement| {
        element.class == "Pipe"
            && !(has(element, TYPE_NAME_FIELD) && has(element, PIPE_RADIUS_FIELD))
    };
    let selected: BTreeSet<u64> = products
        .iter()
        .filter(|element| wanting(element))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if selected.is_empty() {
        return;
    }
    let untyped = products
        .iter()
        .any(|element| element.class == "Pipe" && !has(element, TYPE_NAME_FIELD));
    let options = crate::native_document::Options {
        selected_ids: selected.clone(),
        selected_classes: if untyped {
            ["RbsPipeType".to_string()].into()
        } else {
            BTreeSet::new()
        },
        graph_fields: Some(pipe_read_fields),
        releases: crate::native_document::NATIVE_RELEASES,
        ..crate::native_document::Options::default()
    };
    let mut pipes: BTreeMap<u64, Option<NativePipe>> = BTreeMap::new();
    let mut type_names: BTreeMap<u64, Option<String>> = BTreeMap::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if class == "RbsPipeType" {
            let name = graph
                .objects
                .iter()
                .find(|object| object.class_name == "SymbolInfo")
                .and_then(|object| object.fields.get("m_name"))
                .and_then(crate::graph_value::Fv::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string);
            crate::native_json::agree(&mut type_names, id, name);
        } else if class == "RbsPipeCurve" && selected.contains(&id) {
            crate::native_json::agree(&mut pipes, id, Some(native_pipe(graph, id)));
        }
        Ok(())
    });
    if read.is_err() {
        return;
    }
    for element in products.iter_mut().filter(|element| wanting(element)) {
        let Some(Some(pipe)) = element.id.and_then(|id| pipes.get(&u64::from(id))) else {
            continue;
        };
        if !has(element, TYPE_NAME_FIELD) {
            if let Some((type_id, Some(Some(name)))) = pipe
                .type_id
                .map(|type_id| (type_id, type_names.get(&u64::from(type_id))))
            {
                element.fields.push((
                    TYPE_ID_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: type_id,
                    },
                ));
                element
                    .fields
                    .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
            }
        }
        push_joins(element, &pipe.joins);
        // Each connector's calculated direction (RE-212), for its port.
        if !pipe.directions.is_empty() && !has(element, PIPE_FLOW_DIRECTIONS_FIELD) {
            element.fields.push((
                PIPE_FLOW_DIRECTIONS_FIELD.into(),
                InstanceField::Vector(
                    pipe.directions
                        .iter()
                        .map(|(index, direction)| {
                            InstanceField::Vector(vec![
                                InstanceField::Integer {
                                    value: i64::from(*index),
                                    signed: false,
                                    size: 4,
                                },
                                InstanceField::Integer {
                                    value: *direction,
                                    signed: true,
                                    size: 4,
                                },
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        if has(element, PIPE_RADIUS_FIELD) {
            continue;
        }
        let (Some([start, end]), Some(outer), Some(bbox)) =
            (pipe.ends, pipe.outer_diameter, element_record_bbox(element))
        else {
            continue;
        };
        let radius = outer / 2.0;
        // RE-131's test: the record box gives this radius and is the box the
        // cylinder fills.
        if !ppa::pipe_body(bbox, start, end)
            .is_some_and(|fitted| (fitted.radius_feet - radius).abs() <= ppa::RADIUS_TOLERANCE_FEET)
        {
            continue;
        }
        for (names, point) in [(PIPE_AXIS_START_FIELDS, start), (PIPE_AXIS_END_FIELDS, end)] {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
        element.fields.push((
            PIPE_RADIUS_FIELD.into(),
            InstanceField::Float {
                value: radius,
                size: 8,
            },
        ));
        // Which of its connectors each end is (#847), as the connector
        // entries key it: the pipe itself and the connector's index.
        if let (Some(connectors), Some(id)) = (pipe.connectors, element.id) {
            for (name, index) in PIPE_END_KEYS_FIELDS.iter().zip(connectors) {
                element
                    .fields
                    .push(((*name).into(), end_keys_field(&[(u64::from(id), index)])));
            }
        }
    }
}

/// The fields [`attach_native_ducts`] reads from a duct's `RbsDuctCurve`
/// record and its type's `AbsDuctType` record (`graph_fields`, #1335).
fn duct_read_fields(class: &str) -> Option<&'static [&'static str]> {
    Some(match class {
        "RbsDuctCurve" => &[
            "m_id",
            "m_idType",
            "m_assocLevelId",
            "m_idLevelStart",
            "m_dWidthOrDiameter",
            "m_dHeight",
            "m_vNormal",
            "m_dOffsetStart",
            "m_dOffsetEnd",
            "m_eHorOffset",
            "m_eVertOffset",
            "m_origin",
            "m_dirVec",
            "m_endParams",
            "m_nIndex",
            "m_arrRefs",
            "m_pConnector",
            "m_paramOnCurve",
        ],
        "AbsDuctType" => &["m_id", "m_name"],
        _ => &["m_id"],
    })
}

/// What a duct's own record holds (RE-371): its type, the two ends of its one
/// `GLine`, its section's width and height and the direction its height lies
/// along (`m_vNormal`), feet, its reference level and the elevations of its
/// two ends above it where its justification is centred, and the index of
/// the connector at each end.
#[derive(Debug, Clone, PartialEq)]
struct NativeDuct {
    type_id: Option<u32>,
    ends: Option<[[f64; 3]; 2]>,
    size: Option<(f64, f64)>,
    normal: Option<[f64; 3]>,
    level: Option<u32>,
    offsets: Option<[f64; 2]>,
    connectors: Option<[u32; 2]>,
    /// Each tap: its connector's index and its parameter along the line.
    taps: Vec<(u32, f64)>,
    /// Each connector's first join to another element (RE-171's type-1
    /// reference): its index, the element and that element's connector.
    joins: Vec<(u32, u32, u32)>,
}

/// Each connector a duct's `SegmentConnectorPosition` puts between its
/// line's ends (a tap, RE-371), with its parameter, by index.
fn duct_taps(graph: &crate::native_parameters::ObjectGraph) -> Vec<(u32, f64)> {
    let index_of_token = connector_indices(graph);
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "SegmentConnectorPosition")
        .filter_map(|position| {
            let at = position
                .fields
                .get("m_paramOnCurve")?
                .as_f64()
                .filter(|at| *at > 0.0 && *at < 1.0)?;
            let token = position
                .fields
                .pointer("/m_pConnector/pointer_token")?
                .as_u64()?;
            Some((*index_of_token.get(&token)?, at))
        })
        .collect()
}

/// Each `Connector` object's index (`m_nIndex`), by its pointer token.
fn connector_indices(graph: &crate::native_parameters::ObjectGraph) -> BTreeMap<u64, u32> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "Connector")
        .filter_map(|connector| {
            let index = connector.fields.get("m_nIndex")?.as_u64()?;
            Some((u64::from(connector.token), u32::try_from(index).ok()?))
        })
        .collect()
}

/// Each of a duct's or pipe's connectors' first join to another element: the first of
/// its `m_arrRefs` of connection type 1 (RE-171) that names an element other
/// than the duct, with that element's connector index.
fn segment_joins(graph: &crate::native_parameters::ObjectGraph, own: u64) -> Vec<(u32, u32, u32)> {
    use crate::native_json::identifier_i64;
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "Connector")
        .filter_map(|connector| {
            let index = u32::try_from(connector.fields.get("m_nIndex")?.as_u64()?).ok()?;
            connector
                .fields
                .get("m_arrRefs")?
                .as_array()?
                .iter()
                .find_map(|reference| {
                    if reference.get("m_connType")?.as_u64()? != 1 {
                        return None;
                    }
                    let other = u64::try_from(identifier_i64(reference.get("m_id")?)?).ok()?;
                    let other_index = u32::try_from(reference.get("m_nIndex")?.as_u64()?).ok()?;
                    (other != own).then_some((index, u32::try_from(other).ok()?, other_index))
                })
        })
        .collect()
}

fn native_duct(graph: &crate::native_parameters::ObjectGraph, own: u64) -> NativeDuct {
    use crate::native_json::{finite_triple, identifier_i64};
    let root = only_object(graph, "RbsDuctCurve");
    let field = |name: &str| root.and_then(|root| root.fields.get(name));
    // An unset id is -1, which no `u32` holds.
    let id = |name: &str| {
        field(name)
            .and_then(identifier_i64)
            .and_then(|id| u32::try_from(id).ok())
    };
    let float = |name: &str| {
        field(name)
            .and_then(crate::graph_value::Fv::as_f64)
            .filter(|value| value.is_finite())
    };
    let positive = |name: &str| float(name).filter(|value| *value > 0.0);
    // The offsets are the centreline's elevations where the duct is justified
    // by its centre across and up (both 0, Center and Middle): the one case
    // Revit's export is measured on.
    let centred = [field("m_eHorOffset"), field("m_eVertOffset")]
        .iter()
        .all(|value| value.and_then(crate::graph_value::Fv::as_i64) == Some(0));
    let normal = field("m_vNormal").and_then(finite_triple).filter(|normal| {
        let length = normal.iter().map(|v| v * v).sum::<f64>().sqrt();
        (length - 1.0).abs() <= 1e-6
    });
    NativeDuct {
        type_id: id("m_idType"),
        ends: line_ends(graph),
        size: positive("m_dWidthOrDiameter").zip(positive("m_dHeight")),
        normal,
        // Before Revit 2024 the reference level is `m_idLevelStart`, and
        // `m_assocLevelId` names none (RE-371).
        level: id("m_assocLevelId").or_else(|| id("m_idLevelStart")),
        offsets: float("m_dOffsetStart")
            .zip(float("m_dOffsetEnd"))
            .filter(|_| centred)
            .map(|(start, end)| [start, end]),
        connectors: end_connectors(graph, true),
        taps: duct_taps(graph),
        joins: segment_joins(graph, own),
    }
}

/// `normal` made square to the unit vector `line`, where it is within 1e-3
/// of square already: a duct level to within 0.0002 ft over 14 ft (on
/// Autodesk's `rme_basic` sample) has a normal straight up.
pub fn square_to(normal: [f64; 3], line: [f64; 3]) -> Option<[f64; 3]> {
    let along: f64 = (0..3).map(|axis| normal[axis] * line[axis]).sum();
    if along.abs() > 1e-3 {
        return None;
    }
    let squared: [f64; 3] = std::array::from_fn(|axis| normal[axis] - along * line[axis]);
    let length = squared.iter().map(|v| v * v).sum::<f64>().sqrt();
    Some(squared.map(|v| v / length))
}

/// The box a duct's section of `width` by `height` swept from `start` to
/// `end` fills, its height along `normal` (`None` for a round section of
/// diameter `width`), as [`element_record_bbox`] gives a record box: the
/// minimum corner, then the maximum.
fn swept_duct_box(
    start: [f64; 3],
    end: [f64; 3],
    width: f64,
    height: f64,
    normal: Option<[f64; 3]>,
) -> Option<[f64; 6]> {
    let length = (0..3)
        .map(|axis| (end[axis] - start[axis]).powi(2))
        .sum::<f64>()
        .sqrt();
    if !(length.is_finite() && length > 0.0) {
        return None;
    }
    let unit: [f64; 3] = std::array::from_fn(|axis| (end[axis] - start[axis]) / length);
    let across: [f64; 3] = match normal {
        // A round section reaches its radius out along every direction
        // square to the line.
        None => unit.map(|u| width / 2.0 * (1.0 - u * u).max(0.0).sqrt()),
        Some(n) => {
            // The height lies along the normal, squared to the line, and the
            // width across the normal and the line.
            let n = square_to(n, unit)?;
            let x = [
                n[1] * unit[2] - n[2] * unit[1],
                n[2] * unit[0] - n[0] * unit[2],
                n[0] * unit[1] - n[1] * unit[0],
            ];
            std::array::from_fn(|axis| width / 2.0 * x[axis].abs() + height / 2.0 * n[axis].abs())
        }
    };
    let mut bbox = [0.0; 6];
    for axis in 0..3 {
        bbox[axis] = start[axis].min(end[axis]) - across[axis];
        bbox[axis + 3] = start[axis].max(end[axis]) + across[axis];
    }
    Some(bbox)
}

/// Give each duct what its own `RbsDuctCurve` record holds (RE-371), on the
/// releases the native record path reads (`NATIVE_DUCT_REVIT_VERSIONS`): its
/// type (`m_idType`, an `AbsDuctType` whose `SymbolInfo.m_name` is its name,
/// RE-240's rule for a pipe's) and so its system family, its section
/// (`m_dWidthOrDiameter`, `m_dHeight`), and the line it is drawn along, its
/// one `GLine` between its two end parameters, at any slope: its ends, its
/// length, the direction its section's height lies along (`m_vNormal`), and
/// which of its connectors is at each end, keyed as RE-131's connector
/// entries key a pipe's (#847). The line is held to a box test like
/// RE-131's: the section swept along it must fill the record box, a round
/// section as a circle and any other as its rectangle; an oval duct, whose
/// sweep is neither, keeps its box. Where its justification is centred, its
/// reference level's name (`m_assocLevelId`) and the elevations of its two
/// ends above it (`m_dOffsetStart`, `m_dOffsetEnd`) are given too, for its
/// `Constraints`. A value is written only where every copy of the record
/// agrees. It runs before RE-134's scan (2025 to 2027,
/// `CURVE_FIELDS_SUPPORTED_REVIT_VERSIONS`), so the record's type, size and
/// length win where both read them.
// Tracker: B59.
pub(super) fn attach_native_ducts(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::native_duct_shapes::DuctShape;
    use crate::partition_level_parameters as lp;
    if !NATIVE_DUCT_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let wanting =
        |element: &DecodedElement| element.class == "Duct" && !has(element, DUCT_NORMAL_FIELDS[0]);
    let selected: BTreeSet<u64> = products
        .iter()
        .filter(|element| wanting(element))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if selected.is_empty() {
        return;
    }
    let options = crate::native_document::Options {
        selected_ids: selected.clone(),
        selected_classes: ["AbsDuctType".to_string()].into(),
        graph_fields: Some(duct_read_fields),
        releases: crate::native_document::NATIVE_RELEASES,
        // Each duct's record is the source of its line and section (#1360).
        value_source: Some(DUCT_RECORD_READER),
        ..crate::native_document::Options::default()
    };
    let mut ducts: BTreeMap<u64, Option<NativeDuct>> = BTreeMap::new();
    let mut types: BTreeMap<u64, Option<(String, Option<DuctShape>)>> = BTreeMap::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if class == "AbsDuctType" {
            let name = graph
                .objects
                .iter()
                .find(|object| object.class_name == "SymbolInfo")
                .and_then(|object| object.fields.get("m_name"))
                .and_then(crate::graph_value::Fv::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string);
            let mut shapes = graph
                .objects
                .iter()
                .filter_map(|object| DuctShape::of_profile(&object.class_name));
            let shape = match (shapes.next(), shapes.next()) {
                (Some(shape), None) => Some(shape),
                _ => None,
            };
            crate::native_json::agree(&mut types, id, name.map(|name| (name, shape)));
        } else if class == "RbsDuctCurve" && selected.contains(&id) {
            crate::native_json::agree(&mut ducts, id, Some(native_duct(graph, id)));
        }
        Ok(())
    });
    if read.is_err() {
        return;
    }
    let level_names: BTreeMap<u32, String> = if ducts
        .values()
        .any(|duct| duct.as_ref().is_some_and(|duct| duct.offsets.is_some()))
    {
        crate::partition_level_records::recover_partition_levels(rf, revit_version)
            .unwrap_or_default()
            .into_iter()
            .map(|level| (level.element_id, level.name))
            .collect()
    } else {
        BTreeMap::new()
    };
    let float = |value: f64| InstanceField::Float { value, size: 8 };
    for element in products.iter_mut().filter(|element| wanting(element)) {
        let Some(Some(duct)) = element.id.and_then(|id| ducts.get(&u64::from(id))) else {
            continue;
        };
        let duct_type = duct
            .type_id
            .and_then(|type_id| Some((type_id, types.get(&u64::from(type_id))?.as_ref()?)));
        if let (false, Some((type_id, (name, _)))) = (has(element, TYPE_NAME_FIELD), duct_type) {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_id,
                },
            ));
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        // Each connector's join (RE-171), as a family instance's are given
        // them: a duct tapped into another joins no family instance.
        push_joins(element, &duct.joins);
        let Some((width, height)) = duct.size else {
            continue;
        };
        if !has(element, CURVE_SIZE_FIELDS[0]) {
            for (name, value) in CURVE_SIZE_FIELDS.iter().zip([width, height]) {
                element.fields.push(((*name).into(), float(value)));
            }
        }
        // The line, where its swept section is the record box.
        let shape = duct_type.and_then(|(_, (_, shape))| *shape);
        let (Some([start, end]), Some(normal), Some(bbox), Some(shape)) =
            (duct.ends, duct.normal, element_record_bbox(element), shape)
        else {
            continue;
        };
        let swept = match shape {
            DuctShape::Round => swept_duct_box(start, end, width, height, None),
            DuctShape::Rectangular => swept_duct_box(start, end, width, height, Some(normal)),
            DuctShape::Oval => None,
        };
        let fills = swept.is_some_and(|swept| {
            swept
                .iter()
                .zip(bbox)
                .all(|(a, b)| (a - b).abs() <= crate::partition_pipe_axes::BOX_TOLERANCE_FEET)
        });
        if !fills {
            continue;
        }
        for (names, point) in DUCT_ENDS_FIELDS.iter().zip([start, end]) {
            for (name, value) in names.iter().zip(point) {
                element.fields.push(((*name).into(), float(value)));
            }
        }
        for (name, value) in DUCT_NORMAL_FIELDS.iter().zip(normal) {
            element.fields.push(((*name).into(), float(value)));
        }
        // Its type's record gives the section's shape: a source of the body
        // beside the duct's own (#1360).
        if let (Some((type_id, _)), Some(id)) = (duct_type, element.id) {
            rf.attribute_value_source(type_id, DUCT_RECORD_READER, id, DUCT_TYPE_READER);
        }
        let length = (0..3)
            .map(|axis| (end[axis] - start[axis]).powi(2))
            .sum::<f64>()
            .sqrt();
        element
            .fields
            .push((DUCT_LENGTH_FIELD.into(), float(length)));
        // Which of its connectors each end is (#847), keyed as a pipe's.
        if let (Some(connectors), Some(id)) = (duct.connectors, element.id) {
            for (name, index) in DUCT_END_KEYS_FIELDS.iter().zip(connectors) {
                element
                    .fields
                    .push(((*name).into(), end_keys_field(&[(u64::from(id), index)])));
            }
        }
        // Its taps, each a connector along its line.
        if !duct.taps.is_empty() {
            element.fields.push((
                DUCT_TAPS_FIELD.into(),
                InstanceField::Vector(
                    duct.taps
                        .iter()
                        .map(|&(index, at)| {
                            InstanceField::Vector(vec![
                                InstanceField::Integer {
                                    value: i64::from(index),
                                    signed: false,
                                    size: 4,
                                },
                                float(at),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        // Its reference level and the elevations of its ends above it.
        if let (Some(offsets), Some(level)) = (
            duct.offsets,
            duct.level.and_then(|level| level_names.get(&level)),
        ) {
            if !has(element, lp::LEVEL_PARAMETER_FIELD) {
                element.fields.push((
                    lp::LEVEL_PARAMETER_FIELD.into(),
                    InstanceField::String(level.clone()),
                ));
            }
            for (name, value) in DUCT_OFFSET_FIELDS.iter().zip(offsets) {
                element.fields.push(((*name).into(), float(value)));
            }
        }
    }
    attach_duct_system_families(rf, products);
}

/// The reader name of a duct's own record as the source of its line and
/// section (RE-371, #1360).
pub const DUCT_RECORD_READER: &str = "native_duct";

/// The reader name of a duct's type's record as the source of its
/// section's shape (RE-371, #1360).
pub const DUCT_TYPE_READER: &str = "native_duct_type";

/// What a duct drawn along its own record's line holds (RE-371): its ends,
/// its section's width and height and the direction its height lies along,
/// and whether its section is round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuctBody {
    pub start: [f64; 3],
    pub end: [f64; 3],
    pub width_feet: f64,
    pub height_feet: f64,
    pub normal: [f64; 3],
    pub round: bool,
}

/// The body a duct's own record gave it (`attach_native_ducts`), where its
/// line was held to its box.
pub fn duct_body_from_fields(fields: &[(String, InstanceField)]) -> Option<DuctBody> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    let [start, end] = duct_ends_from_fields(fields)?;
    let (width_feet, height_feet) = curve_size_from_fields(fields)?;
    // Drawn only where its shape is known to be round or rectangular.
    let round = fields.iter().find_map(|(name, value)| match value {
        InstanceField::String(shape) if name == DUCT_SHAPE_FIELD => match shape.as_str() {
            "ROUND" => Some(true),
            "RECTANGULAR" => Some(false),
            _ => None,
        },
        _ => None,
    })?;
    Some(DuctBody {
        start,
        end,
        width_feet,
        height_feet,
        normal: point(DUCT_NORMAL_FIELDS)?,
        round,
    })
}

/// A duct's Start and End Middle Elevation above its reference level, feet
/// ([`DUCT_OFFSET_FIELDS`], RE-371).
pub fn duct_offsets_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    Some([field(DUCT_OFFSET_FIELDS[0])?, field(DUCT_OFFSET_FIELDS[1])?])
}

/// A duct's taps ([`DUCT_TAPS_FIELD`], RE-371): each connector's index and
/// its parameter along the duct's line.
pub fn duct_taps_from_fields(fields: &[(String, InstanceField)]) -> Vec<(u32, f64)> {
    fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(pairs) if name == DUCT_TAPS_FIELD => Some(
                pairs
                    .iter()
                    .filter_map(|pair| match pair {
                        InstanceField::Vector(parts) => match parts.as_slice() {
                            [
                                InstanceField::Integer { value: index, .. },
                                InstanceField::Float { value: at, .. },
                            ] => Some((u32::try_from(*index).ok()?, *at)),
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// Give a duct or pipe the join its own record lists at each connector
/// (RE-371), as [`attach_connector_pairs`] gives a family instance its own:
/// one tapped into a duct, or joined to a family instance whose record the
/// native path does not decode (several on Revit 2015 to 2022), is joined
/// on no family instance's list. A connector already joined keeps its join.
fn push_joins(element: &mut DecodedElement, joins: &[(u32, u32, u32)]) {
    for &(index, other, other_index) in joins {
        let field = connector_element_field(index);
        if element.fields.iter().any(|(name, _)| *name == field) {
            continue;
        }
        element
            .fields
            .push((field, InstanceField::ElementId { tag: 0, id: other }));
        element.fields.push((
            connector_index_field(index),
            InstanceField::Integer {
                value: i64::from(other_index),
                signed: false,
                size: 4,
            },
        ));
    }
}
