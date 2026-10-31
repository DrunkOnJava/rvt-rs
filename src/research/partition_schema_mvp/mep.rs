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
/// ([`crate::partition_pipe_axes`], RE-131). A pipe whose ends are not found,
/// or whose two ends and box no cylinder reproduces, is not given one and
/// keeps its box.
pub(super) fn attach_pipe_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_pipe_axes as ppa;
    if !ppa::supports_revit_version(revit_version) {
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
    let Ok(bodies) = ppa::scan_pipe_bodies(rf, revit_version, &by_stream) else {
        return;
    };
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
