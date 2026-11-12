//! The Revit 2023 record path.

use super::*;

/// Revit 2023 elements from their partition element records (RE-81): each
/// placed instance of a [`crate::partition_element_records::RECOVERED_CATEGORIES`]
/// category, with its box as its body, into the bucket its category
/// feeds. Components nested in doors and windows are left out
/// ([`nested_in_openings`]). Doors and windows bind to their host wall by
/// the 2024 rule (RE-85) on 2023's reference lists, and every element to
/// the one Level its list names (RE-107). On 2016 to 2023 an element in a
/// non-primary design option, or not in the export phase, is left out
/// (RE-315).
pub(super) fn recover_2023_records(
    rf: &mut RevitFile,
    revit_version: u32,
    out: &mut PartitionSchemaMvp,
) -> BTreeMap<u32, Vec<u64>> {
    use crate::partition_element_records as per;
    // The frames that stop are listed only for the log of decisions.
    let want_frames = rf.recording_frame_decisions();
    let (records, mut stages) =
        crate::partition_element_records_2023::scan_records_counted(rf, revit_version, want_frames);
    let names = type_and_family_names_2023(rf, &records);
    let system_types = system_type_names_2023(rf, revit_version, &records);
    let mut selected = select_newest_instance_records(records, &mut stages);
    // Revit's export leaves out an element in a non-primary design option and
    // one that does not exist in the export phase, as on 2024 (RE-40, RE-184).
    // A room is left out by its own phase (RE-326). Applied on the releases
    // measured only (RE-315, RE-372); on 2015 no design option set is read,
    // so `options` excludes nothing there.
    if crate::partition_element_records_2023::applies_phase_filter(revit_version) {
        let options = rf.memoised_design_options();
        let phases = rf.memoised_phase_filter();
        selected.retain(|_, record| {
            if options.excludes(record) {
                stages
                    .counts_mut(&record.stream)
                    .in_non_primary_design_option += 1;
                false
            } else if phases.excludes(record) {
                stages.counts_mut(&record.stream).not_in_export_phase += 1;
                false
            } else {
                true
            }
        });
    }
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
            stages.counts_mut(&record.stream).nested_in_door_or_window += 1;
            continue;
        }
        let Some((_, class)) = per::RECOVERED_CATEGORIES
            .iter()
            .find(|(category, _)| *category == record.builtin_category)
        else {
            stages.counts_mut(&record.stream).category_not_exported += 1;
            continue;
        };
        stages.counts_mut(&record.stream).handed_to_export += 1;
        stages.handed.insert(record.element_id, *class);
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
    // What became of each frame, for the diagnostics (#1389).
    rf.note_element_record_stages(stages);
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
    // --- Curtain walls and their panels and mullions (RE-440) ---
    attach_curtain_walls_2023(rf, &selected, &mut out.walls, &mut out.products);
    // Each record's reference list, as read, for the readers it names (RE-366).
    selected
        .into_iter()
        .map(|(id, record)| (id, record.references))
        .collect()
}

/// Make each 2023 wall whose native record's type (`m_WallAttributesId`) is a
/// `NewCurtainWallType` a curtain wall with that type's name, and give each
/// panel and mullion whose reference list names exactly one of them that wall
/// as its aggregate whole, as on 2024 (RE-46). On 2023 no mullion or grid
/// element names its wall the way 2024's do, but the wall's own type says
/// what it is (RE-440): `Revit_IFC5_Einhoven`'s wall 7187 has a curtain grid
/// of 24 cells whose panels name it, and its type 1937 is a
/// `NewCurtainWallType`.
fn attach_curtain_walls_2023(
    rf: &mut RevitFile,
    selected: &BTreeMap<u32, crate::partition_element_records::PartitionElementRecord>,
    walls: &mut [DecodedElement],
    products: &mut [DecodedElement],
) {
    let wall_ids: BTreeSet<u64> = walls
        .iter()
        .filter_map(|wall| wall.id)
        .map(u64::from)
        .collect();
    let wall_types = crate::native_walls::wall_type_ids(rf, &wall_ids).unwrap_or_default();
    let type_ids: BTreeSet<u64> = wall_types.values().copied().collect();
    let curtain_types =
        crate::native_walls::curtain_wall_type_names(rf, &type_ids).unwrap_or_default();
    let mut curtain_walls = BTreeSet::new();
    for wall in walls.iter_mut() {
        let Some(id) = wall.id else {
            continue;
        };
        let Some((&type_id, name)) = wall_types
            .get(&u64::from(id))
            .and_then(|type_id| Some((type_id, curtain_types.get(type_id)?)))
        else {
            continue;
        };
        let Ok(type_id) = u32::try_from(type_id) else {
            continue;
        };
        curtain_walls.insert(id);
        wall.class = CURTAIN_WALL_CLASS.into();
        wall.fields.retain(|(field, _)| {
            field != TYPE_ID_FIELD && field != TYPE_NAME_FIELD && field != FAMILY_NAME_FIELD
        });
        wall.fields.push((
            TYPE_ID_FIELD.into(),
            InstanceField::ElementId {
                tag: 0,
                id: type_id,
            },
        ));
        wall.fields
            .push((TYPE_NAME_FIELD.into(), InstanceField::String(name.clone())));
    }
    if curtain_walls.is_empty() {
        return;
    }
    for part in products
        .iter_mut()
        .filter(|part| CURTAIN_WALL_PART_CLASSES.contains(&part.class.as_str()))
    {
        let Some(record) = part.id.and_then(|id| selected.get(&id)) else {
            continue;
        };
        let named: BTreeSet<u32> = record
            .references
            .iter()
            .filter_map(|&id| u32::try_from(id).ok())
            .filter(|id| curtain_walls.contains(id))
            .collect();
        let mut named = named.into_iter();
        if let (Some(whole), None) = (named.next(), named.next()) {
            part.fields.push((
                AGGREGATE_WHOLE_FIELD.into(),
                InstanceField::ElementId { tag: 0, id: whole },
            ));
        }
    }
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
///
/// Counts what became of each frame in `stages` (#1389): a record that is not a
/// placed instance, a placed instance with or without volume, a frame another
/// replaces, a nested component and the frame kept.
pub(super) fn select_newest_instance_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    stages: &mut crate::partition_element_records_2023::RecordStages,
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
        let counts = stages.counts_mut(&record.stream);
        if !record.is_exported_instance() {
            counts.not_placed_instances += 1;
            continue;
        }
        counts.placed_instances += 1;
        if !record.has_volume() {
            counts.without_volume += 1;
            stages.note_volumeless(&record);
            continue;
        }
        counts.with_volume += 1;
        let newer = by_id.get(&record.element_id).is_none_or(|held| {
            (partition(&record), record.offset) > (partition(held), held.offset)
        });
        if newer {
            if let Some(replaced) = by_id.insert(record.element_id, record) {
                stages.counts_mut(&replaced.stream).superseded += 1;
            }
        } else {
            stages.counts_mut(&record.stream).superseded += 1;
        }
    }
    for id in nested_component_ids(&by_id) {
        if let Some(record) = by_id.remove(&id) {
            stages.counts_mut(&record.stream).nested_components += 1;
        }
    }
    for record in by_id.values() {
        stages.counts_mut(&record.stream).selected += 1;
    }
    by_id
}
