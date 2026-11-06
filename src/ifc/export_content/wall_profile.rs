//! The cut-out of a wall's edited elevation profile, carried on to the wall's
//! end (#1473, RE-358).
//!
//! A wall whose elevation profile was edited has a sketch of lines in one
//! vertical plane. Revit's export voids the wall with the region between each
//! run of those lines that leaves the profile's bounding rectangle and the
//! rectangle's edge (RE-151, [`crate::partition_schema_mvp::elevation_profile_cuts`]).
//! The sketch spans the wall's location curve, but a wall's body goes on past
//! it at some ends, and there the opening does not reach: the stretch stays as
//! the wall's box has it while Revit's body goes on along the profile's own line
//! (RE-348 section 5: a 0.41 ft stretch of the BIM4LCA office's Basement walls,
//! 0.65 ft too high at the wall's end). At all eight ends measured there the
//! record's end step lists no joined wall (`m_realJoined` is empty) and names one
//! exposed neighbour instead, so the join lists are not asked for.
//!
//! This pass carries the opening on. Where a wall's body goes on past an end of
//! the profile by more than a thousandth of a foot, and an opening reaches that
//! end of the profile with one vertical edge whose two ends each have a
//! neighbour, both ends of the edge move out to the wall's end. Two shapes are
//! read:
//!
//! - **sloped**: one end of the edge has a sloped neighbour (the profile's last
//!   line arriving) and the other a level one (an edge of the profile's
//!   rectangle). The rectangle's corner moves straight out and the line's end
//!   along the line's slope; the region stays on the same side of the line;
//! - **level**: both ends have level neighbours, as the cut-out of a notch
//!   does. The one that lies on the wall's own top or base is the wall's edge;
//!   the other, interior one is the line, and it is carried straight out. The
//!   two ends of the edge are taken by that, not by the order the outline lists
//!   them in. Where both lie on the wall's top or base, or both are interior, it
//!   is not read.
//!
//! A wall of which this is not read keeps the top the opening gives it, with
//! the reason as `ProfileStubRefusal`: the opening does not reach the end with
//! one such edge, both neighbours of a level cut-out are interior or both on the
//! wall's top or base, the stretch is longer than the wall is thick (a
//! heuristic: every stretch measured is half the wall's thickness, and nothing
//! in the file says a wall goes on past its profile by at most that), or the
//! slope would carry the line to the wall's base or across the rectangle's edge. A wall's property keeps the first reason only,
//! and a wall carried at one end and refused at the other is counted in both
//! `exported.wall_profile_sources` and `exported.wall_profile_refusals`. A wall
//! whose body goes on past the profile where no opening reaches that end, or
//! where the cut-out stays outside the wall across the stretch, is not changed
//! and not counted. A wall with no record property set, or drawn from a
//! profile of its own, is left out.

use std::f64::consts::FRAC_PI_4;

use super::{
    ELEMENT_RECORD_PROPERTY_SET, WALL_PROFILE_REFUSAL_PROPERTY, WALL_PROFILE_SOURCE_PROPERTY,
};
use crate::ifc::entities::{IfcEntity, Property, PropertyValue};
use std::collections::HashMap;

/// How far a wall must go on past its profile for a stretch to be one, how
/// near a vertex must lie to the profile's end to be on it, and how far a line
/// must stay from an edge to be off it, feet.
const TOLERANCE_FEET: f64 = 1e-3;

/// The opening does not reach the end of the profile with one vertical edge
/// whose ends are a line's and a rectangle's, or a level cut-out's two lines.
const CUT_NOT_READ: &str = "profile_cut_not_read_at_the_wall_end";

/// A level cut-out whose two lines are both interior to the wall's height.
const LEVEL_CUT_BETWEEN_TWO_LINES: &str = "profile_level_cut_between_two_lines";

/// A level cut-out whose two lines both lie on the wall's top or base.
const LEVEL_CUT_SPANS_THE_WALL: &str = "profile_level_cut_spans_the_wall";

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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Low,
    High,
}

/// What an opening's end was carried on as.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Sloped,
    Level,
}

/// An opening's outline carried on, and as what.
type Carried = (Vec<(f64, f64)>, Kind);

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

/// The `ProfileStubSource` of a wall carried on at `sides`, as `kinds`.
fn source_value(sides: &[Side], kinds: &[Kind]) -> Option<String> {
    let low = sides.contains(&Side::Low);
    let high = sides.contains(&Side::High);
    let sloped = kinds.contains(&Kind::Sloped);
    let level = kinds.contains(&Kind::Level);
    let where_ = match (low, high) {
        (false, false) => return None,
        (true, false) => "the_low_end",
        (false, true) => "the_high_end",
        (true, true) => "both_ends",
    };
    let what = match (sloped, level) {
        (true, false) => "sloped",
        (false, true) => "level",
        _ => "sloped_and_level",
    };
    Some(format!("wall_profile_{what}_past_{where_}"))
}

/// Carry the cut-out of each wall's profile on to the wall's end (see the
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
        let mut kinds: Vec<Kind> = Vec::new();
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
                    Ok(Some((outline, kind))) => {
                        *outline_feet = outline;
                        if !sides.contains(&side) {
                            sides.push(side);
                        }
                        if !kinds.contains(&kind) {
                            kinds.push(kind);
                        }
                    }
                    Ok(None) => {}
                    Err(reason) => {
                        refusal.get_or_insert(reason);
                    }
                }
            }
        }
        if let Some(source) = source_value(&sides, &kinds) {
            mark(entities, wall.index, WALL_PROFILE_SOURCE_PROPERTY, &source);
        }
        if let Some(reason) = refusal {
            mark(entities, wall.index, WALL_PROFILE_REFUSAL_PROPERTY, reason);
        }
    }
}

/// The outline of an opening carried on past one end of its profile, where the
/// wall goes on past that end by `reach` feet, and what it was carried on as.
/// `edge` is the end of the profile along the outline's `u`, `direction` is 1
/// when the wall goes on towards greater `u` and -1 towards lesser, and
/// `height` and `thickness` are the wall's. `Ok(None)` where nothing is to
/// change, `Err` with the reason where the end is not read.
fn extended(
    outline: &[(f64, f64)],
    edge: f64,
    direction: f64,
    reach: f64,
    height: f64,
    thickness: f64,
) -> Result<Option<Carried>, &'static str> {
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
    // On the wall's own top or base: a line there is the wall's edge, not a
    // cut's.
    let on_the_wall = |v: f64| v <= TOLERANCE_FEET || v >= height - TOLERANCE_FEET;
    // The line's end is the one whose other edge slopes, the rectangle's
    // corner the one whose other edge is level. Where both are level, the
    // corner is the one on the wall's top or base, whichever order they are
    // listed in.
    let (run_end, corner, run_before, kind) =
        match (level(outline[a], next_a), level(outline[b], next_b)) {
            (false, true) => (a, b, next_a, Kind::Sloped),
            (true, false) => (b, a, next_b, Kind::Sloped),
            (true, true) => match (on_the_wall(outline[a].1), on_the_wall(outline[b].1)) {
                (false, true) => (a, b, next_a, Kind::Level),
                (true, false) => (b, a, next_b, Kind::Level),
                (false, false) => return unread(LEVEL_CUT_BETWEEN_TWO_LINES),
                (true, true) => return unread(LEVEL_CUT_SPANS_THE_WALL),
            },
            (false, false) => return unread(CUT_NOT_READ),
        };
    let (v_end, v_corner) = (outline[run_end].1, outline[corner].1);
    let slope = if kind == Kind::Level {
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
    Ok(Some((carried, kind)))
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEIGHT: f64 = 3.0;
    const THICKNESS: f64 = 1.0;

    fn key(outline: &[(f64, f64)]) -> Vec<(i64, i64)> {
        let mut keyed: Vec<(i64, i64)> = outline
            .iter()
            .map(|(u, v)| ((u * 1000.0).round() as i64, (v * 1000.0).round() as i64))
            .collect();
        keyed.sort();
        keyed
    }

    fn carried(
        outline: &[(f64, f64)],
        edge: f64,
        direction: f64,
        reach: f64,
    ) -> Result<Option<Carried>, &'static str> {
        extended(outline, edge, direction, reach, HEIGHT, THICKNESS)
    }

    #[test]
    fn a_notch_on_the_base_is_carried_in_either_vertex_order() {
        // The base vertex first, which took the line's end for the edge's
        // lower index and refused the notch as a line on the base.
        let base_first = [(4.0, 0.0), (4.0, 1.6), (3.0, 1.6), (3.0, 0.0)];
        // The interior vertex first.
        let line_first = [(4.0, 1.6), (3.0, 1.6), (3.0, 0.0), (4.0, 0.0)];
        let want = vec![(3000, 0), (3000, 1600), (4500, 0), (4500, 1600)];
        for outline in [base_first, line_first] {
            let Ok(Some((outline, kind))) = carried(&outline, 4.0, 1.0, 0.5) else {
                panic!("the notch on the base was not carried");
            };
            assert_eq!(kind, Kind::Level);
            assert_eq!(key(&outline), want);
        }
    }

    #[test]
    fn a_notch_on_the_top_is_carried_in_either_vertex_order() {
        let top_first = [(4.0, 3.0), (3.0, 3.0), (3.0, 1.0), (4.0, 1.0)];
        let line_first = [(4.0, 1.0), (4.0, 3.0), (3.0, 3.0), (3.0, 1.0)];
        let want = vec![(3000, 1000), (3000, 3000), (4500, 1000), (4500, 3000)];
        for outline in [top_first, line_first] {
            let Ok(Some((outline, kind))) = carried(&outline, 4.0, 1.0, 0.5) else {
                panic!("the notch on the top was not carried");
            };
            assert_eq!(kind, Kind::Level);
            assert_eq!(key(&outline), want);
        }
    }

    #[test]
    fn a_notch_at_the_low_end_is_carried_towards_lesser_u() {
        let outline = [(0.0, 0.0), (0.0, 1.6), (1.0, 1.6), (1.0, 0.0)];
        let Ok(Some((outline, kind))) = carried(&outline, 0.0, -1.0, 0.5) else {
            panic!("the notch at the low end was not carried");
        };
        assert_eq!(kind, Kind::Level);
        assert_eq!(
            key(&outline),
            vec![(-500, 0), (-500, 1600), (1000, 0), (1000, 1600)]
        );
    }

    #[test]
    fn a_level_cut_with_two_interior_lines_is_refused() {
        let outline = [(4.0, 2.0), (3.0, 2.0), (3.0, 1.0), (4.0, 1.0)];
        assert_eq!(
            carried(&outline, 4.0, 1.0, 0.5).err(),
            Some(LEVEL_CUT_BETWEEN_TWO_LINES)
        );
    }

    #[test]
    fn a_level_cut_with_both_lines_on_the_wall_is_refused() {
        let outline = [(4.0, 3.0), (3.0, 3.0), (3.0, 0.0), (4.0, 0.0)];
        assert_eq!(
            carried(&outline, 4.0, 1.0, 0.5).err(),
            Some(LEVEL_CUT_SPANS_THE_WALL)
        );
    }

    #[test]
    fn a_level_cut_wholly_above_the_wall_is_not_a_refusal() {
        let outline = [(4.0, 3.5), (2.0, 3.5), (2.0, 4.0), (4.0, 4.0)];
        assert_eq!(carried(&outline, 4.0, 1.0, 0.5), Ok(None));
    }

    #[test]
    fn a_sloped_line_goes_on_along_its_slope() {
        // The corner at the rectangle's top, the line falling to the edge.
        let outline = [(4.0, 3.3), (0.0, 3.3), (4.0, 2.0)];
        let Ok(Some((outline, kind))) = carried(&outline, 4.0, 1.0, 0.4) else {
            panic!("the sloped line was not carried");
        };
        assert_eq!(kind, Kind::Sloped);
        assert_eq!(key(&outline), vec![(0, 3300), (4400, 1870), (4400, 3300)]);
    }

    #[test]
    fn a_sloped_line_goes_on_the_same_in_either_vertex_order() {
        let corner_first = [(4.0, 3.3), (0.0, 3.3), (4.0, 2.0)];
        let line_first = [(4.0, 2.0), (4.0, 3.3), (0.0, 3.3)];
        let first = carried(&corner_first, 4.0, 1.0, 0.4);
        let second = carried(&line_first, 4.0, 1.0, 0.4);
        let (Ok(Some((first, _))), Ok(Some((second, _)))) = (first, second) else {
            panic!("the sloped line was not carried in both orders");
        };
        assert_eq!(key(&first), key(&second));
    }

    #[test]
    fn a_stretch_longer_than_the_wall_is_thick_is_refused() {
        let outline = [(4.0, 3.3), (0.0, 3.3), (4.0, 2.0)];
        assert_eq!(
            carried(&outline, 4.0, 1.0, 1.5).err(),
            Some(STUB_LONGER_THAN_THE_WALL_IS_THICK)
        );
    }

    #[test]
    fn a_slope_that_crosses_the_rectangle_is_refused() {
        // Rising at 0.5, the line passes the corner's height within the stretch.
        let outline = [(4.0, 3.2), (0.0, 3.2), (0.0, 1.0), (4.0, 3.0)];
        assert_eq!(
            carried(&outline, 4.0, 1.0, 0.5).err(),
            Some(SLOPE_LEAVES_THE_CUT)
        );
    }

    #[test]
    fn a_slope_that_reaches_the_base_is_refused() {
        let outline = [(4.0, 3.3), (0.0, 3.3), (4.0, 0.3)];
        assert_eq!(
            carried(&outline, 4.0, 1.0, 0.5).err(),
            Some(SLOPE_LEAVES_THE_CUT)
        );
    }

    #[test]
    fn an_end_the_outline_does_not_reach_is_left_alone() {
        let outline = [(2.0, 3.3), (0.0, 3.3), (2.0, 2.0)];
        assert_eq!(carried(&outline, 4.0, 1.0, 0.4), Ok(None));
        assert_eq!(carried(&outline, 4.0, 1.0, 0.0), Ok(None));
    }

    #[test]
    fn an_end_that_one_vertex_reaches_is_not_read() {
        // A line rising to the rectangle's corner above the wall: not a
        // refusal, since the opening does not come down into the wall there.
        let above = [(0.0, 3.3), (0.0, 1.0), (4.0, 3.3)];
        assert_eq!(carried(&above, 4.0, 1.0, 0.4), Ok(None));
        // The same ending inside the wall's height is refused.
        let inside = [(0.0, 3.3), (0.0, 1.0), (4.0, 2.5)];
        assert_eq!(carried(&inside, 4.0, 1.0, 0.4).err(), Some(CUT_NOT_READ));
    }
}
