//! Stair names, aggregates, dimensions and railings.

use super::*;

/// Give each stair part the ElementId of the one exported stair its
/// reference list names (#323). On Snowdon Towers every run, landing and
/// stringer Revit aggregates under a stair, but one stringer, names that
/// stair in its reference list. A part that names none, or more than one,
/// stays a standalone element here; [`attach_stair_names`] joins one that
/// names several when exactly one of them has a type listing the part's
/// type (RE-65).
pub(super) fn attach_aggregate_wholes(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let stairs: BTreeSet<u32> = products
        .iter()
        .filter(|element| AGGREGATE_WHOLE_CLASSES.contains(&element.class.as_str()))
        .filter_map(|element| element.id)
        .collect();
    if stairs.is_empty() {
        return;
    }
    for element in products.iter_mut() {
        if !STAIR_PART_CLASSES.contains(&element.class.as_str()) {
            continue;
        }
        let Some((references, _)) = record_references(rf, element) else {
            continue;
        };
        let named: BTreeSet<u32> = references
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| stairs.contains(id) && Some(*id) != element.id)
            .collect();
        if named.len() == 1 {
            let whole = *named.iter().next().expect("one stair");
            element.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
}

/// The word Revit's export puts before a stair part's number (RE-65).
pub(super) fn stair_part_word(class: &str) -> Option<&'static str> {
    match class {
        "StairsRun" => Some("Run"),
        "StairsLanding" => Some("Landing"),
        "StairsStringer" => Some("Stringer"),
        _ => None,
    }
}

/// Name stairs and their parts as Revit's export names them (RE-65).
///
/// - A stair's system family is its type's construction (Assembled or
///   Cast-In-Place Stair), and Revit names the stair `Family:Stair:ElementId`.
/// - A run's family is Monolithic or Non-Monolithic Run, by its run type's
///   monolithic flag (RE-52); a landing's is Non-Monolithic Landing; a
///   stringer's or carriage's is Stringer or Carriage
///   ([`crate::partition_stairs::component_type_at`]). With the type's name
///   these give the part's `ObjectType`.
/// - A part is named after its stair, `<stair> Run 2`, numbered in ElementId
///   order among every record of its category that names the stair,
///   exported or not.
///
/// A stair type lists its run, landing and support types
/// ([`crate::partition_stairs::STAIR_TYPE_COMPONENTS_OFFSET`]). When a
/// part's reference list names several types of its category, or several
/// stairs, the one pair its stair's type lists decides both, and the part
/// joins that stair (#323).
///
/// Measured on Snowdon Towers only (Revit 2024): the other oracles have no
/// stairs.
pub(super) fn attach_stair_names(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_element_records as per;
    use crate::partition_stairs as ps;
    use crate::partition_type_records as ptr;
    use std::collections::BTreeMap;
    if !ps::supports_revit_version(revit_version) || !ptr::supports_revit_version(revit_version) {
        return;
    }
    let has = |element: &DecodedElement, field: &str| {
        element.fields.iter().any(|(name, _)| name == field)
    };
    let id_field = |element: &DecodedElement, field: &str| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == field => Some(*id),
            _ => None,
        })
    };
    if !products.iter().any(|element| element.class == "Stair") {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let type_ids = |rf: &mut RevitFile, category: i64| {
        ptr::type_definition_ids(
            &ptr::scan_type_records(rf, revit_version, category, &declared).unwrap_or_default(),
        )
    };
    let named = |references: &[u64], set: &BTreeSet<u32>| -> BTreeSet<u32> {
        references
            .iter()
            .filter_map(|&slot| u32::try_from(slot).ok())
            .filter(|id| set.contains(id))
            .collect()
    };

    // Stairs: their type, family and name.
    let stair_type_ids = type_ids(rf, per::OST_STAIRS);
    let mut stair_type_of: BTreeMap<u32, u32> = BTreeMap::new();
    for element in products.iter() {
        let (Some(id), "Stair") = (element.id, element.class.as_str()) else {
            continue;
        };
        if let Some((references, _)) = record_references(rf, element) {
            if let Some(type_id) = ptr::unique_type_reference(&references, &stair_type_ids) {
                stair_type_of.insert(id, type_id);
            }
        }
    }
    let stair_types = ps::scan_component_types(
        rf,
        revit_version,
        ps::ComponentKind::Stair,
        &stair_type_of.values().copied().collect(),
    )
    .unwrap_or_default();
    let stair_ids: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "Stair")
        .filter_map(|element| element.id)
        .collect();
    let components_of = |stair: u32| {
        stair_type_of
            .get(&stair)
            .and_then(|type_id| stair_types.get(type_id))
            .map(|found| &found.components)
    };

    // Parts: their type, and their stair where the list names several.
    let part_kinds = [
        ("StairsRun", per::OST_STAIRS_RUNS, None),
        (
            "StairsLanding",
            per::OST_STAIRS_LANDINGS,
            Some(ps::ComponentKind::Landing),
        ),
        (
            "StairsStringer",
            per::OST_STAIRS_STRINGER_CARRIAGE,
            Some(ps::ComponentKind::Support),
        ),
    ];
    let mut part_type: BTreeMap<usize, u32> = BTreeMap::new();
    let mut joins: Vec<(usize, u32)> = Vec::new();
    for (class, category, _) in part_kinds {
        let own_types = type_ids(rf, category);
        for (index, element) in products.iter().enumerate() {
            if element.class != class {
                continue;
            }
            let Some((references, _)) = record_references(rf, element) else {
                continue;
            };
            let candidates = match id_field(element, TYPE_ID_FIELD) {
                Some(id) => BTreeSet::from([id]),
                None => named(&references, &own_types),
            };
            let stairs: Vec<u32> = match id_field(element, AGGREGATE_WHOLE_FIELD) {
                Some(stair) => vec![stair],
                None => named(&references, &stair_ids)
                    .into_iter()
                    .filter(|id| Some(*id) != element.id)
                    .collect(),
            };
            let pairs: Vec<(u32, u32)> = stairs
                .iter()
                .flat_map(|&stair| candidates.iter().map(move |&type_id| (stair, type_id)))
                .filter(|(stair, type_id)| {
                    components_of(*stair).is_some_and(|listed| listed.contains(type_id))
                })
                .collect();
            let chosen_type = match (candidates.len(), pairs.as_slice()) {
                (1, _) => candidates.iter().next().copied(),
                (_, [(_, type_id)]) => Some(*type_id),
                _ => None,
            };
            if let Some(type_id) = chosen_type {
                part_type.insert(index, type_id);
            }
            if let ([_, _, ..], [(stair, _)]) = (stairs.as_slice(), pairs.as_slice()) {
                joins.push((index, *stair));
            }
        }
    }
    for (index, stair) in joins {
        products[index].fields.push((
            AGGREGATE_WHOLE_FIELD.into(),
            InstanceField::ElementId { tag: 0, id: stair },
        ));
    }

    // Families and type names.
    let mut found: BTreeMap<usize, (Option<&'static str>, Option<String>)> = BTreeMap::new();
    for (index, element) in products.iter().enumerate() {
        if let (Some(id), "Stair") = (element.id, element.class.as_str()) {
            if let Some(stair_type) = stair_type_of.get(&id).and_then(|t| stair_types.get(t)) {
                found.insert(index, (stair_type.family, Some(stair_type.name.clone())));
            }
        }
    }
    for (class, _, kind) in part_kinds {
        let ids: BTreeSet<u32> = part_type
            .iter()
            .filter(|(index, _)| products[**index].class == class)
            .map(|(_, id)| *id)
            .collect();
        let of_class = part_type
            .iter()
            .filter(|(index, _)| products[**index].class == class);
        match kind {
            Some(kind) => {
                let types =
                    ps::scan_component_types(rf, revit_version, kind, &ids).unwrap_or_default();
                for (&index, type_id) in of_class {
                    if let Some(read) = types.get(type_id) {
                        found.insert(index, (read.family, Some(read.name.clone())));
                    }
                }
            }
            None => {
                let types = ps::scan_run_types(rf, revit_version, &ids).unwrap_or_default();
                for (&index, type_id) in of_class {
                    if let Some(read) = types.get(type_id) {
                        let family = if read.monolithic {
                            "Monolithic Run"
                        } else {
                            "Non-Monolithic Run"
                        };
                        found.insert(index, (Some(family), read.name.clone()));
                    }
                }
            }
        }
    }
    for (index, (family, name)) in found {
        let element = &mut products[index];
        if has(element, FAMILY_NAME_FIELD) {
            continue;
        }
        if let (Some(name), false) = (&name, has(element, TYPE_NAME_FIELD)) {
            let type_id = part_type
                .get(&index)
                .copied()
                .or_else(|| element.id.and_then(|id| stair_type_of.get(&id).copied()));
            if let Some(type_id) = type_id {
                element.fields.push((
                    TYPE_ID_FIELD.into(),
                    InstanceField::ElementId {
                        tag: 0,
                        id: type_id,
                    },
                ));
            }
            element
                .fields
                .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
        }
        if let (Some(family), true) = (family, has(element, TYPE_NAME_FIELD)) {
            element.fields.push((
                FAMILY_NAME_FIELD.into(),
                InstanceField::String(family.into()),
            ));
            element.fields.push((
                FAMILY_NAME_SOURCE_FIELD.into(),
                InstanceField::String(TYPE_KIND_FAMILY_SOURCE.into()),
            ));
        }
    }

    // The stairs' names, then each part's number among the records of its
    // category that name its stair.
    let mut stair_names: BTreeMap<u32, String> = BTreeMap::new();
    for element in products.iter_mut() {
        let (Some(id), "Stair") = (element.id, element.class.as_str()) else {
            continue;
        };
        let Some(family) = stair_type_of
            .get(&id)
            .and_then(|type_id| stair_types.get(type_id))
            .and_then(|stair_type| stair_type.family)
        else {
            continue;
        };
        let name = format!("{family}:Stair:{id}");
        element.fields.push((
            ELEMENT_NAME_FIELD.into(),
            InstanceField::String(name.clone()),
        ));
        stair_names.insert(id, name);
    }
    if stair_names.is_empty() {
        return;
    }
    let Some(marker) = per::file_bbox_marker(rf, revit_version) else {
        return;
    };
    let assigned = rf.memoised_second_prologue_ids();
    let none = BTreeMap::new();
    let mut members: BTreeMap<(u32, &'static str), BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        let categories: Vec<i64> = part_kinds
            .iter()
            .map(|(_, category, _)| *category)
            .collect();
        let found = per::find_categories_records_assigned(
            &stream,
            inflated.bytes(),
            &categories,
            &declared,
            &marker,
            ids,
        );
        for ((class, _, _), records) in part_kinds.iter().zip(found) {
            let Some(word) = stair_part_word(class) else {
                continue;
            };
            for record in records {
                for stair in named(&record.references, &stair_ids) {
                    if stair_names.contains_key(&stair) {
                        members
                            .entry((stair, word))
                            .or_default()
                            .insert(record.element_id);
                    }
                }
            }
        }
    }
    for element in products.iter_mut() {
        let (Some(id), Some(word)) = (element.id, stair_part_word(&element.class)) else {
            continue;
        };
        let Some(stair) = id_field(element, AGGREGATE_WHOLE_FIELD) else {
            continue;
        };
        let (Some(stair_name), Some(ids)) = (stair_names.get(&stair), members.get(&(stair, word)))
        else {
            continue;
        };
        let Some(rank) = ids.iter().position(|member| *member == id) else {
            continue;
        };
        element.fields.push((
            ELEMENT_NAME_FIELD.into(),
            InstanceField::String(format!("{stair_name} {word} {}", rank + 1)),
        ));
    }
}

/// Give each Revit 2025 stair its actual risers, riser height and tread
/// depth, and each run its risers, its height over its risers, its stair's
/// tread depth and its number of treads, from their native records
/// ([`crate::native_stairs::native_stair_counts`]).
pub(super) fn attach_native_stair_dimensions(rf: &mut RevitFile, products: &mut [DecodedElement]) {
    let ids = |class: &str| -> BTreeSet<u64> {
        products
            .iter()
            .filter(|element| element.class == class)
            .filter_map(|element| element.id.map(u64::from))
            .collect()
    };
    let (stairs, runs) = (ids("Stair"), ids("StairsRun"));
    let Ok((stair_counts, run_counts)) =
        crate::native_stairs::native_stair_counts(rf, &stairs, &runs)
    else {
        return;
    };
    let integer = |value: i64| InstanceField::Integer {
        value,
        signed: false,
        size: 4,
    };
    let scalar = |value: f64| InstanceField::Float { value, size: 8 };
    for element in products.iter_mut() {
        let Some(id) = element.id.map(u64::from) else {
            continue;
        };
        let mut fields = Vec::new();
        if element.class == "Stair" {
            if let Some(found) = stair_counts.get(&id) {
                fields.push((STAIR_RISER_COUNT_FIELD, integer(found.risers)));
                fields.push((STAIR_RISER_HEIGHT_FIELD, scalar(found.riser_height_feet)));
                fields.push((STAIR_TREAD_DEPTH_FIELD, scalar(found.tread_depth_feet)));
            }
        } else if element.class == "StairsRun" {
            if let Some(run) = run_counts.get(&id) {
                fields.push((STAIR_RISER_COUNT_FIELD, integer(run.risers)));
                fields.push((
                    STAIR_RISER_HEIGHT_FIELD,
                    scalar(run.height_feet / run.risers as f64),
                ));
                if let Some(stair) = stair_counts.get(&run.stair) {
                    fields.push((STAIR_TREAD_DEPTH_FIELD, scalar(stair.tread_depth_feet)));
                }
                let treads = run.risers - 1
                    + i64::from(!run.begin_with_riser)
                    + i64::from(!run.end_with_riser);
                fields.push((STAIR_TREAD_COUNT_FIELD, integer(treads)));
            }
        }
        element
            .fields
            .extend(fields.into_iter().map(|(name, value)| (name.into(), value)));
    }
}

/// Give each stair its riser height, tread depth and number of risers, and
/// each of its runs the same riser height and tread depth and, where
/// [`crate::partition_stairs::flight_riser_counts`] can tell, its own number
/// of risers (RE-47). Runs are the stair's parts (#323).
pub(super) fn attach_stair_dimensions(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_stairs as ps;
    if crate::native_stairs::NATIVE_RUN_REVIT_VERSIONS.contains(&revit_version) {
        attach_native_stair_dimensions(rf, products);
        return;
    }
    if !ps::supports_revit_version(revit_version) {
        return;
    }
    let stairs: Vec<u32> = products
        .iter()
        .filter(|element| element.class == "Stair")
        .filter_map(|element| element.id)
        .collect();
    let whole_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == AGGREGATE_WHOLE_FIELD => Some(*id),
            _ => None,
        })
    };
    let runs: Vec<(u32, u32)> = products
        .iter()
        .filter(|element| element.class == "StairsRun")
        .filter_map(|element| Some((element.id?, whole_of(element)?)))
        .filter(|(_, stair)| stairs.contains(stair))
        .collect();
    if stairs.is_empty() {
        return;
    }
    let run_ids: Vec<u32> = runs.iter().map(|(run, _)| *run).collect();
    let Ok((dimensions, run_counts)) =
        ps::scan_stair_dimensions(rf, revit_version, &stairs, &run_ids)
    else {
        return;
    };
    let mut flight_counts: BTreeMap<u32, u32> = BTreeMap::new();
    for (stair, found) in &dimensions {
        let of_stair: Vec<(u32, Option<u32>)> = runs
            .iter()
            .filter(|(_, whole)| whole == stair)
            .map(|(run, _)| (*run, run_counts.get(run).copied()))
            .collect();
        flight_counts.extend(ps::flight_riser_counts(found.riser_count, &of_stair));
    }
    let push = |element: &mut DecodedElement, found: &ps::StairDimensions, risers: Option<u32>| {
        if let Some(risers) = risers {
            element.fields.push((
                STAIR_RISER_COUNT_FIELD.into(),
                InstanceField::Integer {
                    value: i64::from(risers),
                    signed: false,
                    size: 4,
                },
            ));
        }
        element.fields.push((
            STAIR_RISER_HEIGHT_FIELD.into(),
            InstanceField::Float {
                value: found.riser_height_feet,
                size: 8,
            },
        ));
        element.fields.push((
            STAIR_TREAD_DEPTH_FIELD.into(),
            InstanceField::Float {
                value: found.tread_depth_feet,
                size: 8,
            },
        ));
    };
    for element in products.iter_mut() {
        let Some(id) = element.id else {
            continue;
        };
        if element.class == "Stair" {
            if let Some(found) = dimensions.get(&id) {
                push(element, found, Some(found.riser_count));
            }
        } else if element.class == "StairsRun" {
            let Some(found) = whole_of(element).and_then(|stair| dimensions.get(&stair)) else {
                continue;
            };
            let found = *found;
            push(element, &found, flight_counts.get(&id).copied());
        }
    }
}

/// Give each railing of a stair whose path is one line along that stair's
/// runs its `Length` ([`crate::native_stairs::railing_lengths`], RE-254).
pub(super) fn attach_railing_lengths(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if !crate::native_stairs::RAILING_LENGTH_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let railings: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "Railing")
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    let Ok(lengths) = crate::native_stairs::railing_lengths(rf, &railings) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Railing")
    {
        if let Some(length) = element
            .id
            .and_then(|id| lengths.get(&u64::from(id)))
            .copied()
        {
            element.fields.push((
                RAILING_LENGTH_FIELD.into(),
                InstanceField::Float {
                    value: length,
                    size: 8,
                },
            ));
        }
    }
}

/// Give each railing its type's top rail height
/// ([`crate::native_stairs::railing_type_heights`], RE-231).
pub(super) fn attach_railing_heights(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    if !crate::native_stairs::RAILING_HEIGHT_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(u64::from(*id)),
            _ => None,
        })
    };
    let types: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "Railing")
        .filter_map(type_of)
        .collect();
    let Ok(heights) = crate::native_stairs::railing_type_heights(rf, &types) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "Railing")
    {
        if let Some(height) = type_of(element).and_then(|id| heights.get(&id)).copied() {
            element.fields.push((
                RAILING_HEIGHT_FIELD.into(),
                InstanceField::Float {
                    value: height,
                    size: 8,
                },
            ));
        }
    }
}

/// Name Revit 2025 stairs, runs and stringers as Revit's export names them
/// (RE-65), from their native records
/// ([`crate::native_stairs::native_stair_parts`], #323): each takes its
/// type, its type's name and its family, a stair the name
/// `Family:Stair:ElementId` and a part `<stair> Run 2`, numbered in
/// ElementId order among the exported parts of its kind on its stair. A
/// run's type is the one `OST_StairsRuns` type definition its element
/// record names (RE-28). Measured on the five R2025 stair models.
pub(super) fn attach_native_stair_names(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_type_records as ptr;
    if !crate::native_stairs::NATIVE_STAIR_NAME_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let classes = ["Stair", "StairsRun", "StairsStringer"];
    let parts: BTreeSet<u64> = products
        .iter()
        .filter(|element| classes.contains(&element.class.as_str()))
        .filter_map(|element| element.id.map(u64::from))
        .collect();
    if parts.is_empty() {
        return;
    }
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let run_types = ptr::type_definition_ids(
        &ptr::scan_type_records(
            rf,
            revit_version,
            crate::partition_element_records::OST_STAIRS_RUNS,
            &declared,
        )
        .unwrap_or_default(),
    );
    let mut runs: BTreeMap<u64, u64> = BTreeMap::new();
    for element in products
        .iter()
        .filter(|element| element.class == "StairsRun")
    {
        let (Some(id), Some((references, _))) = (element.id, record_references(rf, element)) else {
            continue;
        };
        if let Some(type_id) = ptr::unique_type_reference(&references, &run_types) {
            runs.insert(u64::from(id), u64::from(type_id));
        }
    }
    let Ok(native) = crate::native_stairs::native_stair_parts(rf, &parts, &runs) else {
        return;
    };
    let mut members: BTreeMap<(u64, &'static str), BTreeSet<u64>> = BTreeMap::new();
    for element in products.iter() {
        let (Some(id), Some(word)) = (element.id.map(u64::from), stair_part_word(&element.class))
        else {
            continue;
        };
        if let Some(part) = native.get(&id) {
            members.entry((part.stair, word)).or_default().insert(id);
        }
    }
    let stair_names: BTreeMap<u64, String> = native
        .iter()
        .filter(|(id, part)| part.stair == **id)
        .map(|(id, part)| (*id, format!("{}:Stair:{id}", part.family)))
        .collect();
    for element in products.iter_mut() {
        let Some(id) = element.id.map(u64::from) else {
            continue;
        };
        let Some(part) = native.get(&id) else {
            continue;
        };
        let has = |field: &str| element.fields.iter().any(|(name, _)| name == field);
        if has(FAMILY_NAME_FIELD) {
            continue;
        }
        let (has_type_id, has_type_name, has_name) = (
            has(TYPE_ID_FIELD),
            has(TYPE_NAME_FIELD),
            has(ELEMENT_NAME_FIELD),
        );
        let name = match stair_part_word(&element.class) {
            None => stair_names.get(&id).cloned(),
            Some(word) => stair_names.get(&part.stair).and_then(|stair| {
                let rank = members
                    .get(&(part.stair, word))?
                    .iter()
                    .position(|m| *m == id)?;
                Some(format!("{stair} {word} {}", rank + 1))
            }),
        };
        let Ok(type_id) = u32::try_from(part.type_id) else {
            continue;
        };
        if !has_type_id {
            element.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: type_id,
                },
            ));
        }
        if !has_type_name {
            element.fields.push((
                TYPE_NAME_FIELD.into(),
                InstanceField::String(part.type_name.clone()),
            ));
        }
        element.fields.push((
            FAMILY_NAME_FIELD.into(),
            InstanceField::String(part.family.into()),
        ));
        element.fields.push((
            FAMILY_NAME_SOURCE_FIELD.into(),
            InstanceField::String(TYPE_KIND_FAMILY_SOURCE.into()),
        ));
        if let (Some(name), false) = (name, has_name) {
            element
                .fields
                .push((ELEMENT_NAME_FIELD.into(), InstanceField::String(name)));
        }
    }
}
