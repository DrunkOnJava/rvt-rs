//! Room instances, parameters, outlines and heights.

use super::*;

/// Recover room instances from partition element records (#90, RE-29).
///
/// `OST_Rooms` under the #211 instance rule reproduces the exported
/// room ElementId set exactly; the number, name and host Level come
/// from the room's own parameter block
/// ([`crate::partition_room_parameters`]), joined by ElementId only.
///
/// The parameter join is attached, never required: a room whose block
/// does not decode, or whose blocks disagree, still emits with its
/// record body and keeps the identity the record gives it.
pub fn rooms_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let Some(records) = category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_ROOMS,
    )?
    else {
        return Ok(Vec::new());
    };
    let room_ids: BTreeSet<u32> = records
        .iter()
        .filter(|record| record.is_exported_instance())
        .map(|record| record.element_id)
        .collect();
    let parameters =
        crate::partition_room_parameters::scan_room_parameters(rf, revit_version, &room_ids)
            .unwrap_or_default();
    let mut rooms = room_instances_from_records(records, &parameters, level_ids);
    // RE-117: a room's number and name are the ones its parameter entries
    // give it; RE-29's block stands in where they are not read, and still
    // gives the Level. On Snowdon Towers the block names 1 of 54 rooms, and
    // that one wrongly.
    attach_room_parameter_entries(rf, revit_version, &room_ids, &mut rooms);
    // RE-153: a room's Floor Finish.
    let finishes = crate::partition_room_parameters::scan_room_text_parameter(
        rf,
        revit_version,
        &room_ids,
        crate::partition_room_parameters::ROOM_FINISH_FLOOR_PARAMETER,
    );
    for room in rooms.iter_mut() {
        if let Some(finish) = room.id.and_then(|id| finishes.get(&id)) {
            room.fields.push((
                ROOM_FLOOR_FINISH_FIELD.into(),
                InstanceField::String(finish.clone()),
            ));
        }
    }
    Ok(rooms)
}

/// Give each room in `rooms` the number and name its parameter entries
/// hold (RE-117), in place of any read before that differs; a room whose
/// block agrees keeps the block as its source.
pub(super) fn attach_room_parameter_entries(
    rf: &mut RevitFile,
    revit_version: u32,
    room_ids: &BTreeSet<u32>,
    rooms: &mut [DecodedElement],
) {
    let entries =
        crate::partition_room_parameters::scan_room_parameter_entries(rf, revit_version, room_ids);
    for room in rooms.iter_mut() {
        let Some((number, name)) = room.id.and_then(|id| entries.get(&id)) else {
            continue;
        };
        let text = |wanted: &str| {
            room.fields.iter().find_map(|(field, value)| match value {
                InstanceField::String(text) if field == wanted => Some(text.clone()),
                _ => None,
            })
        };
        // Where RE-29's block already agrees, it keeps its provenance.
        if text("m_name").as_ref() == Some(name) && text(ROOM_NUMBER_FIELD).as_ref() == Some(number)
        {
            continue;
        }
        room.fields.retain(|(field, _)| {
            field != "m_name" && field != ROOM_NUMBER_FIELD && field != ROOM_PARAMETER_SOURCE_FIELD
        });
        room.fields
            .push(("m_name".into(), InstanceField::String(name.clone())));
        room.fields.push((
            ROOM_NUMBER_FIELD.into(),
            InstanceField::String(number.clone()),
        ));
        room.fields.push((
            ROOM_PARAMETER_SOURCE_FIELD.into(),
            InstanceField::String(
                crate::partition_room_parameters::ROOM_PARAMETER_ENTRY_SOURCE.into(),
            ),
        ));
    }
}

/// Give each room the outline of its stored solid (RE-101,
/// [`crate::partition_room_boundaries`]), read from the partition of the
/// room's own record and kept only where it spans the record's box. A room
/// whose solid does not close keeps its box.
pub(super) fn attach_room_outlines(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &mut [DecodedElement],
) {
    use crate::partition_room_boundaries as prb;
    if !prb::ROOM_SOLID_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    // Each stream's room solids are found in one pass (B85): a search per
    // room took 1.2 s on Autodesk's 2025 rme_advanced sample.
    let mut solids = std::collections::HashMap::new();
    for room in rooms.iter_mut() {
        let (Some(id), Some(stream)) = (room.id, element_source_stream(room)) else {
            continue;
        };
        let float = |wanted: &str| {
            room.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let (Some(cx), Some(cy), Some(z), Some(dx), Some(dy), Some(dz)) = (
            float("m_locationX"),
            float("m_locationY"),
            float("m_locationZ"),
            float("m_bboxWidth"),
            float("m_bboxDepth"),
            float("m_bboxHeight"),
        ) else {
            continue;
        };
        let bbox = [
            cx - dx / 2.0,
            cy - dy / 2.0,
            z,
            cx + dx / 2.0,
            cy + dy / 2.0,
            z + dz,
        ];
        let (inflated, headers) = match solids.entry(stream) {
            std::collections::hash_map::Entry::Occupied(held) => held.into_mut(),
            std::collections::hash_map::Entry::Vacant(slot) => {
                let Ok(inflated) = rf.inflated_partition(slot.key()) else {
                    continue;
                };
                let headers = prb::solid_headers_by_id(inflated.bytes());
                slot.insert((inflated, headers))
            }
        };
        let Some(headers) = headers.get(&u64::from(id)) else {
            continue;
        };
        if let Some(profile) = prb::outline_of_solids(inflated.bytes(), headers, bbox) {
            let mut fields = profile.fields();
            for (name, value) in fields.iter_mut() {
                if name == crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD {
                    *value = InstanceField::String(prb::ROOM_OUTLINE_SOURCE.into());
                }
            }
            room.fields.extend(fields);
            // RE-296: a room whose solid is a prism of that outline.
            if ROOM_PRISM_REVIT_VERSIONS.contains(&revit_version)
                && prb::prism_of_solids(inflated.bytes(), headers, bbox)
            {
                room.fields
                    .push((ROOM_PRISM_FIELD.into(), InstanceField::Bool(true)));
            }
        }
    }
}

/// Give each room whose native record holds its stored solid's faces (two
/// `Face` objects or more) its `RoomElem.m_height` as its height: Revit's
/// export writes it as the space's `Height` on every such room of the
/// BIM4LCA office, the R2025 models and the held-out Revit 2026 house
/// (RE-255). A room with no faces in its
/// record gets none: on the office Revit's `Height` is another value on all
/// 20 of those.
pub(super) fn attach_room_heights(
    rf: &mut RevitFile,
    revit_version: u32,
    rooms: &mut [DecodedElement],
) {
    if !ROOM_HEIGHT_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let ids: BTreeSet<u64> = rooms
        .iter()
        .filter_map(|room| room.id.map(u64::from))
        .collect();
    if ids.is_empty() {
        return;
    }
    let options = crate::native_document::Options {
        selected_ids: ids,
        ..crate::native_document::Options::default()
    };
    let mut heights: BTreeMap<u64, f64> = BTreeMap::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let faces = graph
            .objects
            .iter()
            .filter(|o| o.class_name == "Face")
            .count();
        let height = graph
            .objects
            .iter()
            .find(|o| o.class_name == "RoomElem")
            .and_then(|o| o.fields.get("m_height"))
            .and_then(serde_json::Value::as_f64)
            .filter(|h| h.is_finite() && *h > 0.0);
        if let (true, Some(height)) = (faces >= 2, height) {
            heights.insert(record.identity.element_id, height);
        }
        Ok(())
    });
    if read.is_err() {
        return;
    }
    for room in rooms.iter_mut() {
        if let Some(height) = room.id.and_then(|id| heights.get(&u64::from(id))).copied() {
            room.fields.push((
                ROOM_HEIGHT_FIELD.into(),
                InstanceField::Float {
                    value: height,
                    size: 8,
                },
            ));
        }
    }
}

/// Room instance selection with the number / name / Level join
/// attached (#90, RE-29).
///
/// The Level the parameter block names is used only when the record's
/// own counted reference list did **not** already name one, so the
/// RE-27 carrier stays the primary. On `2024_Core_Interior.rvt` both
/// carriers answer for all 116 rooms and they agree 116 of 116.
pub fn room_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    parameters: &BTreeMap<u32, crate::partition_room_parameters::RoomParameters>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    select_instance_records(records)
        .values()
        .map(|record| {
            let mut decoded = element_record_decoded(record, "Room", level_ids);
            if let Some(block) = parameters.get(&record.element_id) {
                decoded
                    .fields
                    .push(("m_name".into(), InstanceField::String(block.name.clone())));
                decoded.fields.push((
                    ROOM_NUMBER_FIELD.into(),
                    InstanceField::String(block.number.clone()),
                ));
                decoded.fields.push((
                    ROOM_PARAMETER_SOURCE_FIELD.into(),
                    InstanceField::String(
                        crate::partition_room_parameters::ROOM_PARAMETER_SOURCE.into(),
                    ),
                ));
                let already_bound = decoded.fields.iter().any(|(name, _)| {
                    name == crate::element_record_level_refs::LEVEL_REFERENCE_FIELD
                });
                if !already_bound {
                    if let Some(id) = block.level_element_id(level_ids) {
                        decoded.fields.push((
                            crate::element_record_level_refs::LEVEL_REFERENCE_FIELD.into(),
                            InstanceField::ElementId { tag: 0, id },
                        ));
                        if let Some(slot) = decoded
                            .fields
                            .iter_mut()
                            .find(|(name, _)| name == "m_level_bound")
                        {
                            slot.1 = InstanceField::Bool(true);
                        }
                    }
                }
            }
            decoded
        })
        .collect()
}
