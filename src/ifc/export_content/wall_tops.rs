//! A wall's top cut at the underside of the floor that covers it (#1067,
//! RE-267).
//!
//! Revit cuts a wall's top at the underside of a floor, roof or footing that
//! its top join (`JoinToRoofGStep` with `m_baseOrTop` 1) or one of its
//! `CutPairGStep`s names, over the part of the wall's plan the element
//! covers. This pass lowers the top of the extrusion a wall is drawn as to
//! that underside when all of these hold, measured on rvt-rs's own bodies:
//!
//! - the element is a slab, roof or footing drawn as a plain extrusion (or
//!   several pieces at one height), so that its underside is level across
//!   the wall. A roof drawn from its slopes is a solid and is left out, which
//!   keeps the R2025 roofs off the walls that sit under them. A roof drawn as
//!   a plain extrusion counts only where its record box is exactly its
//!   type's layers and its plan outline is read: a roof by extrusion that the
//!   release's readers do not read is its record box, deeper than its layers,
//!   whose underside is its lowest point and not a level the walls stop at,
//!   and a roof whose outline is not read is drawn over its box's rectangle,
//!   which an L-shaped roof does not fill (RE-351);
//! - the underside lies between the wall's base and top;
//! - the element's plan covers the whole of the wall's plan and none of the
//!   wall's plan lies in an opening of the element: the wall's plan outside
//!   the element's measures under [`COVER_TOLERANCE_SQUARE_FEET`].
//!
//! A wall the element covers in part is stepped, below (#1067, RE-320). A
//! wall of which any of this is not known is left at its full height. Several
//! elements may qualify; the wall is cut at the lowest underside, since a wall
//! cannot rise through a floor above one that has cut it.
//!
//! The same body, a lower top: the placement and `BoundingBoxHeight` stay,
//! and the wall carries a `BodyHeightSource` and the element's ElementId. The
//! wall gets no `Qto_WallBaseQuantities.Height`, which would be Revit's on 62
//! of the BIM4LCA office's 64 cut walls as built and 66 of 70 at permit, but
//! Revit's export writes none on the others, and nothing read tells them apart
//! (RE-267).
//!
//! # A wall the element covers in part
//!
//! Revit's top is the underside of the floor over the part of the wall's plan
//! the floor covers and the wall's own top elsewhere (RE-267 §2). Where no
//! single element named by the wall covers the whole of its plan, and those
//! that reach it lie inside the wall's height, the wall is drawn as that: a
//! prism of its own plan up to the lowest underside and, over each part of the
//! plan not covered at or below an underside, a prism from that underside to
//! the next one (the wall's top for the last), one item each in the one body
//! representation. The parts are convex pieces: the plan (itself where it is
//! convex, else its ear-clipped triangles joined where two make a convex
//! piece) less each triangle of an outline in turn, with the pieces that share
//! an edge and make a convex piece together joined. The holes and openings of
//! an element count as uncovered where one element alone is named.
//!
//! An element that only a cut pair names, and never the top join, cuts the
//! top only where it reaches the wall's top: a Join Geometry cut takes the
//! volume the two share, which is the wall's top only where the element rises
//! to it (RE-320 §5).
//!
//! A wall whose pieces pass [`MAX_RISERS`], whose plan does not break into
//! convex pieces, or whose several elements share a hole or opening that lies
//! over it is left as it was. The wall's `extrusion` stays the full prism,
//! which the passes that read a wall's plan and its record's box use; the
//! writer, the glTF and the plan outline draw its `solid_shape`.
//!
//! A wall of which all that lies outside the floor is a small stub inside
//! another wall (see [`embedded`]) is cut whole, as Revit cuts it (RE-375): the
//! one such wall measured, 3004978 of the BIM4LCA office as built, has Revit's
//! top over its whole plan, the stub included. It carries
//! [`WALL_TOP_STUB_PROPERTY`], and the rule rests on that one wall: nothing in
//! the 225 pieces of Revit's wall plans outside a floor that RE-375 reads, in
//! five buildings, is under the stub's size and inside other walls but the
//! stub itself, and Revit keeps the wall's own height over most others.
//!
//! # A wall's base (#1384, RE-336)
//!
//! A wall's base join (`JoinToRoofGStep` with `m_baseOrTop` 0) takes its base
//! up to the top of the floor or footing it names, over the part of the wall's
//! plan the element covers. [`cut_wall_bases`] does this for a wall the one
//! named element covers whole, when that top lies between the wall's base and
//! top over all of the plan: at the element's level where it is drawn as a
//! plain extrusion, and along the plane of its shape-edited corners where the
//! floor is drawn as a solid between that plane and its underside (the
//! `native_slab_shape_edit` body of `export_content`). The wall's body is then
//! its profile from that top to its own, the `solid_shape`, and the wall
//! carries a `BodyBaseSource` and `BaseCoverElementId` (a wall the top join
//! cut as well keeps its `BodyHeightSource`: one property set cannot hold two
//! of a name).
//!
//! A named element that covers only part of the wall's plan cuts the base over
//! that part (#1411, RE-349): Revit takes the base up to the element's top where
//! the two share plan and leaves it at the wall's own base where they do not,
//! so the wall is the convex pieces of its plan inside the element's outline,
//! cut at its top, and the rest of the plan from the wall's base
//! (`BodyBaseSource` `wall_base_cut_at_floor_covering_in_part` or
//! `wall_base_sloped_at_floor_covering_in_part`). The element's top over the
//! covered part must lie between the wall's base and top, as for a whole cover.
//! An element that covers the whole plan decides first: a wall whose base join
//! names one takes no cut over part of its plan, whether that cover came to a cut
//! or not, and one that covers part counts only where none covers the whole. The
//! covered part must be read: a wall that is not made of convex pieces, a hole or
//! opening of the element over it, covered pieces that overlap one another (the
//! element's loops or its outline's triangles overlap), none inside, or more than
//! 32 pieces keep the wall's base. A wall whose cut resolves and whose solid
//! cannot be made keeps its prism and says so (`base_cut_solid_not_made`); a wall
//! or element with a clockwise or concave ring is made of counter-clockwise
//! convex pieces, one body item for each, so an arc has many.
//! A part under 1e-4 square feet (RE-320's step tolerance, not measured for
//! bases: the smallest part measured is the 0.20 ft2 (0.45 ft2 once the ends are carried on) that the
//! footing leaves of the two walls, against the 3.6 ft2 it covers) is no part.
//!
//! A wall of which this is not proven keeps its base. When it names a floor
//! with shape-edited corners it carries the reason as `BaseCoverRefusal`: the
//! edits of a floor, roof or ceiling are not read, the wall is not one plain
//! extrusion, no named element drawn as a body covers its plan or two do, the
//! covered part of a plan it covers in part is not read, or the top of the one
//! that does is at or below the wall's base, at or above its top, or crosses
//! one of them.

use std::collections::{BTreeMap, HashMap, HashSet};

use super::{
    ELEMENT_RECORD_PROPERTY_SET, FLOOR_SHAPE_BODY_SOURCE, WALL_BASE_COVER_BODY_BASE_SOURCE,
    WALL_BASE_COVER_IN_PART_BODY_BASE_SOURCE, WALL_BASE_REFUSAL_PROPERTY,
    WALL_BASE_SLOPED_BODY_BASE_SOURCE, WALL_BASE_SLOPED_IN_PART_BODY_BASE_SOURCE,
    WALL_BASE_SOURCE_PROPERTY, WALL_END_AT_JOINED_WALLS_BODY_END_SOURCE,
    WALL_END_NOT_MARKED_PROPERTY, WALL_END_ORDER_DEPENDENT_PROPERTY, WALL_END_REFUSAL_PROPERTY,
    WALL_END_SOURCE_PROPERTY, WALL_TOP_COVER_BODY_HEIGHT_SOURCE,
    WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE, WALL_TOP_STUB_INSIDE_WALL, WALL_TOP_STUB_PROPERTY,
};
use crate::ifc::body_geometry::{
    Body, Placement, PlanPlane, Point2, Ring, between_surfaces_mesh, extrusion_rings, plan_outline,
    section,
};
use crate::ifc::entities::{
    BrepTriangle, Extrusion, IfcEntity, ProfileDef, Property, PropertyValue, SolidShape,
};

/// How much of a wall's plan may lie outside its floor's, or inside the
/// floor's openings, and still be covered, square feet. On the BIM4LCA office
/// as built the walls a floor covers measure at most 4e-6 outside it, and the
/// least among the walls it covers in part is 0.0127 (RE-267).
const COVER_TOLERANCE_SQUARE_FEET: f64 = 1e-4;

/// How far a ring's triangulated area may differ from its own, square feet.
const TRIANGULATION_TOLERANCE_SQUARE_FEET: f64 = 1e-6;

/// How far a floor's underside must lie inside the wall's height, and how
/// far its pieces may differ in height, feet.
const UNDERSIDE_TOLERANCE_FEET: f64 = 1e-3;

/// How much of a wall's plan the elements it is stepped at must cover, and
/// how much it must leave uncovered, square feet: the same tolerance as
/// [`COVER_TOLERANCE_SQUARE_FEET`].
const STEP_TOLERANCE_SQUARE_FEET: f64 = COVER_TOLERANCE_SQUARE_FEET;

/// The most convex pieces a wall's plan outside the elements may come to.
const MAX_RISERS: usize = 32;

/// The most parts of a wall's plan that an element covers, before they are
/// joined into at most [`MAX_RISERS`] pieces: thirty-two times that.
const MAX_COVERED_PARTS: usize = 32 * MAX_RISERS;

/// The area under which what a floor leaves of a wall's plan, all of it inside
/// other walls, may be a stub that Revit cuts although the floor does not cover
/// it, square feet: the one measured, 0.0243 ft2 at the end of a wall, inside
/// the plan of the wall it abuts, is cut by Revit with the rest of the wall.
/// The walls whose small pieces inside other walls Revit keeps (2984235,
/// 2984610, 2992096 and 1478932 on the BIM4LCA office as built) also have
/// larger parts of their plans outside the floor (RE-320 §4). Of the 225
/// pieces of Revit's wall plans outside a floor that RE-375 reads in five
/// buildings, the stub is the one under it and inside other walls.
const EMBEDDED_PIECE_SQUARE_FEET: f64 = 0.05;

/// How much height two walls must share to be one beside the other, feet.
const SHARED_HEIGHT_FEET: f64 = 0.5;

/// The area under which a piece of a plan is no piece, square feet: rounding
/// where a triangle's edge lies along another edge.
const PIECE_ROUNDING_SQUARE_FEET: f64 = 1e-9;

/// How near two points of two pieces must be to be one, feet.
const JOIN_TOLERANCE_FEET: f64 = 1e-7;

/// The entities a wall's top may be cut at (the scorer's `IfcSlab`,
/// `IfcRoof` and `IfcFooting`).
const CUTTER_TYPES: [&str; 3] = ["IFCSLAB", "IFCROOF", "IFCFOOTING"];

type Triangle = [Point2; 3];

/// A wall drawn as one extrusion: its base and top and its plan, counter-
/// clockwise, in model feet.
struct Wall {
    base: f64,
    top: f64,
    plan: Ring,
    area: f64,
    bounds: [f64; 4],
}

/// A wall whose top join or cut pairs name elements, waiting for the pass.
pub(super) struct Pending {
    /// The wall's entity index.
    pub index: usize,
    /// The ElementIds its top join and cut pairs name.
    pub named: Vec<u32>,
    /// Those that only a cut pair names (RE-320).
    pub pair_only: Vec<u32>,
}

/// A floor, roof or footing: its underside and top, the triangles of its
/// outline and the triangles of its holes and openings, in model feet.
struct Cutter {
    underside: f64,
    top: f64,
    outline: Vec<Triangle>,
    cutouts: Vec<Triangle>,
}

/// The signed area of `ring`, positive when it runs counter-clockwise.
fn area(ring: &[Point2]) -> f64 {
    let Some(&(ox, oy)) = ring.first() else {
        return 0.0;
    };
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .map(|(a, b)| (a.0 - ox) * (b.1 - oy) - (b.0 - ox) * (a.1 - oy))
        .sum::<f64>()
        / 2.0
}

/// The bounds of `ring`: least x and y, then greatest x and y.
fn bounds(ring: &[Point2]) -> [f64; 4] {
    ring.iter().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)],
    )
}

/// The area of `subject` inside `triangle` (Sutherland-Hodgman: the subject
/// may be concave, the clip is convex).
fn clipped_area(subject: &[Point2], triangle: &Triangle) -> f64 {
    let mut corners = *triangle;
    if area(&corners) < 0.0 {
        corners.swap(1, 2);
    }
    let mut ring: Ring = subject.to_vec();
    for (a, b) in [
        (corners[0], corners[1]),
        (corners[1], corners[2]),
        (corners[2], corners[0]),
    ] {
        let side = |p: Point2| (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0);
        let mut next: Ring = Vec::with_capacity(ring.len() + 2);
        for (&p, &q) in ring.iter().zip(ring.iter().cycle().skip(1)) {
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                next.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                let t = sp / (sp - sq);
                next.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
            }
        }
        if next.len() < 3 {
            return 0.0;
        }
        ring = next;
    }
    area(&ring).max(0.0)
}

fn overlap(wall: &Wall, triangle: &Triangle) -> f64 {
    let b = bounds(triangle);
    if b[2] < wall.bounds[0]
        || b[0] > wall.bounds[2]
        || b[3] < wall.bounds[1]
        || b[1] > wall.bounds[3]
    {
        return 0.0;
    }
    clipped_area(&wall.plan, triangle)
}

/// Whether the plan of `cutter` covers the whole of the wall's, clear of its
/// holes and openings: the wall's plan outside the outline under
/// [`COVER_TOLERANCE_SQUARE_FEET`], and no more of it inside than the wall
/// has (triangles that overlap), and under that much of it in the cutouts.
fn covers(cutter: &Cutter, wall: &Wall) -> bool {
    let inside: f64 = cutter.outline.iter().map(|t| overlap(wall, t)).sum();
    if (wall.area - inside).abs() > COVER_TOLERANCE_SQUARE_FEET {
        return false;
    }
    let cut: f64 = cutter.cutouts.iter().map(|t| overlap(wall, t)).sum();
    cut <= COVER_TOLERANCE_SQUARE_FEET
}

/// `ring` counter-clockwise.
fn counter_clockwise(mut ring: Ring) -> Ring {
    if area(&ring) < 0.0 {
        ring.reverse();
    }
    ring
}

/// Which side of the line from `a` to `b` the point `p` lies on: positive on
/// the left.
fn side_of(a: Point2, b: Point2, p: Point2) -> f64 {
    (b.0 - a.0) * (p.1 - a.1) - (b.1 - a.1) * (p.0 - a.0)
}

/// The part of the convex `ring` on the left of the line from `a` to `b`, or
/// on its right.
fn half_plane(ring: &[Point2], a: Point2, b: Point2, left: bool) -> Ring {
    let sign = if left { 1.0 } else { -1.0 };
    let mut kept: Ring = Vec::with_capacity(ring.len() + 2);
    for (&p, &q) in ring.iter().zip(ring.iter().cycle().skip(1)) {
        let (sp, sq) = (sign * side_of(a, b, p), sign * side_of(a, b, q));
        if sp >= 0.0 {
            kept.push(p);
        }
        if (sp >= 0.0) != (sq >= 0.0) {
            let t = sp / (sp - sq);
            kept.push((p.0 + t * (q.0 - p.0), p.1 + t * (q.1 - p.1)));
        }
    }
    kept
}

/// The part of the convex `subject` inside the convex, counter-clockwise
/// `clip`: fewer than three points where it has no area.
fn clip_convex(subject: &[Point2], clip: &[Point2]) -> Ring {
    let mut ring: Ring = subject.to_vec();
    for (&a, &b) in clip.iter().zip(clip.iter().cycle().skip(1)) {
        ring = half_plane(&ring, a, b, true);
        if ring.len() < 3 {
            return Vec::new();
        }
    }
    ring
}

/// The convex pieces of the convex `piece` that lie outside the convex,
/// counter-clockwise `clip`: where `clip` does not reach it, the piece
/// whole.
fn subtract_convex(piece: &[Point2], clip: &[Point2]) -> Vec<Ring> {
    if area(&clip_convex(piece, clip)).abs() <= PIECE_ROUNDING_SQUARE_FEET {
        return vec![piece.to_vec()];
    }
    let mut outside = Vec::new();
    let mut inside: Ring = piece.to_vec();
    for (&a, &b) in clip.iter().zip(clip.iter().cycle().skip(1)) {
        let part = half_plane(&inside, a, b, false);
        if part.len() >= 3 && area(&part) > PIECE_ROUNDING_SQUARE_FEET {
            outside.push(part);
        }
        inside = half_plane(&inside, a, b, true);
        if inside.len() < 3 {
            break;
        }
    }
    outside
}

fn near(p: Point2, q: Point2) -> bool {
    (p.0 - q.0).abs() <= JOIN_TOLERANCE_FEET && (p.1 - q.1).abs() <= JOIN_TOLERANCE_FEET
}

/// `ring` without a point that repeats its neighbour or lies on the line
/// between its neighbours.
fn tidy(mut ring: Ring) -> Ring {
    loop {
        let n = ring.len();
        if n < 3 {
            return ring;
        }
        let redundant = (0..n).find(|&i| {
            let (a, b, c) = (ring[(i + n - 1) % n], ring[i], ring[(i + 1) % n]);
            let span = ((c.0 - a.0).powi(2) + (c.1 - a.1).powi(2)).sqrt();
            near(a, b)
                || span <= JOIN_TOLERANCE_FEET
                || side_of(a, c, b).abs() <= JOIN_TOLERANCE_FEET * span
        });
        match redundant {
            Some(i) => {
                ring.remove(i);
            }
            None => return ring,
        }
    }
}

/// Whether the counter-clockwise `ring` turns left at every corner.
fn is_convex(ring: &[Point2]) -> bool {
    let n = ring.len();
    n >= 3 && (0..n).all(|i| side_of(ring[i], ring[(i + 1) % n], ring[(i + 2) % n]) >= -1e-9)
}

/// The convex pieces `a` and `b` make together, where they share an edge
/// and the whole is convex.
fn join_convex(a: &[Point2], b: &[Point2]) -> Option<Ring> {
    let (n, m) = (a.len(), b.len());
    for s in 0..n {
        let (p0, p1) = (a[s], a[(s + 1) % n]);
        for t in 0..m {
            let (q0, q1) = (b[t], b[(t + 1) % m]);
            if !(near(p0, q1) && near(p1, q0)) {
                continue;
            }
            let mut ring: Ring = Vec::with_capacity(n + m - 2);
            ring.extend((0..n).map(|k| a[(s + 1 + k) % n]));
            ring.extend((0..m - 2).map(|k| b[(t + 2 + k) % m]));
            let ring = tidy(ring);
            return is_convex(&ring).then_some(ring);
        }
    }
    None
}

/// `pieces` with every two that make a convex piece together joined.
fn join_pieces(mut pieces: Vec<Ring>) -> Vec<Ring> {
    loop {
        let found = (0..pieces.len()).find_map(|i| {
            ((i + 1)..pieces.len())
                .find_map(|j| join_convex(&pieces[i], &pieces[j]).map(|ring| (i, j, ring)))
        });
        let Some((i, j, ring)) = found else {
            return pieces;
        };
        pieces.remove(j);
        pieces[i] = ring;
    }
}

/// How much of the wall's plan `cutter` covers, square feet: inside its
/// outline and not inside its holes and openings.
fn reach(cutter: &Cutter, wall: &Wall) -> f64 {
    let inside: f64 = cutter.outline.iter().map(|t| overlap(wall, t)).sum();
    let cut: f64 = cutter.cutouts.iter().map(|t| overlap(wall, t)).sum();
    inside - cut
}

/// Whether what the floor leaves of a wall's plan, the convex `pieces`, is
/// small and lies wholly inside the plans of other walls: the stub of a wall
/// that butts into another, which Revit cuts at the floor's underside with the
/// rest of the wall (RE-320 §4). No width or area of a piece separates it from
/// the pieces Revit keeps, some of them inside other walls too; only a plan
/// with nothing else outside the floor does, so a wall of which that is all is
/// cut whole, by [`steps`] (RE-375: the rule rests on the one wall).
fn embedded(pieces: &[Ring], neighbours: &[&Wall]) -> bool {
    let size: f64 = pieces.iter().map(|piece| area(piece)).sum();
    if pieces.is_empty() || size >= EMBEDDED_PIECE_SQUARE_FEET {
        return false;
    }
    let parts: Vec<Ring> = neighbours
        .iter()
        .filter_map(|other| convex_pieces(&other.plan))
        .flatten()
        .collect();
    let mut left: Vec<Ring> = pieces.to_vec();
    for part in &parts {
        left = left
            .iter()
            .flat_map(|piece| subtract_convex(piece, part))
            .collect();
        // Each neighbour piece can split each piece left in three, so a
        // crafted file's neighbours grew `left` without end (#1845). Nothing
        // left is all inside; more than `steps` holds is no stub.
        if left.is_empty() {
            return true;
        }
        if left.len() > 4 * MAX_RISERS {
            crate::budget::stopped("wall_top_embedded_pieces");
            return false;
        }
    }
    left.iter().map(|piece| area(piece)).sum::<f64>() <= 0.01 * size
}

/// The convex pieces `plan` is made of: itself where it is convex, else its
/// ear-clipped triangles with every two that make a convex piece together
/// joined. `None` where the triangulation is not the plan's area.
fn convex_pieces(plan: &[Point2]) -> Option<Vec<Ring>> {
    if is_convex(plan) {
        return Some(vec![plan.to_vec()]);
    }
    let cap = section(plan, &[])?;
    let mut drawn = 0.0;
    let mut triangles: Vec<Ring> = Vec::with_capacity(cap.cap_triangles.len());
    for corners in &cap.cap_triangles {
        let mut triangle: Ring = Vec::with_capacity(3);
        for &corner in corners {
            triangle.push(*cap.cap_points.get(usize::try_from(corner).ok()?)?);
        }
        let triangle = counter_clockwise(triangle);
        drawn += area(&triangle);
        triangles.push(triangle);
    }
    let expected = area(plan);
    let tolerance = TRIANGULATION_TOLERANCE_SQUARE_FEET + 1e-9 * expected;
    if (drawn - expected).abs() > tolerance {
        return None;
    }
    let pieces = join_pieces(triangles);
    pieces
        .iter()
        .all(|piece| is_convex(piece))
        .then_some(pieces)
}

/// The elements a wall's top is stepped at, by underside, lowest first.
type Levels<'a> = Vec<(f64, Vec<&'a Cutter>)>;

/// What [`steps`] finds: the pieces of the plan above each underside, and
/// whether the lowest underside's were a stub inside other walls, which is
/// dropped (RE-375), so that the wall is cut whole at it.
struct Stepped {
    levels: Vec<(f64, Vec<Ring>)>,
    stub: bool,
}

/// The convex pieces of the wall's plan that lie above each underside of
/// `levels`: outside the outlines of the elements at that underside and the
/// lower ones, and inside their holes and openings. Each is the part of the
/// plan whose top is higher than that underside. `None` where there are more
/// than [`MAX_RISERS`] pieces, where the plan does not break into convex
/// pieces, or where several elements share a hole or opening the wall
/// crosses. What the lowest underside leaves is dropped where it is a stub
/// inside other walls ([`embedded`]).
fn steps(wall: &Wall, levels: &[(f64, Vec<&Cutter>)], neighbours: &[&Wall]) -> Option<Stepped> {
    let alone = levels.len() == 1 && levels[0].1.len() == 1;
    let base = convex_pieces(&wall.plan)?;
    let mut pieces = base.clone();
    let mut out = Vec::with_capacity(levels.len());
    let mut stub = false;
    for (level, group) in levels {
        for cutter in group {
            for triangle in &cutter.outline {
                if overlap(wall, triangle) <= PIECE_ROUNDING_SQUARE_FEET {
                    continue;
                }
                let clip = counter_clockwise(triangle.to_vec());
                pieces = pieces
                    .iter()
                    .flat_map(|piece| subtract_convex(piece, &clip))
                    .collect();
                if pieces.len() > 4 * MAX_RISERS {
                    crate::budget::stopped("wall_top_pieces");
                    return None;
                }
            }
        }
        let mut held: Vec<Ring> = Vec::new();
        for cutter in group {
            for triangle in &cutter.cutouts {
                if overlap(wall, triangle) <= PIECE_ROUNDING_SQUARE_FEET {
                    continue;
                }
                if !alone {
                    return None;
                }
                let clip = counter_clockwise(triangle.to_vec());
                let mut parts: Vec<Ring> = base
                    .iter()
                    .map(|piece| clip_convex(piece, &clip))
                    .filter(|part| part.len() >= 3 && area(part) > PIECE_ROUNDING_SQUARE_FEET)
                    .collect();
                for other in pieces.iter().chain(held.iter()) {
                    parts = parts
                        .iter()
                        .flat_map(|piece| subtract_convex(piece, other))
                        .collect();
                }
                held.extend(parts);
                if held.len() > 4 * MAX_RISERS {
                    crate::budget::stopped("wall_top_pieces");
                    return None;
                }
            }
        }
        let mut above: Vec<Ring> =
            join_pieces(pieces.iter().cloned().chain(held).map(tidy).collect())
                .into_iter()
                .filter(|piece| area(piece) >= STEP_TOLERANCE_SQUARE_FEET)
                .collect();
        if above.len() > MAX_RISERS {
            return None;
        }
        if out.is_empty() && embedded(&above, neighbours) {
            above.clear();
            stub = true;
        }
        out.push((*level, above));
    }
    Some(Stepped { levels: out, stub })
}

/// The wall `entity` is, where it is one plain extrusion with a plan and no
/// holes.
fn build_wall(entity: &IfcEntity) -> Option<Wall> {
    let IfcEntity::BuildingElement {
        ifc_type,
        extrusion: Some(body),
        solid_shape: None,
        representation_map_index: None,
        location_feet: Some(location),
        rotation_radians,
        ..
    } = entity
    else {
        return None;
    };
    if !matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE")
        || !(body.height_feet.is_finite() && body.height_feet > 0.0)
        || !location.iter().all(|v| v.is_finite())
    {
        return None;
    }
    let placement = Placement::new(Some(*location), *rotation_radians);
    let (mut plan, holes) = plan_outline(Body::Extrusion(body), &placement)?;
    if !holes.is_empty() {
        return None;
    }
    if area(&plan) < 0.0 {
        plan.reverse();
    }
    let size = area(&plan);
    if !(size.is_finite() && size > TRIANGULATION_TOLERANCE_SQUARE_FEET) {
        return None;
    }
    Some(Wall {
        base: location[2],
        top: location[2] + body.height_feet,
        bounds: bounds(&plan),
        plan,
        area: size,
    })
}

/// The triangles of the area inside `ring`, in model feet through
/// `placement`, or `None` where the triangulation is not that area.
fn ring_triangles(ring: &[Point2], placement: &Placement) -> Option<Vec<Triangle>> {
    let expected = area(ring).abs();
    let cap = section(ring, &[])?;
    let mut triangles = Vec::with_capacity(cap.cap_triangles.len());
    let mut drawn = 0.0;
    for corners in &cap.cap_triangles {
        let mut triangle: Triangle = [(0.0, 0.0); 3];
        for (slot, &corner) in triangle.iter_mut().zip(corners) {
            let (x, y) = *cap.cap_points.get(usize::try_from(corner).ok()?)?;
            let at = placement.apply([x, y, 0.0]);
            *slot = (at[0], at[1]);
        }
        drawn += area(&triangle).abs();
        triangles.push(triangle);
    }
    let tolerance = TRIANGULATION_TOLERANCE_SQUARE_FEET + 1e-9 * expected;
    ((drawn - expected).abs() <= tolerance).then_some(triangles)
}

/// The two triangles of the bounds of `ring`, in model feet through
/// `placement`: the area of a hole that does not triangulate, and more.
fn box_triangles(ring: &[Point2], placement: &Placement) -> Vec<Triangle> {
    let [x0, y0, x1, y1] = bounds(ring);
    let at = |x: f64, y: f64| {
        let p = placement.apply([x, y, 0.0]);
        (p[0], p[1])
    };
    let (a, b, c, d) = (at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1));
    vec![[a, b, c], [a, c, d]]
}

/// The slab, roof or footing made of the entities `pieces` (a floor of
/// separate loops is several under one Tag), where each is one plain
/// extrusion whose outline triangulates, and they stand at one height. With
/// `shape_edited`, each is instead a floor drawn along the plane of its
/// shape-edited corners (its `BodySource` is [`FLOOR_SHAPE_BODY_SOURCE`], not
/// any other solid) and all are as thick as one another (#1384).
fn build_cutter(
    entities: &[IfcEntity],
    pieces: &[usize],
    voids: &HashMap<usize, Vec<&[Point2]>>,
    shape_edited: bool,
) -> Option<Cutter> {
    let mut heights: Option<(f64, f64)> = None;
    let mut top = f64::NEG_INFINITY;
    let mut outline = Vec::new();
    let mut cutouts = Vec::new();
    let mut shared_height: Option<f64> = None;
    for &index in pieces {
        let IfcEntity::BuildingElement {
            extrusion: Some(body),
            solid_shape,
            representation_map_index: None,
            location_feet: Some(location),
            rotation_radians,
            property_set,
            ..
        } = entities.get(index)?
        else {
            return None;
        };
        // A floor whose shape is edited is drawn as a solid between its sloped
        // top and bottom (#1384); its extrusion is the box prism, which gives
        // its outline.
        match (solid_shape, shape_edited) {
            (None, false) | (Some(SolidShape::FacetedBrep { .. }), true) => {}
            _ => return None,
        }
        if !(body.height_feet.is_finite() && body.height_feet > 0.0)
            || !location.iter().all(|v| v.is_finite())
        {
            return None;
        }
        if shape_edited {
            let drawn_along_plane = property_set.as_ref().is_some_and(|set| {
                set.properties.iter().any(|property| {
                    property.name == "BodySource"
                        && matches!(
                            &property.value,
                            PropertyValue::Text(source) if source == FLOOR_SHAPE_BODY_SOURCE
                        )
                })
            });
            let height = *shared_height.get_or_insert(body.height_feet);
            if !drawn_along_plane || (height - body.height_feet).abs() > UNDERSIDE_TOLERANCE_FEET {
                return None;
            }
        }
        let (least, greatest) = heights.unwrap_or((location[2], location[2]));
        heights = Some((least.min(location[2]), greatest.max(location[2])));
        top = top.max(location[2] + body.height_feet);
        let placement = Placement::new(Some(*location), *rotation_radians);
        let (outer, holes) = extrusion_rings(body)?;
        outline.extend(ring_triangles(&outer, &placement)?);
        let openings = voids.get(&index).into_iter().flatten().copied();
        for ring in holes.iter().map(Vec::as_slice).chain(openings) {
            if ring.len() >= 3 {
                cutouts.extend(
                    ring_triangles(ring, &placement)
                        .unwrap_or_else(|| box_triangles(ring, &placement)),
                );
            }
        }
    }
    let (least, greatest) = heights?;
    (greatest - least <= UNDERSIDE_TOLERANCE_FEET).then_some(Cutter {
        underside: least,
        top,
        outline,
        cutouts,
    })
}

/// How a wall's top is cut: at one underside across its plan, or at an
/// underside over part of it and higher over the rest.
enum Cut {
    /// At `level` over the whole plan, by the element `id`. With `stub`, the
    /// element covers all of the plan but a stub inside other walls, which
    /// Revit cuts with the rest (RE-375).
    Whole { level: f64, id: u32, stub: bool },
    /// The undersides, lowest first, each with the pieces of the plan above
    /// it, and the ElementIds of the elements they are cut at.
    Stepped {
        steps: Vec<(f64, Vec<Ring>)>,
        ids: Vec<u32>,
    },
}

/// Whether the slab, roof or footing made of `pieces` (ElementId `id`) has an
/// underside a wall's top may be cut at: any slab or footing, and a roof only
/// where it is one of `plates`, its record box exactly its type's layers and
/// its plan outline read (RE-351). A roof whose box is deeper than its layers
/// is not a level plate (a roof by extrusion, a slope its reader does not
/// read): its box is drawn as a plain extrusion, but the box's underside is
/// its lowest point, not a level the walls under it stop at. The layers are
/// the proof that the underside is level; the outline is what the roof covers.
fn level_underside(
    entities: &[IfcEntity],
    pieces: &[usize],
    id: u32,
    plates: &HashSet<u32>,
) -> bool {
    plates.contains(&id)
        || !pieces.iter().any(|&index| {
            matches!(
                entities.get(index),
                Some(IfcEntity::BuildingElement { ifc_type, .. }) if ifc_type.as_str() == "IFCROOF"
            )
        })
}

/// For each of `pending` (a wall's entity index and the ElementIds its top
/// join and cut pairs name) that is cut: its index and how.
fn decide(entities: &[IfcEntity], pending: &[Pending], plates: &HashSet<u32>) -> Vec<(usize, Cut)> {
    let Gathered {
        pieces,
        voids,
        profiled,
    } = gather(entities);
    let walls: Vec<(usize, Wall)> = entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| build_wall(entity).map(|wall| (index, wall)))
        .collect();
    let mut cutters: HashMap<u32, Option<Cutter>> = HashMap::new();
    let mut cuts = Vec::new();
    for Pending {
        index,
        named,
        pair_only,
    } in pending
    {
        if profiled.contains(index) {
            continue;
        }
        let Some(wall) = entities.get(*index).and_then(build_wall) else {
            continue;
        };
        let mut lowest: Option<(f64, u32)> = None;
        let mut partial: Vec<(f64, u32)> = Vec::new();
        for &id in named {
            let Some(found) = pieces.get(&id) else {
                continue;
            };
            if !level_underside(entities, found, id, plates) {
                continue;
            }
            let Some(cutter) = cutters
                .entry(id)
                .or_insert_with(|| build_cutter(entities, found, &voids, false))
                .as_ref()
            else {
                continue;
            };
            let level = cutter.underside;
            if !(level > wall.base + UNDERSIDE_TOLERANCE_FEET
                && level < wall.top - UNDERSIDE_TOLERANCE_FEET)
            {
                continue;
            }
            if pair_only.contains(&id) && cutter.top < wall.top - UNDERSIDE_TOLERANCE_FEET {
                continue;
            }
            if covers(cutter, &wall) {
                if lowest.is_none_or(|(held, _)| level < held) {
                    lowest = Some((level, id));
                }
            } else if reach(cutter, &wall) > STEP_TOLERANCE_SQUARE_FEET {
                partial.push((level, id));
            }
        }
        if let Some((level, id)) = lowest {
            cuts.push((
                *index,
                Cut::Whole {
                    level,
                    id,
                    stub: false,
                },
            ));
            continue;
        }
        if partial.is_empty() {
            continue;
        }
        partial.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        let mut levels: Levels<'_> = Vec::new();
        let mut ids: Vec<Vec<u32>> = Vec::new();
        let mut held: Option<f64> = None;
        for &(level, id) in &partial {
            let Some(cutter) = cutters.get(&id).and_then(Option::as_ref) else {
                continue;
            };
            if held.is_some_and(|first| (level - first).abs() <= UNDERSIDE_TOLERANCE_FEET) {
                if let (Some((_, group)), Some(named)) = (levels.last_mut(), ids.last_mut()) {
                    group.push(cutter);
                    named.push(id);
                }
            } else {
                held = Some(level);
                levels.push((level, vec![cutter]));
                ids.push(vec![id]);
            }
        }
        let neighbours: Vec<&Wall> = walls
            .iter()
            .filter(|(other_index, other)| {
                other_index != index
                    && wall.top.min(other.top) - wall.base.max(other.base) > SHARED_HEIGHT_FEET
                    && other.bounds[0] <= wall.bounds[2]
                    && other.bounds[2] >= wall.bounds[0]
                    && other.bounds[1] <= wall.bounds[3]
                    && other.bounds[3] >= wall.bounds[1]
            })
            .map(|(_, other)| other)
            .collect();
        let Some(found) = steps(&wall, &levels, &neighbours) else {
            continue;
        };
        let lowest_id = ids.first().and_then(|named| named.first()).copied();
        let Some(lowest_id) = lowest_id else {
            continue;
        };
        if found
            .levels
            .first()
            .is_some_and(|(_, above)| above.is_empty())
        {
            cuts.push((
                *index,
                Cut::Whole {
                    level: found.levels[0].0,
                    id: lowest_id,
                    stub: found.stub,
                },
            ));
        } else {
            cuts.push((
                *index,
                Cut::Stepped {
                    steps: found.levels,
                    ids: ids.into_iter().flatten().collect(),
                },
            ));
        }
    }
    cuts
}

/// `point` in the local frame of `placement`.
fn to_local(placement: &Placement, point: Point2) -> Point2 {
    let (dx, dy) = (point.0 - placement.origin[0], point.1 - placement.origin[1]);
    (
        placement.cos * dx + placement.sin * dy,
        placement.cos * dy - placement.sin * dx,
    )
}

/// Cut the top of each wall of `pending` at the underside of the floor that
/// covers it (see the module): a whole wall's extrusion height becomes the
/// underside less its base, and a wall a floor covers in part gets the stepped
/// body of the module as its `solid_shape`.
pub(super) fn cut_wall_tops(
    entities: &mut [IfcEntity],
    pending: &[Pending],
    plates: &HashSet<u32>,
) {
    if pending.is_empty() {
        return;
    }
    let cuts = decide(entities, pending, plates);
    for (index, cut) in cuts {
        let Some(IfcEntity::BuildingElement {
            extrusion: Some(body),
            location_feet: Some(location),
            rotation_radians,
            solid_shape,
            property_set,
            ..
        }) = entities.get_mut(index)
        else {
            continue;
        };
        let (named, source, stub) = match cut {
            Cut::Whole { level, id, stub } => {
                body.height_feet = level - location[2];
                (vec![id], WALL_TOP_COVER_BODY_HEIGHT_SOURCE, stub)
            }
            Cut::Stepped { steps, ids } => {
                let placement = Placement::new(Some(*location), *rotation_radians);
                let top = body.height_feet;
                let under = steps[0].0 - location[2];
                let mut items =
                    Vec::with_capacity(1 + steps.iter().map(|s| s.1.len()).sum::<usize>());
                items.push(SolidShape::ExtrudedArea(Extrusion {
                    height_feet: under,
                    ..body.clone()
                }));
                for (k, (level, rings)) in steps.iter().enumerate() {
                    let from = level - location[2];
                    let to = steps.get(k + 1).map_or(top, |next| next.0 - location[2]);
                    items.extend(rings.iter().map(|ring| SolidShape::PlacedExtrusion {
                        profile: ProfileDef::ArbitraryClosed {
                            points: ring.iter().map(|&p| to_local(&placement, p)).collect(),
                        },
                        origin_feet: [0.0, 0.0, from],
                        axis: [0.0, 0.0, 1.0],
                        ref_direction: [1.0, 0.0, 0.0],
                        depth_feet: to - from,
                    }));
                }
                *solid_shape = Some(SolidShape::Compound(items));
                (ids, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE, false)
            }
        };
        if let Some(set) = property_set
            .as_mut()
            .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
        {
            set.properties.push(Property {
                name: "BodyHeightSource".into(),
                value: PropertyValue::Text(source.into()),
            });
            set.properties.push(Property {
                name: "TopCoverElementId".into(),
                value: PropertyValue::Integer(i64::from(named[0])),
            });
            if named.len() > 1 {
                set.properties.push(Property {
                    name: "TopCoverElementIds".into(),
                    value: PropertyValue::Text(
                        named
                            .iter()
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(","),
                    ),
                });
            }
            // RE-375: what the floor left of the plan was a stub, cut with the rest.
            if stub {
                set.properties.push(Property {
                    name: WALL_TOP_STUB_PROPERTY.into(),
                    value: PropertyValue::Text(WALL_TOP_STUB_INSIDE_WALL.into()),
                });
            }
        }
    }
}

/// What the wall pass knows of the top of a floor, roof or ceiling (#1384).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum SlabTop {
    /// Its record was read and no vertex of its top is moved.
    Flat,
    /// Its top is the plane `z = a·x + b·y + c`.
    Plane(PlanPlane),
    /// Its top is not known: its record was not read, or its shape edits are
    /// not, or it is a roof or a ceiling with an edit.
    NotRead,
}

/// A wall whose base join names elements, waiting for the pass (#1384).
pub(super) struct PendingBase {
    /// The wall's entity index.
    pub index: usize,
    /// The ElementIds its base join names.
    pub named: Vec<u32>,
}

/// How a wall's base is cut. `covered` is `None` where the element covers the
/// whole of the wall's plan, and otherwise the convex, counter-clockwise pieces
/// of the plan it covers, in model feet (#1411): the rest of the plan stands
/// from the wall's own base.
enum BaseCut {
    /// At one level.
    Level {
        level: f64,
        id: u32,
        covered: Option<Vec<Ring>>,
    },
    /// Along the plane of a shape-edited top.
    Plane {
        plane: PlanPlane,
        id: u32,
        covered: Option<Vec<Ring>>,
    },
}

impl BaseCut {
    /// The pieces of the wall's plan the cut is made over, where not the whole.
    fn covered(&self) -> Option<&[Ring]> {
        match self {
            BaseCut::Level { covered, .. } | BaseCut::Plane { covered, .. } => covered.as_deref(),
        }
    }
}

/// The convex pieces of the wall's plan inside the outline of `cutter`,
/// counter-clockwise: none where the plan is not made of convex pieces, where
/// a hole or opening of the element lies over it, where two of the pieces
/// overlap (the element's loops, or its outline's triangles, overlap, so that
/// solids made over the pieces would overlap), where nothing is inside, or
/// where it falls in more than [`MAX_RISERS`] pieces (#1411).
fn covered_pieces(cutter: &Cutter, wall: &Wall) -> Option<Vec<Ring>> {
    let cut: f64 = cutter.cutouts.iter().map(|t| overlap(wall, t)).sum();
    if cut > COVER_TOLERANCE_SQUARE_FEET {
        return None;
    }
    let base = convex_pieces(&wall.plan)?;
    let mut inside: Vec<Ring> = Vec::new();
    for triangle in &cutter.outline {
        if overlap(wall, triangle) <= PIECE_ROUNDING_SQUARE_FEET {
            continue;
        }
        let clip = counter_clockwise(triangle.to_vec());
        for piece in &base {
            let part = clip_convex(piece, &clip);
            if part.len() >= 3 && area(&part) > PIECE_ROUNDING_SQUARE_FEET {
                inside.push(part);
            }
        }
        // The outline is every triangle of every entity sharing the
        // element's Tag, so a crafted file made millions of parts before the
        // `MAX_RISERS` check below, and `join_pieces` is quadratic in them
        // (#1845). Joining merges parts, so this bound is the looser one.
        if inside.len() > MAX_COVERED_PARTS {
            crate::budget::stopped("wall_top_covered_parts");
            return None;
        }
    }
    let inside: Vec<Ring> = join_pieces(inside.into_iter().map(tidy).collect());
    if inside.is_empty() || inside.len() > MAX_RISERS {
        return None;
    }
    // Pieces that overlap (the element's loops overlap under one Tag, or its
    // outline's triangles do) would make solids that overlap: the sum of their
    // areas is then more than the wall's plan inside the outline, which no
    // bound on the wall's whole plan sees.
    for (k, piece) in inside.iter().enumerate() {
        for other in inside.iter().skip(k + 1) {
            if area(&clip_convex(piece, other)).abs() > COVER_TOLERANCE_SQUARE_FEET {
                return None;
            }
        }
    }
    Some(inside)
}

/// What the passes read of `entities` besides the walls.
struct Gathered<'a> {
    /// The indices of the floors, roofs and footings under each ElementId.
    pieces: HashMap<u32, Vec<usize>>,
    /// The outlines of the openings that void each, by entity index.
    voids: HashMap<usize, Vec<&'a [Point2]>>,
    /// The walls with an edited elevation profile, by entity index.
    profiled: HashSet<usize>,
}

fn gather(entities: &[IfcEntity]) -> Gathered<'_> {
    let mut pieces: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut voids: HashMap<usize, Vec<&[Point2]>> = HashMap::new();
    let mut profiled: HashSet<usize> = HashSet::new();
    for (index, entity) in entities.iter().enumerate() {
        match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(guid),
                ..
            } if CUTTER_TYPES.contains(&ifc_type.as_str()) => {
                if let Ok(id) = guid.parse::<u32>() {
                    pieces.entry(id).or_default().push(index);
                }
            }
            IfcEntity::VoidOpening {
                host, outline_feet, ..
            } => voids
                .entry(*host)
                .or_default()
                .push(outline_feet.as_slice()),
            IfcEntity::WallProfileOpening { host, .. } => {
                profiled.insert(*host);
            }
            _ => {}
        }
    }
    Gathered {
        pieces,
        voids,
        profiled,
    }
}

/// What the pass decides for a wall whose base join names elements.
enum BaseDecision {
    /// Cut it so.
    Cut(BaseCut),
    /// Leave it: why, as a code.
    Left(&'static str),
}

/// What the pass has read once, for every wall.
struct BaseContext<'a> {
    entities: &'a [IfcEntity],
    gathered: Gathered<'a>,
    shapes: &'a HashMap<u32, SlabTop>,
}

/// How the base of `wall` is cut at the one element of
/// `named` that covers it (#1384): at that element's level, or along the plane
/// its shape-edited corners lie in, when that top lies between the wall's base
/// and top over the whole of the wall's plan, or, where no named element covers
/// the whole plan and one covers part of it, over that part (#1411). A wall that
/// names an element that covers the whole plan is decided by those alone, as it
/// was before. Left
/// as it was, with the reason, where the wall is not one plain extrusion, where
/// a named element's edits are not read, where no named element drawn as a
/// body covers any of the plan (or the covered part is not read), where two
/// cover the whole of it or, with none that does, part of it, or where the top
/// crosses the wall's base or its top. A wall
/// with an edited elevation profile is cut like any other: the openings that
/// profile cuts void the solid as they void the full prism, and Revit raises
/// the base of the Basement walls of the BIM4LCA office that have one to the
/// footing's top as it does the others (RE-336).
fn decide_base(
    context: &BaseContext<'_>,
    cutters: &mut HashMap<u32, Option<Cutter>>,
    wall: Option<Wall>,
    named: &[u32],
) -> BaseDecision {
    let BaseContext {
        entities,
        gathered,
        shapes,
    } = context;
    let Some(wall) = wall else {
        return BaseDecision::Left("wall_not_one_plain_extrusion");
    };
    // What each kind of cover found: the whole of the plan, or part of it.
    #[derive(Default)]
    struct Found {
        eligible: Vec<BaseCut>,
        crossed: bool,
        above: bool,
        below: bool,
    }
    let mut whole = Found::default();
    let mut partial = Found::default();
    let mut built = false;
    let mut partly_unread = false;
    let mut whole_named = false;
    for &id in named {
        // First of all, before the element is looked for among those that cut:
        // a floor, roof or ceiling whose top is not known leaves the wall,
        // whatever else the wall names.
        let top = shapes.get(&id).copied();
        if top == Some(SlabTop::NotRead) {
            return BaseDecision::Left("slab_shape_not_read");
        }
        let plane = match top {
            Some(SlabTop::Plane(plane)) => Some(plane),
            _ => None,
        };
        let Some(found) = gathered.pieces.get(&id) else {
            continue;
        };
        let Some(cutter) = cutters
            .entry(id)
            .or_insert_with(|| build_cutter(entities, found, &gathered.voids, plane.is_some()))
            .as_ref()
        else {
            continue;
        };
        built = true;
        // The whole plan, or the part of it the element covers (#1411): Revit
        // takes the wall's base up to the element's top where the two share
        // plan, and leaves it where they do not.
        let part = if covers(cutter, &wall) {
            whole_named = true;
            None
        } else if reach(cutter, &wall) > STEP_TOLERANCE_SQUARE_FEET {
            let Some(pieces) = covered_pieces(cutter, &wall) else {
                partly_unread = true;
                continue;
            };
            Some(pieces)
        } else {
            continue;
        };
        let kind = if part.is_some() {
            &mut partial
        } else {
            &mut whole
        };
        let (low, high, cut) = match plane {
            Some(plane) => {
                let corners: Vec<Point2> = match &part {
                    Some(pieces) => pieces.iter().flatten().copied().collect(),
                    None => wall.plan.clone(),
                };
                let (low, high) = corners
                    .iter()
                    .map(|&(x, y)| plane[0] * x + plane[1] * y + plane[2])
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), h| {
                        (lo.min(h), hi.max(h))
                    });
                (
                    low,
                    high,
                    BaseCut::Plane {
                        plane,
                        id,
                        covered: part,
                    },
                )
            }
            _ => (
                cutter.top,
                cutter.top,
                BaseCut::Level {
                    level: cutter.top,
                    id,
                    covered: part,
                },
            ),
        };
        if low > wall.base + UNDERSIDE_TOLERANCE_FEET && high < wall.top - UNDERSIDE_TOLERANCE_FEET
        {
            kind.eligible.push(cut);
        } else if high > wall.base + UNDERSIDE_TOLERANCE_FEET
            && low < wall.top - UNDERSIDE_TOLERANCE_FEET
        {
            kind.crossed = true;
        } else if high <= wall.base + UNDERSIDE_TOLERANCE_FEET {
            kind.below = true;
        } else {
            kind.above = true;
        }
    }
    // An element that covers the whole plan decides, as it did before #1411,
    // and a wall that names one takes no cut over part of its plan whatever
    // that cover came to (nothing measured says Revit cuts it at the other).
    // One that covers only part of the plan counts where none covers the whole.
    let decided = if whole_named { whole } else { partial };
    let mut eligible = decided.eligible;
    if let Some(cut) = eligible.pop() {
        return if eligible.is_empty() {
            BaseDecision::Cut(cut)
        } else {
            BaseDecision::Left("several_elements_cover_the_wall")
        };
    }
    if decided.crossed {
        BaseDecision::Left("top_crosses_the_wall_base_or_top")
    } else if decided.above {
        BaseDecision::Left("top_at_or_above_the_wall_top")
    } else if decided.below {
        BaseDecision::Left("top_at_or_below_the_wall_base")
    } else if partly_unread && !whole_named {
        BaseDecision::Left("partly_covered_plan_not_read")
    } else if built {
        BaseDecision::Left("no_element_covers_the_wall_plan")
    } else {
        BaseDecision::Left("named_element_body_not_read")
    }
}

/// The decision for each of `pending` that is cut, or that names an element
/// with shape-edited corners and is left: a wall that names only unedited
/// elements and is not cut was not going to be raised by a sloped top, and
/// carries no reason.
fn decide_bases(
    entities: &[IfcEntity],
    pending: &[PendingBase],
    shapes: &HashMap<u32, SlabTop>,
) -> Vec<(usize, BaseDecision)> {
    let context = BaseContext {
        entities,
        gathered: gather(entities),
        shapes,
    };
    let mut cutters: HashMap<u32, Option<Cutter>> = HashMap::new();
    let mut decisions = Vec::new();
    for PendingBase { index, named } in pending {
        let decision = decide_base(
            &context,
            &mut cutters,
            entities.get(*index).and_then(build_wall),
            named,
        );
        let edited = named
            .iter()
            .any(|id| matches!(shapes.get(id), Some(SlabTop::Plane(_) | SlabTop::NotRead)));
        if matches!(decision, BaseDecision::Cut(_)) || edited {
            decisions.push((*index, decision));
        }
    }
    decisions
}

/// Cut the base of each wall of `pending` at the top of the floor or footing
/// that covers it (see [`decide_bases`], #1384, RE-336): a level cut is the
/// wall's profile extruded from that level, and one along a sloped top is the
/// solid between that top and the wall's own. The extrusion stays the full
/// prism, which the passes that read a wall's plan and its record's box use;
/// the writer, the glTF and the plan outline draw the `solid_shape`.
pub(super) fn cut_wall_bases(
    entities: &mut [IfcEntity],
    pending: &[PendingBase],
    shapes: &HashMap<u32, SlabTop>,
    steps: &HashMap<usize, SteppedPlan>,
) {
    if pending.is_empty() {
        return;
    }
    let decisions = decide_bases(entities, pending, shapes);
    for (index, decision) in decisions {
        let Some(IfcEntity::BuildingElement {
            extrusion: Some(body),
            location_feet: Some(location),
            rotation_radians,
            solid_shape,
            property_set,
            ..
        }) = entities.get_mut(index)
        else {
            continue;
        };
        let cut = match decision {
            BaseDecision::Cut(cut) => cut,
            BaseDecision::Left(reason) => {
                if let Some(set) = property_set
                    .as_mut()
                    .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
                {
                    set.properties.push(Property {
                        name: WALL_BASE_REFUSAL_PROPERTY.into(),
                        value: PropertyValue::Text(reason.into()),
                    });
                }
                continue;
            }
        };
        let Some(shape) =
            wall_base_solid(body, *location, *rotation_radians, &cut, steps.get(&index))
        else {
            // The cut resolved and its solid cannot be made (a piece of the
            // plan does not break into convex pieces, a part rises nowhere
            // above the cut, or the sloped top crosses a piece): the wall keeps
            // its prism and says so.
            if let Some(set) = property_set
                .as_mut()
                .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
            {
                set.properties.push(Property {
                    name: WALL_BASE_REFUSAL_PROPERTY.into(),
                    value: PropertyValue::Text(WALL_BASE_REFUSAL_SOLID_NOT_MADE.into()),
                });
            }
            continue;
        };
        let (source, id) = base_cut_source(&cut);
        *solid_shape = Some(shape);
        if let Some(set) = property_set
            .as_mut()
            .filter(|set| set.name == ELEMENT_RECORD_PROPERTY_SET)
        {
            set.properties.push(Property {
                name: WALL_BASE_SOURCE_PROPERTY.into(),
                value: PropertyValue::Text(source.into()),
            });
            set.properties.push(Property {
                name: "BaseCoverElementId".into(),
                value: PropertyValue::Integer(i64::from(id)),
            });
        }
    }
}

/// The `BodyBaseSource` and the ElementId of the element a base is cut at.
fn base_cut_source(cut: &BaseCut) -> (&'static str, u32) {
    match cut {
        BaseCut::Level {
            id, covered: None, ..
        } => (WALL_BASE_COVER_BODY_BASE_SOURCE, *id),
        BaseCut::Level { id, .. } => (WALL_BASE_COVER_IN_PART_BODY_BASE_SOURCE, *id),
        BaseCut::Plane {
            id, covered: None, ..
        } => (WALL_BASE_SLOPED_BODY_BASE_SOURCE, *id),
        BaseCut::Plane { id, .. } => (WALL_BASE_SLOPED_IN_PART_BODY_BASE_SOURCE, *id),
    }
}

/// A piece of a wall's plan in the wall's frame and the height of its top
/// above the wall's base, feet.
type Piece = (Ring, f64);

/// The item for the part `ring` of a piece (in the wall's frame) whose base is
/// cut at `cut` and whose top is `top` above the wall's base; none where it
/// rises nowhere above the cut or is crossed by the sloped top.
fn base_cut_item(
    cut: &BaseCut,
    ring: &[Point2],
    top: f64,
    placement: &Placement,
    base: f64,
) -> Option<SolidShape> {
    match cut {
        BaseCut::Level { level, .. } => {
            let from = level - base;
            (top - from > 1e-9).then(|| SolidShape::PlacedExtrusion {
                profile: ProfileDef::ArbitraryClosed {
                    points: ring.to_vec(),
                },
                origin_feet: [0.0, 0.0, from],
                axis: [0.0, 0.0, 1.0],
                ref_direction: [1.0, 0.0, 0.0],
                depth_feet: top - from,
            })
        }
        BaseCut::Plane { plane, .. } => {
            let bottom = |p: Point2| {
                let at = placement.apply([p.0, p.1, 0.0]);
                plane[0] * at[0] + plane[1] * at[1] + plane[2] - base
            };
            between_surfaces_mesh(ring, &[], bottom, |_| top).map(|mesh| SolidShape::FacetedBrep {
                vertices_feet: mesh.vertices,
                triangles: mesh
                    .triangles
                    .into_iter()
                    .map(|[a, b, c]| BrepTriangle(a, b, c))
                    .collect(),
            })
        }
    }
}

/// The items of one piece of a wall whose base is cut at `cut`: the piece
/// whole where the element covers the whole plan, and otherwise the part of it
/// inside the pieces the element covers, cut, and the rest, standing from the
/// wall's own base (#1411). None where any part cannot be made.
fn base_cut_piece(
    cut: &BaseCut,
    piece: &Piece,
    placement: &Placement,
    base: f64,
) -> Option<Vec<SolidShape>> {
    let (ring, top) = piece;
    let Some(covered) = cut.covered() else {
        return Some(vec![base_cut_item(cut, ring, *top, placement, base)?]);
    };
    // The piece in model feet, counter-clockwise, as the convex pieces it is
    // made of (a wall's ring may run either way, and a layered wall's is not
    // convex): the clips below take convex pieces.
    let model: Ring = counter_clockwise(
        ring.iter()
            .map(|&(x, y)| {
                let at = placement.apply([x, y, 0.0]);
                (at[0], at[1])
            })
            .collect(),
    );
    let local = |part: &Ring| -> Ring { part.iter().map(|&p| to_local(placement, p)).collect() };
    let mut items = Vec::new();
    let mut held = 0.0;
    for convex in convex_pieces(&model)? {
        let mut outside: Vec<Ring> = vec![convex.clone()];
        for clip in covered {
            let part = clip_convex(&convex, clip);
            if part.len() >= 3 && area(&part) > PIECE_ROUNDING_SQUARE_FEET {
                held += area(&part);
                items.push(base_cut_item(cut, &local(&part), *top, placement, base)?);
            }
            outside = outside
                .iter()
                .flat_map(|part| subtract_convex(part, clip))
                .collect();
        }
        for part in &outside {
            if part.len() >= 3 && area(part) > PIECE_ROUNDING_SQUARE_FEET {
                held += area(part);
                items.push(SolidShape::PlacedExtrusion {
                    profile: ProfileDef::ArbitraryClosed {
                        points: local(part),
                    },
                    origin_feet: [0.0, 0.0, 0.0],
                    axis: [0.0, 0.0, 1.0],
                    ref_direction: [1.0, 0.0, 0.0],
                    depth_feet: *top,
                });
            }
        }
    }
    // What was made covers the piece once: parts that overlap, or leave a
    // gap, are a body that is not the piece's.
    let expected = area(&model);
    ((held - expected).abs() <= COVER_TOLERANCE_SQUARE_FEET && !items.is_empty()).then_some(items)
}

/// The solid of a wall whose base is cut at `cut`, one item for each of
/// `pieces`, or for the parts of each that the element covers and does not;
/// none if any piece rises nowhere above the cut or is crossed by the sloped
/// top, so that no block is left out without a trace (#1384, #1396, #1411).
fn base_cut_solid(
    cut: &BaseCut,
    pieces: &[Piece],
    placement: &Placement,
    base: f64,
) -> Option<SolidShape> {
    let mut items = Vec::with_capacity(pieces.len());
    for piece in pieces {
        items.extend(base_cut_piece(cut, piece, placement, base)?);
    }
    if items.len() == 1 {
        items.pop()
    } else {
        Some(SolidShape::Compound(items))
    }
}

/// The solid of the wall drawn as `body` at `location`, whose base is cut at
/// `cut`: its plan to its top, or, for a wall whose ends were drawn to the
/// walls beneath it, that plan and the blocks of `plan` (#1396).
fn wall_base_solid(
    body: &Extrusion,
    location: [f64; 3],
    rotation: Option<f64>,
    cut: &BaseCut,
    plan: Option<&SteppedPlan>,
) -> Option<SolidShape> {
    let (outer, holes) = extrusion_rings(body)?;
    if !holes.is_empty() {
        return None;
    }
    let base = location[2];
    let placement = Placement::new(Some(location), rotation);
    let pieces = match plan {
        Some(plan) => plan.pieces(&placement, base, base + body.height_feet),
        None => vec![(outer, body.height_feet)],
    };
    base_cut_solid(cut, &pieces, &placement, base)
}

/// Draw the wall of each of `steps` that no base cut has made a solid of as
/// its whole plan to the lowest top of its pieces and what stands above that
/// (#1396): the first item is the whole plan, as in RE-320's stepped body, so
/// the glTF can lay the type's layers across it.
pub(super) fn finish_wall_ends(entities: &mut [IfcEntity], steps: &HashMap<usize, SteppedPlan>) {
    for (&index, plan) in steps {
        let Some(IfcEntity::BuildingElement {
            extrusion: Some(body),
            location_feet: Some(location),
            rotation_radians,
            solid_shape,
            ..
        }) = entities.get_mut(index)
        else {
            continue;
        };
        if solid_shape.is_some() {
            continue;
        }
        let placement = Placement::new(Some(*location), *rotation_radians);
        let base = location[2];
        let pieces = plan.pieces(&placement, base, base + body.height_feet);
        let low = pieces
            .iter()
            .map(|(_, top)| *top)
            .fold(f64::INFINITY, f64::min);
        // `decide_ends` let the wall through only with every piece above its
        // base, so this is a guard and not a path.
        if !(low.is_finite() && low > 1e-9) {
            continue;
        }
        let mut items = vec![SolidShape::ExtrudedArea(Extrusion {
            height_feet: low,
            ..body.clone()
        })];
        items.extend(
            pieces
                .into_iter()
                .filter(|(_, top)| *top > low + 1e-9)
                .map(|(ring, top)| SolidShape::PlacedExtrusion {
                    profile: ProfileDef::ArbitraryClosed { points: ring },
                    origin_feet: [0.0, 0.0, low],
                    axis: [0.0, 0.0, 1.0],
                    ref_direction: [1.0, 0.0, 0.0],
                    depth_feet: top - low,
                }),
        );
        *solid_shape = Some(SolidShape::Compound(items));
    }
}

/// How far a wall's elevation range may lie outside another's and still hold
/// its base, and how far a wall's width may lie outside another's length and
/// still be spanned by it, feet (#1396).
const END_TOLERANCE_FEET: f64 = 1e-3;

/// How far a wall's rotation may lie from a quarter turn and still count as
/// along a model axis, in quarter turns: its record box turns by the angle its
/// record stores in single precision, which is off by about 1.5e-7 radian,
/// 3e-5 foot over a wall 200 feet long (#1396).
const ROTATION_TOLERANCE_QUARTERS: f64 = 1e-6;

/// Why a wall with an edited elevation profile keeps an end whose record joins
/// a wall that is not read as a wall body: not a plain wall, or one whose top
/// a floor cut in steps with a piece that is not read (#1410, RE-348).
const WALL_END_REFUSAL_WALL_NOT_READ: &str = "joined_wall_not_read";

/// Why such a wall keeps an end whose record joins a wall that stands at a
/// height the end is read over, but whose top is not known there: a floor cut
/// it whole and its own top is not recorded, or no piece of its stepped top
/// stands over the end below its own top (#1410, RE-348).
const WALL_END_REFUSAL_TOP_NOT_READ: &str = "joined_wall_top_not_read";

/// Why such a wall keeps an end whose record joins a wall that is not a
/// perpendicular wall the end meets: it does not span the end's width at its
/// face, the end's line does not end inside it, or the end would pass its far
/// face (#1410, RE-348).
const WALL_END_REFUSAL_NOT_AT_THE_END: &str = "joined_wall_does_not_meet_the_end";

/// Why such a wall keeps an end where the walls its record joins there do not
/// stand over the whole of its height (#1410, RE-348).
const WALL_END_REFUSAL_HEIGHT_NOT_COVERED: &str = "joined_walls_do_not_reach_the_wall_height";

/// Why such a wall keeps an end whose joined wall has its face inside the end
/// the record's box gives, which would take material away (#1410, RE-348).
const WALL_END_REFUSAL_FACE_INSIDE: &str = "joined_wall_face_inside_the_wall";

/// Why such a wall keeps an end that reaches further at a greater height than
/// below it: the body the end is drawn as is columns from the base, which
/// cannot overhang (#1410, RE-348).
const WALL_END_REFUSAL_OVERHANG: &str = "joined_wall_faces_overhang_the_lower_end";

/// Why such a wall keeps its end where the cut of its base, which resolved on
/// the plan its record gives, would not over the longer plan (#1396).
const WALL_END_REFUSAL_BASE_CUT_LOST: &str = "base_cut_lost_on_extended_plan";

/// Why such a wall keeps an end its record trimmed, or whose curve end lists
/// walls, where no end of the curve lies at the end of the record box, so that
/// the walls it joins there cannot be placed (#1410, RE-348).
const WALL_END_REFUSAL_CURVE_NOT_AT_BOX_END: &str = "end_join_curve_not_at_the_box_end";

/// Why such a wall keeps an end its record trimmed, where the record gives no
/// list of the walls joined at it: no `JoinEndGStep` for that end, several, or
/// an entry that is not an ElementId (#1410, RE-348).
const WALL_END_REFUSAL_LIST_NOT_READ: &str = "end_join_list_not_read";

/// Why such a wall keeps its end where its base join names elements, none of
/// which cuts the base over the longer plan either, so the longer plan would
/// only add base that Revit's wall does not have (#1396).
const WALL_END_REFUSAL_BASE_NOT_CUT: &str = "base_not_cut_over_the_extended_plan";

/// Why such a wall keeps its end where the cut of its base resolves over the
/// longer plan and the solid it makes cannot be (#1411, RE-349).
const WALL_END_REFUSAL_BASE_SOLID: &str = "base_solid_not_made_over_the_extended_plan";

/// Why a wall whose base cut resolved keeps its prism: the solid of the cut
/// cannot be made (#1411, RE-349).
const WALL_BASE_REFUSAL_SOLID_NOT_MADE: &str = "base_cut_solid_not_made";

/// Why such a wall keeps its end where another wall, one its record does not
/// join there, stands in the stretch the end would be drawn over: the added
/// end would lie inside that wall (#1396).
const WALL_END_REFUSAL_IN_THE_WAY: &str = "wall_stands_in_the_added_end";

/// A wall drawn as an upright rectangle along a model axis: its plan bounds
/// (least x and y, then greatest x and y), its base and top, and which of its
/// extrusion's two plan sides lies along x (#1396).
struct AxisWall {
    index: usize,
    base: f64,
    top: f64,
    bounds: [f64; 4],
    turned: bool,
}

/// How many quarter turns `rotation` is, where it is one to within
/// [`ROTATION_TOLERANCE_QUARTERS`].
fn quarter_turns(rotation: Option<f64>) -> Option<i64> {
    let quarters = rotation.unwrap_or(0.0) / std::f64::consts::FRAC_PI_2;
    (quarters.is_finite() && (quarters - quarters.round()).abs() <= ROTATION_TOLERANCE_QUARTERS)
        .then_some(quarters.round() as i64)
}

/// The wall `entity` is, where it is one plain rectangle whose sides lie along
/// the model's axes. A wall drawn as a solid is not: it is no longer its
/// plan's prism.
fn axis_wall(index: usize, entity: &IfcEntity) -> Option<AxisWall> {
    let IfcEntity::BuildingElement {
        ifc_type,
        extrusion: Some(body),
        solid_shape: None,
        representation_map_index: None,
        location_feet: Some(location),
        rotation_radians,
        ..
    } = entity
    else {
        return None;
    };
    if !matches!(ifc_type.as_str(), "IFCWALL" | "IFCWALLSTANDARDCASE")
        || body.profile_override.is_some()
        || !location.iter().all(|v| v.is_finite())
    {
        return None;
    }
    let turned = quarter_turns(*rotation_radians)?.rem_euclid(2) == 1;
    let (across_x, across_y) = if turned {
        (body.depth_feet, body.width_feet)
    } else {
        (body.width_feet, body.depth_feet)
    };
    if !(across_x.is_finite()
        && across_y.is_finite()
        && across_x > 0.0
        && across_y > 0.0
        && body.height_feet.is_finite()
        && body.height_feet > 0.0)
    {
        return None;
    }
    Some(AxisWall {
        index,
        base: location[2],
        top: location[2] + body.height_feet,
        bounds: [
            location[0] - across_x / 2.0,
            location[1] - across_y / 2.0,
            location[0] + across_x / 2.0,
            location[1] + across_y / 2.0,
        ],
        turned,
    })
}

/// A wall an end may follow: where it stands, and the plan it has there
/// (#1410).
struct Followed {
    index: usize,
    base: f64,
    /// The height of its top: its own where `own_top`, and otherwise the height
    /// a floor cut it to, its own not being recorded.
    top: f64,
    own_top: bool,
    /// The plan of each piece it is drawn in, in model feet, with the heights
    /// the piece stands between: its plan to the lowest underside of the floors
    /// that cut its top in steps (RE-320), and each piece above it.
    slabs: Vec<(Ring, f64, f64)>,
}

/// The `BodyHeightSource` of the wall `entity`, where it has one.
fn body_height_source(entity: &IfcEntity) -> Option<&str> {
    let IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    } = entity
    else {
        return None;
    };
    set.properties.iter().find_map(|property| {
        match (&property.value, property.name == "BodyHeightSource") {
            (PropertyValue::Text(source), true) => Some(source.as_str()),
            _ => None,
        }
    })
}

/// The wall `entity` is, with its ElementId, where it is drawn as its plan's
/// prism, or as the stepped body of RE-320 (#1410). Any other solid is not read.
fn followed_wall(index: usize, entity: &IfcEntity) -> Option<(u32, Followed)> {
    let IfcEntity::BuildingElement {
        type_guid,
        extrusion: Some(body),
        solid_shape,
        location_feet: Some(location),
        rotation_radians,
        ..
    } = entity
    else {
        return None;
    };
    let id: u32 = type_guid.as_deref()?.parse().ok()?;
    let base = location[2];
    let top = base + body.height_feet;
    let source = body_height_source(entity);
    let slabs = match solid_shape {
        None => {
            let wall = build_wall(entity)?;
            vec![(wall.plan, base, top)]
        }
        Some(SolidShape::Compound(items))
            if source == Some(WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE) =>
        {
            let first = build_wall(&stepped_whole(entity)?)?;
            let placement = Placement::new(Some(*location), *rotation_radians);
            let mut slabs = vec![(first.plan, base, first.top)];
            for item in items.iter().skip(1) {
                let SolidShape::PlacedExtrusion {
                    profile: ProfileDef::ArbitraryClosed { points },
                    origin_feet,
                    depth_feet,
                    ..
                } = item
                else {
                    return None;
                };
                let ring: Ring = points
                    .iter()
                    .map(|&(x, y)| {
                        let at = placement.apply([x, y, 0.0]);
                        (at[0], at[1])
                    })
                    .collect();
                let from = base + origin_feet[2];
                slabs.push((ring, from, from + depth_feet));
            }
            slabs
        }
        _ => return None,
    };
    Some((
        id,
        Followed {
            index,
            base,
            top,
            own_top: source != Some(WALL_TOP_COVER_BODY_HEIGHT_SOURCE),
            slabs,
        },
    ))
}

/// Every wall of `entities` an end may follow, by ElementId. An id more than
/// one entity holds is left out, and so is a wall of `based`, whose base join
/// names elements: the cut of its base runs after the ends are read, and a
/// floor that raises it would leave it standing from higher than its record
/// box's base, which is where `followed_wall` takes it to stand from.
fn followed_walls(entities: &[IfcEntity], based: &HashSet<usize>) -> HashMap<u32, Followed> {
    let mut found: HashMap<u32, Followed> = HashMap::new();
    let mut repeated: HashSet<u32> = HashSet::new();
    for (index, entity) in entities.iter().enumerate() {
        if based.contains(&index) {
            continue;
        }
        if let Some((id, wall)) = followed_wall(index, entity) {
            if found.insert(id, wall).is_some() {
                repeated.insert(id);
            }
        }
    }
    for id in repeated {
        found.remove(&id);
    }
    found
}

/// The wall `entity` is, where its top was cut in steps by the floors that
/// cover it in part (RE-320), as the whole of its plan to the lowest underside;
/// the pieces that stand higher over part of the plan are items of its body,
/// which [`followed_wall`] reads (#1396, #1410).
fn stepped_whole(entity: &IfcEntity) -> Option<IfcEntity> {
    let IfcEntity::BuildingElement {
        solid_shape: Some(SolidShape::Compound(items)),
        property_set: Some(set),
        ..
    } = entity
    else {
        return None;
    };
    let stepped = set.properties.iter().any(|property| {
        property.name == "BodyHeightSource"
            && matches!(&property.value, PropertyValue::Text(source)
                if source == WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE)
    });
    let Some(SolidShape::ExtrudedArea(first)) = items.first() else {
        return None;
    };
    if !stepped {
        return None;
    }
    let mut whole = entity.clone();
    if let IfcEntity::BuildingElement {
        solid_shape,
        extrusion: Some(body),
        ..
    } = &mut whole
    {
        *solid_shape = None;
        body.height_feet = first.height_feet;
    }
    Some(whole)
}

/// The trims at the low and high end of a wall drawn from its record box,
/// feet: how far its join trim cut each end in from the box (RE-26). None for
/// any other body. They stay the record's once a wall's end is drawn to the
/// wall beneath it: the body's end is then `BodyEndSource`'s, not the box's
/// less the trim.
fn join_trims(entity: Option<&IfcEntity>) -> Option<[f64; 2]> {
    let Some(IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    }) = entity
    else {
        return None;
    };
    if set.name != ELEMENT_RECORD_PROPERTY_SET {
        return None;
    }
    let read = |name: &str| {
        set.properties
            .iter()
            .find_map(|property| match (&property.value, property.name == name) {
                (PropertyValue::LengthFeet(trim), true) if trim.is_finite() && *trim >= 0.0 => {
                    Some(*trim)
                }
                _ => None,
            })
    };
    Some([read("JoinTrimStart")?, read("JoinTrimEnd")?])
}

/// A wall whose record joins others at the ends of its location curve,
/// waiting for the pass (#1410).
pub(super) struct PendingEnds {
    /// The wall's entity index.
    pub index: usize,
    /// The location curve's start and its end, plan model feet.
    pub curve: [[f64; 2]; 2],
    /// The ElementIds its record joins at the curve's start and at its end, in
    /// the record's order; `None` where it gives no list for that end.
    pub lists: [Option<Vec<u32>>; 2],
}

/// The part of the stretch an end grows by that the wall's body reaches up to
/// one height: from `inner` to `outer` feet beyond the end the box gives, up to
/// `top` (#1410).
struct Strip {
    inner: f64,
    outer: f64,
    top: f64,
}

/// An end of a wall that is drawn to the walls its record joins there.
struct End {
    /// The axis of the wall's length: 0 for x.
    long: usize,
    /// Whether it is the end at the low coordinate.
    at_low: bool,
    /// How far the end moves, feet: the furthest the walls it follows reach.
    gap: f64,
    /// The least ElementId of the walls it follows where it moves.
    reached: Option<u32>,
    /// The strips of the stretch it grows by, the one that reaches farthest from
    /// the end the box gives first: `gap` is its outer distance.
    strips: Vec<Strip>,
    /// Whether another order of the walls the record joins would draw another
    /// end (see [`ReadEnd`]).
    order_dependent: bool,
}

/// The ends of one wall that are drawn to the walls its record joins there.
struct WallEnds {
    index: usize,
    turned: bool,
    ends: Vec<End>,
    /// The wall's plan and the strips of its ends, where a strip stops below the
    /// wall's top.
    plan: Option<SteppedPlan>,
}

/// A wall whose ends are drawn to the walls its record joins there, where a
/// strip stops below the wall's top: its body is its own plan to its top and
/// one column from the base for each strip of each end, to the height the walls
/// the end follows stand to there (#1410). Model feet.
#[derive(Debug)]
pub(super) struct SteppedPlan {
    /// The wall's plan before its ends moved.
    original: Ring,
    /// The plan of each strip, and the height its top is at.
    blocks: Vec<(Ring, f64)>,
}

impl SteppedPlan {
    /// The pieces in the frame of `placement`, each with the height of its top
    /// above `base`; the wall's own plan to `top` (a height above sea level)
    /// first.
    fn pieces(&self, placement: &Placement, base: f64, top: f64) -> Vec<Piece> {
        let local =
            |ring: &Ring| -> Ring { ring.iter().map(|&p| to_local(placement, p)).collect() };
        let mut pieces = vec![(local(&self.original), top - base)];
        pieces.extend(
            self.blocks
                .iter()
                .map(|(ring, block_top)| (local(ring), block_top - base)),
        );
        pieces
    }
}

/// Where an end is read: the wall, its axis, which end, the end its body has
/// and the end of its record box, and the span of its width across the axis.
struct EndSite<'a> {
    wall: &'a AxisWall,
    long: usize,
    at_low: bool,
    end: f64,
    box_end: f64,
    strip: (f64, f64),
}

/// How far an end follows the wall it joins over one stretch of height.
struct Band {
    from: f64,
    to: f64,
    gap: f64,
    reached: u32,
}

/// Why an end cannot be read, and whether the faces of the walls the record
/// joins that were read before it was refused show the end short of or past
/// them: only then does the record show the wall's end is not the one Revit
/// draws, and only then does a wall carry the reason (#1410, RE-348).
struct Refusal {
    reason: &'static str,
    off: bool,
}

/// The refusal for `reason` after the `bands` read so far.
fn refusal(reason: &'static str, bands: &[Band]) -> Refusal {
    Refusal {
        reason,
        off: bands.iter().any(|band| band.gap > 0.0),
    }
}

/// What the plan of a wall is over the strip an end spans.
enum Standing {
    /// Nothing of it lies in the strip.
    Absent,
    /// It spans the strip: its face toward the end, and the least and greatest
    /// coordinate its plan has along the axis there.
    Face { at: f64, least: f64, most: f64 },
    /// It lies in part of the strip, or is not a wall across the axis.
    Partial,
}

fn coordinate(point: &Point2, axis: usize) -> f64 {
    if axis == 0 { point.0 } else { point.1 }
}

/// What the `rings` are over the strip `strip` (least and greatest coordinate
/// across the axis `long`) that the end at the low or high coordinate of the
/// axis spans: their face toward the end, where they span all of it (#1410).
fn standing(rings: &[&Ring], long: usize, at_low: bool, strip: (f64, f64)) -> Standing {
    let across = 1 - long;
    let clip = rect(long, (-1.0e7, 1.0e7), strip);
    let mut polygons: Vec<Ring> = Vec::new();
    for ring in rings {
        let part = clip_convex(ring, &clip);
        if part.len() < 3 || area(&part).abs() <= PIECE_ROUNDING_SQUARE_FEET {
            continue;
        }
        let b = bounds(ring);
        if b[across + 2] - b[across] <= b[long + 2] - b[long] {
            return Standing::Partial;
        }
        polygons.push(part);
    }
    if polygons.is_empty() {
        return Standing::Absent;
    }
    let along = |point: &Point2| coordinate(point, long);
    let least = polygons
        .iter()
        .flatten()
        .map(along)
        .fold(f64::INFINITY, f64::min);
    let most = polygons
        .iter()
        .flatten()
        .map(along)
        .fold(f64::NEG_INFINITY, f64::max);
    let at = if at_low { most } else { least };
    let mut spans: Vec<(f64, f64)> = polygons
        .iter()
        .filter_map(|polygon| {
            let on: Vec<f64> = polygon
                .iter()
                .filter(|point| (along(point) - at).abs() <= END_TOLERANCE_FEET)
                .map(|point| coordinate(point, across))
                .collect();
            let low = on.iter().copied().fold(f64::INFINITY, f64::min);
            let high = on.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (!on.is_empty()).then_some((low, high))
        })
        .collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reached = strip.0;
    for (low, high) in spans {
        if low <= reached + END_TOLERANCE_FEET {
            reached = reached.max(high);
        }
    }
    if reached < strip.1 - END_TOLERANCE_FEET {
        return Standing::Partial;
    }
    Standing::Face { at, least, most }
}

/// `heights` less the span `from` to `to`.
fn without_heights(heights: &[(f64, f64)], from: f64, to: f64) -> Vec<(f64, f64)> {
    let mut left = Vec::new();
    for &(a, b) in heights {
        if from.min(b) - a > END_TOLERANCE_FEET {
            left.push((a, from.min(b)));
        }
        if b - to.max(a) > END_TOLERANCE_FEET {
            left.push((to.max(a), b));
        }
    }
    left
}

/// What reading an end gave: its bands, and whether another order of the walls
/// its record joins would have drawn another end.
struct ReadEnd {
    bands: Vec<Band>,
    /// Whether at some height two of the walls the record lists stand with
    /// faces that differ: only then does the order of the list decide the end.
    order_dependent: bool,
}

/// How far the end at `site` follows the walls `list` names, at each height of
/// the wall: the face of the first wall of `list` that stands there. Revit
/// joins a wall's end to each wall its record lists, and at each height the end
/// follows the first of them in that order that stands there (#1410, RE-348).
/// The reason the end cannot be read, where it cannot.
///
/// A wall joins the end as a whole, so whether its line ends inside the wall,
/// and how thick the wall is, are read from its whole plan (its first slab, the
/// plan to the lowest underside of the floors that cut its top in steps, of
/// which the pieces above are parts) once for all of its heights; the face at
/// each height is read from the slabs that stand there. The walls after the one
/// that decides a height are read over that height too, for the one thing they
/// can say: whether their face differs, so that the order matters. They cannot
/// refuse the end.
///
/// `m_joinedEnd` of each entry of the list is not read, so nothing here verifies
/// that the walls are joined by the ends the list says.
fn read_end(
    site: &EndSite<'_>,
    list: &[u32],
    followed: &HashMap<u32, Followed>,
) -> Result<ReadEnd, Refusal> {
    let mut remaining: Vec<(f64, f64)> = vec![(site.wall.base, site.wall.top)];
    let mut bands: Vec<Band> = Vec::new();
    let mut order_dependent = false;
    for &id in list {
        let Some(other) = followed.get(&id) else {
            // A wall that is not read may stand over a height that remains.
            if remaining.is_empty() {
                continue;
            }
            return Err(refusal(WALL_END_REFUSAL_WALL_NOT_READ, &bands));
        };
        if other.index == site.wall.index {
            continue;
        }
        let open = !remaining.is_empty();
        if open
            && !other.own_top
            && remaining
                .iter()
                .any(|&(_, b)| b > other.top + END_TOLERANCE_FEET)
        {
            return Err(refusal(WALL_END_REFUSAL_TOP_NOT_READ, &bands));
        }
        let (lo, hi) = (site.wall.base.max(other.base), site.wall.top.min(other.top));
        if hi - lo <= END_TOLERANCE_FEET {
            continue;
        }
        let over_remaining = remaining
            .iter()
            .any(|&(a, b)| b.min(hi) - a.max(lo) > END_TOLERANCE_FEET);
        let Some(first) = other.slabs.first() else {
            if over_remaining {
                return Err(refusal(WALL_END_REFUSAL_WALL_NOT_READ, &bands));
            }
            continue;
        };
        let (least, most) = match standing(&[&first.0], site.long, site.at_low, site.strip) {
            Standing::Face { least, most, .. } if holds(least, most, site.box_end) => (least, most),
            _ => {
                if over_remaining {
                    return Err(refusal(WALL_END_REFUSAL_NOT_AT_THE_END, &bands));
                }
                continue;
            }
        };
        let mut cuts: Vec<f64> = vec![lo, hi];
        for z in other
            .slabs
            .iter()
            .flat_map(|(_, from, to)| [*from, *to])
            .chain(remaining.iter().flat_map(|&(a, b)| [a, b]))
        {
            if z > lo + END_TOLERANCE_FEET && z < hi - END_TOLERANCE_FEET {
                cuts.push(z);
            }
        }
        cuts.sort_by(f64::total_cmp);
        cuts.dedup_by(|a, b| (*a - *b).abs() <= END_TOLERANCE_FEET);
        for pair in cuts.windows(2) {
            let (z0, z1) = (pair[0], pair[1]);
            let middle = (z0 + z1) / 2.0;
            // A height an earlier wall of the list decided is only compared.
            let decided = !remaining.iter().any(|&(a, b)| a <= middle && middle < b);
            let rings: Vec<&Ring> = other
                .slabs
                .iter()
                .filter(|(_, slab_from, slab_to)| *slab_from <= middle && middle < *slab_to)
                .map(|(ring, _, _)| ring)
                .collect();
            match standing(&rings, site.long, site.at_low, site.strip) {
                Standing::Face { at, .. } => {
                    let gap = if site.at_low {
                        site.end - at
                    } else {
                        at - site.end
                    };
                    let gap = if gap.abs() <= END_TOLERANCE_FEET {
                        0.0
                    } else {
                        gap
                    };
                    if decided {
                        if bands
                            .iter()
                            .find(|band| band.from <= middle && middle < band.to)
                            .is_some_and(|band| (band.gap - gap).abs() > END_TOLERANCE_FEET)
                        {
                            order_dependent = true;
                        }
                        continue;
                    }
                    if gap < 0.0 {
                        return Err(Refusal {
                            reason: WALL_END_REFUSAL_FACE_INSIDE,
                            off: true,
                        });
                    }
                    if gap > most - least + END_TOLERANCE_FEET {
                        return Err(refusal(WALL_END_REFUSAL_NOT_AT_THE_END, &bands));
                    }
                    bands.push(Band {
                        from: z0,
                        to: z1,
                        gap,
                        reached: id,
                    });
                }
                Standing::Absent if !decided => {
                    return Err(refusal(WALL_END_REFUSAL_TOP_NOT_READ, &bands));
                }
                Standing::Partial if !decided => {
                    return Err(refusal(WALL_END_REFUSAL_NOT_AT_THE_END, &bands));
                }
                Standing::Absent | Standing::Partial => {}
            }
        }
        remaining = without_heights(&remaining, other.base, other.top);
    }
    if !remaining.is_empty() {
        return Err(refusal(WALL_END_REFUSAL_HEIGHT_NOT_COVERED, &bands));
    }
    bands.sort_by(|a, b| a.from.total_cmp(&b.from));
    Ok(ReadEnd {
        bands,
        order_dependent,
    })
}

/// The strips an end is drawn as from its `bands`: one for each distance the
/// end reaches beyond the next, the farthest first, up to the top of the
/// last band that reaches that far. The columns of the strips stand from the
/// wall's base, so an end that reaches further at a greater height than below
/// it is not drawn.
fn strips_of(bands: &[Band]) -> Result<Vec<Strip>, &'static str> {
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for band in bands {
        match merged.last_mut() {
            Some(last) if (last.0 - band.gap).abs() <= END_TOLERANCE_FEET => last.1 = band.to,
            _ => merged.push((band.gap, band.to)),
        }
    }
    let mut strips = Vec::new();
    for (k, &(gap, to)) in merged.iter().enumerate() {
        let next = merged.get(k + 1).map_or(0.0, |&(next_gap, _)| next_gap);
        if next > gap + END_TOLERANCE_FEET {
            return Err(WALL_END_REFUSAL_OVERHANG);
        }
        if gap - next > END_TOLERANCE_FEET {
            strips.push(Strip {
                inner: next,
                outer: gap,
                top: to,
            });
        }
    }
    Ok(strips)
}

/// The rectangle that spans `along` on the axis `long` and `across` on the
/// other, counter-clockwise.
fn rect(long: usize, along: (f64, f64), across: (f64, f64)) -> Ring {
    let (x, y) = if long == 0 {
        (along, across)
    } else {
        (across, along)
    };
    vec![(x.0, y.0), (x.1, y.0), (x.1, y.1), (x.0, y.1)]
}

fn range(bounds: &[f64; 4], axis: usize) -> (f64, f64) {
    (bounds[axis], bounds[axis + 2])
}

/// The bounds of the rectangle that spans `along` on the axis `long` and
/// `across` on the other.
fn rect_bounds(long: usize, along: (f64, f64), across: (f64, f64)) -> [f64; 4] {
    if long == 0 {
        [along.0, across.0, along.1, across.1]
    } else {
        [across.0, along.0, across.1, along.1]
    }
}

/// Whether two sets of bounds share more than a line.
fn overlaps(a: &[f64; 4], b: &[f64; 4]) -> bool {
    a[2].min(b[2]) - a[0].max(b[0]) > END_TOLERANCE_FEET
        && a[3].min(b[3]) - a[1].max(b[1]) > END_TOLERANCE_FEET
}

/// Whether the span `a` to `b` holds `at`.
fn holds(a: f64, b: f64, at: f64) -> bool {
    a - END_TOLERANCE_FEET <= at && at <= b + END_TOLERANCE_FEET
}

/// What moving the ends of a wall did to its place: the shift of its origin in
/// model feet and its rotation.
struct Moved {
    delta: [f64; 2],
    rotation: f64,
}

/// Move `ends` of the wall `entity`: its extrusion grows and its origin follows
/// the middle.
fn move_ends(entity: &mut IfcEntity, turned: bool, ends: &[End]) -> Option<Moved> {
    let IfcEntity::BuildingElement {
        extrusion: Some(body),
        location_feet: Some(location),
        rotation_radians,
        ..
    } = entity
    else {
        return None;
    };
    let before = *location;
    for end in ends {
        if (end.long == 0) != turned {
            body.width_feet += end.gap;
        } else {
            body.depth_feet += end.gap;
        }
        location[end.long] += if end.at_low {
            -end.gap / 2.0
        } else {
            end.gap / 2.0
        };
    }
    Some(Moved {
        delta: [location[0] - before[0], location[1] - before[1]],
        rotation: rotation_radians.filter(|r| r.is_finite()).unwrap_or(0.0),
    })
}

/// Put the openings of the edited elevation profile of the wall at `host` back
/// where they were in model space after its origin moved: each is placed in the
/// wall's frame, its outline along the opening's axis and its plane offset to
/// the axis's left (see [`IfcEntity::WallProfileOpening`]).
fn shift_profile_openings(entities: &mut [IfcEntity], host: usize, moved: &Moved) {
    let (sin, cos) = moved.rotation.sin_cos();
    let local = (
        cos * moved.delta[0] + sin * moved.delta[1],
        cos * moved.delta[1] - sin * moved.delta[0],
    );
    for entity in entities.iter_mut() {
        let (opening_host, outline_feet, axis_turn_radians, plane_offset_feet) = match entity {
            IfcEntity::WallProfileOpening {
                host,
                outline_feet,
                axis_turn_radians,
                plane_offset_feet,
                ..
            }
            | IfcEntity::WallProfilePocket {
                host,
                outline_feet,
                axis_turn_radians,
                plane_offset_feet,
                ..
            } => (host, outline_feet, axis_turn_radians, plane_offset_feet),
            _ => continue,
        };
        if *opening_host != host {
            continue;
        }
        let (axis_sin, axis_cos) = axis_turn_radians.sin_cos();
        let along = axis_cos * local.0 + axis_sin * local.1;
        let left = axis_cos * local.1 - axis_sin * local.0;
        for point in outline_feet.iter_mut() {
            point.0 -= along;
        }
        *plane_offset_feet -= left;
    }
}

/// Write how the wall at `index` was drawn at `ends` to its record's
/// properties: one `BodyEndSource`, `source`, and the least ElementId of the
/// walls its ends reach.
fn mark_ends(entities: &mut [IfcEntity], index: usize, ends: &[End], source: &str) {
    let Some(IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    }) = entities.get_mut(index)
    else {
        return;
    };
    if set.name != ELEMENT_RECORD_PROPERTY_SET {
        return;
    }
    set.properties.push(Property {
        name: WALL_END_SOURCE_PROPERTY.into(),
        value: PropertyValue::Text(source.into()),
    });
    let dependent = ends.iter().filter(|end| end.order_dependent).count();
    if dependent > 0 {
        set.properties.push(Property {
            name: WALL_END_ORDER_DEPENDENT_PROPERTY.into(),
            value: PropertyValue::Integer(i64::try_from(dependent).unwrap_or(i64::MAX)),
        });
    }
    if let Some(id) = ends.iter().filter_map(|end| end.reached).min() {
        set.properties.push(Property {
            name: "EndCoverElementId".into(),
            value: PropertyValue::Integer(i64::from(id)),
        });
    }
}

/// Write why the wall at `index` keeps its end to its record's properties.
fn mark_end_refusal(entities: &mut [IfcEntity], index: usize, reason: &str) {
    let Some(IfcEntity::BuildingElement {
        property_set: Some(set),
        ..
    }) = entities.get_mut(index)
    else {
        return;
    };
    if set.name == ELEMENT_RECORD_PROPERTY_SET {
        set.properties.push(Property {
            name: WALL_END_REFUSAL_PROPERTY.into(),
            value: PropertyValue::Text(reason.into()),
        });
    }
}

/// The ends to draw and the reasons for the ends to keep (see
/// [`extend_wall_ends`]), read from `entities` and changing none. A wall with a
/// base join that names elements is tried: its ends are moved on a copy, and
/// the base cut is decided again over the longer plan and its solid made.
fn decide_ends(
    entities: &[IfcEntity],
    pending: &[PendingBase],
    joined: &[PendingEnds],
    shapes: &HashMap<u32, SlabTop>,
    refused: &mut BTreeMap<usize, &'static str>,
    unmarked: &mut BTreeMap<usize, &'static str>,
) -> Vec<WallEnds> {
    let profiled: HashSet<usize> = entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::WallProfileOpening { host, .. } => Some(*host),
            _ => None,
        })
        .collect();
    let by_index: HashMap<usize, &PendingEnds> =
        joined.iter().map(|ends| (ends.index, ends)).collect();
    let candidates: Vec<(AxisWall, [f64; 2])> = entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| axis_wall(index, entity))
        .filter(|wall| profiled.contains(&wall.index) && by_index.contains_key(&wall.index))
        .filter_map(|wall| join_trims(entities.get(wall.index)).map(|trims| (wall, trims)))
        .collect();
    if candidates.is_empty() {
        return Vec::new();
    }
    // A joined wall whose base join names elements may be raised by them (the
    // cut of its base runs after this pass), so it is not read as standing
    // from its base.
    let based: HashSet<usize> = pending.iter().map(|base| base.index).collect();
    let followed = followed_walls(entities, &based);
    let context = BaseContext {
        entities,
        gathered: gather(entities),
        shapes,
    };
    let mut cutters: HashMap<u32, Option<Cutter>> = HashMap::new();
    let mut accepted = Vec::new();
    for (wall, trims) in candidates {
        let Some(record) = by_index.get(&wall.index) else {
            continue;
        };
        let [x0, y0, x1, y1] = wall.bounds;
        let long = usize::from(y1 - y0 > x1 - x0);
        let (low, high) = range(&wall.bounds, long);
        let (side_low, side_high) = range(&wall.bounds, 1 - long);
        let mut ends: Vec<End> = Vec::new();
        let mut at_an_end = [false, false];
        for at_low in [true, false] {
            let end = if at_low { low } else { high };
            let trim = if at_low { trims[0] } else { trims[1] };
            let box_end = if at_low { low - trim } else { high + trim };
            // The curve's start or end that lies at the end of the record box
            // is the end whose joins the record lists.
            let slots: Vec<usize> = (0..2)
                .filter(|&slot| (record.curve[slot][long] - box_end).abs() <= END_TOLERANCE_FEET)
                .collect();
            let [slot] = slots.as_slice() else {
                // Neither curve end, or both, lies at this end of the box. An
                // end the record trimmed has joined walls that cannot be placed;
                // an untrimmed one has none to follow.
                if trim > END_TOLERANCE_FEET {
                    unmarked
                        .entry(wall.index)
                        .or_insert(WALL_END_REFUSAL_CURVE_NOT_AT_BOX_END);
                }
                continue;
            };
            at_an_end[*slot] = true;
            // An end whose record lists no wall has none to follow and is not
            // read, with no marker (a free end); one the record trimmed that
            // gives no list at all (no step, several, or an id that is not one)
            // is counted.
            let list = match record.lists[*slot].as_deref() {
                Some([]) => continue,
                Some(list) => list,
                None => {
                    if trim > END_TOLERANCE_FEET {
                        unmarked
                            .entry(wall.index)
                            .or_insert(WALL_END_REFUSAL_LIST_NOT_READ);
                    }
                    continue;
                }
            };
            let site = EndSite {
                wall: &wall,
                long,
                at_low,
                end,
                box_end,
                strip: (side_low, side_high),
            };
            let strips = read_end(&site, list, &followed).and_then(|read| {
                strips_of(&read.bands)
                    .map_err(|reason| Refusal { reason, off: true })
                    .map(|strips| {
                        let reached = read
                            .bands
                            .iter()
                            .filter(|band| band.gap > END_TOLERANCE_FEET)
                            .map(|band| band.reached)
                            .min();
                        (strips, reached, read.order_dependent)
                    })
            });
            let (strips, reached, order_dependent) = match strips {
                Ok(read) => read,
                Err(refused_read) => {
                    // An end the faces read show short or past is marked on the
                    // wall; one they do not is only counted.
                    let notes = if refused_read.off {
                        &mut *refused
                    } else {
                        &mut *unmarked
                    };
                    notes.entry(wall.index).or_insert(refused_read.reason);
                    continue;
                }
            };
            if strips.is_empty() {
                continue;
            }
            // The strips lie against the faces of the walls the record joins,
            // which they may enter; a wall the record does not join that
            // stands in one is a wall the record does not account for.
            let in_the_way = strips.iter().any(|strip| {
                let along = if at_low {
                    (low - strip.outer, low - strip.inner)
                } else {
                    (high + strip.inner, high + strip.outer)
                };
                let block = rect_bounds(long, along, (side_low, side_high));
                followed.iter().any(|(id, other)| {
                    other.index != wall.index
                        && !list.contains(id)
                        && other.slabs.iter().any(|(ring, from, to)| {
                            (*to).min(strip.top) - (*from).max(wall.base) > END_TOLERANCE_FEET
                                && overlaps(&bounds(ring), &block)
                        })
                })
            });
            if in_the_way {
                refused
                    .entry(wall.index)
                    .or_insert(WALL_END_REFUSAL_IN_THE_WAY);
                continue;
            }
            ends.push(End {
                long,
                at_low,
                gap: strips.first().map_or(0.0, |strip| strip.outer),
                reached,
                strips,
                order_dependent,
            });
        }
        // A curve end that lists walls and lies at neither end of the box.
        for (slot, lists) in record.lists.iter().enumerate() {
            if !at_an_end[slot] && lists.as_deref().is_some_and(|list| !list.is_empty()) {
                unmarked
                    .entry(wall.index)
                    .or_insert(WALL_END_REFUSAL_CURVE_NOT_AT_BOX_END);
            }
        }
        if ends.is_empty() {
            continue;
        }
        let plan = ends
            .iter()
            .flat_map(|end| end.strips.iter())
            .any(|strip| strip.top < wall.top - END_TOLERANCE_FEET)
            .then(|| SteppedPlan {
                original: rect(long, (low, high), (side_low, side_high)),
                blocks: ends
                    .iter()
                    .flat_map(|end| {
                        end.strips.iter().map(move |strip| {
                            let along = if end.at_low {
                                (low - strip.outer, low - strip.inner)
                            } else {
                                (high + strip.inner, high + strip.outer)
                            };
                            (rect(long, along, (side_low, side_high)), strip.top)
                        })
                    })
                    .collect(),
            });
        let Some(mut trial) = entities.get(wall.index).cloned() else {
            continue;
        };
        if move_ends(&mut trial, wall.turned, &ends).is_none() {
            continue;
        }
        // Every piece of the stepped body must have a height above the base:
        // `finish_wall_ends` and the base pass make nothing of one that has not.
        if let (
            Some(plan),
            IfcEntity::BuildingElement {
                extrusion: Some(body),
                location_feet: Some(location),
                rotation_radians,
                ..
            },
        ) = (&plan, &trial)
        {
            let placement = Placement::new(Some(*location), *rotation_radians);
            let pieces = plan.pieces(&placement, location[2], location[2] + body.height_feet);
            if !pieces.iter().all(|(_, top)| *top > END_TOLERANCE_FEET) {
                refused
                    .entry(wall.index)
                    .or_insert(WALL_END_REFUSAL_IN_THE_WAY);
                continue;
            }
        }
        let named = pending
            .iter()
            .find(|base| base.index == wall.index)
            .map(|base| base.named.as_slice());
        if let Some(named) = named {
            let cut_before = matches!(
                decide_base(
                    &context,
                    &mut cutters,
                    entities.get(wall.index).and_then(build_wall),
                    named
                ),
                BaseDecision::Cut(_)
            );
            // None where the cut resolves and its solid is made; otherwise why
            // not: the cut does not resolve over the longer plan (and did, or
            // did not, on the plan the record gives), or it resolves and its
            // solid cannot be made.
            let lost = match decide_base(&context, &mut cutters, build_wall(&trial), named) {
                BaseDecision::Cut(cut) => match &trial {
                    IfcEntity::BuildingElement {
                        extrusion: Some(body),
                        location_feet: Some(location),
                        rotation_radians,
                        ..
                    } if wall_base_solid(
                        body,
                        *location,
                        *rotation_radians,
                        &cut,
                        plan.as_ref(),
                    )
                    .is_some() =>
                    {
                        None
                    }
                    _ => Some(WALL_END_REFUSAL_BASE_SOLID),
                },
                BaseDecision::Left(_) if cut_before => Some(WALL_END_REFUSAL_BASE_CUT_LOST),
                BaseDecision::Left(_) => Some(WALL_END_REFUSAL_BASE_NOT_CUT),
            };
            if let Some(reason) = lost {
                refused.insert(wall.index, reason);
                continue;
            }
        }
        accepted.push(WallEnds {
            index: wall.index,
            turned: wall.turned,
            ends,
            plan,
        });
    }
    accepted
}

/// Draw each wall with an edited elevation profile whose end its join trim cut
/// back from inside the walls it meets to the faces of those walls (#1396,
/// RE-337; #1409, #1410, RE-348).
///
/// A wall's record lists, for each end of its location curve, the walls Revit
/// joins there (`JoinEndGStep.m_realJoined`, in an order the record keeps). The
/// trim solver of RE-26 takes the end to the face of the wall whose centreline
/// lies on the end, however little of the wall's height that wall overlaps:
/// the walls of the BIM4LCA office's Basement that stand on the sloped footings
/// of RE-336 are cut back to the face of a Level_01 wall that overlaps them by
/// 0.66 ft of 9.5. Revit's body runs on, at each height, to the face of the
/// first wall of that list that stands there: the Basement wall beneath, then
/// the wall that stands between it and Level_01, then Level_01's (RE-348). The
/// end is read the same way at every height of the wall, over the part of each
/// joined wall's plan that lies across the wall's width (a wall whose top a
/// floor cut in steps by its pieces, RE-320, up to its own top), and drawn as
/// the wall's own plan to its top and a strip of each distance the end reaches
/// beyond the next, each from the base to the height the walls it follows
/// stand to. Where the walls the record lists do not stand over the whole of
/// the wall's height, where one is not read, where one does not meet the end,
/// or where the end would reach further at a greater height than below, the
/// end is kept and says why (`BodyEndRefusal`).
///
/// This applies to a wall with an edited elevation profile (a
/// [`IfcEntity::WallProfileOpening`] voids it) that is a plain rectangle along
/// a model axis, drawn from its record box, and the ends whose joins its
/// record lists. The wall's `extrusion` is its full plan to its top, which the
/// passes that read its plan use; its `solid_shape` is the stepped body, made
/// by [`cut_wall_bases`] or [`finish_wall_ends`]. The openings of its edited
/// profile, placed in its frame, are moved back with the origin.
///
/// A wall whose base join names elements keeps its ends unless the cut of its
/// base ([`decide_base`]) still resolves over the longer plan: a longer plan
/// that the elements it names do not cover at all leaves the wall uncut, with
/// more of the base than Revit's wall has, and a cut that resolved on the plan
/// the record gives would be lost (a longer plan a footing covers in part is
/// cut over that part, RE-349). A wall that names none has no base to lose.
///
/// A wall that keeps an end carries `BodyEndRefusal` with the reason, whether
/// or not the other end is drawn: a wall with one end drawn and the other kept
/// carries `BodyEndSource` and `BodyEndRefusal` both. A property set holds one
/// property of a name, so a wall carries one reason. Every other wall is left
/// as it was. The result is the stepped bodies [`cut_wall_bases`] and
/// [`finish_wall_ends`] make, by wall.
pub(super) fn extend_wall_ends(
    entities: &mut [IfcEntity],
    pending: &[PendingBase],
    joined: &[PendingEnds],
    shapes: &HashMap<u32, SlabTop>,
    handoffs: &mut BTreeMap<usize, Vec<Property>>,
) -> HashMap<usize, SteppedPlan> {
    let mut stepped = HashMap::new();
    if joined.is_empty()
        || !entities
            .iter()
            .any(|entity| matches!(entity, IfcEntity::WallProfileOpening { .. }))
    {
        return stepped;
    }
    // One reason for each wall: a property set holds one property of a name.
    let mut refused: BTreeMap<usize, &'static str> = BTreeMap::new();
    let mut unmarked: BTreeMap<usize, &'static str> = BTreeMap::new();
    let accepted = decide_ends(
        entities,
        pending,
        joined,
        shapes,
        &mut refused,
        &mut unmarked,
    );
    for WallEnds {
        index,
        turned,
        ends,
        plan,
    } in accepted
    {
        let Some(moved) = entities
            .get_mut(index)
            .and_then(|entity| move_ends(entity, turned, &ends))
        else {
            continue;
        };
        shift_profile_openings(entities, index, &moved);
        if let Some(plan) = plan {
            stepped.insert(index, plan);
        }
        mark_ends(
            entities,
            index,
            &ends,
            WALL_END_AT_JOINED_WALLS_BODY_END_SOURCE,
        );
    }
    // A wall carries a refusal only where the faces read show its end short of or
    // past the walls the record joins; an end the rule could not read and has no
    // such evidence for is Revit's end for all the record shows, so the wall
    // carries nothing and the reason is counted from the hand-off instead
    // (`exported.wall_end_reads_unmarked`). Each end is in one map or the other,
    // so a wall in both had one end refused with face evidence and the other
    // without: both are counted, each in its own counter (#1947).
    for (index, reason) in unmarked {
        handoffs.entry(index).or_default().push(Property {
            name: WALL_END_NOT_MARKED_PROPERTY.into(),
            value: PropertyValue::Text(reason.into()),
        });
    }
    for (index, reason) in refused {
        mark_end_refusal(entities, index, reason);
    }
    stepped
}
