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
        for (name, value) in CURVE_SIZE_FIELDS
            .iter()
            .zip([curve.width_feet, curve.height_feet])
        {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
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
        if element.class == "Duct" {
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
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Duct")
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
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Duct")
    {
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
}

fn native_pipe(graph: &crate::native_parameters::ObjectGraph) -> NativePipe {
    use crate::native_json::{finite_triple, identifier_i64};
    let object = |class: &str| {
        let mut found = graph
            .objects
            .iter()
            .filter(move |object| object.class_name == class);
        match (found.next(), found.next()) {
            (Some(object), None) => Some(object),
            _ => None,
        }
    };
    let positive = |class: &str, field: &str| {
        object(class)?
            .fields
            .get(field)?
            .as_f64()
            .filter(|value| value.is_finite() && *value > 0.0)
    };
    let type_id = object("RbsPipeCurve")
        .and_then(|curve| curve.fields.get("m_idType"))
        .and_then(identifier_i64)
        .and_then(|id| u32::try_from(id).ok());
    // The centreline is the pipe's one line, from its first end parameter to
    // its second.
    let ends = object("GLine").and_then(|line| {
        let origin = finite_triple(line.fields.get("m_origin")?)?;
        let direction = finite_triple(line.fields.get("m_dirVec")?)?;
        let [start, end] = line
            .fields
            .get("m_endParams")?
            .as_array()?
            .iter()
            .map(serde_json::Value::as_f64)
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
    });
    // Each end's connector: a `SegmentConnectorPosition` names its
    // `Connector` by pointer and sits at parameter 0 (the line's start) or 1
    // (its end).
    let mut at_ends: [Vec<u32>; 2] = [Vec::new(), Vec::new()];
    for position in graph
        .objects
        .iter()
        .filter(|object| object.class_name == "SegmentConnectorPosition")
    {
        let end = match position
            .fields
            .get("m_paramOnCurve")
            .and_then(serde_json::Value::as_f64)
        {
            Some(0.0) => 0,
            Some(1.0) => 1,
            _ => {
                at_ends = [Vec::new(), Vec::new()];
                break;
            }
        };
        let index = position
            .fields
            .pointer("/m_pConnector/pointer_token")
            .and_then(serde_json::Value::as_u64)
            .and_then(|token| {
                let mut found = graph.objects.iter().filter(|object| {
                    object.class_name == "Connector" && u64::from(object.token) == token
                });
                match (found.next(), found.next()) {
                    (Some(connector), None) => connector.fields.get("m_nIndex")?.as_u64(),
                    _ => None,
                }
            })
            .and_then(|index| u32::try_from(index).ok());
        match index {
            Some(index) => at_ends[end].push(index),
            None => {
                at_ends = [Vec::new(), Vec::new()];
                break;
            }
        }
    }
    let connectors = match (at_ends[0].as_slice(), at_ends[1].as_slice()) {
        ([start], [end]) if start != end => Some([*start, *end]),
        _ => None,
    };
    NativePipe {
        type_id,
        ends,
        outer_diameter: positive("PipeDomainDesignPropertyManager", "m_dOuterDiameter"),
        connectors,
        directions: pipe_directions(graph),
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
            .and_then(serde_json::Value::as_u64)
            .and_then(|token| index_of_token.get(&token))
            .zip(
                calculation
                    .fields
                    .get("m_eDirection")
                    .and_then(serde_json::Value::as_i64),
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
                .and_then(serde_json::Value::as_str)
                .filter(|name| !name.is_empty())
                .map(str::to_string);
            crate::native_json::agree(&mut type_names, id, name);
        } else if class == "RbsPipeCurve" && selected.contains(&id) {
            crate::native_json::agree(&mut pipes, id, Some(native_pipe(graph)));
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
