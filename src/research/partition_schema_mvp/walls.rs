//! Wall axes, arcs, tapers, profile openings, layers and instances.

use super::*;
use std::f64::consts::FRAC_PI_4;

/// A wall's centreline and its type's thickness (RE-54).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallAxis {
    /// Plan start, model feet.
    pub start: [f64; 2],
    /// Plan end, model feet.
    pub end: [f64; 2],
    /// The type's layers summed, feet.
    pub thickness_feet: f64,
    /// How far past its start and its end the body reaches at a butt join,
    /// where the join lists decide it (RE-70).
    pub join_reach_feet: [Option<f64>; 2],
}

/// The centreline and thickness [`WALL_AXIS_START_FIELDS`],
/// [`WALL_AXIS_END_FIELDS`] and [`WALL_TYPE_THICKNESS_FIELD`] record, with
/// the [`WALL_JOIN_REACH_FIELDS`] present.
pub fn wall_axis_from_fields(fields: &[(String, InstanceField)]) -> Option<WallAxis> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [sx, sy] = WALL_AXIS_START_FIELDS.map(float);
    let [ex, ey] = WALL_AXIS_END_FIELDS.map(float);
    Some(WallAxis {
        start: [sx?, sy?],
        end: [ex?, ey?],
        thickness_feet: float(WALL_TYPE_THICKNESS_FIELD)?,
        join_reach_feet: WALL_JOIN_REACH_FIELDS.map(float),
    })
}

/// The ElementIds [`WALL_TOP_CUTTERS_FIELD`] records: what a wall's top join
/// and cut pairs name (RE-267). Empty where it holds none.
pub fn wall_top_cutters_from_fields(fields: &[(String, InstanceField)]) -> Vec<u32> {
    fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == WALL_TOP_CUTTERS_FIELD => Some(
                items
                    .iter()
                    .filter_map(|item| match item {
                        InstanceField::ElementId { id, .. } => Some(*id),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => None,
        })
        .unwrap_or_default()
}

/// The ElementIds [`WALL_BOUNDED_FIELD`] records: the elements a wall's roof
/// and floor joins name (RE-270). `None` where the field is absent, an empty
/// list where the wall is joined to none.
pub fn wall_bounded_from_fields(fields: &[(String, InstanceField)]) -> Option<Vec<u32>> {
    fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(items) if name == WALL_BOUNDED_FIELD => Some(
            items
                .iter()
                .filter_map(|item| match item {
                    InstanceField::ElementId { id, .. } => Some(*id),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    })
}

/// A curved wall's location arc and its type's thickness (RE-75).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallArc {
    /// Plan centre, model feet.
    pub centre: [f64; 2],
    /// Radius, feet.
    pub radius: f64,
    /// Start and end angle, radians.
    pub angles: [f64; 2],
    /// Plan unit X axis.
    pub x_axis: [f64; 2],
    /// Plan unit Y axis.
    pub y_axis: [f64; 2],
    /// The type's layers summed, feet.
    pub thickness_feet: f64,
}

impl WallArc {
    /// The plan point `offset` feet out from the arc (away from its centre)
    /// at `angle`.
    pub fn point(&self, angle: f64, offset: f64) -> [f64; 2] {
        let (sin, cos) = angle.sin_cos();
        let radius = self.radius + offset;
        [0, 1].map(|axis| {
            self.centre[axis] + radius * (cos * self.x_axis[axis] + sin * self.y_axis[axis])
        })
    }
}

/// A tapered wall's exterior face (RE-86).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallTaper {
    /// How far the exterior face leans from vertical, radians, wider at
    /// the base.
    pub angle_radians: f64,
    /// Plan unit vector towards the exterior face.
    pub exterior: [f64; 2],
}

/// Whether the wall carries [`WALL_EXTERIOR_FACE_ANGLE_FIELD`], so its line
/// alone does not give its body.
pub fn wall_is_tapered(fields: &[(String, InstanceField)]) -> bool {
    fields
        .iter()
        .any(|(name, _)| name == WALL_EXTERIOR_FACE_ANGLE_FIELD)
}

/// The [`WALL_EXTERIOR_FACE_ANGLE_FIELD`] and [`WALL_EXTERIOR_FIELDS`]
/// record; `None` without either.
pub fn wall_taper_from_fields(fields: &[(String, InstanceField)]) -> Option<WallTaper> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [x, y] = WALL_EXTERIOR_FIELDS.map(float);
    Some(WallTaper {
        angle_radians: float(WALL_EXTERIOR_FACE_ANGLE_FIELD)?,
        exterior: [x?, y?],
    })
}

/// The arc [`WALL_ARC_FIELDS`] and [`WALL_TYPE_THICKNESS_FIELD`] record.
pub fn wall_arc_from_fields(fields: &[(String, InstanceField)]) -> Option<WallArc> {
    let float = |wanted: &str| {
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Float { value, .. } if name == wanted => Some(*value),
            _ => None,
        })
    };
    let [cx, cy, radius, start, end, xx, xy, yx, yy] = WALL_ARC_FIELDS.map(float);
    Some(WallArc {
        centre: [cx?, cy?],
        radius: radius?,
        angles: [start?, end?],
        x_axis: [xx?, xy?],
        y_axis: [yx?, yy?],
        thickness_feet: float(WALL_TYPE_THICKNESS_FIELD)?,
    })
}

/// A sketch line: its ElementId, start and end, model feet.
// Tracker: B55.
pub type SketchSegment = (u32, [f64; 3], [f64; 3]);

/// A sketch line in its plane: its ElementId, and its ends along and up the
/// plane.
// Tracker: B55.
pub(super) type PlaneEdge = (u32, (f64, f64), (f64, f64));

/// Give each wall whose elevation profile was edited the openings that cuts
/// (RE-151's 63rd opening).
///
/// The profile is the wall's sketch, the curves its sketch lists (RE-97),
/// each read as a straight line
/// ([`crate::partition_beam_axes::scan_sketch_curves`]) whose ends lie in its
/// record's box. Where every line lies in one vertical plane and they close
/// one loop, each run of lines that leaves the loop's bounding rectangle cuts
/// the region between it and the rectangle's edge, which Revit's export
/// writes as an opening voiding the wall ([`elevation_profile_cuts`]). A wall
/// whose sketch holds an arc, or does not close, is left alone.
///
/// Measured on Core Interior's wall 55840 (Revit 2024), whose one opening
/// Revit's export tags 55859.
pub(super) fn attach_wall_profile_openings(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
) {
    use crate::partition_element_records as per;
    let wall_ids: BTreeSet<u32> = walls.iter().filter_map(|wall| wall.id).collect();
    if wall_ids.is_empty() || !crate::partition_beam_axes::supports_revit_version(revit_version) {
        return;
    }
    let declared = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Ok(lines) =
        per::scan_category_records_multi(rf, revit_version, &[per::OST_SKETCH_LINES], &declared)
    else {
        return;
    };
    let line_ids: BTreeSet<u32> = lines.iter().map(|record| record.element_id).collect();
    let lists: BTreeMap<u32, Vec<u32>> =
        crate::element_record_plan_profiles::scan_sketch_curve_lists(rf, revit_version, &line_ids)
            .into_iter()
            .filter(|(owner, _)| wall_ids.contains(owner))
            .collect();
    if lists.is_empty() {
        return;
    }
    let wanted: BTreeSet<u32> = lists.values().flatten().copied().collect();
    let Ok(curves) = crate::partition_beam_axes::scan_sketch_curves(rf, revit_version, &wanted)
    else {
        return;
    };
    let boxes: BTreeMap<u32, [f64; 6]> = lines
        .iter()
        .filter(|record| wanted.contains(&record.element_id))
        .map(|record| (record.element_id, record.bbox_feet))
        .collect();
    let in_box = |point: [f64; 3], bbox: &[f64; 6]| {
        (0..3).all(|axis| {
            point[axis] >= bbox[axis] - PROFILE_EPS_FEET
                && point[axis] <= bbox[axis + 3] + PROFILE_EPS_FEET
        })
    };
    for wall in walls.iter_mut() {
        let Some(curve_ids) = wall.id.and_then(|id| lists.get(&id)) else {
            continue;
        };
        let segments: Option<Vec<SketchSegment>> = curve_ids
            .iter()
            .map(|id| {
                let line = curves.get(id)?.line?;
                let bbox = boxes.get(id)?;
                let (start, end) = (line.start(), line.end());
                (in_box(start, bbox) && in_box(end, bbox)).then_some((*id, start, end))
            })
            .collect();
        let Some(segments) = segments else {
            continue;
        };
        for (k, (tag, outline)) in elevation_profile_cuts(&segments).into_iter().enumerate() {
            let points = outline
                .iter()
                .map(|point| {
                    InstanceField::Vector(
                        point
                            .iter()
                            .map(|value| InstanceField::Float {
                                value: *value,
                                size: 8,
                            })
                            .collect(),
                    )
                })
                .collect();
            wall.fields.push((
                format!("{PROFILE_OPENING_FIELD}_{k}"),
                InstanceField::Vector(points),
            ));
            wall.fields.push((
                format!("{PROFILE_OPENING_FIELD}_{k}_tag"),
                InstanceField::ElementId { tag: 0, id: tag },
            ));
        }
    }
}

/// The openings a wall's edited elevation profile cuts, from its sketch
/// lines (ElementId, start, end, model feet): each region between a
/// run of lines that leaves the profile's bounding rectangle and the
/// rectangle's edge, as model-feet points in the profile's plane,
/// counter-clockwise in its own along-and-up coordinates, tagged with the
/// highest ElementId of the run, as Revit's export tags wall 55840's one
/// opening (55859, also the last line of its sketch). Empty where the lines
/// do not lie in one vertical plane, do not close one loop, or never leave
/// the rectangle.
// Tracker: B55.
pub fn elevation_profile_cuts(segments: &[SketchSegment]) -> Vec<(u32, Vec<[f64; 3]>)> {
    let eps = PROFILE_EPS_FEET;
    let points: Vec<[f64; 3]> = segments.iter().flat_map(|(_, a, b)| [*a, *b]).collect();
    let plan = |a: [f64; 3], b: [f64; 3]| ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    let mut far = (0.0, [0.0; 3], [0.0; 3]);
    for a in &points {
        for b in &points {
            let distance = plan(*a, *b);
            if distance > far.0 {
                far = (distance, *a, *b);
            }
        }
    }
    let (length, origin, toward) = far;
    if length <= eps {
        return Vec::new();
    }
    let axis = [
        (toward[0] - origin[0]) / length,
        (toward[1] - origin[1]) / length,
    ];
    // A point's place along the plane's axis and up it, and how far off the
    // plane it lies.
    let to_uv = |p: [f64; 3]| {
        let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
        (
            (dx * axis[0] + dy * axis[1], p[2]),
            (dx * axis[1] - dy * axis[0]).abs(),
        )
    };
    if points.iter().any(|p| to_uv(*p).1 > eps) {
        return Vec::new();
    }
    let edges: Vec<PlaneEdge> = segments
        .iter()
        .map(|(id, a, b)| (*id, to_uv(*a).0, to_uv(*b).0))
        .collect();
    let meets = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() <= eps && (a.1 - b.1).abs() <= eps;
    // Chain the lines into one loop: its corners, and the line leaving each.
    let Some(first) = edges.first() else {
        return Vec::new();
    };
    let mut corners = vec![first.1];
    let mut ids = vec![first.0];
    let mut end = first.2;
    let mut used = vec![false; edges.len()];
    used[0] = true;
    for _ in 1..edges.len() {
        let Some((index, next)) = edges.iter().enumerate().find_map(|(index, (_, a, b))| {
            if used[index] {
                None
            } else if meets(*a, end) {
                Some((index, *b))
            } else if meets(*b, end) {
                Some((index, *a))
            } else {
                None
            }
        }) else {
            return Vec::new();
        };
        used[index] = true;
        corners.push(end);
        ids.push(edges[index].0);
        end = next;
    }
    if !meets(end, corners[0]) {
        return Vec::new();
    }
    let count = corners.len();
    let area: f64 = (0..count)
        .map(|i| {
            let (a, b) = (corners[i], corners[(i + 1) % count]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    if area < 0.0 {
        corners = std::iter::once(corners[0])
            .chain(corners[1..].iter().rev().copied())
            .collect();
        ids.reverse();
    }
    let corner = |i: usize| corners[i % count];
    let (u_min, u_max, v_min, v_max) = corners.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(a, b, c, d), (u, v)| (a.min(*u), b.max(*u), c.min(*v), d.max(*v)),
    );
    let (width, height) = (u_max - u_min, v_max - v_min);
    let perimeter = 2.0 * (width + height);
    // Where a point lies along the rectangle's edge, counter-clockwise from
    // its lower left corner; `None` off the edge.
    let along = |(u, v): (f64, f64)| {
        if (v - v_min).abs() <= eps {
            Some(u - u_min)
        } else if (u - u_max).abs() <= eps {
            Some(width + v - v_min)
        } else if (v - v_max).abs() <= eps {
            Some(width + height + u_max - u)
        } else if (u - u_min).abs() <= eps {
            Some(2.0 * width + height + v_max - v)
        } else {
            None
        }
    };
    let on_side = |a: (f64, f64), b: (f64, f64)| {
        [
            (a.1 - v_min, b.1 - v_min),
            (a.0 - u_max, b.0 - u_max),
            (a.1 - v_max, b.1 - v_max),
            (a.0 - u_min, b.0 - u_min),
        ]
        .iter()
        .any(|(x, y)| x.abs() <= eps && y.abs() <= eps)
    };
    let inside: Vec<bool> = (0..count)
        .map(|i| !on_side(corner(i), corner(i + 1)))
        .collect();
    let Some(start) = (0..count).find(|i| !inside[*i] && inside[(i + 1) % count]) else {
        return Vec::new();
    };
    let rectangle = [
        ((u_min, v_min), 0.0),
        ((u_max, v_min), width),
        ((u_max, v_max), width + height),
        ((u_min, v_max), 2.0 * width + height),
    ];
    let mut cuts = Vec::new();
    let mut i = start + 1;
    while i <= start + count {
        if !inside[i % count] {
            i += 1;
            continue;
        }
        let run_start = i;
        while inside[i % count] {
            i += 1;
        }
        let (a, b) = (corner(run_start), corner(i));
        let (Some(at_a), Some(at_b)) = (along(a), along(b)) else {
            return Vec::new();
        };
        let back = (at_b - at_a).rem_euclid(perimeter);
        if back <= eps {
            continue;
        }
        // The run from A to B, then back along the rectangle's edge,
        // clockwise, to A; reversed to run counter-clockwise.
        let mut outline: Vec<(f64, f64)> = (run_start..=i).map(corner).collect();
        let mut passed: Vec<(f64, (f64, f64))> = rectangle
            .iter()
            .map(|(point, at)| ((at_b - at).rem_euclid(perimeter), *point))
            .filter(|(distance, _)| *distance > eps && *distance < back - eps)
            .collect();
        passed.sort_by(|x, y| x.0.total_cmp(&y.0));
        outline.extend(passed.into_iter().map(|(_, point)| point));
        outline.reverse();
        let tag = (run_start..i)
            .map(|edge| ids[edge % count])
            .max()
            .expect("a run holds a line");
        cuts.push((
            tag,
            outline
                .into_iter()
                .map(|(u, v)| [origin[0] + axis[0] * u, origin[1] + axis[1] * u, v])
                .collect(),
        ));
    }
    cuts
}

/// The openings a wall's edited elevation profile cuts, as the partition MVP
/// gives them ([`PROFILE_OPENING_FIELD`]): each one's tag and outline.
// Tracker: B55.
pub fn profile_openings_from_fields(
    fields: &[(String, InstanceField)],
) -> Vec<(u32, Vec<[f64; 3]>)> {
    (0..)
        .map_while(|k| {
            let outline_name = format!("{PROFILE_OPENING_FIELD}_{k}");
            let tag_name = format!("{outline_name}_tag");
            let outline = fields.iter().find_map(|(name, value)| match value {
                InstanceField::Vector(points) if *name == outline_name => Some(points),
                _ => None,
            })?;
            let tag = fields.iter().find_map(|(name, value)| match value {
                InstanceField::ElementId { id, .. } if *name == tag_name => Some(*id),
                _ => None,
            })?;
            let points: Option<Vec<[f64; 3]>> = outline
                .iter()
                .map(|point| {
                    let InstanceField::Vector(values) = point else {
                        return None;
                    };
                    let mut xyz = values.iter().map(|value| match value {
                        InstanceField::Float { value, .. } => Some(*value),
                        _ => None,
                    });
                    Some([xyz.next()??, xyz.next()??, xyz.next()??])
                })
                .collect();
            Some((tag, points?))
        })
        .collect()
}

/// Give each of `walls` whether its top is joined to a roof or floor
/// ([`WALL_TOP_JOINED_FIELD`], RE-228) from its native record, on the
/// releases in [`WALL_TOP_JOIN_RECORD_REVIT_VERSIONS`], whose walls come from
/// their element records and take nothing else from the native records but
/// their type Function (RE-284). The other facts [`attach_wall_layers`] reads
/// from them on 2024 and later (a wall's height, its top cut, its areas) are
/// not measured on these releases.
pub(super) fn attach_wall_top_joins(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
) {
    if !WALL_TOP_JOIN_RECORD_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let ids: BTreeSet<u64> = walls
        .iter()
        .filter_map(|wall| wall.id.map(u64::from))
        .collect();
    let Ok(joined) = crate::native_walls::wall_top_joins(rf, &ids) else {
        return;
    };
    for wall in walls.iter_mut() {
        if let Some(joined) = wall.id.and_then(|id| joined.get(&u64::from(id))) {
            wall.fields
                .push((WALL_TOP_JOINED_FIELD.into(), InstanceField::Bool(*joined)));
        }
    }
}

pub(super) fn attach_wall_layers(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &mut [DecodedElement],
) {
    use crate::partition_compound_structure as pcs;
    if !pcs::WALL_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
        || pcs::layer_layout(revit_version).is_none()
    {
        return;
    }
    // A 2023 wall keeps its record body, cut back at its joins (RE-120),
    // and takes only its layers and exterior side: its location line is not
    // read (RE-114).
    let centreline_bodies = !crate::partition_element_records_2023::is_32bit_release(revit_version);
    let type_of = |element: &DecodedElement| {
        element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        })
    };
    let wall_ids: BTreeSet<u32> = walls
        .iter()
        .filter(|wall| type_of(wall).is_some())
        .filter_map(|wall| wall.id)
        .collect();
    if wall_ids.is_empty() {
        return;
    }
    let types: BTreeSet<u32> = walls.iter().filter_map(type_of).collect();
    let declared: BTreeSet<u32> = match crate::elem_table::parse_records(rf) {
        Ok(records) => crate::elem_table::declared_ids(&records),
        Err(_) => return,
    };
    let Some((materials, names)) = materials_and_names(rf, revit_version, &declared) else {
        return;
    };
    let (Ok(layers), Ok(orientations), Ok(lines), Ok(arcs), Ok(appearances), Ok(face_angles)) = (
        pcs::scan_type_layers(rf, revit_version, &types, &materials, &declared),
        pcs::scan_wall_orientations(rf, revit_version, &wall_ids),
        pcs::scan_wall_lines(rf, revit_version, &wall_ids),
        pcs::scan_wall_arcs(rf, revit_version, &wall_ids),
        crate::partition_materials::scan_material_appearances(rf, revit_version, &declared),
        pcs::scan_wall_type_face_angles(rf, revit_version, &types),
    ) else {
        return;
    };
    // W6: each wall's native record, for the walls its ends meet, whether
    // its top is joined to a roof or floor (RE-228), and whether anything
    // joins to or cuts it (RE-266).
    let crate::native_walls::WallRecords {
        walls: native,
        top_joined,
        uncut,
        top_cutters,
        whole,
        bounded,
    } = if centreline_bodies {
        let ids: BTreeSet<u64> = wall_ids.iter().map(|&id| u64::from(id)).collect();
        crate::native_walls::native_walls_and_top_joins(rf, &ids).unwrap_or_default()
    } else {
        Default::default()
    };
    for wall in walls.iter_mut() {
        if let Some(joined) = wall.id.and_then(|id| top_joined.get(&u64::from(id))) {
            wall.fields
                .push((WALL_TOP_JOINED_FIELD.into(), InstanceField::Bool(*joined)));
        }
        if wall.id.is_some_and(|id| uncut.contains(&u64::from(id))) {
            wall.fields
                .push((WALL_UNCUT_FIELD.into(), InstanceField::Bool(true)));
        }
        // RE-267: the elements its top join and cut pairs name, an id that
        // does not fit an ElementId leaving the wall out.
        if let Some(named) = wall.id.and_then(|id| top_cutters.get(&u64::from(id))) {
            let items: Option<Vec<InstanceField>> = named
                .iter()
                .map(|&id| {
                    Some(InstanceField::ElementId {
                        tag: 0,
                        id: u32::try_from(id).ok()?,
                    })
                })
                .collect();
            if let Some(items) = items {
                wall.fields
                    .push((WALL_TOP_CUTTERS_FIELD.into(), InstanceField::Vector(items)));
            }
        }
        if wall.id.is_some_and(|id| whole.contains(&u64::from(id))) {
            wall.fields
                .push((WALL_WHOLE_FIELD.into(), InstanceField::Bool(true)));
        }
        if let Some(named) = wall.id.and_then(|id| bounded.get(&u64::from(id))) {
            let items: Option<Vec<InstanceField>> = named
                .iter()
                .map(|&id| {
                    Some(InstanceField::ElementId {
                        tag: 0,
                        id: u32::try_from(id).ok()?,
                    })
                })
                .collect();
            if let Some(items) = items {
                wall.fields
                    .push((WALL_BOUNDED_FIELD.into(), InstanceField::Vector(items)));
            }
        }
        let picked = wall.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(u64::from(*id)),
            _ => None,
        });
        let named = wall
            .id
            .and_then(|id| native.get(&u64::from(id)))
            .and_then(|native| native.type_id);
        if picked.is_some() && picked == named {
            wall.fields
                .push((WALL_TYPE_CONFIRMED_FIELD.into(), InstanceField::Bool(true)));
        }
    }
    // RE-91: a layer that takes its category's material takes the one the
    // document's object styles give walls, where it is set and named.
    let category_material = crate::partition_materials::scan_category_material(
        rf,
        revit_version,
        crate::partition_element_records::OST_WALLS,
    )
    .ok()
    .flatten()
    .filter(|id| names.contains_key(id));
    let layers: BTreeMap<u32, Vec<pcs::CompoundLayer>> = layers
        .into_iter()
        .map(|(type_id, type_layers)| {
            let type_layers = type_layers
                .into_iter()
                .map(|layer| pcs::CompoundLayer {
                    material: layer.material.or(category_material),
                    ..layer
                })
                .collect();
            (type_id, type_layers)
        })
        .collect();
    for wall in walls.iter_mut() {
        let (Some(id), Some(type_id)) = (wall.id, type_of(wall)) else {
            continue;
        };
        // W6: a wall its partition data gives no orientation takes its
        // native record's word and flip, which equal the partition data's on
        // every wall that has both.
        let orientation = orientations
            .get(&id)
            .map(|orientation| (orientation.word, orientation.flip))
            .or_else(|| {
                let wall = native.get(&u64::from(id))?;
                Some((u32::try_from(wall.cross_section?).ok()?, wall.flipped?))
            });
        let (Some(type_layers), Some((word, flip))) = (layers.get(&type_id), orientation) else {
            continue;
        };
        let thickness: f64 = type_layers.iter().map(|layer| layer.width_feet).sum();
        let measured = thickness.is_finite() && thickness > 0.0;
        // RE-86: a wall with word 2 whose type leans only its exterior face
        // is tapered; its line is still its centreline at the top. Only a
        // straight tapered wall is measured.
        let taper = face_angles
            .get(&type_id)
            .filter(|_| word == 2 && lines.contains_key(&id))
            .and_then(|&[first, exterior, third]| {
                (first == 0.0 && third == 0.0 && exterior > 0.0 && exterior < FRAC_PI_4)
                    .then_some(exterior)
            });
        let centred = centreline_bodies && measured && (word == 1 || taper.is_some());
        let float = |value: f64| InstanceField::Float { value, size: 8 };
        // The direction the exterior side is taken from: the line's, or the
        // arc's tangent at its middle (RE-75).
        let (dx, dy) = if let Some(line) = lines.get(&id) {
            let (start, end) = (line.start(), line.end());
            let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
            let length = dx.hypot(dy);
            if !length.is_finite() || length <= 1e-9 {
                continue;
            }
            if centred {
                for (names, point) in [(WALL_AXIS_START_FIELDS, start), (WALL_AXIS_END_FIELDS, end)]
                {
                    for (name, value) in names.iter().zip(point) {
                        wall.fields.push(((*name).into(), float(value)));
                    }
                }
            }
            (dx / length, dy / length)
        } else if let Some(arc) = arcs.get(&id) {
            if arc.x_axis[2].abs() > 1e-9 || arc.y_axis[2].abs() > 1e-9 {
                continue;
            }
            if centred {
                let values = [
                    arc.centre[0],
                    arc.centre[1],
                    arc.radius,
                    arc.start_angle,
                    arc.end_angle,
                    arc.x_axis[0],
                    arc.x_axis[1],
                    arc.y_axis[0],
                    arc.y_axis[1],
                ];
                for (name, value) in WALL_ARC_FIELDS.iter().zip(values) {
                    wall.fields.push(((*name).into(), float(value)));
                }
            }
            let (sin, cos) = ((arc.start_angle + arc.end_angle) * 0.5).sin_cos();
            let tangent = [
                -sin * arc.x_axis[0] + cos * arc.y_axis[0],
                -sin * arc.x_axis[1] + cos * arc.y_axis[1],
            ];
            let length = tangent[0].hypot(tangent[1]);
            if !length.is_finite() || length <= 1e-9 {
                continue;
            }
            (tangent[0] / length, tangent[1] / length)
        } else {
            continue;
        };
        if centred {
            wall.fields
                .push((WALL_TYPE_THICKNESS_FIELD.into(), float(thickness)));
            if let Some(angle) = taper {
                wall.fields
                    .push((WALL_EXTERIOR_FACE_ANGLE_FIELD.into(), float(angle)));
            }
        }
        let exterior = if flip { [dy, -dx] } else { [-dy, dx] };
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if items.is_empty()) {
            continue;
        }
        for (name, value) in WALL_EXTERIOR_FIELDS.iter().zip(exterior) {
            wall.fields
                .push(((*name).into(), InstanceField::Float { value, size: 8 }));
        }
        wall.fields.push((WALL_LAYERS_FIELD.into(), bands));
    }
    // RE-88: a wall its data does not place (no orientation, line or arc)
    // still carries its type's layers, for its materials. With no exterior
    // side they are not drawn.
    for wall in walls.iter_mut() {
        if wall
            .fields
            .iter()
            .any(|(name, _)| name == WALL_LAYERS_FIELD)
        {
            continue;
        }
        let Some(type_layers) = type_of(wall).and_then(|type_id| layers.get(&type_id)) else {
            continue;
        };
        let bands = layer_bands_field(type_layers, &appearances, &names);
        if matches!(&bands, InstanceField::Vector(items) if items.is_empty()) {
            continue;
        }
        wall.fields.push((WALL_LAYERS_FIELD.into(), bands));
    }
    if centreline_bodies {
        attach_wall_butt_joins(rf, revit_version, walls, &layers, &native);
        attach_native_tee_reaches(walls, &native);
        attach_wall_lengths(walls, &native);
    }
}

/// Recover wall instances from partition element records (#211), with
/// the join-trimmed body of RE-26.
pub fn walls_from_partition_category_records(
    rf: &mut RevitFile,
    revit_version: u32,
    level_ids: &BTreeSet<u32>,
) -> Result<Vec<DecodedElement>> {
    let Some(records) = category_records(
        rf,
        revit_version,
        crate::partition_element_records::OST_WALLS,
    )?
    else {
        return Ok(Vec::new());
    };
    Ok(wall_instances_from_records(records, level_ids))
}

/// Wall instance selection with the join-trimmed body attached
/// (RE-26).
///
/// The record box is the wall's untrimmed prism; Revit's exported
/// body is the same prism after the joins cut it back, and
/// [`crate::element_record_wall_joins`] recovers the cut from the
/// recovered wall set alone. A wall the solver declines keeps its
/// record box and says so in `m_wall_body_source`.
pub fn wall_instances_from_records(
    records: Vec<crate::partition_element_records::PartitionElementRecord>,
    level_ids: &BTreeSet<u32>,
) -> Vec<DecodedElement> {
    use crate::element_record_wall_joins as joins;

    let selected = select_instance_records(records);
    let boxes: Vec<crate::partition_element_records::PartitionElementRecord> =
        selected.values().cloned().collect();
    let trims = joins::join_trims(&boxes);
    selected
        .values()
        .map(|record| {
            let mut decoded = element_record_decoded(record, "Wall", level_ids);
            if let Some(trim) = trims.get(&record.element_id) {
                apply_wall_join_trim(&mut decoded, record, trim);
            }
            decoded
        })
        .collect()
}
