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
//!
//! # The top at the profile's highest point (#1600, RE-378)
//!
//! A profile edited down from the wall's unconnected height closes below the
//! top of the wall's record box, and Revit's body ends at the profile's highest
//! point. The openings above are cut from the profile's own bounding rectangle,
//! so the strip between that rectangle's top and the box's top stays solid and
//! the wall's `Height`, the extrusion's, is the box's. The BIM4LCA office's
//! terrain files hold the one wall measured: 1387731, whose profile closes
//! 3.6765 ft below its box over the whole of its 173.9 ft, and whose `Height`
//! Revit writes as the profile's 4.4 m where the box gives 5.520605 m.
//!
//! [`take_top_at_profile`] lowers such a wall's extrusion to the profile's
//! highest point when all of these hold, and says so in
//! `ProfileTopSource`. The rule rests on that one wall, so every condition is
//! the narrowest the measured wall allows:
//!
//! - the record names nothing joined to or cutting the wall, so that no other
//!   pass decides its top or base;
//! - the profile spans the wall: its lowest and highest points along its own
//!   axis lie at the wall's two ends, within a thousandth of a foot. A wall that
//!   goes on past its profile has a box that holds the profile's line carried on
//!   (RE-358: `rac_basic`'s wall 493612 is 3.529 m high for a profile of 3.5 m),
//!   and its top is not the profile's;
//! - the profile's lowest line is the wall's base;
//! - the profile's highest point lies more than a thousandth of a foot below the
//!   top of the box.
//!
//! A wall of which any of this is not known keeps the box's top, as before.
//!
//! # The profile's cut in the wall's body (#1607, RE-380)
//!
//! RE-151's regions, each between a run of the profile's lines and the
//! profile's bounding rectangle, make the wall's shape, but they are not what
//! Revit's export writes as openings. Revit writes the wall's body as the profile
//! (a tessellation) and an opening only for each pocket of the profile against
//! its convex hull ([`crate::partition_schema_mvp::elevation_profile_pockets`]):
//! none on a straight sloped top; on the terrain wall 1387731 the first three
//! lines of its top closed by a chord; and on Core Interior's wall 55840 the
//! stretch of its stepped top closed through the corner of the profile's
//! rectangle that the hull's edge cuts off (301.65 ft2, not the 71.23 ft2 the
//! stretch and the edge hold). [`cut_profiles_from_bodies`] runs
//! after every pass that reads a wall's body or its openings. For a wall whose
//! regions can all be made into solids it takes them out of the wall's body as a
//! boolean difference, writes none of them as openings, and writes its pockets
//! instead, so that the openings a wall carries are Revit's and its shape is
//! the same. A wall it cannot do this for keeps the regions as its openings, as
//! before, and its pockets are not written. The wall's `ProfileCutSource` says
//! which it is.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use super::{
    ELEMENT_RECORD_PROPERTY_SET, WALL_PROFILE_CHECK_PROPERTY, WALL_PROFILE_CUT_IN_BODY,
    WALL_PROFILE_CUT_PROPERTY, WALL_PROFILE_REFUSAL_PROPERTY, WALL_PROFILE_SOURCE_PROPERTY,
    WALL_PROFILE_TOP_AT_PROFILE, WALL_PROFILE_TOP_PROPERTY,
};
use crate::ifc::entities::{
    Extrusion, IfcBooleanOp, IfcEntity, ProfileDef, Property, PropertySet, PropertyValue,
    SolidShape,
};
use crate::walker::{DecodedElement, InstanceField};
use std::collections::{BTreeMap, HashMap};

/// How far a wall must go on past its profile for a stretch to be one, how
/// near a vertex must lie to the profile's end to be on it, and how far a line
/// must stay from an edge to be off it, feet.
const TOLERANCE_FEET: f64 = 1e-3;

/// How far the line's height at a wall's end may lie from the top of the
/// wall's box to be the box's top, feet.
const HEIGHT_TOLERANCE_FEET: f64 = 1e-3;

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

/// Where the line of a sloped end, carried on to the wall's end, stands against
/// the top of the wall's box: its slope per foot out towards the end, and its
/// height at the wall's end less the box's top. The box holds the wall's
/// highest point, so where the line rises towards the end and is that point
/// the gap is nil; where it falls, the box's top is the wall's own top
/// elsewhere and the gap says nothing of the stretch. A measurement only: no
/// end is carried or refused by it (#1473).
struct Check {
    slope: f64,
    gap: f64,
}

/// The key a sloped end's [`Check`] is counted under: whether the end was
/// carried, its side, whether the line rises or falls towards the wall's end,
/// and how the line's height there stands against the box's top.
fn check_key(carried: bool, side: Side, check: &Check) -> String {
    let state = if carried { "carried" } else { "kept" };
    let side = match side {
        Side::Low => "low",
        Side::High => "high",
    };
    let direction = if check.slope > 1e-6 {
        "rising"
    } else if check.slope < -1e-6 {
        "falling"
    } else {
        "level"
    };
    let gap = if check.gap.abs() <= HEIGHT_TOLERANCE_FEET {
        "equal"
    } else if check.gap < 0.0 {
        "below"
    } else {
        "above"
    };
    format!("{state}_sloped_{side}_{direction}_{gap}")
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
pub(super) fn extend_profile_stubs(
    entities: &mut [IfcEntity],
    pending: &[Pending],
    handoffs: &mut BTreeMap<usize, Vec<Property>>,
) {
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
                let mut check = None;
                let read = extended_with(
                    outline_feet.as_slice(),
                    edge,
                    direction,
                    reach,
                    frame.height,
                    *thickness_feet,
                    &mut check,
                );
                let key = match (&read, &check) {
                    (Ok(Some((_, Kind::Level))), _) => Some(format!(
                        "carried_level_{}",
                        if side == Side::Low { "low" } else { "high" }
                    )),
                    (_, Some(check)) => Some(check_key(matches!(read, Ok(Some(_))), side, check)),
                    _ => None,
                };
                if let Some(key) = key {
                    handoffs.entry(wall.index).or_default().push(Property {
                        name: WALL_PROFILE_CHECK_PROPERTY.into(),
                        value: PropertyValue::Text(key),
                    });
                }
                match read {
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
/// change, `Err` with the reason where the end is not read. `check` is set
/// where a sloped end was read, whatever follows.
fn extended_with(
    outline: &[(f64, f64)],
    edge: f64,
    direction: f64,
    reach: f64,
    height: f64,
    thickness: f64,
    check: &mut Option<Check>,
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
    let v_new = v_end + slope * reach;
    if kind == Kind::Sloped {
        *check = Some(Check {
            slope,
            gap: v_new - height,
        });
    }
    if reach > thickness + TOLERANCE_FEET {
        return Err(STUB_LONGER_THAN_THE_WALL_IS_THICK);
    }
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

/// The height a wall's body takes when its top is taken at the highest point of
/// its edited elevation profile (see the module documentation), or `None`
/// where it keeps the box's top. `heights` are the lowest and highest height of
/// the profile's lines and `ends` the plan position of its lowest and highest
/// points along its own axis, all model feet; `frame` is the wall's extrusion
/// as the exporter built it, whose base is `location[2]`.
fn top_at_profile(heights: (f64, f64), ends: ([f64; 2], [f64; 2]), frame: &Frame) -> Option<f64> {
    let (lowest, highest) = heights;
    let numbers = [
        lowest,
        highest,
        ends.0[0],
        ends.0[1],
        ends.1[0],
        ends.1[1],
        frame.width,
        frame.depth,
        frame.height,
        frame.location[0],
        frame.location[1],
        frame.location[2],
        frame.rotation,
    ];
    if numbers.iter().any(|number| !number.is_finite()) {
        return None;
    }
    // The wall's long axis, as the exporter places its profile openings along
    // it, and its two ends about the extrusion's centre.
    let turn = frame.rotation
        + if frame.width >= frame.depth {
            0.0
        } else {
            FRAC_PI_2
        };
    let axis = [turn.cos(), turn.sin()];
    let along = |point: [f64; 2]| {
        (point[0] - frame.location[0]) * axis[0] + (point[1] - frame.location[1]) * axis[1]
    };
    let (first, second) = (along(ends.0), along(ends.1));
    let (low, high) = (first.min(second), first.max(second));
    let half = frame.width.max(frame.depth) / 2.0;
    if (low + half).abs() > TOLERANCE_FEET || (half - high).abs() > TOLERANCE_FEET {
        return None;
    }
    if (lowest - frame.location[2]).abs() > TOLERANCE_FEET {
        return None;
    }
    let top = highest - frame.location[2];
    (top > TOLERANCE_FEET && frame.height - top > TOLERANCE_FEET).then_some(top)
}

/// Take the top of the wall `decoded` is drawn as, `body` at `location` with
/// `rotation`, at the highest point of its edited elevation profile where
/// [`top_at_profile`] says so and nothing else cuts the wall, and say so in
/// `set`, the wall's record property set (see the module documentation).
pub(super) fn take_top_at_profile(
    decoded: &DecodedElement,
    location: [f64; 3],
    rotation: Option<f64>,
    body: &mut Extrusion,
    set: &mut PropertySet,
) {
    let uncut = decoded.fields.iter().any(|(name, value)| {
        name == crate::partition_schema_mvp::WALL_UNCUT_FIELD
            && matches!(value, InstanceField::Bool(true))
    });
    if !uncut || set.name != ELEMENT_RECORD_PROPERTY_SET || body.profile_override.is_some() {
        return;
    }
    let (Some(heights), Some((low, high))) = (
        crate::partition_schema_mvp::profile_height_from_fields(&decoded.fields),
        crate::partition_schema_mvp::profile_extent_from_fields(&decoded.fields),
    ) else {
        return;
    };
    let frame = Frame {
        width: body.width_feet,
        depth: body.depth_feet,
        height: body.height_feet,
        location,
        rotation: rotation.filter(|turn| turn.is_finite()).unwrap_or(0.0),
    };
    let ends = ([low[0], low[1]], [high[0], high[1]]);
    if let Some(height) = top_at_profile(heights, ends, &frame) {
        body.height_feet = height;
        set.properties.push(Property {
            name: WALL_PROFILE_TOP_PROPERTY.into(),
            value: PropertyValue::Text(WALL_PROFILE_TOP_AT_PROFILE.into()),
        });
    }
}

/// How far the prism that makes a profile's cut reaches beyond the wall's two
/// faces, feet, so that its faces lie in none of the wall's.
const CUT_MARGIN_FEET: f64 = 0.01;

/// The solid of a region RE-151 reads from a wall's profile: its outline in the
/// wall's vertical plane, extruded through the wall and placed as the opening it
/// is written as is. `None` where the entity is not one, or is not a solid.
fn cut_prism(entity: &IfcEntity) -> Option<SolidShape> {
    let IfcEntity::WallProfileOpening {
        outline_feet,
        axis_turn_radians,
        plane_offset_feet,
        thickness_feet,
        ..
    } = entity
    else {
        return None;
    };
    let finite = outline_feet
        .iter()
        .flat_map(|point| [point.0, point.1])
        .chain([*axis_turn_radians, *plane_offset_feet, *thickness_feet])
        .all(f64::is_finite);
    if outline_feet.len() < 3 || !finite || *thickness_feet <= 0.0 {
        return None;
    }
    let (sin, cos) = axis_turn_radians.sin_cos();
    let start = plane_offset_feet + thickness_feet / 2.0 + CUT_MARGIN_FEET;
    Some(SolidShape::PlacedExtrusion {
        profile: ProfileDef::ArbitraryClosed {
            points: outline_feet.clone(),
        },
        origin_feet: [-sin * start, cos * start, 0.0],
        axis: [sin, -cos, 0.0],
        ref_direction: [cos, sin, 0.0],
        depth_feet: thickness_feet + 2.0 * CUT_MARGIN_FEET,
    })
}

/// Take the solids `prisms` out of the body of the wall `host` as a boolean
/// difference and say so in the wall's record property set. `false`, with the
/// wall as it was, where the entity is not a wall with a body of its own.
fn cut_body(host: Option<&mut IfcEntity>, prisms: Vec<SolidShape>) -> bool {
    let Some(IfcEntity::BuildingElement {
        ifc_type,
        extrusion,
        solid_shape,
        representation_map_index: None,
        property_set,
        ..
    }) = host
    else {
        return false;
    };
    if !matches!(
        ifc_type.to_ascii_uppercase().as_str(),
        "IFCWALL" | "IFCWALLSTANDARDCASE"
    ) {
        return false;
    }
    let Some(body) = solid_shape
        .clone()
        .or_else(|| extrusion.clone().map(SolidShape::ExtrudedArea))
    else {
        return false;
    };
    *solid_shape = Some(
        prisms
            .into_iter()
            .fold(body, |whole, prism| SolidShape::BooleanResult {
                op: IfcBooleanOp::Difference,
                operand_a: Box::new(whole),
                operand_b: Box::new(prism),
            }),
    );
    if let Some(set) = property_set {
        if set.name == ELEMENT_RECORD_PROPERTY_SET {
            set.properties.push(Property {
                name: WALL_PROFILE_CUT_PROPERTY.into(),
                value: PropertyValue::Text(WALL_PROFILE_CUT_IN_BODY.into()),
            });
        }
    }
    true
}

/// Make the cut of each wall's edited elevation profile in the wall's body and
/// write the openings Revit's export writes for it (see the module
/// documentation). A wall whose regions are not all solids, or whose entity is
/// not a wall with a body of its own, is left with its regions as its openings.
pub(super) fn cut_profiles_from_bodies(entities: &mut [IfcEntity]) {
    let mut regions: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    let mut pockets: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, entity) in entities.iter().enumerate() {
        match entity {
            IfcEntity::WallProfileOpening { host, .. } => {
                regions.entry(*host).or_default().push(index);
            }
            IfcEntity::WallProfilePocket { host, .. } => {
                pockets.entry(*host).or_default().push(index);
            }
            _ => {}
        }
    }
    for (host, region_indices) in regions {
        let Some(prisms) = region_indices
            .iter()
            .map(|index| cut_prism(&entities[*index]))
            .collect::<Option<Vec<SolidShape>>>()
        else {
            continue;
        };
        if !cut_body(entities.get_mut(host), prisms) {
            continue;
        }
        for index in region_indices {
            if let IfcEntity::WallProfileOpening { outline_feet, .. } = &mut entities[index] {
                outline_feet.clear();
            }
        }
        for index in pockets.remove(&host).unwrap_or_default() {
            let opening = match &entities[index] {
                IfcEntity::WallProfilePocket {
                    host,
                    tag,
                    name,
                    outline_feet,
                    axis_turn_radians,
                    plane_offset_feet,
                    thickness_feet,
                } => IfcEntity::WallProfileOpening {
                    host: *host,
                    tag: tag.clone(),
                    name: name.clone(),
                    outline_feet: outline_feet.clone(),
                    axis_turn_radians: *axis_turn_radians,
                    plane_offset_feet: *plane_offset_feet,
                    thickness_feet: *thickness_feet,
                },
                _ => continue,
            };
            entities[index] = opening;
        }
    }
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
        extended_with(
            outline, edge, direction, reach, HEIGHT, THICKNESS, &mut None,
        )
    }

    fn checked(outline: &[(f64, f64)], edge: f64, direction: f64, reach: f64) -> Option<Check> {
        let mut check = None;
        let _ = extended_with(
            outline, edge, direction, reach, HEIGHT, THICKNESS, &mut check,
        );
        check
    }

    #[test]
    fn a_line_rising_to_the_top_of_the_box_has_no_gap() {
        // 2.0 + 0.5 x 0.4 = 2.2 at the wall's end, and the box is 2.2 high.
        let outline = [(4.0, 3.0), (0.0, 3.0), (0.0, 0.0), (4.0, 2.0)];
        let mut check = None;
        let _ = extended_with(&outline, 4.0, 1.0, 0.4, 2.2, THICKNESS, &mut check);
        let Some(check) = check else {
            panic!("the sloped end was not checked");
        };
        assert!(check.gap.abs() < 1e-9);
        assert_eq!(
            check_key(true, Side::High, &check),
            "carried_sloped_high_rising_equal"
        );
    }

    #[test]
    fn a_line_falling_to_the_end_stands_below_the_box() {
        let outline = [(4.0, 3.3), (0.0, 3.3), (4.0, 2.0)];
        let Some(check) = checked(&outline, 4.0, 1.0, 0.4) else {
            panic!("the sloped end was not checked");
        };
        assert!(check.gap < -1.0);
        assert_eq!(
            check_key(false, Side::Low, &check),
            "kept_sloped_low_falling_below"
        );
    }

    #[test]
    fn a_level_end_and_an_unread_end_are_not_checked() {
        let notch = [(4.0, 0.0), (4.0, 1.6), (3.0, 1.6), (3.0, 0.0)];
        assert!(checked(&notch, 4.0, 1.0, 0.5).is_none());
        let one_vertex = [(0.0, 3.3), (0.0, 1.0), (4.0, 2.5)];
        assert!(checked(&one_vertex, 4.0, 1.0, 0.4).is_none());
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

    /// The BIM4LCA office terrain files' wall 1387731: 173.8565 ft long about
    /// its centre, 18.1122 ft tall from 182.0866 ft, along the x axis.
    fn terrain_wall() -> Frame {
        Frame {
            width: 173.8565,
            depth: 0.4921,
            height: 18.1122,
            location: [209.3321, 121.5711, 182.0866],
            rotation: 0.0,
        }
    }

    const ENDS: ([f64; 2], [f64; 2]) = ([122.4038, 121.5711], [296.2603, 121.5718]);

    #[test]
    fn a_profile_that_spans_the_wall_and_closes_below_its_box_gives_its_top() {
        let top = top_at_profile((182.0866, 196.5223), ENDS, &terrain_wall());
        let Some(top) = top else {
            panic!("the profile's top was not taken");
        };
        assert!((top - 14.4357).abs() < 1e-4, "{top}");
    }

    #[test]
    fn a_profile_that_stops_short_of_an_end_keeps_the_box() {
        // rac_basic's wall 493612 is of this kind: its box holds the line carried on.
        let short = ([122.4038, 121.5711], [296.1203, 121.5718]);
        assert_eq!(
            top_at_profile((182.0866, 196.5223), short, &terrain_wall()),
            None
        );
        let other_end = ([122.5438, 121.5711], [296.2603, 121.5718]);
        assert_eq!(
            top_at_profile((182.0866, 196.5223), other_end, &terrain_wall()),
            None
        );
    }

    #[test]
    fn a_profile_that_starts_above_the_base_keeps_the_box() {
        assert_eq!(
            top_at_profile((182.5, 196.5223), ENDS, &terrain_wall()),
            None
        );
    }

    #[test]
    fn a_profile_as_high_as_the_box_keeps_the_box() {
        let box_top = 182.0866 + 18.1122;
        assert_eq!(
            top_at_profile((182.0866, box_top), ENDS, &terrain_wall()),
            None
        );
        assert_eq!(
            top_at_profile((182.0866, box_top - 0.0005), ENDS, &terrain_wall()),
            None
        );
        assert_eq!(
            top_at_profile((182.0866, box_top + 1.0), ENDS, &terrain_wall()),
            None
        );
    }

    #[test]
    fn a_profile_with_no_height_above_the_base_keeps_the_box() {
        assert_eq!(
            top_at_profile((182.0866, 182.0866), ENDS, &terrain_wall()),
            None
        );
        assert_eq!(
            top_at_profile((182.0866, 181.0), ENDS, &terrain_wall()),
            None
        );
    }

    #[test]
    fn a_wall_turned_across_the_axes_is_read_along_its_own_axis() {
        // The same wall turned a quarter turn about its centre, its long side
        // along y: the profile's ends lie at the wall's ends along y.
        let mut turned = terrain_wall();
        turned.rotation = FRAC_PI_2;
        let ends = (
            [209.3321, 121.5711 - 86.92825],
            [209.3321, 121.5711 + 86.92825],
        );
        let Some(top) = top_at_profile((182.0866, 196.5223), ends, &turned) else {
            panic!("the turned wall's profile top was not taken");
        };
        assert!((top - 14.4357).abs() < 1e-4, "{top}");
        // Its ends read along x, as an unturned wall's would be, are not its ends.
        assert_eq!(top_at_profile((182.0866, 196.5223), ENDS, &turned), None);
    }

    #[test]
    fn a_wall_drawn_with_a_thickness_longer_than_its_length_is_read_along_its_depth() {
        let mut upright = terrain_wall();
        upright.width = 0.4921;
        upright.depth = 173.8565;
        upright.rotation = 0.0;
        let along_y = (
            [209.3321, 121.5711 - 86.92825],
            [209.3321, 121.5711 + 86.92825],
        );
        assert!(top_at_profile((182.0866, 196.5223), along_y, &upright).is_some());
    }

    #[test]
    fn numbers_that_are_not_finite_keep_the_box() {
        assert_eq!(
            top_at_profile((182.0866, f64::NAN), ENDS, &terrain_wall()),
            None
        );
        let mut frame = terrain_wall();
        frame.location[2] = f64::INFINITY;
        assert_eq!(top_at_profile((182.0866, 196.5223), ENDS, &frame), None);
        let nan_end = ([f64::NAN, 121.5711], [296.2603, 121.5718]);
        assert_eq!(
            top_at_profile((182.0866, 196.5223), nan_end, &terrain_wall()),
            None
        );
    }
}
