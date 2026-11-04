//! Wall lengths, tee reaches, butt joins and join trims.

use super::*;

/// Where each layer of a wall ends at its layered butt joins (RE-71).
#[derive(Debug, Clone, PartialEq)]
pub struct WallLayerEnds {
    /// Plan unit vector towards the wall's exterior face.
    pub exterior: [f64; 2],
    /// The layers' widths, exterior first, feet.
    pub widths: Vec<f64>,
    /// How far past its start and its end each layer reaches along its
    /// exterior-side and its interior-side edge, exterior first, where a
    /// layered or angled join decides it. The two are equal where the end
    /// is square.
    pub reach_feet: [Option<Vec<[f64; 2]>>; 2],
}

/// The [`WALL_JOIN_LAYER_EDGE_REACH_FIELDS`] or
/// [`WALL_JOIN_LAYER_REACH_FIELDS`] a wall carries, with its layers' widths
/// and exterior side. `None` without a layered or angled join at either
/// end, or where a reach list does not match the layers.
pub fn wall_layer_ends_from_fields(fields: &[(String, InstanceField)]) -> Option<WallLayerEnds> {
    let floats = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == wanted => items
                .iter()
                .map(|item| match item {
                    InstanceField::Float { value, .. } => Some(*value),
                    _ => None,
                })
                .collect::<Option<Vec<f64>>>(),
            _ => None,
        })
    };
    let square = WALL_JOIN_LAYER_REACH_FIELDS.map(floats);
    let slanted = WALL_JOIN_LAYER_EDGE_REACH_FIELDS.map(floats);
    if square.iter().chain(&slanted).all(Option::is_none) {
        return None;
    }
    let widths: Vec<f64> = fields
        .iter()
        .find_map(|(name, value)| {
            (name == WALL_LAYERS_FIELD).then(|| layer_bands_from_field(value))
        })
        .flatten()?
        .iter()
        .map(|band| band.width_feet)
        .collect();
    if square
        .iter()
        .flatten()
        .any(|reach| reach.len() != widths.len())
        || slanted
            .iter()
            .flatten()
            .any(|reach| reach.len() != 2 * widths.len())
    {
        return None;
    }
    let [start, end] = [0, 1].map(|slot| match (&slanted[slot], &square[slot]) {
        (Some(edges), _) => Some(edges.chunks(2).map(|pair| [pair[0], pair[1]]).collect()),
        (None, Some(reaches)) => Some(reaches.iter().map(|reach| [*reach, *reach]).collect()),
        (None, None) => None,
    });
    let reach_feet = [start, end];
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [Some(ex), Some(ey)] = WALL_EXTERIOR_FIELDS.map(float) else {
        return None;
    };
    Some(WallLayerEnds {
        exterior: [ex, ey],
        widths,
        reach_feet,
    })
}

/// Give each wall with a centreline whose native curve, cut back where its
/// body meets the walls joined at its ends, is as long as the curve, that
/// length ([`WALL_LENGTH_FIELD`]): the centreline's length, which Revit's
/// export writes as its `Length` whether it measures the curve or the cut
/// back curve (RE-252). The reach each end's join lists give
/// ([`WALL_JOIN_REACH_FIELDS`], or the largest of its layers' where they end
/// apart) is the second reading the record's must agree with.
pub(super) fn attach_wall_lengths(
    walls: &mut [DecodedElement],
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
) {
    const SAME_LENGTH_FEET: f64 = 1e-4;
    let joined = crate::native_walls::joined_at_ends(native);
    let near = |a: [f64; 2], b: [f64; 2]| {
        (a[0] - b[0]).hypot(a[1] - b[1]) <= NATIVE_WALL_LINE_TOLERANCE_FEET
    };
    for wall in walls.iter_mut() {
        let Some(id) = wall.id.map(u64::from) else {
            continue;
        };
        if wall.class == CURTAIN_WALL_CLASS {
            if let Some(curve) = native.get(&id) {
                let length = (curve.end[0] - curve.start[0]).hypot(curve.end[1] - curve.start[1]);
                if length.is_finite() && length > 0.0 {
                    wall.fields.push((
                        WALL_LENGTH_FIELD.into(),
                        InstanceField::Float {
                            value: length,
                            size: 8,
                        },
                    ));
                }
            }
            continue;
        }
        let (Some(axis), Some(curve)) = (wall_axis_from_fields(&wall.fields), native.get(&id))
        else {
            continue;
        };
        let ends = if near(curve.start, axis.start) && near(curve.end, axis.end) {
            [0, 1]
        } else if near(curve.start, axis.end) && near(curve.end, axis.start) {
            [1, 0]
        } else {
            continue;
        };
        let reach_of = |slot: usize| {
            wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == WALL_JOIN_REACH_FIELDS[slot] => {
                    Some(*value)
                }
                _ => None,
            })
        };
        let layer_reach_of = |slot: usize| {
            wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Vector(items) if name == WALL_JOIN_LAYER_REACH_FIELDS[slot] => items
                    .iter()
                    .map(|item| match item {
                        InstanceField::Float { value, .. } => Some(*value),
                        _ => None,
                    })
                    .collect::<Option<Vec<f64>>>()?
                    .into_iter()
                    .max_by(|a, b| a.abs().total_cmp(&b.abs())),
                _ => None,
            })
        };
        let mut reaches = [None, None];
        for (slot, end) in ends.into_iter().enumerate() {
            reaches[end] = reach_of(slot).or_else(|| layer_reach_of(slot));
        }
        let curve_length = (curve.end[0] - curve.start[0]).hypot(curve.end[1] - curve.start[1]);
        if curve_length.is_finite() && curve_length > 0.0 {
            wall.fields.push((
                WALL_CURVE_LENGTH_FIELD.into(),
                InstanceField::Float {
                    value: curve_length,
                    size: 8,
                },
            ));
        }
        let Some([start, end]) = crate::native_walls::trimmed_ends(id, native, &joined, reaches)
        else {
            continue;
        };
        let cutback = end - start;
        if cutback.is_finite() && cutback > 0.0 {
            wall.fields.push((
                WALL_CUTBACK_LENGTH_FIELD.into(),
                InstanceField::Float {
                    value: cutback,
                    size: 8,
                },
            ));
        }
        let length = (curve.end[0] - curve.start[0]).hypot(curve.end[1] - curve.start[1]);
        if ((end - start) - length).abs() <= SAME_LENGTH_FEET && length.is_finite() && length > 0.0
        {
            wall.fields.push((
                WALL_LENGTH_FIELD.into(),
                InstanceField::Float {
                    value: length,
                    size: 8,
                },
            ));
        }
    }
}

/// How far a wall's elevation range may lie outside another's and still
/// contain its base, feet (#1396).
const BASE_ELEVATION_TOLERANCE_FEET: f64 = 1e-3;

/// How much further than the host's a face must reach to replace it, feet
/// (#1396): a wall whose end is cut at a face that agrees with the host's to
/// within this keeps the host's.
const FACE_REPLACEMENT_MINIMUM_FEET: f64 = 1e-3;

/// The reach of the end of wall `id` at a T joint whose host lies wholly above
/// the wall's base, where the end is cut against another wall instead (#1396,
/// RE-337): the one wall the end's `JoinEndGStep.m_geomJoined` names, other
/// than the host, that meets it with its side and holds the wall's base. The
/// host cuts only the top of such a wall, and Revit's body at the base stops
/// at that wall's face, a step below where it stops at the host's. `None`
/// where the host holds the base, where no wall or several others do, or where
/// that wall does not cross at a right angle.
fn tee_reach_at_base(
    id: u64,
    end: usize,
    host: u64,
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
    thickness: &BTreeMap<u64, f64>,
    spans: &BTreeMap<u64, (f64, f64)>,
) -> Option<f64> {
    let (base, _) = *spans.get(&id)?;
    let holds = |other: u64| {
        spans.get(&other).map(|&(low, high)| {
            low - BASE_ELEVATION_TOLERANCE_FEET <= base
                && base <= high + BASE_ELEVATION_TOLERANCE_FEET
        })
    };
    if holds(host)? {
        return None;
    }
    let mut holding = native
        .get(&id)?
        .geometry_joined
        .get(end)?
        .iter()
        .filter(|&&(other, at)| {
            other != id && other != host && matches!(at, 2 | 3) && holds(other) == Some(true)
        })
        .map(|&(other, _)| other);
    let (Some(other), None) = (holding.next(), holding.next()) else {
        return None;
    };
    crate::native_walls::tee_face_reach(id, end, other, native, *thickness.get(&other)?)
}

/// At each end of a wall with a centreline where its native record says
/// another wall meets that end with its middle, the body stops half that
/// wall's thickness short ([`crate::native_walls::tee_reach`], RE-222),
/// in place of the reach its join lists give. Where that host lies wholly
/// above the wall's base, the end stops at the face of the wall that holds the
/// base ([`tee_reach_at_base`], #1396). An end whose layers each end
/// on their own line (RE-71, RE-74) keeps them, and so does one whose other
/// wall's thickness is not known: its type's, or its record box's thin
/// extent where it runs along a model axis.
pub(super) fn attach_native_tee_reaches(
    walls: &mut [DecodedElement],
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
) {
    // A wall's type thickness, or, for a wall along a model axis, its record
    // box's thin extent, which is its thickness there (RE-26).
    let along_axis = |id: u64| {
        native.get(&id).is_some_and(|curve| {
            let (dx, dy) = (curve.end[0] - curve.start[0], curve.end[1] - curve.start[1]);
            dx.abs() <= 1e-9 || dy.abs() <= 1e-9
        })
    };
    let thickness: BTreeMap<u64, f64> = walls
        .iter()
        .filter_map(|wall| {
            let id = u64::from(wall.id?);
            let float = |wanted: &str| {
                wall.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::Float { value, .. } if name == wanted => Some(*value),
                    _ => None,
                })
            };
            let thickness = float(WALL_TYPE_THICKNESS_FIELD).or_else(|| {
                float(crate::element_record_wall_joins::WALL_THICKNESS_FIELD)
                    .filter(|_| along_axis(id))
            })?;
            Some((id, thickness))
        })
        .collect();
    // Each wall's elevation range: its base and its record box's top.
    let spans: BTreeMap<u64, (f64, f64)> = walls
        .iter()
        .filter_map(|wall| {
            let float = |wanted: &str| {
                wall.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::Float { value, .. } if name == wanted => Some(*value),
                    _ => None,
                })
            };
            let base = float("m_locationZ")?;
            let top = base + float("m_bboxHeight")?;
            (base.is_finite() && top.is_finite() && top > base)
                .then_some((u64::from(wall.id?), (base, top)))
        })
        .collect();
    let near = |a: [f64; 2], b: [f64; 2]| {
        (a[0] - b[0]).hypot(a[1] - b[1]) <= NATIVE_WALL_LINE_TOLERANCE_FEET
    };
    for wall in walls.iter_mut() {
        let Some(id) = wall.id.map(u64::from) else {
            continue;
        };
        let (Some(axis), Some(curve)) = (wall_axis_from_fields(&wall.fields), native.get(&id))
        else {
            continue;
        };
        let ends = if near(curve.start, axis.start) && near(curve.end, axis.end) {
            [0, 1]
        } else if near(curve.start, axis.end) && near(curve.end, axis.start) {
            [1, 0]
        } else {
            continue;
        };
        for (slot, end) in ends.into_iter().enumerate() {
            let layered = [
                WALL_JOIN_LAYER_REACH_FIELDS[slot],
                WALL_JOIN_LAYER_EDGE_REACH_FIELDS[slot],
            ];
            if wall
                .fields
                .iter()
                .any(|(name, _)| layered.contains(&name.as_str()))
            {
                continue;
            }
            if let Some(mut reach) = crate::native_walls::tee_reach(id, end, native, |other| {
                thickness.get(&other).copied()
            }) {
                if let Some(at_base) = crate::native_walls::tee_host(id, end, native)
                    .and_then(|host| tee_reach_at_base(id, end, host, native, &thickness, &spans))
                    .filter(|at_base| (at_base - reach).abs() > FACE_REPLACEMENT_MINIMUM_FEET)
                {
                    reach = at_base;
                }
                wall.fields
                    .retain(|(name, _)| name != WALL_JOIN_REACH_FIELDS[slot]);
                wall.fields.push((
                    WALL_JOIN_REACH_FIELDS[slot].into(),
                    InstanceField::Float {
                        value: reach,
                        size: 8,
                    },
                ));
            }
        }
    }
}

/// Each wall with a centreline, with its type's layers, its exterior side and
/// its elevation range, which [`crate::element_record_wall_joins::butt_joins`]
/// and [`crate::element_record_wall_joins::corner_joins`] read.
fn wall_lines(
    walls: &[DecodedElement],
    type_layers: &BTreeMap<u32, Vec<crate::partition_compound_structure::CompoundLayer>>,
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
) -> Vec<crate::element_record_wall_joins::WallLine> {
    use crate::element_record_wall_joins as joins;
    let float = |wall: &DecodedElement, wanted: &str| {
        wall.fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    walls
        .iter()
        .filter_map(|wall| {
            let axis = wall_axis_from_fields(&wall.fields)?;
            let base = float(wall, "m_locationZ")?;
            let type_id = wall.fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
                _ => None,
            })?;
            let layers: Vec<(f64, u32)> = type_layers
                .get(&type_id)?
                .iter()
                .filter(|layer| layer.width_feet > 0.0)
                .map(|layer| (layer.width_feet, layer.function))
                .collect();
            let [Some(ex), Some(ey)] = WALL_EXTERIOR_FIELDS.map(|name| float(wall, name)) else {
                return None;
            };
            Some(joins::WallLine {
                element_id: wall.id?,
                start: axis.start,
                end: axis.end,
                thickness_feet: axis.thickness_feet,
                layers,
                exterior: [ex, ey],
                base_feet: base,
                top_feet: base + float(wall, "m_bboxHeight")?,
                shells: native
                    .get(&u64::from(wall.id?))
                    .and_then(|native| native.shells),
            })
        })
        .collect()
}

/// Where three or more walls meet end to end at a butt join, the end of each
/// wall the record's join order decides
/// ([`crate::element_record_wall_joins::corner_joins`], RE-322): how far past
/// its line's end the body reaches, every layer alike, in place of the reach a
/// join list or a T joint gave that end. It names the wall it stops at or runs
/// through only where one perpendicular wall alone decides it; at a corner
/// where several do, or where it meets a wall that continues it flush, there
/// is no one partner and none is named. Two single-layer walls that meet at a
/// mitre get the slanted end it makes, along their exterior-side and
/// interior-side edges, and no partner.
pub(super) fn attach_wall_corner_reaches(
    walls: &mut [DecodedElement],
    type_layers: &BTreeMap<u32, Vec<crate::partition_compound_structure::CompoundLayer>>,
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
) {
    use crate::element_record_wall_joins as joins;
    let lines = wall_lines(walls, type_layers, native);
    if lines.len() < 3 {
        return;
    }
    let near = |a: [f64; 2], b: [f64; 2]| {
        (a[0] - b[0]).hypot(a[1] - b[1]) <= NATIVE_WALL_LINE_TOLERANCE_FEET
    };
    // The record's joins are indexed by the end of its own curve, the
    // reaches by the end of the wall's axis: which of the two orders each
    // wall's ends are in. A wall whose curve is not its axis, either way
    // round, is left out, and so is any corner it is a member of.
    let reversed: BTreeMap<u32, bool> = lines
        .iter()
        .filter_map(|line| {
            let curve = native.get(&u64::from(line.element_id))?;
            if near(curve.start, line.start) && near(curve.end, line.end) {
                Some((line.element_id, false))
            } else if near(curve.start, line.end) && near(curve.end, line.start) {
                Some((line.element_id, true))
            } else {
                None
            }
        })
        .collect();
    let by_axis_end = |id: u32, end: u32| -> Option<u32> {
        if end > 1 {
            return Some(end);
        }
        Some(if *reversed.get(&id)? { 1 - end } else { end })
    };
    let narrow = |join: &Option<Vec<(u64, u32)>>| -> Option<Vec<(u32, u32)>> {
        join.as_ref()?
            .iter()
            .map(|&(id, end)| {
                let id = u32::try_from(id).ok()?;
                Some((id, by_axis_end(id, end)?))
            })
            .collect()
    };
    let ends: BTreeMap<u32, joins::NativeEnds> = native
        .iter()
        .filter_map(|(&id, wall)| {
            let id = u32::try_from(id).ok()?;
            let flipped = *reversed.get(&id)?;
            let (first, second) = if flipped { (1, 0) } else { (0, 1) };
            Some((
                id,
                joins::NativeEnds {
                    members: [narrow(&wall.joins[first]), narrow(&wall.joins[second])],
                    join_types: [wall.join_types[first], wall.join_types[second]],
                    other_cutters: wall
                        .partners
                        .iter()
                        .any(|other| !native.contains_key(other)),
                },
            ))
        })
        .collect();
    let found = joins::corner_joins(&lines, &ends);
    let layer_counts: BTreeMap<u32, usize> = lines
        .iter()
        .map(|line| (line.element_id, line.layers.len()))
        .collect();
    for wall in walls.iter_mut() {
        let Some((id, reaches)) = wall.id.and_then(|id| Some((id, found.get(&id)?))) else {
            continue;
        };
        for (slot, cut) in reaches.iter().enumerate() {
            let Some(cut) = *cut else {
                continue;
            };
            let replaced = [
                WALL_JOIN_PARTNER_FIELDS[slot],
                WALL_JOIN_THROUGH_FIELDS[slot],
                WALL_JOIN_REACH_FIELDS[slot],
                WALL_JOIN_LAYER_REACH_FIELDS[slot],
                WALL_JOIN_LAYER_EDGE_REACH_FIELDS[slot],
            ];
            wall.fields
                .retain(|(name, _)| !replaced.contains(&name.as_str()));
            let float = |value: f64| InstanceField::Float { value, size: 8 };
            match cut {
                joins::CornerEnd::Reach {
                    reach_feet: reach,
                    partner,
                } => {
                    if let Some((partner, through)) = partner {
                        wall.fields.push((
                            WALL_JOIN_PARTNER_FIELDS[slot].into(),
                            InstanceField::ElementId {
                                tag: 0,
                                id: partner,
                            },
                        ));
                        wall.fields.push((
                            WALL_JOIN_THROUGH_FIELDS[slot].into(),
                            InstanceField::Bool(through),
                        ));
                    }
                    wall.fields
                        .push((WALL_JOIN_REACH_FIELDS[slot].into(), float(reach)));
                    let layers = layer_counts.get(&id).copied().unwrap_or(1);
                    if layers > 1 {
                        wall.fields.push((
                            WALL_JOIN_LAYER_REACH_FIELDS[slot].into(),
                            InstanceField::Vector(vec![float(reach); layers]),
                        ));
                    }
                }
                joins::CornerEnd::Mitre(edges) => {
                    wall.fields.push((
                        WALL_JOIN_LAYER_EDGE_REACH_FIELDS[slot].into(),
                        InstanceField::Vector(edges.into_iter().map(float).collect()),
                    ));
                }
            }
        }
    }
}

/// Give each wall with a centreline the wall it butt-joins at each end and
/// whether it runs through there, where its and its partner's join lists
/// decide it, and, between single-layer walls, how far past the end its
/// body reaches ([`crate::element_record_wall_joins::butt_joins`], RE-70).
pub(super) fn attach_wall_butt_joins(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
    type_layers: &BTreeMap<u32, Vec<crate::partition_compound_structure::CompoundLayer>>,
    native: &BTreeMap<u64, crate::native_walls::NativeWall>,
) {
    use crate::element_record_wall_joins as joins;
    let lines = wall_lines(walls, type_layers, native);
    if lines.len() < 2 {
        return;
    }
    let every: BTreeSet<u32> = walls.iter().filter_map(|wall| wall.id).collect();
    let read: BTreeSet<u32> = lines.iter().map(|line| line.element_id).collect();
    let Ok(partners) = crate::partition_compound_structure::scan_wall_join_partners(
        rf,
        revit_version,
        &every,
        &read,
    ) else {
        return;
    };
    // RE-127: the joined-wall lists decide the L joints the join lists do not.
    let joined = crate::partition_compound_structure::scan_wall_joined_entries(
        rf,
        revit_version,
        &every,
        &read,
    )
    .unwrap_or_default();
    let found = joins::butt_joins(&lines, &partners, &joined);
    for wall in walls.iter_mut() {
        let Some(ends) = wall.id.and_then(|id| found.get(&id)) else {
            continue;
        };
        for (slot, join) in ends.iter().enumerate() {
            let Some(join) = join else {
                continue;
            };
            wall.fields.push((
                WALL_JOIN_PARTNER_FIELDS[slot].into(),
                InstanceField::ElementId {
                    tag: 0,
                    id: join.partner,
                },
            ));
            wall.fields.push((
                WALL_JOIN_THROUGH_FIELDS[slot].into(),
                InstanceField::Bool(join.runs_through),
            ));
            if let Some(reach) = join.reach_feet {
                wall.fields.push((
                    WALL_JOIN_REACH_FIELDS[slot].into(),
                    InstanceField::Float {
                        value: reach,
                        size: 8,
                    },
                ));
            }
            if let Some(edges) = &join.layer_edge_reach_feet {
                wall.fields.push((
                    WALL_JOIN_LAYER_EDGE_REACH_FIELDS[slot].into(),
                    InstanceField::Vector(
                        edges
                            .iter()
                            .flatten()
                            .map(|value| InstanceField::Float {
                                value: *value,
                                size: 8,
                            })
                            .collect(),
                    ),
                ));
            }
            if let Some(reaches) = &join.layer_reach_feet {
                wall.fields.push((
                    WALL_JOIN_LAYER_REACH_FIELDS[slot].into(),
                    InstanceField::Vector(
                        reaches
                            .iter()
                            .map(|value| InstanceField::Float {
                                value: *value,
                                size: 8,
                            })
                            .collect(),
                    ),
                ));
            }
        }
    }
}

/// Rewrite a wall's plan centre and plan extents to the trimmed body.
pub(super) fn apply_wall_join_trim(
    decoded: &mut DecodedElement,
    record: &crate::partition_element_records::PartitionElementRecord,
    trim: &crate::element_record_wall_joins::WallJoinTrim,
) {
    use crate::element_record_wall_joins as joins;

    let low = record.bbox_feet[trim.axis] + trim.start_feet;
    let high = record.bbox_feet[trim.axis + 3] - trim.end_feet;
    let (centre_field, extent_field) = if trim.axis == 0 {
        ("m_locationX", "m_bboxWidth")
    } else {
        ("m_locationY", "m_bboxDepth")
    };
    for (name, value) in decoded.fields.iter_mut() {
        if name == centre_field {
            *value = InstanceField::Float {
                value: (low + high) * 0.5,
                size: 8,
            };
        } else if name == extent_field {
            *value = InstanceField::Float {
                value: high - low,
                size: 8,
            };
        }
    }
    decoded.fields.push((
        joins::WALL_BODY_SOURCE_FIELD.into(),
        InstanceField::String(joins::WALL_BODY_JOIN_TRIMMED.into()),
    ));
    decoded.fields.push((
        joins::WALL_THICKNESS_FIELD.into(),
        InstanceField::Float {
            value: trim.thickness_feet,
            size: 8,
        },
    ));
    decoded.fields.push((
        joins::WALL_TRIM_START_FIELD.into(),
        InstanceField::Float {
            value: trim.start_feet,
            size: 8,
        },
    ));
    decoded.fields.push((
        joins::WALL_TRIM_END_FIELD.into(),
        InstanceField::Float {
            value: trim.end_feet,
            size: 8,
        },
    ));
}
