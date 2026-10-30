//! The Revit 2023 record path.

use super::*;

/// Revit 2023 elements from their partition element records (RE-81): each
/// placed instance of a [`crate::partition_element_records::RECOVERED_CATEGORIES`]
/// category, with its box as its body, into the bucket its category
/// feeds. Components nested in doors and windows are left out
/// ([`nested_in_openings`]). Doors and windows bind to their host wall by
/// the 2024 rule (RE-85) on 2023's reference lists, and every element to
/// the one Level its list names (RE-107). No names, types, joins or design
/// options: their 2023 layouts are not decoded.
pub(super) fn recover_2023_records(
    rf: &mut RevitFile,
    revit_version: u32,
    out: &mut PartitionSchemaMvp,
) {
    use crate::partition_element_records as per;
    let records = crate::partition_element_records_2023::scan_records(rf, revit_version);
    let names = type_and_family_names_2023(rf, &records);
    let system_types = system_type_names_2023(rf, revit_version, &records);
    let selected = select_newest_instance_records(records);
    // Only a family instance can be nested; a host wall and the doors it
    // hosts name each other too (RE-76's class tag, at the same place on
    // 2023).
    let family_instance = rf.memoised_schema_classes().ok().and_then(|classes| {
        classes
            .classes
            .iter()
            .find(|class| class.name == "FamilyInstance")
            .map(|class| class.tag)
    });
    let nested = match family_instance {
        Some(tag) => nested_in_openings(&selected, tag),
        None => BTreeSet::new(),
    };
    // Each record names its Level in its reference list, as on 2024
    // (RE-107).
    let level_ids = level_element_ids(rf, revit_version).unwrap_or_default();
    for record in selected.values() {
        if nested.contains(&record.element_id) {
            continue;
        }
        let Some((_, class)) = per::RECOVERED_CATEGORIES
            .iter()
            .find(|(category, _)| *category == record.builtin_category)
        else {
            continue;
        };
        let mut decoded = element_record_decoded(record, class, &level_ids);
        if let Some((type_id, type_name, family_name)) = names.get(&record.element_id) {
            decoded.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: *type_id,
                },
            ));
            decoded.fields.push((
                TYPE_NAME_FIELD.into(),
                InstanceField::String(type_name.clone()),
            ));
            decoded.fields.push((
                FAMILY_NAME_FIELD.into(),
                InstanceField::String(family_name.clone()),
            ));
        } else if let Some((type_id, type_name)) = system_types.get(&record.element_id) {
            decoded.fields.push((
                TYPE_ID_FIELD.into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: *type_id,
                },
            ));
            decoded.fields.push((
                TYPE_NAME_FIELD.into(),
                InstanceField::String(type_name.clone()),
            ));
        }
        match record.builtin_category {
            per::OST_WALLS => out.walls.push(decoded),
            per::OST_DOORS => out.doors.push(decoded),
            per::OST_WINDOWS => out.windows.push(decoded),
            per::OST_FLOORS | per::OST_BUILDING_PAD => out.slabs.push(decoded),
            per::OST_ROOMS => out.rooms.push(decoded),
            per::OST_COLUMNS => out.columns.push(decoded),
            _ => out.products.push(decoded),
        }
    }
    // --- Walls cut back by the walls they join, by 2024's solver (RE-26,
    // RE-29): a 2023 record box is the untrimmed wall too, and its
    // reference list names the walls it joins (RE-120) ---
    let wall_records: Vec<per::PartitionElementRecord> = out
        .walls
        .iter()
        .filter_map(|wall| wall.id.and_then(|id| selected.get(&id)).cloned())
        .collect();
    let trims = crate::element_record_wall_joins::join_trims(&wall_records);
    for wall in out.walls.iter_mut() {
        if let Some((record, trim)) = wall
            .id
            .and_then(|id| Some((selected.get(&id)?, trims.get(&id)?)))
        {
            apply_wall_join_trim(wall, record, trim);
        }
    }
    // --- Beams stopped at the faces of the columns their records name
    // (RE-122) ---
    let framing: Vec<per::PartitionElementRecord> = out
        .products
        .iter()
        .filter(|element| element.class == "StructuralFraming")
        .filter_map(|element| element.id.and_then(|id| selected.get(&id)).cloned())
        .collect();
    let columns: Vec<per::PartitionElementRecord> = selected
        .values()
        .filter(|record| {
            matches!(
                record.builtin_category,
                per::OST_COLUMNS | per::OST_STRUCTURAL_COLUMNS
            ) && !nested.contains(&record.element_id)
        })
        .cloned()
        .collect();
    let beam_trims = crate::element_record_beam_cuts::beam_column_trims(&framing, &columns);
    for beam in out.products.iter_mut() {
        if let Some(trim) = beam.id.and_then(|id| beam_trims.get(&id)) {
            apply_beam_column_trim(beam, trim);
        }
    }
    // --- Rooms' numbers and names (RE-117) ---
    let room_ids: BTreeSet<u32> = out.rooms.iter().filter_map(|room| room.id).collect();
    attach_room_parameter_entries(rf, revit_version, &room_ids, &mut out.rooms);
    let wall_ids: BTreeSet<u32> = out.walls.iter().filter_map(|wall| wall.id).collect();
    for element in out.doors.iter_mut().chain(out.windows.iter_mut()) {
        if let Some(record) = element.id.and_then(|id| selected.get(&id)) {
            attach_host_candidates(record, &wall_ids, element);
        }
    }
    bind_opening_hosts(&out.walls, &mut out.doors);
    bind_opening_hosts(&out.walls, &mut out.windows);
}

/// Each exported 2023 wall's, floor's, roof's and ceiling's type and its
/// name, by ElementId (RE-111).
///
/// A 2023 system-family type has no record and no name entry (RE-109). Its
/// name is in its element data, read as on 2024
/// ([`crate::partition_names::find_element_data_names_2023`]). Of the ids
/// an element's record names that have such a name and neither a record
/// nor a name entry, the type is the one whose naming records are of a
/// strict subset of the categories that name each of the others: a wall
/// type is named by walls and the doors and windows they host, a floor
/// type by floors, where the document's other named elements are named by
/// records of more categories. No such one, no type.
pub(super) fn system_type_names_2023(
    rf: &mut RevitFile,
    revit_version: u32,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> BTreeMap<u32, (u32, String)> {
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared = crate::elem_table::declared_ids(&table);
    let entries = crate::partition_names::name_entries_2023(rf, &declared);
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let mut named_by: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    for record in records {
        for &reference in &record.references {
            if let Ok(id) = u32::try_from(reference) {
                if !recorded.contains(&id) && !entries.contains_key(&id) {
                    named_by
                        .entry(id)
                        .or_default()
                        .insert(record.builtin_category);
                }
            }
        }
    }
    let wanted: BTreeSet<u32> = named_by.keys().copied().collect();
    let mut names: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in crate::partition_names::find_element_data_names_32(
            inflated.bytes(),
            revit_version,
            &wanted,
        ) {
            match names.get(&id) {
                None => {
                    names.insert(id, Some(name));
                }
                Some(held) if held.as_deref() != Some(name.as_str()) => {
                    names.insert(id, None);
                }
                _ => {}
            }
        }
    }
    let mut out = BTreeMap::new();
    for record in records.iter().filter(|record| {
        record.is_exported_instance()
            && SYSTEM_TYPE_CATEGORIES_2023.contains(&record.builtin_category)
    }) {
        let candidates: BTreeSet<u32> = record
            .references
            .iter()
            .filter_map(|&reference| u32::try_from(reference).ok())
            .filter(|id| matches!(names.get(id), Some(Some(_))))
            .collect();
        let most_specific = candidates.iter().copied().filter(|id| {
            candidates.iter().all(|other| {
                other == id
                    || (named_by[id].is_subset(&named_by[other]) && named_by[id] != named_by[other])
            })
        });
        let mut picked = most_specific;
        let (Some(type_id), None) = (picked.next(), picked.next()) else {
            continue;
        };
        if let Some(Some(name)) = names.get(&type_id) {
            out.insert(record.element_id, (type_id, name.clone()));
        }
    }
    out
}

pub(super) fn type_and_family_names_2023(
    rf: &mut RevitFile,
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> BTreeMap<u32, (u32, String, String)> {
    use crate::partition_element_records::PLACEMENT_KIND_SYMBOL;
    let Ok(table) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let names =
        crate::partition_names::name_entries_2023(rf, &crate::elem_table::declared_ids(&table));
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let mut symbols: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut symbol_records: BTreeMap<
        u32,
        &crate::partition_element_records::PartitionElementRecord,
    > = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.placement_kind == PLACEMENT_KIND_SYMBOL)
    {
        symbols
            .entry(record.builtin_category)
            .or_default()
            .insert(record.element_id);
        symbol_records.insert(record.element_id, record);
    }
    let family_of = |type_id: u32| {
        let record = symbol_records.get(&type_id)?;
        let mut families = record.references.iter().filter_map(|&reference| {
            let id = u32::try_from(reference).ok()?;
            (id != type_id && !recorded.contains(&id))
                .then(|| names.get(&id))
                .flatten()
        });
        let family = families.next()?;
        families.next().is_none().then(|| family.clone())
    };
    let mut out = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.is_exported_instance())
    {
        // The type is anywhere in the list: a type made after the instance
        // has the larger id, and another type can sit between them.
        let Some(set) = symbols.get(&record.builtin_category) else {
            continue;
        };
        let mut types = record.references.iter().filter_map(|&reference| {
            let id = u32::try_from(reference).ok()?;
            (set.contains(&id) && names.contains_key(&id)).then_some(id)
        });
        let (Some(type_id), None) = (types.next(), types.next()) else {
            continue;
        };
        let (Some(type_name), Some(family_name)) = (names.get(&type_id), family_of(type_id)) else {
            continue;
        };
        out.insert(record.element_id, (type_id, type_name.clone(), family_name));
    }
    out
}

/// [`select_instance_records`] for Revit 2023 (RE-107): of an element's
/// frames, the newest, in the highest-numbered partition and then the
/// latest in it. A 2023 project's frames can disagree about their height,
/// where a Level moved after an earlier save: on modelo_bim four columns
/// run from -13.12 ft in `Partitions/1` and from -5.58 ft in
/// `Partitions/2`, and Revit's export draws them from -1.7 m (-5.58 ft),
/// on the Level the newer frame names. The greatest extent would keep the
/// stale one.
pub(super) fn select_newest_instance_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
) -> BTreeMap<u32, crate::partition_element_records::PartitionElementRecord> {
    use crate::partition_element_records::PartitionElementRecord;
    let partition = |record: &PartitionElementRecord| {
        record
            .stream
            .rsplit('/')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(0)
    };
    let mut by_id: BTreeMap<u32, PartitionElementRecord> = BTreeMap::new();
    for record in records {
        if !record.is_exported_instance() || !record.has_volume() {
            continue;
        }
        let newer = by_id.get(&record.element_id).is_none_or(|held| {
            (partition(&record), record.offset) > (partition(held), held.offset)
        });
        if newer {
            by_id.insert(record.element_id, record);
        }
    }
    without_nested_components(by_id)
}
