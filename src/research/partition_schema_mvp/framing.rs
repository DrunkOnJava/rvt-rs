//! Beams and columns: sections, axes, cuts, trims and instances.

use super::*;

/// The I section [`BEAM_I_SECTION_FIELDS`] record.
pub fn beam_i_section_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_type_parameters::ISection> {
    let [width, depth, web, flange] = BEAM_I_SECTION_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some(crate::partition_type_parameters::ISection {
        width_feet: width?,
        depth_feet: depth?,
        web_feet: web?,
        flange_feet: flange?,
    })
}

/// The hollow section [`HOLLOW_SECTION_FIELDS`] record.
pub fn hollow_section_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<crate::partition_type_parameters::HollowSection> {
    let [width, height, wall, radius] = HOLLOW_SECTION_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some(crate::partition_type_parameters::HollowSection {
        width_feet: width?,
        height_feet: height?,
        wall_feet: wall?,
        outer_radius_feet: radius?,
    })
}

/// Whether `class` is a column's, architectural or structural.
pub(super) fn is_column_class(class: &str) -> bool {
    matches!(class, "Column" | "StructuralColumn")
}

/// The ElementIds [`COLUMN_CUT_PARTNERS_FIELD`] and
/// [`COLUMN_CLEAN_PARTNERS_FIELD`] record: what a column's cut pairs and
/// clean pairs name (RE-291). `None` where either is absent.
pub fn column_partners_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<(Vec<u32>, Vec<u32>)> {
    let ids = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == wanted => items
                .iter()
                .map(|item| match item {
                    InstanceField::ElementId { id, .. } => Some(*id),
                    _ => None,
                })
                .collect::<Option<Vec<u32>>>(),
            _ => None,
        })
    };
    Some((
        ids(COLUMN_CUT_PARTNERS_FIELD)?,
        ids(COLUMN_CLEAN_PARTNERS_FIELD)?,
    ))
}

/// The sides [`COLUMN_RECTANGLE_FIELDS`] record.
pub fn column_rectangle_from_fields(fields: &[(String, InstanceField)]) -> Option<[f64; 2]> {
    let [b, h] = COLUMN_RECTANGLE_FIELDS.map(|wanted| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    });
    Some([b?, h?])
}

/// The diameter [`COLUMN_DIAMETER_FIELD`] records.
pub fn column_diameter_from_fields(fields: &[(String, InstanceField)]) -> Option<f64> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Float { value, .. } if name == COLUMN_DIAMETER_FIELD => Some(*value),
        _ => None,
    })
}

/// Give each column whose type is a hollow structural section
/// ([`crate::partition_type_parameters::hollow_section`]) and holds no I
/// section that section (W6), each whose type is a plain rectangle
/// ([`crate::partition_type_parameters::rectangular_section`]) its two sides
/// (RE-291) and each whose type is round
/// ([`crate::partition_type_parameters::round_section`]) its diameter
/// (RE-353). The exporter draws a hollow section only where its record box is
/// the section's outside.
pub(super) fn attach_column_sections(
    rf: &mut RevitFile,
    revit_version: u32,
    groups: [&mut Vec<DecodedElement>; 2],
) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let mut elements: Vec<&mut DecodedElement> = groups
        .into_iter()
        .flat_map(|group| group.iter_mut())
        .filter(|element| HOLLOW_SECTION_CLASSES.contains(&element.class.as_str()))
        .filter(|element| beam_i_section_from_fields(&element.fields).is_none())
        .collect();
    let types: BTreeSet<u32> = elements
        .iter()
        .filter_map(|element| type_of(element))
        .collect();
    if types.is_empty() {
        return;
    }
    let recording = rf.recording_value_sources();
    let (held, places) = crate::partition_type_parameters::type_named_lengths_at(
        rf,
        revit_version,
        &types,
        recording,
    );
    for element in elements.iter_mut() {
        let Some(by_name) = type_of(element).and_then(|id| held.get(&id)) else {
            continue;
        };
        if let Some(section) = crate::partition_type_parameters::hollow_section(by_name) {
            let values = [
                section.width_feet,
                section.height_feet,
                section.wall_feet,
                section.outer_radius_feet,
            ];
            for (name, value) in HOLLOW_SECTION_FIELDS.iter().zip(values) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
            if let (true, Some(id), Some(type_id)) = (recording, element.id, type_of(element)) {
                crate::partition_type_parameters::note_parameter_source(
                    rf,
                    "type_hollow_section",
                    (id, type_id),
                    crate::partition_type_parameters::HOLLOW_SECTION_PARAMETER_NAMES
                        .map(String::from),
                    &places,
                );
            }
        } else if let Some(sides) = crate::partition_type_parameters::rectangular_section(by_name) {
            for (name, value) in COLUMN_RECTANGLE_FIELDS.iter().zip(sides) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        } else if let Some(value) = crate::partition_type_parameters::round_section(by_name) {
            element.fields.push((
                COLUMN_DIAMETER_FIELD.into(),
                InstanceField::Float { value, size: 8 },
            ));
        }
    }
}

/// Give each structural-framing element and structural column whose type is
/// an I section ([`crate::partition_type_parameters::type_i_sections`]) that
/// section (RE-103, RE-104), and each such upright column its X axis in plan
/// from its transform. The exporter draws a beam's section only where its
/// body along its line holds it, and a column's only where the section
/// turned to that axis fills its record box.
pub(super) fn attach_beam_sections(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    // RE-365: on the 32-bit releases a beam's section only; a column's placement
    // is not measured there.
    let classes: &[&str] = if crate::partition_element_records_2023::is_32bit_release(revit_version)
    {
        &["StructuralFraming"]
    } else {
        &I_SECTION_CLASSES
    };
    let types: BTreeSet<u32> = products
        .iter()
        .filter(|element| classes.contains(&element.class.as_str()))
        .filter_map(type_of)
        .collect();
    let recording = rf.recording_value_sources();
    let (sections, places) =
        crate::partition_type_parameters::type_i_sections_at(rf, revit_version, &types, recording);
    let columns: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "StructuralColumn")
        .filter(|element| type_of(element).is_some_and(|id| sections.contains_key(&id)))
        .filter_map(|element| element.id)
        .collect();
    let transforms = if columns.is_empty() {
        BTreeMap::new()
    } else {
        crate::partition_instance_transforms::scan_instance_transforms(rf, revit_version, &columns)
            .unwrap_or_default()
    };
    for element in products
        .iter_mut()
        .filter(|element| classes.contains(&element.class.as_str()))
    {
        let Some(section) = type_of(element).and_then(|id| sections.get(&id)).copied() else {
            continue;
        };
        let values = [
            section.width_feet,
            section.depth_feet,
            section.web_feet,
            section.flange_feet,
        ];
        for (name, value) in BEAM_I_SECTION_FIELDS.iter().zip(values) {
            element
                .fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        if let (true, Some(id), Some(type_id)) = (recording, element.id, type_of(element)) {
            crate::partition_type_parameters::note_parameter_source(
                rf,
                "type_i_section",
                (id, type_id),
                crate::partition_type_parameters::I_SECTION_PARAMETERS,
                &places,
            );
        }
        let axis = element
            .id
            .and_then(|id| transforms.get(&id))
            .filter(|transform| transform.is_upright())
            .and_then(|transform| transform.plan_axis());
        if let Some(axis) = axis {
            for (name, value) in COLUMN_I_AXIS_FIELDS.iter().zip(axis) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
    }
}

/// The two ends of a beam's location line, when the partition MVP gave it one
/// (RE-49).
pub fn beam_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<([f64; 3], [f64; 3])> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some((point(BEAM_AXIS_START_FIELDS)?, point(BEAM_AXIS_END_FIELDS)?))
}

/// The two ends of a beam's location line for its properties
/// ([`BEAM_LINE_START_FIELDS`], #1697), when the partition MVP gave it one.
pub fn beam_line_from_fields(fields: &[(String, InstanceField)]) -> Option<([f64; 3], [f64; 3])> {
    let field = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let point = |names: [&str; 3]| Some([field(names[0])?, field(names[1])?, field(names[2])?]);
    Some((point(BEAM_LINE_START_FIELDS)?, point(BEAM_LINE_END_FIELDS)?))
}

/// Whether a beam's Cross-Section Rotation ([`BEAM_ROTATION_FIELD`], RE-276)
/// was read and is exactly 0. A beam whose rotation is unread is not
/// unrotated.
pub fn beam_unrotated(fields: &[(String, InstanceField)]) -> bool {
    fields.iter().any(|(name, value)| {
        name == BEAM_ROTATION_FIELD
            && matches!(value, InstanceField::Float { value, .. } if *value == 0.0)
    })
}

/// Whether a column cut a beam's body (RE-122): its record box fields are then
/// the cut box's, which `apply_beam_column_trim` wrote over the record's, so the
/// beam keeps that body and neither a location line nor a rotation is read for it.
fn beam_column_cut(element: &DecodedElement) -> bool {
    element
        .fields
        .iter()
        .any(|(name, _)| name == crate::element_record_beam_cuts::BEAM_BODY_SOURCE_FIELD)
}

/// Give each structural-framing element the location line its data carries,
/// when the line and the element's record box make a beam solid
/// ([`crate::partition_beam_axes::beam_body`], RE-49), or, for an unrotated
/// beam ([`beam_unrotated`], so after [`attach_beam_rotations`]), a level
/// beam whose box is longer than its line
/// ([`crate::partition_beam_axes::beam_body_in_box`], RE-328). A line that
/// leaves the box, or a box no solid along the line reproduces, is not
/// attached. A beam a column cuts (RE-122) is skipped: its record box fields are
/// the cut box's, a line is tested against them, and its body is the cut box's.
pub(super) fn attach_beam_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    use crate::partition_beam_axes as pba;
    if !pba::beam_axis_read_on(revit_version) {
        return;
    }
    let beams: BTreeSet<u32> = products
        .iter()
        .filter(|element| element.class == "StructuralFraming" && !beam_column_cut(element))
        .filter_map(|element| element.id)
        .collect();
    if beams.is_empty() {
        return;
    }
    let Ok(lines) = pba::scan_first_bounded_lines(rf, revit_version, &beams) else {
        return;
    };
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StructuralFraming" && !beam_column_cut(element))
    {
        let Some(line) = element.id.and_then(|id| lines.get(&id)) else {
            continue;
        };
        let (start, end) = (line.start(), line.end());
        let unrotated = beam_unrotated(&element.fields);
        let bbox = element_record_bbox(element);
        // #1697: the line gives the beam's `Length`, `Span`, `Slope` and `Roll`
        // in Revit's export whether or not a solid along it is drawn (291 of 291
        // beams with a line in the BIM4LCA office and housing), so a line that
        // lies inside the record box is kept for them apart from the body's.
        let inside = bbox.is_some_and(|bbox| {
            [start, end].iter().all(|p| {
                (0..3).all(|axis| {
                    p[axis] >= bbox[axis] - pba::END_TOLERANCE_FEET
                        && p[axis] <= bbox[axis + 3] + pba::END_TOLERANCE_FEET
                })
            })
        });
        if inside {
            for (names, point) in [(BEAM_LINE_START_FIELDS, start), (BEAM_LINE_END_FIELDS, end)] {
                for (name, value) in names.iter().zip(point) {
                    element
                        .fields
                        .push(((*name).into(), InstanceField::Float { value, size: 8 }));
                }
            }
        }
        let resolves = bbox.is_some_and(|bbox| {
            pba::beam_body(bbox, start, end).is_some()
                || (unrotated && pba::beam_body_in_box(bbox, start, end).is_some())
        });
        if !resolves {
            continue;
        }
        for (names, point) in [(BEAM_AXIS_START_FIELDS, start), (BEAM_AXIS_END_FIELDS, end)] {
            for (name, value) in names.iter().zip(point) {
                element
                    .fields
                    .push(((*name).into(), InstanceField::Float { value, size: 8 }));
            }
        }
    }
}

/// Give each structural-framing element the Cross-Section Rotation its
/// native record holds, in [`BEAM_ROTATION_FIELD`] (W7, RE-276): the
/// `m_rotation`, radians, of its `FamInstCurveDriver`, which revit-ifc's
/// `RollCalculator` reads from the element's `STRUCTURAL_BEND_DIR_ANGLE`
/// parameter (CLEANROOM.md source 5). A beam whose copies disagree, or whose
/// record is not read, gets none, nor does one a column cuts (RE-122), which
/// is not drawn along a line.
///
/// The same pass gives every framing element its structural usage in
/// [`STRUCT_USAGE_FIELD`] (#1841), the `m_structUsage` of its record's root: a
/// usage whose copies disagree, or whose record is not read, is not given. A
/// horizontal bracing on [`BRACE_TYPE_REVIT_VERSIONS`] also carries
/// [`BRACE_TYPE_FIELD`].
pub(super) fn attach_beam_rotations(
    rf: &mut RevitFile,
    revit_version: u32,
    products: &mut [DecodedElement],
) {
    let beams: BTreeSet<u64> = products
        .iter()
        .filter(|element| element.class == "StructuralFraming")
        .filter_map(|element| element.id)
        .map(u64::from)
        .collect();
    if beams.is_empty() {
        return;
    }
    let options = crate::native_document::Options {
        selected_ids: beams,
        releases: crate::native_document::NATIVE_RELEASES,
        ..Default::default()
    };
    let mut rotations: BTreeMap<u64, Option<f64>> = BTreeMap::new();
    let mut usages: BTreeMap<u64, Option<u64>> = BTreeMap::new();
    let read = crate::native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let rotation = graph
            .objects
            .iter()
            .filter(|object| object.class_name == "FamInstCurveDriver")
            .find_map(|object| object.fields.get("m_rotation"))
            .and_then(crate::graph_value::Fv::as_f64)
            .filter(|rotation| rotation.is_finite());
        let usage = graph
            .objects
            .first()
            .and_then(|root| root.fields.get("m_structUsage"))
            .and_then(crate::graph_value::Fv::as_u64);
        crate::native_json::agree(&mut rotations, record.identity.element_id, rotation);
        crate::native_json::agree(&mut usages, record.identity.element_id, usage);
        Ok(())
    });
    if read.is_err() {
        return;
    }
    for element in products
        .iter_mut()
        .filter(|element| element.class == "StructuralFraming")
    {
        let Some(id) = element.id.map(u64::from) else {
            continue;
        };
        let usage = usages
            .get(&id)
            .copied()
            .flatten()
            .and_then(|value| i64::try_from(value).ok());
        if let Some(value) = usage {
            element.fields.push((
                STRUCT_USAGE_FIELD.into(),
                InstanceField::Integer {
                    value,
                    signed: false,
                    size: 4,
                },
            ));
            if u64::try_from(value) == Ok(crate::ifc::category_map::HORIZONTAL_BRACING_USAGE)
                && BRACE_TYPE_REVIT_VERSIONS.contains(&revit_version)
            {
                element
                    .fields
                    .push((BRACE_TYPE_FIELD.into(), InstanceField::Bool(true)));
            }
        }
        if beam_column_cut(element) {
            continue;
        }
        let Some(Some(value)) = rotations.get(&id).copied() else {
            continue;
        };
        element.fields.push((
            BEAM_ROTATION_FIELD.into(),
            InstanceField::Float { value, size: 8 },
        ));
    }
}

/// Recover architectural column instances from partition element
/// records (#204, instance rule replaced in #211).
///
/// The `OST_Columns` sweep carries the type-symbol envelopes too, so
/// the family/type join of #215 costs no extra inflate — see
/// [`column_instances_from_records`].
pub fn columns_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let (column_records, wall_records) = column_and_wall_records(rf, revit_version)?;
    let wall_instances: Vec<crate::partition_element_records::PartitionElementRecord> =
        select_instance_records(wall_records)
            .into_values()
            .collect();
    Ok(column_instances_from_records(
        column_records,
        level_ids,
        &wall_instances,
    ))
}

/// Back-compat alias for the #204 entry point.
pub fn columns_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    column_instances_from_records(records, level_ids, &[])
}

/// Column instance selection with the family/type symbol join
/// attached (#215, RE-26).
///
/// The same category sweep carries both sides: the placed instances
/// (`is_exported_instance`) and the type-symbol envelopes
/// (`is_type_symbol`, RE-21 §5). Each instance names its symbol in
/// the counted reference list at `+0x88` —
/// [`crate::partition_element_records::PartitionElementRecord::type_symbol_reference`]
/// — and the symbol's own bounding box is the section in family
/// coordinates.
///
/// Measured on `2024_Core_Interior.rvt`: 256 of 256 exported columns
/// join to symbol `5755`, which is the `IfcColumnType.Tag` Revit's
/// own export writes for every one of them, and the section it gives
/// is the 2 ft square the instance envelope already carried — so the
/// join moves no vertex on this file. What it changes is where the
/// profile comes from: the type says the section is a rectangle,
/// instead of a box happening to be square.
pub fn column_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
    wall_records: &[crate::partition_element_records::PartitionElementRecord],
) -> Vec<DecodedElement> {
    use crate::partition_element_records::PartitionElementRecord;
    use std::collections::BTreeMap;

    let symbols: BTreeMap<u32, [f64; 6]> = records
        .iter()
        .filter(|record| record.is_type_symbol())
        .map(|record| (record.element_id, record.bbox_feet))
        .collect();
    let symbol_ids: BTreeSet<u32> = symbols.keys().copied().collect();
    let selected = select_instance_records(records);
    let instances: Vec<PartitionElementRecord> = selected.values().cloned().collect();
    let cuts = crate::element_record_column_cuts::column_cut_boxes(&instances, wall_records);
    let joined = crate::element_record_column_cuts::column_joined_walls(&instances, wall_records);
    selected
        .values()
        .map(|record: &PartitionElementRecord| {
            let mut decoded = element_record_decoded(record, "Column", level_ids);
            for &wall in joined.get(&record.element_id).into_iter().flatten() {
                decoded.fields.push((
                    crate::element_record_column_cuts::COLUMN_JOINED_WALL_FIELD.into(),
                    InstanceField::ElementId { tag: 0, id: wall },
                ));
            }
            if let Some(symbol) = record
                .type_symbol_reference(&symbol_ids)
                .and_then(|id| symbols.get(&id).map(|bbox| (id, *bbox)))
            {
                attach_type_symbol_profile(&mut decoded, record, symbol.0, &symbol.1);
            }
            if let Some(cut) = cuts.get(&record.element_id) {
                apply_column_join_cut(&mut decoded, cut);
            }
            decoded
        })
        .collect()
}

/// The frame [`column_instances_from_records`] takes a type symbol's box from
/// (#1368): the last of the symbol's frames, with the partitions that held one
/// and whether they all gave the same box.
pub(super) struct SymbolFrame {
    stream: String,
    offset: usize,
    bbox: [f64; 6],
    partitions: BTreeSet<u32>,
    agree: bool,
}

pub(super) type SymbolFrames = BTreeMap<u32, SymbolFrame>;

/// The frame of each type symbol in `records`, the last of its frames.
pub(super) fn type_symbol_frames(
    records: &[crate::partition_element_records::PartitionElementRecord],
) -> SymbolFrames {
    let mut out = SymbolFrames::new();
    for record in records.iter().filter(|record| record.is_type_symbol()) {
        let partition = crate::partition_room_parameters::partition_number(&record.stream);
        let held = out.entry(record.element_id).or_insert_with(|| SymbolFrame {
            stream: record.stream.clone(),
            offset: record.offset,
            bbox: record.bbox_feet,
            partitions: Default::default(),
            agree: true,
        });
        held.agree &= held.bbox == record.bbox_feet;
        held.stream = record.stream.clone();
        held.offset = record.offset;
        held.bbox = record.bbox_feet;
        held.partitions.insert(partition);
    }
    out
}

/// Note, for each column in `columns` that took its section from its type
/// symbol ([`attach_type_symbol_profile`]), the symbol's frame as the source
/// of its profile (#1368): `agreeing` where every frame of the symbol gives
/// the same box, `frame` (the last) where they do not.
pub(super) fn note_type_sections(
    rf: &mut RevitFile,
    columns: &[DecodedElement],
    frames: &SymbolFrames,
) {
    use crate::ifc::element_provenance::{CopyRule, SourcePart, ValueSource};
    for column in columns {
        let Some(id) = column.id else {
            continue;
        };
        let symbol = column.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_SYMBOL_FIELD => Some(*id),
            _ => None,
        });
        let Some((symbol, frame)) = symbol.and_then(|symbol| Some((symbol, frames.get(&symbol)?)))
        else {
            continue;
        };
        let part = SourcePart {
            element_id: symbol,
            stream: frame.stream.clone(),
            offset: frame.offset,
        };
        let rule = if frame.agree {
            CopyRule::Agreeing
        } else {
            CopyRule::Frame
        };
        let mut source =
            ValueSource::from_parts("type_section", rule, frame.partitions.len(), vec![part]);
        if frame.agree {
            source = source.with_partitions(frame.partitions.iter().copied().collect());
        }
        rf.note_value_source(id, source);
    }
}

/// Give each column whose record box starts at z = 0.0 the bottom its
/// geometry node holds (RE-173).
pub(super) fn apply_column_node_bottoms(rf: &mut RevitFile, lists: [&mut Vec<DecodedElement>; 2]) {
    let is_column = |e: &DecodedElement| matches!(e.class.as_str(), "Column" | "StructuralColumn");
    let mut targets = BTreeMap::new();
    for list in lists.iter() {
        for element in list.iter().filter(|e| is_column(e)) {
            if let (Some(id), Some(bbox)) = (element.id, element_record_bbox(element)) {
                if bbox[2] == 0.0 {
                    targets.insert(id, bbox);
                }
            }
        }
    }
    let bottoms = crate::partition_column_nodes::column_bottoms(rf, &targets);
    for list in lists {
        for element in list.iter_mut().filter(|e| is_column(e)) {
            let Some(&bottom) = element.id.and_then(|id| bottoms.get(&id)) else {
                continue;
            };
            let Some(bbox) = targets.get(&element.id.unwrap_or_default()) else {
                continue;
            };
            for (name, value) in element.fields.iter_mut() {
                match name.as_str() {
                    "m_locationZ" => {
                        *value = InstanceField::Float {
                            value: bottom,
                            size: 8,
                        }
                    }
                    "m_bboxHeight" => {
                        *value = InstanceField::Float {
                            value: bbox[5] - bottom,
                            size: 8,
                        }
                    }
                    _ => {}
                }
            }
            element.fields.push((
                COLUMN_Z_SOURCE_FIELD.into(),
                InstanceField::String(COLUMN_Z_GEOMETRY_NODE.into()),
            ));
        }
    }
}

/// Give each column that a joined slab or roof cuts the z-range of its cut
/// body, from its geometry node (RE-178). Only the body follows it.
pub(super) fn apply_column_cut_bodies(rf: &mut RevitFile, lists: [&mut Vec<DecodedElement>; 2]) {
    let is_column = |e: &DecodedElement| matches!(e.class.as_str(), "Column" | "StructuralColumn");
    let targets: BTreeMap<u32, [f64; 6]> = lists
        .iter()
        .flat_map(|list| list.iter().filter(|e| is_column(e)))
        .filter_map(|e| Some((e.id?, element_record_bbox(e)?)))
        .collect();
    let extents = crate::partition_column_nodes::column_cut_extents(rf, &targets);
    for list in lists {
        for element in list.iter_mut().filter(|e| is_column(e)) {
            let Some(&(bottom, top)) = element.id.and_then(|id| extents.get(&id)) else {
                continue;
            };
            for (name, value) in [
                (COLUMN_BODY_BOTTOM_FIELD, bottom),
                (COLUMN_BODY_TOP_FIELD, top),
            ] {
                element
                    .fields
                    .push((name.into(), InstanceField::Float { value, size: 8 }));
            }
        }
    }
}

/// Rewrite a column's placement and extents to the cut body, and say
/// where that body came from (#239, RE-29 §5).
///
/// The type section recovered by [`attach_type_symbol_profile`] is
/// the *uncut* family section and stays on the element as the type's
/// own fact; what changes is the emitted solid, which is the record
/// prism minus the walls the record names.
pub(super) fn apply_column_join_cut(
    decoded: &mut DecodedElement,
    cut: &crate::element_record_column_cuts::ColumnCut,
) {
    use crate::element_record_column_cuts as cuts;

    let bbox = cut.bbox_feet;
    let replacements: [(&str, f64); 6] = [
        ("m_locationX", (bbox[0] + bbox[3]) * 0.5),
        ("m_locationY", (bbox[1] + bbox[4]) * 0.5),
        ("m_locationZ", bbox[2]),
        ("m_bboxWidth", bbox[3] - bbox[0]),
        ("m_bboxDepth", bbox[4] - bbox[1]),
        ("m_bboxHeight", bbox[5] - bbox[2]),
    ];
    for (name, value) in decoded.fields.iter_mut() {
        if let Some((_, replacement)) = replacements
            .iter()
            .find(|(field, _)| *field == name.as_str())
        {
            *value = InstanceField::Float {
                value: *replacement,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        cuts::COLUMN_BODY_SOURCE_FIELD.into(),
        InstanceField::String(cuts::COLUMN_BODY_JOIN_CUT.into()),
    ));
    decoded.fields.push((
        cuts::COLUMN_CUT_WALL_COUNT_FIELD.into(),
        InstanceField::Integer {
            value: cut.wall_count as i64,
            signed: false,
            size: 8,
        },
    ));
}

/// Attach the type symbol's section to a placed instance, when the
/// section agrees with the instance's own plan envelope.
pub(super) fn attach_type_symbol_profile(
    decoded: &mut DecodedElement,
    record: &crate::partition_element_records::PartitionElementRecord,
    symbol_id: u32,
    symbol_bbox: &[f64; 6],
) {
    let width = symbol_bbox[3] - symbol_bbox[0];
    let depth = symbol_bbox[4] - symbol_bbox[1];
    if !(width.is_finite() && depth.is_finite()) || width <= 0.0 || depth <= 0.0 {
        return;
    }
    let (dx, dy, _) = record.extents_feet();
    if (width - dx).abs() > TYPE_PROFILE_EPS_FEET || (depth - dy).abs() > TYPE_PROFILE_EPS_FEET {
        return;
    }
    decoded.fields.push((
        TYPE_SYMBOL_FIELD.into(),
        InstanceField::ElementId {
            tag: 0,
            id: symbol_id,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_WIDTH_FIELD.into(),
        InstanceField::Float {
            value: width,
            size: 8,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_DEPTH_FIELD.into(),
        InstanceField::Float {
            value: depth,
            size: 8,
        },
    ));
    decoded.fields.push((
        TYPE_PROFILE_SOURCE_FIELD.into(),
        InstanceField::String(TYPE_PROFILE_SOURCE.into()),
    ));
}

/// Rewrite a beam's plan centre and extent along its run to the
/// column-cut body (RE-122).
pub(super) fn apply_beam_column_trim(
    decoded: &mut DecodedElement,
    trim: &crate::element_record_beam_cuts::BeamColumnTrim,
) {
    use crate::element_record_beam_cuts as cuts;
    let (centre_field, extent_field) = if trim.axis == 0 {
        ("m_locationX", "m_bboxWidth")
    } else {
        ("m_locationY", "m_bboxDepth")
    };
    for (name, value) in decoded.fields.iter_mut() {
        if name == centre_field {
            *value = InstanceField::Float {
                value: (trim.low_feet + trim.high_feet) * 0.5,
                size: 8,
            };
        } else if name == extent_field {
            *value = InstanceField::Float {
                value: trim.high_feet - trim.low_feet,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        cuts::BEAM_BODY_SOURCE_FIELD.into(),
        InstanceField::String(cuts::BEAM_BODY_COLUMN_CUT.into()),
    ));
}
