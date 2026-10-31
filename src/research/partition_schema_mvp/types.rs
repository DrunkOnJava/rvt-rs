//! Type picks, type and family names, system families and type functions of record-backed elements.

use super::*;

/// Attach each element's IFC export overrides (RE-45): the element's own
/// "Export to IFC As" and predefined type, else those its type sets
/// ([`TYPE_ID_FIELD`], from RE-38 or #322). A type's `IfcCoveringType`
/// names `IfcCovering`.
pub(super) fn attach_ifc_export_overrides(
    rf: &mut RevitFile,
    revit_version: u32,
    lists: [&mut Vec<DecodedElement>; 6],
) {
    use crate::partition_ifc_export_overrides as ieo;
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let parameters = ieo::scan_export_parameters(rf, revit_version, &declared).unwrap_or_default();
    if parameters.is_empty() {
        return;
    }
    let none = ieo::ExportParameters::default();
    for elements in lists {
        for element in elements.iter_mut() {
            let own = element
                .id
                .and_then(|id| parameters.get(&id))
                .unwrap_or(&none);
            let of_type = element
                .fields
                .iter()
                .find_map(|(name, value)| match value {
                    InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
                    _ => None,
                })
                .and_then(|id| parameters.get(&id));
            if let Some(entity) = own.effective_export_as(of_type) {
                element
                    .fields
                    .push((IFC_EXPORT_AS_FIELD.into(), InstanceField::String(entity)));
                element.fields.push((
                    IFC_EXPORT_RELEASE_FIELD.into(),
                    InstanceField::Integer {
                        value: i64::from(revit_version),
                        signed: false,
                        size: 4,
                    },
                ));
            }
            if let Some(predefined) = own.effective_predefined_type(of_type) {
                element.fields.push((
                    IFC_PREDEFINED_TYPE_FIELD.into(),
                    InstanceField::String(predefined),
                ));
            }
        }
    }
}

/// Give elements of system families (walls, floors, roofs, ceilings,
/// railings) their type's name (#322).
///
/// The type is the one type-definition record of the element's category
/// that its reference list names (RE-28,
/// [`crate::partition_type_records::unique_type_reference`]). Its name is
/// read from its serialised data
/// ([`crate::partition_names::find_element_data_names`]). Such a type has no
/// family in the file, since Revit names the system family ("Basic Wall")
/// in its own UI language, so only [`TYPE_ID_FIELD`] and [`TYPE_NAME_FIELD`]
/// are set.
pub(super) fn attach_system_type_names(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &mut [&mut DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if elements.is_empty() || !ptr::supports_revit_version(revit_version) {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let mut type_ids_by_category: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut picks: Vec<(usize, u32)> = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        let Some((references, category)) = record_references(rf, element) else {
            continue;
        };
        let type_ids = type_ids_by_category.entry(category).or_insert_with(|| {
            let records =
                ptr::scan_type_records(rf, revit_version, category, &declared).unwrap_or_default();
            ptr::type_definition_ids(&records)
        });
        if let Some(type_id) = ptr::unique_type_reference(&references, type_ids) {
            picks.push((index, type_id));
        }
    }
    // B45: a railing whose reference list names no type definition holds its
    // type in its own data object.
    let unpicked: Vec<(usize, u32)> = elements
        .iter()
        .enumerate()
        .filter(|(index, element)| {
            element.class == "Railing" && !picks.iter().any(|(picked, _)| picked == index)
        })
        .filter_map(|(index, element)| Some((index, element.id?)))
        .collect();
    let mut railing_names = BTreeMap::new();
    if revit_version >= 2024 && !unpicked.is_empty() {
        let (railing_picks, names) = railing_type_picks(rf, &unpicked);
        picks.extend(railing_picks);
        railing_names = names;
    }
    if picks.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = picks.iter().map(|(_, id)| *id).collect();
    let mut names = type_data_names(rf, &header, &wanted);
    // RE-230: a curtain wall type's data gives no name; its native record
    // does.
    let unread: BTreeSet<u64> = wanted
        .iter()
        .filter(|id| !names.contains_key(id))
        .map(|id| u64::from(*id))
        .collect();
    let curtain_names = if unread.is_empty() {
        BTreeMap::new()
    } else {
        crate::native_walls::curtain_wall_type_names(rf, &unread).unwrap_or_default()
    };
    for (id, name) in curtain_names {
        if let Ok(id) = u32::try_from(id) {
            names.insert(id, Some(name));
        }
    }
    // A railing type with no element data keeps its name in its type object
    // (RE-66, measured on Revit 2024, and B45's RE1 railing on 2025).
    let railing_types: BTreeSet<u32> = picks
        .iter()
        .filter(|(index, id)| {
            elements[*index].class == "Railing" && !matches!(names.get(id), Some(Some(_)))
        })
        .map(|(_, id)| *id)
        .collect();
    let railing_tags = rf
        .memoised_schema_classes()
        .ok()
        .and_then(|classes| crate::partition_names::RailingTypeTags::from_classes(&classes));
    if let Some(tags) = railing_tags.filter(|_| revit_version >= 2024 && !railing_types.is_empty())
    {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            for (id, name) in crate::partition_names::find_railing_type_names(
                inflated.bytes(),
                &railing_types,
                tags,
            ) {
                match names.get(&id) {
                    Some(Some(held)) if *held != name => {
                        names.insert(id, None);
                    }
                    Some(None) => {}
                    _ => {
                        names.insert(id, Some(name));
                    }
                }
            }
        }
    }
    for (id, name) in railing_names {
        if !matches!(names.get(&id), Some(Some(_))) {
            names.insert(id, Some(name));
        }
    }
    attach_type_picks(elements, picks, &names);
}

/// The type of each of `railings` (index, ElementId) that its own
/// `BaseRailing` data object (RE-153) names: the one ElementId in the
/// object's payload, at any offset, of a railing type's `StairsRailingAttr`
/// object. RE1 Architecture's railing 462556 holds its type 446543, Revit's
/// `IfcRailingType` Tag, at +283 of its 329-byte object. With the picks, the
/// picked types' names their own objects hold
/// ([`crate::partition_names::railing_type_name_in`]). Revit 2024 and later.
// Tracker: B45.
pub(super) fn railing_type_picks(
    rf: &mut RevitFile,
    railings: &[(usize, u32)],
) -> (Vec<(usize, u32)>, BTreeMap<u32, String>) {
    let Ok(classes) = rf.memoised_schema_classes() else {
        return Default::default();
    };
    let tag_of = |name: &str| {
        classes
            .classes
            .iter()
            .find(|class| class.name == name)
            .map(|class| u32::from(class.tag))
    };
    let (Some(railing_tag), Some(type_tag)) = (tag_of("BaseRailing"), tag_of("StairsRailingAttr"))
    else {
        return Default::default();
    };
    let name_tags = crate::partition_names::RailingTypeTags::from_classes(&classes);
    let wanted: BTreeSet<u32> = railings.iter().map(|(_, id)| *id).collect();
    let mut type_ids: BTreeSet<u32> = BTreeSet::new();
    // Each type's name where its copies that read one agree; a copy whose
    // name does not read takes no part.
    let mut type_names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    let mut held: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let Ok(objects) = rf.memoised_partition_data_objects(&stream) else {
            continue;
        };
        for &(p, object) in objects.iter() {
            let class = object.class & 0xffff;
            if class == type_tag {
                let name = name_tags.and_then(|tags| {
                    crate::partition_names::railing_type_name_in(&buf[p..object.end], tags)
                });
                type_ids.insert(object.element_id);
                if let Some(name) = name {
                    let held = type_names
                        .entry(object.element_id)
                        .or_insert_with(|| Some(name.clone()));
                    if held.as_deref() != Some(name.as_str()) {
                        *held = None;
                    }
                }
            } else if class == railing_tag && wanted.contains(&object.element_id) {
                let payload =
                    &buf[p + crate::partition_room_parameters::DATA_OBJECT_HEADER..object.end];
                let values = held.entry(object.element_id).or_default();
                for word in payload.windows(4) {
                    values.insert(u32::from_le_bytes(word.try_into().expect("4 bytes")));
                }
            }
        }
    }
    let picks: Vec<(usize, u32)> = railings
        .iter()
        .filter_map(|(index, id)| {
            let mut named = held.get(id)?.intersection(&type_ids);
            match (named.next(), named.next()) {
                (Some(&type_id), None) => Some((*index, type_id)),
                _ => None,
            }
        })
        .collect();
    let names = picks
        .iter()
        .filter_map(|(_, type_id)| Some((*type_id, type_names.get(type_id)?.clone()?)))
        .collect();
    (picks, names)
}

/// Each wanted type's name, read from its serialised data
/// ([`crate::partition_names::find_element_data_names`]), from the latest
/// partition that holds one (#807, RE-145); `None` when two copies there
/// disagree. On the BIM4LCA office's permit file every type has an older copy
/// with its Finnish name and a newer one with its English name.
pub(super) fn type_data_names(
    rf: &mut RevitFile,
    header: &[u8; 10],
    wanted: &BTreeSet<u32>,
) -> BTreeMap<u32, Option<String>> {
    use crate::partition_room_parameters::{LatestCopy, partition_number};
    let mut names = LatestCopy::default();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let partition = partition_number(&stream);
        for (id, name) in
            crate::partition_names::find_element_data_names(inflated.bytes(), header, wanted)
        {
            names.offer(id, partition, name);
        }
    }
    names.into_options()
}

/// Set [`TYPE_ID_FIELD`] and [`TYPE_NAME_FIELD`] on each picked element
/// whose type has one agreed name.
pub(super) fn attach_type_picks(
    elements: &mut [&mut DecodedElement],
    picks: Vec<(usize, u32)>,
    names: &BTreeMap<u32, Option<String>>,
) {
    for (index, type_id) in picks {
        let Some(Some(name)) = names.get(&type_id) else {
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
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
    }
}

/// Give wall sweeps their type and family, or Revit's name for one without a
/// type (RE-69).
///
/// A wall sweep's type is a system-family type with no type-definition
/// record of the sweep category, so RE-44's join finds none. Its record
/// names it among other things: its profile (a family type with a name
/// entry, RE-38), its material, the walls it runs along. The type is the
/// one id the record names that has a type object
/// ([`crate::partition_names::find_type_object_ids`]), no name entry, and
/// an element-data name; the sweep is then named `Wall Sweep:<type>:<id>`.
/// A sweep whose record names no such object at all is a sweep of a wall
/// type's own structure, which Revit names by its ElementId alone.
///
/// Measured on Snowdon Towers only (Revit 2024): 249 sweeps take a type and
/// 9 are named by ElementId, all as Revit's export names them.
pub(super) fn attach_wall_sweep_types(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if revit_version != 2024 {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let mut lists: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in products.iter().enumerate() {
        if element.class != "WallSweep"
            || has(element, TYPE_NAME_FIELD)
            || has(element, FAMILY_NAME_FIELD)
            || has(element, ELEMENT_NAME_FIELD)
        {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| Some(*id) != element.id)
            .collect();
        lists.push((index, named));
    }
    if lists.is_empty() {
        return;
    }
    let entries = rf.memoised_element_names();
    let wanted: BTreeSet<u32> = lists
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .filter(|id| !entries.entries.contains_key(id))
        .collect();
    let mut objects: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        objects.extend(crate::partition_names::find_type_object_ids(
            inflated.bytes(),
            &wanted,
        ));
    }
    let names = type_data_names(rf, &header, &objects);
    for (index, ids) in lists {
        let types: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|id| objects.contains(id))
            .collect();
        let element = &mut products[index];
        match types.as_slice() {
            [] => {
                if let Some(id) = element.id {
                    element.fields.push((
                        ELEMENT_NAME_FIELD.into(),
                        InstanceField::String(id.to_string()),
                    ));
                }
            }
            [type_id] => {
                let Some(Some(name)) = names.get(type_id) else {
                    continue;
                };
                element.fields.push((
                    TYPE_ID_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: *type_id,
                    },
                ));
                element
                    .fields
                    .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
                element.fields.push((
                    FAMILY_NAME_FIELD.into(),
                    InstanceField::String("Wall Sweep".into()),
                ));
                element.fields.push((
                    FAMILY_NAME_SOURCE_FIELD.into(),
                    InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
                ));
            }
            _ => {}
        }
    }
}

/// Give each pipe, wall sweep and ramp still without a type, or a name of
/// its own, the type its native record names
/// ([`crate::native_element_types::system_type_names`], RE-240). Its family
/// is then its class's system family ([`system_family`]), and a sweep's is
/// Wall Sweep, as RE-69 names it.
pub(super) fn attach_native_type_names(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let wanted: BTreeMap<u64, String> = products
        .iter()
        .filter(|element| {
            crate::native_element_types::reads_class(&element.class)
                && !has(element, TYPE_NAME_FIELD)
                && !has(element, ELEMENT_NAME_FIELD)
        })
        .filter_map(|element| Some((u64::from(element.id?), element.class.clone())))
        .collect();
    if wanted.is_empty() {
        return;
    }
    let Ok(found) = crate::native_element_types::system_type_names(rf, &wanted) else {
        return;
    };
    for element in products.iter_mut() {
        let Some((type_id, name)) = element.id.and_then(|id| found.get(&u64::from(id))) else {
            continue;
        };
        let Ok(type_id) = u32::try_from(*type_id) else {
            continue;
        };
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
        if element.class == "WallSweep" {
            element.fields.push((
                FAMILY_NAME_FIELD.into(),
                InstanceField::String("Wall Sweep".into()),
            ));
            element.fields.push((
                FAMILY_NAME_SOURCE_FIELD.into(),
                InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
            ));
        }
    }
}

/// Give model text its type and family (RE-67).
///
/// A model text element is in the Generic Models category, but it is not a
/// family instance: its reference list names a text type, which has no name
/// entry (RE-38) and no type-definition record of the category. The text
/// type is the one id the list names whose type object carries a font
/// ([`crate::partition_names::find_text_type_names`]), and Revit names the
/// element `Model Text:<type>:<ElementId>`. Measured on Snowdon Towers only
/// (Revit 2024), where it names all 7 model texts as Revit's export does.
pub(super) fn attach_model_text_types(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if revit_version != 2024 {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let mut lists: Vec<(usize, BTreeSet<u32>)> = Vec::new();
    for (index, element) in products.iter().enumerate() {
        if element.class != "GenericModel"
            || has(element, TYPE_NAME_FIELD)
            || has(element, FAMILY_NAME_FIELD)
        {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| Some(*id) != element.id)
            .collect();
        lists.push((index, named));
    }
    if lists.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = lists
        .iter()
        .flat_map(|(_, ids)| ids.iter().copied())
        .collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in crate::partition_names::find_text_type_names(inflated.bytes(), &wanted) {
            match names.get(&id) {
                Some(Some(held)) if *held != name => {
                    names.insert(id, None);
                }
                Some(None) => {}
                _ => {
                    names.insert(id, Some(name));
                }
            }
        }
    }
    for (index, ids) in lists {
        let text_types: Vec<(u32, &String)> = ids
            .iter()
            .filter_map(|id| match names.get(id) {
                Some(Some(name)) => Some((*id, name)),
                _ => None,
            })
            .collect();
        let [(type_id, name)] = text_types.as_slice() else {
            continue;
        };
        let element = &mut products[index];
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: *type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String((*name).clone()),
        ));
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String("Model Text".into()),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(TYPE_KIND_FAMILY_SOURCE.into()),
        ));
    }
}

/// Give a curtain panel that is a wall its wall type (RE-64).
///
/// Revit lets a curtain grid cell hold a basic wall instead of a panel. The
/// element keeps the panel category, but its reference list names a wall
/// type rather than a panel type, so [`attach_system_type_names`] finds no
/// type for it. Here the type is the one wall-type record the list names,
/// taken only when that type has compound layers
/// ([`crate::partition_compound_structure::scan_type_layers`]): a panel's
/// list also names its curtain wall, whose type is a wall type with none.
/// [`attach_system_family_names`] then names its family "Basic Wall".
pub(super) fn attach_panel_wall_types(
    rf: &mut RevitFile,
    revit_version: u32,
    panels: &mut [&mut DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if panels.is_empty()
        || !ptr::supports_revit_version(revit_version)
        || !crate::partition_compound_structure::COMPOUND_STRUCTURE_SUPPORTED_REVIT_VERSIONS
            .contains(&revit_version)
    {
        return;
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return;
    };
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let wall_types = ptr::type_definition_ids(
        &ptr::scan_type_records(
            rf,
            revit_version,
            crate::partition_element_records::OST_WALLS,
            &declared,
        )
        .unwrap_or_default(),
    );
    let mut picks: Vec<(usize, u32)> = Vec::new();
    for (index, panel) in panels.iter().enumerate() {
        let Some((references, _)) = record_references(rf, panel) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &wall_types) {
            picks.push((index, type_id));
        }
    }
    if picks.is_empty() {
        return;
    }
    let candidates: BTreeSet<u32> = picks.iter().map(|(_, id)| *id).collect();
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(rf, revit_version, ptr::OST_MATERIALS, &declared)
            .unwrap_or_default()
            .iter()
            .map(|record| record.element_id)
            .collect();
    let layered: BTreeSet<u32> = crate::partition_compound_structure::scan_type_layers(
        rf,
        revit_version,
        &candidates,
        &materials,
        &declared,
    )
    .unwrap_or_default()
    .into_iter()
    .filter(|(_, layers)| !layers.is_empty())
    .map(|(id, _)| id)
    .collect();
    picks.retain(|(_, id)| layered.contains(id));
    let names = type_data_names(rf, &header, &layered);
    attach_type_picks(panels, picks, &names);
    // RE-72: Revit exports such a panel as a curtain wall of its own.
    for panel in panels.iter_mut() {
        let wall_type = panel.fields.iter().any(|(name, value)| {
            matches!(value, InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD && layered.contains(id))
        });
        if wall_type {
            panel
                .fields
                .push((CURTAIN_PANEL_WALL_FIELD.into(), InstanceField::Bool(true)));
        }
    }
}

/// Name each record-backed element's class (RE-76): the tag its record
/// holds at `+0x4a` is the definition ordinal of a class in the file's own
/// schema, `SWall`, `ArcWall`, `Floor`, `FamilyInstance` and so on. An
/// element whose tag resolves to no class gets no name.
pub(super) fn attach_revit_classes(
    classes: &crate::formats::SchemaClasses,
    elements: &mut [DecodedElement],
) {
    for element in elements.iter_mut() {
        let tag = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Integer { value, .. } if name == CLASS_TAG_FIELD => {
                u16::try_from(*value).ok()
            }
            _ => None,
        });
        if let Some(class) = tag.and_then(|tag| classes.by_tag(tag)) {
            element.fields.push((
                REVIT_CLASS_FIELD.into(),
                InstanceField::String(class.name.clone()),
            ));
        }
    }
}

pub(super) fn attach_family_and_type_names(rf: &mut RevitFile, elements: &mut [DecodedElement]) {
    let names = rf.memoised_element_names();
    if names.entries.is_empty() {
        return;
    }
    for element in elements.iter_mut() {
        let Some(own) = element.id else {
            continue;
        };
        let Some((references, category)) = record_references(rf, element) else {
            continue;
        };
        let Some(type_id) =
            crate::partition_names::resolve_type(&names, &references, category, own)
        else {
            continue;
        };
        let Some(family_id) = crate::partition_names::resolve_family(&names, type_id) else {
            continue;
        };
        let (Some(type_entry), Some(family_entry)) =
            (names.entries.get(&type_id), names.entries.get(&family_id))
        else {
            continue;
        };
        element.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        element.fields.push((
            TYPE_NAME_FIELD.into(),
            InstanceField::String(type_entry.name.clone()),
        ));
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(family_entry.name.clone()),
        ));
    }
}

/// The Revit system family of a record-backed element's type (RE-61,
/// RE-63). Revit derives the name rather than storing it, from the type's
/// kind, and the category gives it where only one system family fits:
/// - a curtain wall (RE-46) is a Curtain Wall, and a railing a Railing;
/// - a floor is a Floor, and a building pad a Pad;
/// - a slab edge is a Slab Edge, and a ramp a Ramp (RE-64);
/// - a wall, ceiling or roof whose type has compound layers (`has_layers`)
///   is a Basic Wall, a Compound Ceiling or a Basic Roof, since a Curtain
///   or Stacked Wall, a Basic Ceiling and Sloped Glazing have none;
/// - a curtain panel whose type is a wall type with layers is a Basic Wall
///   (RE-64);
/// - a pipe is a Pipe Types, Revit's one pipe system family (RE-130).
///
/// Measured against the `Family:Type:ElementId` names Revit's own IFC
/// export gives the same elements, on every record-backed wall, floor,
/// ceiling, roof and building pad of Snowdon Towers, Core Interior and RE1
/// Architecture: 1,444 walls, 283 floors, 74 ceilings, 20 roofs and 1 pad.
pub fn system_family(class: &str, has_layers: bool) -> Option<&'static str> {
    match (class, has_layers) {
        (CURTAIN_WALL_CLASS, _) => Some("Curtain Wall"),
        ("Railing", _) => Some("Railing"),
        ("Floor", _) => Some("Floor"),
        ("BuildingPad", _) => Some("Pad"),
        ("SlabEdge", _) => Some("Slab Edge"),
        ("Ramp", _) => Some("Ramp"),
        ("Pipe", _) => Some("Pipe Types"),
        ("Wall" | "CurtainWallPanel", true) => Some("Basic Wall"),
        ("Ceiling", true) => Some("Compound Ceiling"),
        ("Roof", true) => Some("Basic Roof"),
        _ => None,
    }
}

/// The name Revit's export gives system family `family` in a file saved
/// in `locale` (`BasicFileInfo`'s "Locale when saved", e.g. `ENU`, `PTB`).
///
/// A system family's name is not in the file: Revit supplies it in the
/// language the file is saved in. Four files saved in `PTB` (two Revit
/// 2023, two Revit 2025) name their walls, floors and roofs this way
/// in Revit's own export, 28 walls, 3 floors and 1 roof, and no other
/// system family occurs in them (RE-123). Every other family, and every
/// other locale, keeps the `ENU` name [`system_family`] gives, which is
/// what the `ENU` files measure.
pub fn localized_system_family(family: &'static str, locale: Option<&str>) -> &'static str {
    match (locale, family) {
        (Some("PTB"), "Basic Wall") => "Parede básica",
        (Some("PTB"), "Floor") => "Piso",
        (Some("PTB"), "Basic Roof") => "Telhado básico",
        _ => family,
    }
}

/// Give each system-family element with a type name (#322) its system
/// family's name ([`system_family`]), so it is named `Family:Type:ElementId`
/// as Revit names it (RE-63). Whether a wall's, ceiling's or roof's type has
/// compound layers is the type's: an element of the type drawn in layers
/// shows it, and otherwise the type's own data is read
/// ([`crate::partition_compound_structure::scan_type_layers`]).
pub(super) fn attach_system_family_names(
    rf: &mut RevitFile,
    revit_version: u32,
    elements: &mut [&mut Vec<DecodedElement>],
) {
    use crate::partition_compound_structure as pcs;
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let shows_layers = |element: &DecodedElement| {
        element.fields.iter().any(|(name, _)| {
            name == WALL_LAYERS_FIELD
                || name == SLAB_LAYERS_FIELD
                || name == ROOF_TYPE_THICKNESS_FIELD
        })
    };
    let mut layered: BTreeSet<u32> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| shows_layers(element))
        .filter_map(type_of)
        .collect();
    let pending: BTreeSet<u32> = elements
        .iter()
        .flat_map(|list| list.iter())
        .filter(|element| {
            matches!(
                element.class.as_str(),
                "Wall" | "Ceiling" | "Roof" | "CurtainWallPanel"
            )
        })
        .filter_map(type_of)
        .filter(|id| !layered.contains(id))
        .collect();
    if !pending.is_empty() && pcs::layer_layout(revit_version).is_some() {
        if let Ok(records) = crate::elem_table::parse_records(rf) {
            let declared = crate::elem_table::declared_ids(&records);
            // Revit 2023 has no type records, and its materials no element
            // records either: a layer's material is only held to be
            // declared (RE-112).
            let materials: BTreeSet<u32> =
                if crate::partition_element_records_2023::is_32bit_release(revit_version) {
                    declared.clone()
                } else {
                    crate::partition_type_records::scan_type_records(
                        rf,
                        revit_version,
                        crate::partition_type_records::OST_MATERIALS,
                        &declared,
                    )
                    .unwrap_or_default()
                    .iter()
                    .map(|record| record.element_id)
                    .collect()
                };
            if let Ok(types) =
                pcs::scan_type_layers(rf, revit_version, &pending, &materials, &declared)
            {
                layered.extend(
                    types
                        .into_iter()
                        .filter(|(_, layers)| !layers.is_empty())
                        .map(|(id, _)| id),
                );
            }
        }
    }
    let locale = rf.basic_file_info().ok().and_then(|info| info.locale);
    for element in elements.iter_mut().flat_map(|list| list.iter_mut()) {
        let has = |field: &str| element.fields.iter().any(|(name, _)| name == field);
        if !has(TYPE_NAME_FIELD) || has(FAMILY_NAME_FIELD) {
            continue;
        }
        let has_layers = type_of(element).is_some_and(|id| layered.contains(&id));
        let Some(family) = system_family(&element.class, has_layers) else {
            continue;
        };
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(localized_system_family(family, locale.as_deref()).into()),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(SYSTEM_FAMILY_SOURCE.into()),
        ));
    }
}

/// Give each element of `groups` whose type is known its type's Function,
/// the integer entry `FUNCTION_PARAM` in the type's own data object
/// (RE-158). A type whose entries disagree, or that has none, gives nothing.
pub(super) fn attach_type_functions(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 6],
) {
    use crate::partition_room_parameters as prp;
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let types: BTreeSet<u32> = groups
        .iter()
        .flat_map(|elements| elements.iter().filter_map(type_of))
        .collect();
    let functions = prp::scan_integer_parameter(rf, revit_version, &types, prp::FUNCTION_PARAMETER);
    for element in groups.into_iter().flat_map(|elements| elements.iter_mut()) {
        if let Some(&function) = type_of(element).and_then(|id| functions.get(&id)) {
            element.fields.push((
                TYPE_FUNCTION_FIELD.into(),
                InstanceField::Integer {
                    value: i64::from(function),
                    signed: false,
                    size: 4,
                },
            ));
        }
    }
}

/// Give each of `walls` with no [`TYPE_FUNCTION_FIELD`] its type's Function
/// from the type's native record ([`crate::native_wall_types`], RE-185).
/// Nothing on a release not measured, or where the native records are not
/// read.
pub(super) fn attach_wall_type_functions<'a>(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: impl IntoIterator<Item = &'a mut DecodedElement>,
) {
    if !WALL_TYPE_FUNCTION_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let has_function = |element: &DecodedElement| {
        element
            .fields
            .iter()
            .any(|(name, _)| name == TYPE_FUNCTION_FIELD)
    };
    let mut walls: Vec<&mut DecodedElement> = walls
        .into_iter()
        .filter(|element| !has_function(element))
        .collect();
    let ids: BTreeSet<u64> = walls
        .iter()
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if ids.is_empty() {
        return;
    }
    let Ok(functions) = crate::native_wall_types::wall_type_functions(rf, &ids) else {
        return;
    };
    for element in walls.iter_mut() {
        if let Some(&function) = element.id.and_then(|id| functions.get(&u64::from(id))) {
            element.fields.push((
                TYPE_FUNCTION_FIELD.into(),
                InstanceField::Integer {
                    value: function,
                    signed: true,
                    size: 8,
                },
            ));
        }
    }
}
