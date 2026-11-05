//! The sloped line of a wall's edited elevation profile, carried on to the
//! wall's end (#1473, RE-358).
//!
//! A wall whose elevation profile was edited has a sketch of lines in one
//! vertical plane. Revit's export voids the wall with the region between each
//! run of those lines that leaves the profile's bounding rectangle and the
//! rectangle's edge (RE-151, [`crate::partition_schema_mvp::elevation_profile_cuts`]).
//! The sketch spans the wall's location curve, but a wall's body goes on past
//! it where a join extends the wall, and there the opening does not reach: the
//! stretch stays at the top of the wall's box while Revit's top goes on
//! sloping along the profile's own line (RE-348 section 5: a 0.41 ft stub of
//! the BIM4LCA office's Basement walls, 0.65 ft too high at the wall's end).
//!
//! This pass carries the opening on. Where a wall's box goes on past an end of
//! the profile by more than a thousandth of a foot, and an opening reaches that
//! end of the profile with one vertical edge, at one end of which the profile's
//! last line arrives and at the other a horizontal edge of the rectangle
//! leaves, both ends of the vertical edge move out to the wall's end: the
//! rectangle's corner straight out, the line's end along the line's slope. The
//! region stays on the same side of the line.
//!
//! A wall of which this is not read keeps the top the opening gives it, with
//! the reason as `ProfileStubRefusal`: the opening does not reach the profile's
//! end with one edge of the shape above, the stretch is longer than the wall
//! is thick (a join's extension is at most that), or the slope would carry the
//! line to the wall's base or across the rectangle's edge. A wall whose box
//! goes on past the profile where no opening reaches that end, or where the
//! opening's line stays above the wall's top across the stretch, is not
//! changed and not counted. A wall with no record property set, or drawn from
//! a profile of its own, is left out.

use std::f64::consts::FRAC_PI_4;

use super::{
    ELEMENT_RECORD_PROPERTY_SET, WALL_PROFILE_PAST_BOTH_ENDS_SOURCE,
    WALL_PROFILE_PAST_HIGH_END_SOURCE, WALL_PROFILE_PAST_LOW_END_SOURCE,
    WALL_PROFILE_REFUSAL_PROPERTY, WALL_PROFILE_SOURCE_PROPERTY,
};
use crate::ifc::entities::{IfcEntity, Property, PropertyValue};
use std::collections::HashMap;

/// How far a wall must go on past its profile for a stretch to be one, how
/// near a vertex must lie to the profile's end to be on it, and how far a line
/// must stay from an edge to be off it, feet.
const TOLERANCE_FEET: f64 = 1e-3;

/// The opening does not reach the end of the profile with one vertical edge
/// whose ends are a line's and a rectangle's.
const CUT_NOT_READ: &str = "profile_cut_not_read_at_the_wall_end";

/// The wall goes on past its profile by more than it is thick.
const STUB_LONGER_THAN_THE_WALL_IS_THICK: &str = "profile_stub_longer_than_the_wall_is_thick";

/// The slope carries the line to the wall's base or across the rectangle's
/// edge before the wall's end.
const SLOPE_LEAVES_THE_CUT: &str = "profile_slope_leaves_the_cut";

/// A profile-edited wall waiting for the pass: its entity index and the plan
/// position of the lowest and highest points of its profile along the
/// profile's axis, model feet.
pub(super) struct Pending {
    pub index: usize,
    pub low: [f64; 2],
    pub high: [f64; 2],
}

/// An end of a profile along its own axis.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    Low,
    High,
}

/// The extrusion of a wall and where it stands, copied from its entity.
struct Frame {
    width: f64,
    depth: f64,
    height: f64,
    location: [f64; 3],
    rotation: f64,
}

/// The frame of the wall at `index` where it is one plain extrusion that
/// carries Revit's record property set, so that what the pass does can be
/// counted.
fn wall_frame(entities: &[IfcEntity], index: usize) -> Option<Frame> {
    let IfcEntity::BuildingElement {
        extrusion: Some(body),
        location_feet: Some(location),
        rotation_radians,
        property_set: Some(set),
        ..
    } = entities.get(index)?
    else {
        return None;
    };
    if body.profile_override.is_some() || set.name != ELEMENT_RECORD_PROPERTY_SET {
        return None;
    }
    Some(Frame {
        width: body.width_feet,
        depth: body.depth_feet,
        height: body.height_feet,
        location: *location,
        rotation: rotation_radians.filter(|r| r.is_finite()).unwrap_or(0.0),
    })
}

/// Write `value` as the property `name` of the wall at `index`.
fn mark(entities: &mut [IfcEntity], index: usize, name: &str, value: &str) {
    let Some(IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    }) = entities.get_mut(index)
    else {
        return;
    };
    if set.name == ELEMENT_RECORD_PROPERTY_SET {
        set.properties.push(Property {
            name: name.into(),
            value: PropertyValue::Text(value.into()),
        });
    }
}

/// Carry the sloped line of each wall's profile on to the wall's end (see the
/// module documentation).
pub(super) fn extend_profile_stubs(entities: &mut [IfcEntity], pending: &[Pending]) {
    if pending.is_empty() {
        return;
    }
    let mut openings: HashMap<usize, Vec<usize>> = HashMap::new();
    for (index, entity) in entities.iter().enumerate() {
        if let IfcEntity::WallProfileOpening { host, .. } = entity {
            openings.entry(*host).or_default().push(index);
        }
    }
    for wall in pending {
        let Some(indices) = openings.get(&wall.index) else {
            continue;
        };
        let Some(frame) = wall_frame(entities, wall.index) else {
            continue;
        };
        let mut sides: Vec<Side> = Vec::new();
        let mut refusal: Option<&'static str> = None;
        for &index in indices {
            let IfcEntity::WallProfileOpening {
                outline_feet,
                axis_turn_radians,
                thickness_feet,
                ..
            } = &mut entities[index]
            else {
                continue;
            };
            let turn = frame.rotation + *axis_turn_radians;
            let axis = [turn.cos(), turn.sin()];
            let along = |point: [f64; 2]| {
                (point[0] - frame.location[0]) * axis[0] + (point[1] - frame.location[1]) * axis[1]
            };
            let (first, second) = (along(wall.low), along(wall.high));
            let (low, high) = (first.min(second), first.max(second));
            let long = if axis_turn_radians.abs() < FRAC_PI_4 {
                frame.width
            } else {
                frame.depth
            };
            let half = long / 2.0;
            for (side, edge, direction, reach) in [
                (Side::Low, low, -1.0, low + half),
                (Side::High, high, 1.0, half - high),
            ] {
                match extended(
                    outline_feet.as_slice(),
                    edge,
                    direction,
                    reach,
                    frame.height,
                    *thickness_feet,
                ) {
                    Ok(Some(outline)) => {
                        *outline_feet = outline;
                        if !sides.contains(&side) {
                            sides.push(side);
                        }
                    }
                    Ok(None) => {}
                    Err(reason) => {
                        refusal.get_or_insert(reason);
                    }
                }
            }
        }
        let source = match sides.as_slice() {
            [] => None,
            [Side::Low] => Some(WALL_PROFILE_PAST_LOW_END_SOURCE),
            [Side::High] => Some(WALL_PROFILE_PAST_HIGH_END_SOURCE),
            _ => Some(WALL_PROFILE_PAST_BOTH_ENDS_SOURCE),
        };
        if let Some(source) = source {
            mark(entities, wall.index, WALL_PROFILE_SOURCE_PROPERTY, source);
        }
        if let Some(reason) = refusal {
            mark(entities, wall.index, WALL_PROFILE_REFUSAL_PROPERTY, reason);
        }
    }
}

/// The outline of an opening carried on past one end of its profile, where the
/// wall goes on past that end by `reach` feet. `edge` is the end of the
/// profile along the outline's `u`, `direction` is 1 when the wall goes on
/// towards greater `u` and -1 towards lesser, and `height` and `thickness` are
/// the wall's. `Ok(None)` where nothing is to change, `Err` with the reason
/// where the end is not read.
fn extended(
    outline: &[(f64, f64)],
    edge: f64,
    direction: f64,
    reach: f64,
    height: f64,
    thickness: f64,
) -> Result<Option<Vec<(f64, f64)>>, &'static str> {
    if reach <= TOLERANCE_FEET {
        return Ok(None);
    }
    // How far a point lies out past the profile's end, negative inside.
    let out = |point: (f64, f64)| direction * (point.0 - edge);
    let touching: Vec<usize> = (0..outline.len())
        .filter(|at| out(outline[*at]).abs() <= TOLERANCE_FEET)
        .collect();
    if touching.is_empty() {
        return Ok(None);
    }
    // An end that is not read is a refusal only where the opening comes down
    // into the wall's height there.
    let dips = touching
        .iter()
        .any(|at| outline[*at].1 < height - TOLERANCE_FEET);
    let unread = |reason: &'static str| {
        if dips { Err(reason) } else { Ok(None) }
    };
    let count = outline.len();
    if count < 3 || touching.len() != 2 {
        return unread(CUT_NOT_READ);
    }
    let (a, b) = (touching[0], touching[1]);
    if (a + 1) % count != b && (b + 1) % count != a {
        return unread(CUT_NOT_READ);
    }
    // The vertex of `at` beside it that is not its partner on the edge.
    let beside = |at: usize, partner: usize| {
        let before = (at + count - 1) % count;
        if before == partner {
            (at + 1) % count
        } else {
            before
        }
    };
    let (next_a, next_b) = (outline[beside(a, b)], outline[beside(b, a)]);
    if out(next_a) > -TOLERANCE_FEET || out(next_b) > -TOLERANCE_FEET {
        return unread(CUT_NOT_READ);
    }
    let level = |from: (f64, f64), to: (f64, f64)| (from.1 - to.1).abs() <= TOLERANCE_FEET;
    // The line's end is the one whose other edge slopes, the rectangle's
    // corner the one whose other edge is level.
    let (run_end, corner, run_before) = match (level(outline[a], next_a), level(outline[b], next_b))
    {
        (false, true) => (a, b, next_a),
        (true, false) => (b, a, next_b),
        (true, true) => (a, b, next_a),
        (false, false) => return unread(CUT_NOT_READ),
    };
    let (v_end, v_corner) = (outline[run_end].1, outline[corner].1);
    let slope = if level(outline[run_end], run_before) {
        0.0
    } else {
        (v_end - run_before.1) / -out(run_before)
    };
    if reach > thickness + TOLERANCE_FEET {
        return Err(STUB_LONGER_THAN_THE_WALL_IS_THICK);
    }
    let v_new = v_end + slope * reach;
    let cut_above = v_corner > v_end;
    if (v_corner - v_end).abs() <= TOLERANCE_FEET
        || (v_corner - v_new).abs() <= TOLERANCE_FEET
        || (v_corner > v_end) != (v_corner > v_new)
        || v_new <= TOLERANCE_FEET
    {
        return Err(SLOPE_LEAVES_THE_CUT);
    }
    let cuts = if cut_above {
        v_end.min(v_new) < height - TOLERANCE_FEET
    } else {
        v_end.max(v_new) > TOLERANCE_FEET
    };
    if !cuts {
        return Ok(None);
    }
    let u_new = edge + direction * reach;
    let mut carried = outline.to_vec();
    carried[run_end] = (u_new, v_new);
    carried[corner] = (u_new, v_corner);
    Ok(Some(carried))
}
