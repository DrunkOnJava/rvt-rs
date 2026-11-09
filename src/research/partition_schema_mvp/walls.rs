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
    /// Whether an angled wall's body must be the rectangle its box closes on
    /// (RE-364): on Revit 2015 to 2020 nothing marks a slanted wall.
    pub box_must_close: bool,
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
        box_must_close: fields.iter().any(|(name, value)| {
            name == WALL_AXIS_BOX_MUST_CLOSE_FIELD && matches!(value, InstanceField::Bool(true))
        }),
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

/// The ElementIds [`WALL_BASE_CUTTERS_FIELD`] records: what a wall's base join
/// names (#1384). Empty where it holds none.
pub fn wall_base_cutters_from_fields(fields: &[(String, InstanceField)]) -> Vec<u32> {
    fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == WALL_BASE_CUTTERS_FIELD => Some(
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

/// The walls joined at each end of a wall's location curve, as
/// [`WALL_CURVE_FIELD`] and [`WALL_END_JOINS_FIELDS`] record them (#1410).
#[derive(Debug, Clone, PartialEq)]
pub struct WallEndJoins {
    /// The curve's start and its end, plan model feet.
    pub curve: [[f64; 2]; 2],
    /// The ElementIds joined at the start and at the end, in the record's
    /// order; `None` where the record gives no list for that end.
    pub lists: [Option<Vec<u32>>; 2],
}

/// [`WallEndJoins`] of the wall whose fields these are; `None` where it has no
/// location curve field.
pub fn wall_end_joins_from_fields(fields: &[(String, InstanceField)]) -> Option<WallEndJoins> {
    let flat: Vec<f64> = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(items) if name == WALL_CURVE_FIELD => items
            .iter()
            .map(|item| match item {
                InstanceField::Float { value, .. } => Some(*value),
                _ => None,
            })
            .collect::<Option<Vec<f64>>>(),
        _ => None,
    })?;
    let [x0, y0, x1, y1] = flat.as_slice() else {
        return None;
    };
    let list = |slot: usize| -> Option<Vec<u32>> {
        let field: &str = WALL_END_JOINS_FIELDS.get(slot).copied()?;
        fields.iter().find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == field => items
                .iter()
                .map(|item| match item {
                    InstanceField::ElementId { id, .. } => Some(*id),
                    _ => None,
                })
                .collect::<Option<Vec<u32>>>(),
            _ => None,
        })
    };
    Some(WallEndJoins {
        curve: [[*x0, *y0], [*x1, *y1]],
        lists: [list(0), list(1)],
    })
}

/// The ElementIds [`WALL_TOP_PAIR_ONLY_FIELD`] records: those of a wall's top
/// cutters that only a cut pair names (RE-320). Empty where it holds none.
pub fn wall_top_pair_only_from_fields(fields: &[(String, InstanceField)]) -> Vec<u32> {
    fields
        .iter()
        .find_map(|(name, value)| match value {
            InstanceField::Vector(items) if name == WALL_TOP_PAIR_ONLY_FIELD => Some(
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

/// Releases a wall's profile openings are measured on: Core Interior's wall
/// 55840 (Revit 2024). Its own list, so that a release the sketch and beam
/// lines are read on does not take openings that are not measured there.
const WALL_PROFILE_OPENING_REVIT_VERSIONS: &[u32] = &[2024];

/// Releases the order of a wall's end joins is measured on: the BIM4LCA office
/// (Revit 2024), whose Basement walls 2057668, 2057670, 2058499 and 2058501 are
/// the four walls it rests on (RE-348). Its own list, a decision of its own
/// (#1410): the profile openings the rule asks for are read on
/// [`WALL_PROFILE_OPENING_REVIT_VERSIONS`], and a release added to that list
/// takes the rule only when it is added here.
const WALL_END_JOIN_REVIT_VERSIONS: &[u32] = &[2024];

/// Releases a wall is drawn along its location line and cut by its joins: 2024
/// to 2027, and Revit 2015 to 2023 against the same Autodesk samples saved by
/// Revit 2024. None of 2015 to 2023 is scored against a Revit export here.
const WALL_CENTRELINE_REVIT_VERSIONS: std::ops::RangeInclusive<u32> = 2015..=2027;

/// Releases the cut-out of a wall's elevation profile is carried on to the
/// wall's end on: the BIM4LCA office (Revit 2024), whose Basement walls
/// 2057668, 2057670, 2058499, 2058501, 2056705, 2056770, 2070689 and 2070691
/// are the walls it rests on (RE-358). Its own list, a decision of its own
/// (#1473): the profile openings are read on
/// [`WALL_PROFILE_OPENING_REVIT_VERSIONS`], and a release added to that list
/// takes the rule only when it is added here. The walls the record joins at an
/// end are read on [`WALL_END_JOIN_REVIT_VERSIONS`], which the rule asks for.
const WALL_PROFILE_STUB_REVIT_VERSIONS: &[u32] = &[2024];

/// Releases a wall's top is taken at the highest point of its edited elevation
/// profile: the BIM4LCA office's terrain files (Revit 2024), whose wall 1387731
/// is the wall it rests on (RE-378). Its own list, a decision of its own
/// (#1600). The rule asks for the profile's extent ([`PROFILE_EXTENT_FIELD`]),
/// which is attached on [`WALL_PROFILE_STUB_REVIT_VERSIONS`], so a release
/// added here is added there too.
const WALL_PROFILE_TOP_REVIT_VERSIONS: &[u32] = &[2024];

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
    if wall_ids.is_empty() || !WALL_PROFILE_OPENING_REVIT_VERSIONS.contains(&revit_version) {
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
        let cuts = elevation_profile_cuts(&segments);
        if !cuts.is_empty() && WALL_PROFILE_STUB_REVIT_VERSIONS.contains(&revit_version) {
            if let Some((low, high)) = profile_extent(&segments) {
                wall.fields.push((
                    PROFILE_EXTENT_FIELD.into(),
                    InstanceField::Vector(vec![point_field(&low), point_field(&high)]),
                ));
            }
        }
        if !cuts.is_empty() && WALL_PROFILE_TOP_REVIT_VERSIONS.contains(&revit_version) {
            if let Some((low, high)) = profile_heights(&segments) {
                wall.fields.push((
                    PROFILE_HEIGHT_FIELD.into(),
                    InstanceField::Vector(vec![
                        InstanceField::Float {
                            value: low,
                            size: 8,
                        },
                        InstanceField::Float {
                            value: high,
                            size: 8,
                        },
                    ]),
                ));
            }
        }
        // #1607: the pockets of the profile against its hull, which are what
        // Revit's export writes as the wall's openings, beside the regions.
        let pockets = if cuts.is_empty() {
            Vec::new()
        } else {
            elevation_profile_pockets(&segments)
        };
        for (field, found) in [
            (PROFILE_OPENING_FIELD, cuts),
            (PROFILE_POCKET_FIELD, pockets),
        ] {
            for (k, (tag, outline)) in found.into_iter().enumerate() {
                let points = outline.iter().map(point_field).collect();
                wall.fields
                    .push((format!("{field}_{k}"), InstanceField::Vector(points)));
                wall.fields.push((
                    format!("{field}_{k}_tag"),
                    InstanceField::ElementId { tag: 0, id: tag },
                ));
            }
        }
    }
}

/// A model-feet point as a vector of three floats, as the openings' outlines
/// and [`PROFILE_EXTENT_FIELD`] hold them.
fn point_field(point: &[f64; 3]) -> InstanceField {
    InstanceField::Vector(
        point
            .iter()
            .map(|value| InstanceField::Float {
                value: *value,
                size: 8,
            })
            .collect(),
    )
}

/// The lowest and highest height reached by an elevation profile's sketch
/// lines, model feet. `None` where there are no lines or a height is not finite.
// Tracker: B55.
fn profile_heights(segments: &[SketchSegment]) -> Option<(f64, f64)> {
    let mut heights = segments.iter().flat_map(|(_, a, b)| [a[2], b[2]]);
    let first = heights.next()?;
    let (low, high) = heights.fold((first, first), |(low, high), z| (low.min(z), high.max(z)));
    (low.is_finite() && high.is_finite()).then_some((low, high))
}

/// The lowest and highest height of the elevation profile a wall's openings
/// are cut from ([`PROFILE_HEIGHT_FIELD`]), model feet. `None` for a wall that
/// has none.
// Tracker: B55.
pub fn profile_height_from_fields(fields: &[(String, InstanceField)]) -> Option<(f64, f64)> {
    let values = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(values) if name == PROFILE_HEIGHT_FIELD => Some(values),
        _ => None,
    })?;
    match values.as_slice() {
        [
            InstanceField::Float { value: low, .. },
            InstanceField::Float { value: high, .. },
        ] => Some((*low, *high)),
        _ => None,
    }
}

/// The lowest and highest points of an elevation profile's sketch lines along
/// the profile's axis (the direction of the two points furthest apart in
/// plan, as [`elevation_profile_cuts`] takes it), model feet. `None` where
/// there are no lines or they never leave one point in plan.
// Tracker: B55.
fn profile_extent(segments: &[SketchSegment]) -> Option<([f64; 3], [f64; 3])> {
    let points: Vec<[f64; 3]> = segments.iter().flat_map(|(_, a, b)| [*a, *b]).collect();
    let first = *points.first()?;
    let mut far = (0.0_f64, first, first);
    for a in &points {
        for b in &points {
            let distance = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
            if distance > far.0 {
                far = (distance, *a, *b);
            }
        }
    }
    let (length, origin, toward) = far;
    if length <= PROFILE_EPS_FEET {
        return None;
    }
    let axis = [
        (toward[0] - origin[0]) / length,
        (toward[1] - origin[1]) / length,
    ];
    let along =
        |point: &[f64; 3]| (point[0] - origin[0]) * axis[0] + (point[1] - origin[1]) * axis[1];
    let low = points.iter().min_by(|a, b| along(a).total_cmp(&along(b)))?;
    let high = points.iter().max_by(|a, b| along(a).total_cmp(&along(b)))?;
    Some((*low, *high))
}

/// The two ends of the elevation profile a wall's openings are cut from
/// ([`PROFILE_EXTENT_FIELD`]), model feet: the lowest and highest along the
/// profile's axis. `None` for a wall that has none.
// Tracker: B55.
pub fn profile_extent_from_fields(
    fields: &[(String, InstanceField)],
) -> Option<([f64; 3], [f64; 3])> {
    let points = fields.iter().find_map(|(name, value)| match value {
        InstanceField::Vector(points) if name == PROFILE_EXTENT_FIELD => Some(points),
        _ => None,
    })?;
    let point = |field: &InstanceField| -> Option<[f64; 3]> {
        let InstanceField::Vector(values) = field else {
            return None;
        };
        let mut xyz = values.iter().map(|value| match value {
            InstanceField::Float { value, .. } => Some(*value),
            _ => None,
        });
        Some([xyz.next()??, xyz.next()??, xyz.next()??])
    };
    match points.as_slice() {
        [low, high] => Some((point(low)?, point(high)?)),
        _ => None,
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
    let Some(ProfileLoop {
        origin,
        axis,
        corners,
        ids,
    }) = profile_loop(segments)
    else {
        return Vec::new();
    };
    let count = corners.len();
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

/// A wall's elevation profile as one loop in its own plane (see
/// [`elevation_profile_cuts`]).
struct ProfileLoop {
    /// Where the plane's axis starts, model feet.
    origin: [f64; 3],
    /// The plane's axis, a unit vector in plan.
    axis: [f64; 2],
    /// The loop's corners along the axis and up, counter-clockwise.
    corners: Vec<(f64, f64)>,
    /// The ElementId of the line leaving each corner.
    ids: Vec<u32>,
}

/// The lines of an elevation profile (ElementId, start, end, model feet) as one
/// loop in the vertical plane they lie in. `None` where they do not lie in one
/// plane or do not close one loop.
// Tracker: B55.
fn profile_loop(segments: &[SketchSegment]) -> Option<ProfileLoop> {
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
        return None;
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
        return None;
    }
    let edges: Vec<PlaneEdge> = segments
        .iter()
        .map(|(id, a, b)| (*id, to_uv(*a).0, to_uv(*b).0))
        .collect();
    let meets = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() <= eps && (a.1 - b.1).abs() <= eps;
    // Chain the lines into one loop: its corners, and the line leaving each.
    let first = edges.first()?;
    let mut corners = vec![first.1];
    let mut ids = vec![first.0];
    let mut end = first.2;
    let mut used = vec![false; edges.len()];
    used[0] = true;
    for _ in 1..edges.len() {
        let (index, next) = edges.iter().enumerate().find_map(|(index, (_, a, b))| {
            if used[index] {
                None
            } else if meets(*a, end) {
                Some((index, *b))
            } else if meets(*b, end) {
                Some((index, *a))
            } else {
                None
            }
        })?;
        used[index] = true;
        corners.push(end);
        ids.push(edges[index].0);
        end = next;
    }
    if !meets(end, corners[0]) {
        return None;
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
    Some(ProfileLoop {
        origin,
        axis,
        corners,
        ids,
    })
}

/// Which of a loop's `corners` lie on the boundary of its convex hull
/// (Andrew's monotone chain): a corner within the sketch's tolerance of a hull
/// edge does, so that a notch cut in a side is a stretch between its two
/// shoulders and not between the side's ends.
fn hull_corners(corners: &[(f64, f64)]) -> Vec<bool> {
    let mut order: Vec<usize> = (0..corners.len()).collect();
    order.sort_by(|a, b| {
        corners[*a]
            .0
            .total_cmp(&corners[*b].0)
            .then(corners[*a].1.total_cmp(&corners[*b].1))
    });
    let cross = |o: usize, a: usize, b: usize| {
        let (o, a, b) = (corners[o], corners[a], corners[b]);
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<usize> = Vec::new();
    for &point in &order {
        while hull.len() >= 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0 {
            hull.pop();
        }
        hull.push(point);
    }
    let lower = hull.len() + 1;
    for &point in order.iter().rev().skip(1) {
        while hull.len() >= lower && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0
        {
            hull.pop();
        }
        hull.push(point);
    }
    hull.pop();
    let edges: Vec<((f64, f64), (f64, f64))> = hull
        .iter()
        .enumerate()
        .map(|(k, index)| (corners[*index], corners[hull[(k + 1) % hull.len()]]))
        .collect();
    corners
        .iter()
        .map(|point| {
            edges
                .iter()
                .any(|(a, b)| distance_to_segment(*point, *a, *b) <= PROFILE_EPS_FEET)
        })
        .collect()
}

/// How far a point stands from the segment `a` to `b`, feet.
fn distance_to_segment(point: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let squared = dx * dx + dy * dy;
    let along = if squared > 0.0 {
        (((point.0 - a.0) * dx + (point.1 - a.1) * dy) / squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point.0 - (a.0 + along * dx)).hypot(point.1 - (a.1 + along * dy))
}

/// The pockets of an edited elevation profile against its convex hull: each
/// stretch of the profile's loop between two corners of the hull that the
/// hull's edge cuts off, as model-feet points in the profile's plane,
/// counter-clockwise in its own along-and-up coordinates, tagged with the
/// ElementId of the first line of the stretch in that walk. These are the
/// openings Revit's export writes for a wall's profile (#1607, RE-380), and a
/// profile whose region is convex, as a straight sloped top, has none, as
/// Revit's export writes none for the terrain wall 1351524 and the office's
/// eight.
/// A stretch no further than the sketch's tolerance from its edge is not one.
///
/// What closes the stretch depends on where the hull's edge ends, against the
/// loop's bounding rectangle, and only the ends measured are read:
///
/// - an end off the rectangle's edge: the hull's edge closes the stretch, as
///   on the terrain wall 1387731 (Tag 1389018), whose first three lines of its
///   top are closed by the edge back to the first;
/// - the ends on a side of the rectangle in common, as a notch cut in it: the
///   hull's edge runs along the side, so it closes the stretch;
/// - the ends on two adjacent sides, neither at a corner: the rectangle's
///   corner between them closes it, as on Core Interior's wall 55840 (Tag
///   55859), whose polygon takes in the corner the hull's edge cuts off, 301.65
///   ft2 where the stretch and the edge hold 71.23.
///
/// A stretch whose hull edge ends otherwise (at a corner of the rectangle with
/// the other end on a side it does not share) is not read: no wall of the five
/// Revit 2024 files with Revit's export has one. Empty where the lines do not
/// lie in one vertical plane or do not close one loop.
// Tracker: B55.
pub fn elevation_profile_pockets(segments: &[SketchSegment]) -> Vec<(u32, Vec<[f64; 3]>)> {
    let Some(ProfileLoop {
        origin,
        axis,
        corners,
        ids,
    }) = profile_loop(segments)
    else {
        return Vec::new();
    };
    let count = corners.len();
    let (u_min, u_max, v_min, v_max) = corners.iter().fold(
        (f64::MAX, f64::MIN, f64::MAX, f64::MIN),
        |(a, b, c, d), (u, v)| (a.min(*u), b.max(*u), c.min(*v), d.max(*v)),
    );
    // The sides of the loop's bounding rectangle a point lies on: left, right,
    // bottom, top.
    let sides = |(u, v): (f64, f64)| {
        [
            (u - u_min).abs() <= PROFILE_EPS_FEET,
            (u - u_max).abs() <= PROFILE_EPS_FEET,
            (v - v_min).abs() <= PROFILE_EPS_FEET,
            (v - v_max).abs() <= PROFILE_EPS_FEET,
        ]
    };
    let touching = |on: [bool; 4]| on.iter().filter(|side| **side).count();
    let on_hull = hull_corners(&corners);
    let at: Vec<usize> = (0..count).filter(|i| on_hull[*i]).collect();
    let mut pockets = Vec::new();
    for (k, &from) in at.iter().enumerate() {
        let to = at[(k + 1) % at.len()];
        let lines = (to + count - from) % count;
        if lines < 2 {
            continue;
        }
        let mut outline: Vec<(f64, f64)> = (0..=lines)
            .map(|step| corners[(from + step) % count])
            .collect();
        let (a, b) = (corners[from], corners[to]);
        let chord = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
        if chord <= PROFILE_EPS_FEET {
            continue;
        }
        // Which side of the hull's edge each point stands on, and how far.
        let across =
            |p: (f64, f64)| ((b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)) / chord;
        // The stretch's farthest corner from the edge, signed.
        let farthest = outline[1..lines]
            .iter()
            .map(|p| across(*p))
            .fold(
                0.0_f64,
                |most, side| {
                    if side.abs() > most.abs() { side } else { most }
                },
            );
        if farthest.abs() <= PROFILE_EPS_FEET {
            continue;
        }
        let (on_a, on_b) = (sides(a), sides(b));
        let (touch_a, touch_b) = (touching(on_a), touching(on_b));
        // Both ends on the rectangle's edge, and on no side in common (on a side
        // in common the hull's edge runs along it and closes the stretch): read
        // for two adjacent sides, neither end at a corner, and not otherwise.
        if touch_a > 0 && touch_b > 0 && !(0..4).any(|side| on_a[side] && on_b[side]) {
            let (vertical_a, vertical_b) = (on_a[0] || on_a[1], on_b[0] || on_b[1]);
            if touch_a != 1 || touch_b != 1 || vertical_a == vertical_b {
                continue;
            }
            let (vertical, horizontal) = if vertical_a {
                (on_a, on_b)
            } else {
                (on_b, on_a)
            };
            let corner = (
                if vertical[0] { u_min } else { u_max },
                if horizontal[2] { v_min } else { v_max },
            );
            // The corner is what the hull's edge cuts off, so it stands on the
            // other side of the edge from the stretch.
            if across(corner) * farthest >= 0.0 {
                continue;
            }
            outline.push(corner);
        }
        let area: f64 = outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .map(|(p, q)| p.0 * q.1 - q.0 * p.1)
            .sum();
        if area < 0.0 {
            outline.reverse();
        }
        // Tag: the line the stretch starts with. Revit's export names it on 12 of
        // the 15 pockets measured (Core Interior's, the terrain wall's and 10 of
        // Snowdon Towers' 13); the highest id of the stretch is right on 6, the
        // lowest on 8, and the last line is Revit's on the 3 of Snowdon's wall
        // 698364, which nothing read explains (RE-380 section 5).
        let tag = ids[from % count];
        pockets.push((
            tag,
            outline
                .into_iter()
                .map(|(u, v)| [origin[0] + axis[0] * u, origin[1] + axis[1] * u, v])
                .collect(),
        ));
    }
    pockets
}

/// The openings a wall's edited elevation profile cuts, as the partition MVP
/// gives them ([`PROFILE_OPENING_FIELD`]): each one's tag and outline.
// Tracker: B55.
pub fn profile_openings_from_fields(
    fields: &[(String, InstanceField)],
) -> Vec<(u32, Vec<[f64; 3]>)> {
    indexed_outlines(fields, PROFILE_OPENING_FIELD)
}

/// The pockets a wall's edited elevation profile has against its hull, as the
/// partition MVP gives them ([`PROFILE_POCKET_FIELD`]): each one's tag and
/// outline (#1607, RE-380).
// Tracker: B55.
pub fn profile_pockets_from_fields(
    fields: &[(String, InstanceField)],
) -> Vec<(u32, Vec<[f64; 3]>)> {
    indexed_outlines(fields, PROFILE_POCKET_FIELD)
}

/// The tagged outlines `<prefix>_<k>` and `<prefix>_<k>_tag` of a wall's fields,
/// for `k` from 0 on while there is one.
fn indexed_outlines(fields: &[(String, InstanceField)], prefix: &str) -> Vec<(u32, Vec<[f64; 3]>)> {
    (0..)
        .map_while(|k| {
            let outline_name = format!("{prefix}_{k}");
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

/// Read in one pass what [`attach_wall_type_functions`] and
/// [`attach_wall_top_joins`] take from `walls`' native records: the type ids
/// of the walls with no type Function yet and whether each wall's top is
/// joined, on the releases each is measured on. The records are the same, so
/// two passes over them would decode each wall twice (#1237). An error is
/// left to those two to meet again.
pub(super) fn read_wall_records(
    rf: &mut RevitFile,
    revit_version: u32,
    walls: &[DecodedElement],
    reads: &mut crate::native_walls::WallReads,
) {
    let mut type_walls: BTreeSet<u64> = BTreeSet::new();
    let mut join_walls: BTreeSet<u64> = BTreeSet::new();
    for wall in walls {
        let Some(id) = wall.id.map(u64::from) else {
            continue;
        };
        if WALL_TYPE_FUNCTION_REVIT_VERSIONS.contains(&revit_version) && !has_type_function(wall) {
            type_walls.insert(id);
        }
        if WALL_TOP_JOIN_RECORD_REVIT_VERSIONS.contains(&revit_version) {
            join_walls.insert(id);
        }
    }
    let _ = reads.read(rf, &type_walls, &join_walls);
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
    reads: &mut crate::native_walls::WallReads,
) {
    if !WALL_TOP_JOIN_RECORD_REVIT_VERSIONS.contains(&revit_version) {
        return;
    }
    let ids: BTreeSet<u64> = walls
        .iter()
        .filter_map(|wall| wall.id.map(u64::from))
        .collect();
    let Ok(joined) = reads.top_joins(rf, &ids) else {
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
    reads: &mut crate::native_walls::WallReads,
) {
    use crate::partition_compound_structure as pcs;
    if !pcs::WALL_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
        || pcs::layer_layout(revit_version).is_none()
    {
        return;
    }
    // A wall before 2015 keeps its record body, cut back at its joins (RE-120),
    // and takes only its layers and exterior side.
    let centreline_bodies = WALL_CENTRELINE_REVIT_VERSIONS.contains(&revit_version);
    let narrow = crate::partition_element_records_2023::is_32bit_release(revit_version);
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
        base_cutters,
        pair_only,
        whole,
        bounded,
        end_joins,
    } = if centreline_bodies {
        let ids: BTreeSet<u64> = wall_ids.iter().map(|&id| u64::from(id)).collect();
        crate::native_walls::native_walls_and_top_joins_into(rf, &ids, reads).unwrap_or_default()
    } else {
        Default::default()
    };
    for wall in walls.iter_mut() {
        // RE-347: a straight wall's native line and flip, so a door or
        // window set in it is one RE-343's Rule 1 measures.
        let line = wall
            .id
            .and_then(|id| native.get(&u64::from(id)))
            .filter(|record| record.flipped.is_some())
            .map(|record| {
                (
                    record.end[0] - record.start[0],
                    record.end[1] - record.start[1],
                )
            });
        if let Some((dx, dy)) = line {
            let length = dx.hypot(dy);
            if length.is_finite() && length > 1e-9 {
                for (name, value) in WALL_NATIVE_DIRECTION_FIELDS
                    .iter()
                    .zip([dx / length, dy / length])
                {
                    wall.fields
                        .push(((*name).into(), InstanceField::Float { value, size: 8 }));
                }
            }
        }
        if let Some(joined) = wall.id.and_then(|id| top_joined.get(&u64::from(id))) {
            wall.fields
                .push((WALL_TOP_JOINED_FIELD.into(), InstanceField::Bool(*joined)));
        }
        // RE-364 section 9: on 2015 to 2023 a record's control joins read
        // empty, but its steps do not, and every wall of Autodesk's samples
        // holds the step classes, so the uncut flag, of its later save. Its
        // Height is that save's on every wall both saves hold.
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
        // #1384: the elements its base join names.
        if let Some(named) = wall.id.and_then(|id| base_cutters.get(&u64::from(id))) {
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
                    .push((WALL_BASE_CUTTERS_FIELD.into(), InstanceField::Vector(items)));
            }
        }
        // #1410: the walls joined at each end of its location curve, in the
        // order its record lists them. An id that does not fit an ElementId
        // leaves that end out.
        let joins = wall
            .id
            .filter(|_| WALL_END_JOIN_REVIT_VERSIONS.contains(&revit_version))
            .and_then(|id| end_joins.get(&u64::from(id)));
        if let Some(joins) = joins {
            let curve: Vec<InstanceField> = joins
                .curve
                .iter()
                .flatten()
                .map(|&value| InstanceField::Float { value, size: 8 })
                .collect();
            wall.fields
                .push((WALL_CURVE_FIELD.into(), InstanceField::Vector(curve)));
            for (name, listed) in WALL_END_JOINS_FIELDS.iter().zip(&joins.real) {
                let items: Option<Vec<InstanceField>> = listed.as_ref().and_then(|ids| {
                    ids.iter()
                        .map(|&id| {
                            Some(InstanceField::ElementId {
                                tag: 0,
                                id: u32::try_from(id).ok()?,
                            })
                        })
                        .collect()
                });
                if let Some(items) = items {
                    wall.fields
                        .push(((*name).into(), InstanceField::Vector(items)));
                }
            }
        }
        if let Some(only) = wall.id.and_then(|id| pair_only.get(&u64::from(id))) {
            let items: Option<Vec<InstanceField>> = only
                .iter()
                .map(|&id| {
                    Some(InstanceField::ElementId {
                        tag: 0,
                        id: u32::try_from(id).ok()?,
                    })
                })
                .collect();
            if let Some(items) = items {
                wall.fields.push((
                    WALL_TOP_PAIR_ONLY_FIELD.into(),
                    InstanceField::Vector(items),
                ));
            }
        }
        // RE-364 section 9: the whole flag also reads as on the later save,
        // but that save draws 25 of these walls from their centrelines and
        // writes them no areas. Here they keep their record boxes, so the
        // areas and volumes would be the boxes', and on all 203 of their old
        // saves the box's volume differs from the later body's.
        if !narrow && wall.id.is_some_and(|id| whole.contains(&u64::from(id))) {
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
        // RE-180, RE-181: Revit 2015 to 2020 write no cross-section word, so
        // nothing marks a slanted wall and each is drawn vertical.
        let vertical = word == 1 || !pcs::wall_orientation_has_word(revit_version);
        // RE-364: on 2015 to 2023 a wall along a model axis keeps its record
        // box cut back at its joins, whose ends Revit's own export of a 2018
        // project confirms where the line's overrun them: its control joins,
        // which tell a butt end, are not read there.
        let along_a_model_axis = lines.get(&id).is_some_and(|line| {
            let (start, end) = (line.start(), line.end());
            let (dx, dy) = (end[0] - start[0], end[1] - start[1]);
            let length = dx.hypot(dy);
            (dx / length).abs() <= 1e-6 || (dy / length).abs() <= 1e-6
        });
        let centred = centreline_bodies
            && measured
            && (vertical || taper.is_some())
            && !(narrow && along_a_model_axis);
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
                if !pcs::wall_orientation_has_word(revit_version) {
                    wall.fields.push((
                        WALL_AXIS_BOX_MUST_CLOSE_FIELD.into(),
                        InstanceField::Bool(true),
                    ));
                }
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
                rf.attribute_value_source(type_id, "type_face_angles", id, "wall_type_face_angle");
            }
            rf.attribute_value_source(type_id, "type_layers", id, "wall_type_layers");
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
        // The thickness and the layers it is drawn in are the type's (#1368).
        rf.attribute_value_source(type_id, "type_layers", id, "wall_type_layers");
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
        if let (Some(id), Some(type_id)) = (wall.id, type_of(wall)) {
            rf.attribute_value_source(type_id, "type_layers", id, "wall_type_layers");
        }
    }
    if centreline_bodies {
        attach_wall_butt_joins(rf, revit_version, walls, &layers, &native);
        attach_native_tee_reaches(walls, &native);
        attach_wall_corner_reaches(walls, &layers, &native);
        // A 2015 to 2023 record is read without its control joins (RE-364), and
        // the rule takes a wall with none listed as joined to nothing, so its
        // `Length` is left out.
        if !crate::partition_element_records_2023::is_32bit_release(revit_version) {
            attach_wall_lengths(walls, &native);
        }
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

#[cfg(test)]
mod pocket_tests {
    use super::*;

    /// The BIM4LCA office's terrain wall 1387731 (Revit 2024): the sketch lines
    /// of its profile, whose one opening Revit's export tags 1389018.
    fn terrain_wall() -> Vec<SketchSegment> {
        vec![
            (
                1388545,
                [122.4038, 121.5711, 196.5223],
                [122.4038, 121.5711, 182.0866],
            ),
            (
                1388546,
                [122.4038, 121.5711, 182.0866],
                [296.2603, 121.5718, 182.0866],
            ),
            (
                1388547,
                [296.2603, 121.5718, 182.0866],
                [296.2603, 121.5718, 183.0898],
            ),
            (
                1388558,
                [122.4038, 121.5711, 196.5223],
                [138.8080, 121.5711, 195.6988],
            ),
            (
                1388568,
                [138.8080, 121.5711, 195.6988],
                [171.6386, 121.5713, 192.4075],
            ),
            (
                1389018,
                [171.6386, 121.5713, 192.4075],
                [215.9444, 121.5714, 192.4075],
            ),
            (
                1389150,
                [215.9444, 121.5714, 192.4075],
                [296.2603, 121.5718, 183.0898],
            ),
        ]
    }

    /// The terrain wall 1351524, whose slope steepens to the right, so that its
    /// profile is convex and Revit's export writes it no opening.
    fn convex_wall() -> Vec<SketchSegment> {
        vec![
            (
                1,
                [122.4038, 99.6879, 193.2415],
                [122.4038, 99.6879, 177.1654],
            ),
            (
                2,
                [122.4038, 99.6879, 177.1654],
                [242.2201, 99.6879, 177.1654],
            ),
            (
                3,
                [242.2201, 99.6879, 177.1654],
                [242.2201, 99.6879, 182.1176],
            ),
            (
                4,
                [242.2201, 99.6879, 182.1176],
                [138.8080, 99.6879, 192.4255],
            ),
            (
                5,
                [138.8080, 99.6879, 192.4255],
                [122.4038, 99.6879, 193.2415],
            ),
        ]
    }

    /// Core Interior's wall 55840 (Revit 2024), along y, whose one opening
    /// Revit's export tags 55859.
    fn core_interior_wall() -> Vec<SketchSegment> {
        vec![
            (55847, [43.5, 81.0, 76.0], [43.5, 81.0, 56.4167]),
            (55848, [43.5, 81.0, 56.4167], [43.5, 51.0833, 56.4167]),
            (55849, [43.5, 51.0833, 56.4167], [43.5, 51.0833, 58.5]),
            (55850, [43.5, 77.4167, 76.0], [43.5, 81.0, 76.0]),
            (55853, [43.5, 77.4167, 76.0], [43.5, 77.4167, 69.1667]),
            (55854, [43.5, 77.4167, 69.1667], [43.5, 71.4167, 69.1667]),
            (55855, [43.5, 60.5, 63.0833], [43.5, 60.5, 61.8333]),
            (55856, [43.5, 60.5, 61.8333], [43.5, 56.9167, 61.8333]),
            (55857, [43.5, 71.4167, 69.1667], [43.5, 60.5, 63.0833]),
            (55858, [43.5, 56.9167, 61.8333], [43.5, 56.9167, 58.5]),
            (55859, [43.5, 56.9167, 58.5], [43.5, 51.0833, 58.5]),
        ]
    }

    /// Two coordinates of each point, hundredths of a foot, sorted.
    fn corners(outline: &[[f64; 3]], first: usize, second: usize) -> Vec<(i64, i64)> {
        let mut found: Vec<(i64, i64)> = outline
            .iter()
            .map(|p| {
                (
                    (p[first] * 100.0).round() as i64,
                    (p[second] * 100.0).round() as i64,
                )
            })
            .collect();
        found.sort_unstable();
        found
    }

    #[test]
    fn the_terrain_wall_has_the_pocket_revit_writes() {
        let pockets = elevation_profile_pockets(&terrain_wall());
        assert_eq!(pockets.len(), 1);
        let (tag, outline) = &pockets[0];
        // The line its stretch starts with, 1389018 (also the highest id of its
        // three lines), and not the last line of the sketch's top, 1389150,
        // which the hull's edge runs along.
        assert_eq!(*tag, 1389018);
        assert_eq!(
            corners(outline, 0, 2),
            vec![
                (12240, 19652),
                (13881, 19570),
                (17164, 19241),
                (21594, 19241)
            ]
        );
        // Counter-clockwise along the wall's axis and up.
        let area: f64 = outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .map(|(a, b)| a[0] * b[2] - b[0] * a[2])
            .sum();
        assert!(area > 0.0);
    }

    #[test]
    fn a_convex_profile_has_no_pocket() {
        assert!(elevation_profile_pockets(&convex_wall()).is_empty());
    }

    #[test]
    fn core_interiors_wall_has_the_pocket_revit_writes() {
        let pockets = elevation_profile_pockets(&core_interior_wall());
        assert_eq!(pockets.len(), 1);
        let (tag, outline) = &pockets[0];
        assert_eq!(*tag, 55859);
        // The stretch's eight corners and the rectangle's corner the hull's edge
        // cuts off, (51.08, 76): Revit's polygon has nine, and 301.65 ft2.
        assert_eq!(
            corners(outline, 1, 2),
            vec![
                (5108, 5850),
                (5108, 7600),
                (5692, 5850),
                (5692, 6183),
                (6050, 6183),
                (6050, 6308),
                (7142, 6917),
                (7742, 6917),
                (7742, 7600),
            ]
        );
        // The wall runs toward -y, so counter-clockwise along the wall's axis
        // and up is clockwise in (y, z).
        let area: f64 = outline
            .iter()
            .zip(outline.iter().cycle().skip(1))
            .map(|(a, b)| a[1] * b[2] - b[1] * a[2])
            .sum();
        assert!(
            (area.abs() / 2.0 - 301.65).abs() < 0.01,
            "area {}",
            area / 2.0
        );
    }

    /// A 10 by 8 ft wall with a notch 3 ft wide and 2 ft deep cut in its top.
    fn notched_wall() -> Vec<SketchSegment> {
        vec![
            (1, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            (2, [10.0, 0.0, 0.0], [10.0, 0.0, 8.0]),
            (3, [10.0, 0.0, 8.0], [6.0, 0.0, 8.0]),
            (4, [6.0, 0.0, 8.0], [6.0, 0.0, 6.0]),
            (5, [6.0, 0.0, 6.0], [3.0, 0.0, 6.0]),
            (6, [3.0, 0.0, 6.0], [3.0, 0.0, 8.0]),
            (7, [3.0, 0.0, 8.0], [0.0, 0.0, 8.0]),
            (8, [0.0, 0.0, 8.0], [0.0, 0.0, 0.0]),
        ]
    }

    #[test]
    fn a_notch_in_one_side_is_closed_by_the_hulls_edge_along_it() {
        let pockets = elevation_profile_pockets(&notched_wall());
        assert_eq!(pockets.len(), 1);
        let (tag, outline) = &pockets[0];
        // The line leaving the shoulder the stretch starts at, not the highest id.
        assert_eq!(*tag, 4);
        assert_eq!(
            corners(outline, 0, 2),
            vec![(300, 600), (300, 800), (600, 600), (600, 800)]
        );
    }

    #[test]
    fn a_hull_edge_from_a_corner_to_a_side_is_not_read() {
        // A dent in a top that slopes from the left corner (0, 8) down to the
        // right side (10, 6): the hull's edge runs from a corner of the
        // rectangle to a side, which no measured wall has.
        let dented = vec![
            (1, [0.0, 0.0, 8.0], [0.0, 0.0, 0.0]),
            (2, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            (3, [10.0, 0.0, 0.0], [10.0, 0.0, 6.0]),
            (4, [10.0, 0.0, 6.0], [7.0, 0.0, 4.0]),
            (5, [7.0, 0.0, 4.0], [3.0, 0.0, 5.0]),
            (6, [3.0, 0.0, 5.0], [0.0, 0.0, 8.0]),
        ];
        assert!(elevation_profile_pockets(&dented).is_empty());
    }

    #[test]
    fn a_rectangle_has_no_pocket() {
        let rectangle = vec![
            (1, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            (2, [10.0, 0.0, 0.0], [10.0, 0.0, 8.0]),
            (3, [10.0, 0.0, 8.0], [0.0, 0.0, 8.0]),
            (4, [0.0, 0.0, 8.0], [0.0, 0.0, 0.0]),
        ];
        assert!(elevation_profile_pockets(&rectangle).is_empty());
    }

    #[test]
    fn lines_that_do_not_close_a_loop_give_none() {
        let mut open = terrain_wall();
        open.remove(3);
        assert!(elevation_profile_pockets(&open).is_empty());
    }

    #[test]
    fn lines_off_one_plane_give_none() {
        let mut bent = terrain_wall();
        bent[5].1[1] += 1.0;
        bent[5].2[1] += 1.0;
        assert!(elevation_profile_pockets(&bent).is_empty());
    }

    #[test]
    fn the_same_lines_listed_backwards_give_the_same_pocket() {
        let backwards: Vec<SketchSegment> = terrain_wall()
            .into_iter()
            .rev()
            .map(|(id, a, b)| (id, b, a))
            .collect();
        let pockets = elevation_profile_pockets(&backwards);
        assert_eq!(pockets.len(), 1);
        // The Tag is the line the walk starts with, and the walk follows the
        // sketch's first line: listed backwards it starts at the stretch's other
        // end. The shape is the same.
        assert!(
            matches!(pockets[0].0, 1389018 | 1388558),
            "tag {}",
            pockets[0].0
        );
        assert_eq!(pockets[0].1.len(), 4);
    }

    #[test]
    fn a_top_split_in_two_straight_lines_is_not_a_pocket() {
        let split = vec![
            (1, [0.0, 0.0, 0.0], [10.0, 0.0, 0.0]),
            (2, [10.0, 0.0, 0.0], [10.0, 0.0, 5.0]),
            (3, [10.0, 0.0, 5.0], [5.0, 0.0, 7.5]),
            (4, [5.0, 0.0, 7.5], [0.0, 0.0, 10.0]),
            (5, [0.0, 0.0, 10.0], [0.0, 0.0, 0.0]),
        ];
        assert!(elevation_profile_pockets(&split).is_empty());
    }
}
