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
//!   keeps the R2025 roofs off the walls that sit under them;
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
//! convex pieces, whose several elements share a hole or opening that lies
//! over it, or of which only a small stub lies outside the floor, inside
//! another wall (see [`embedded`]), is left as it was. The wall's `extrusion`
//! stays the full prism, which the passes that read a wall's plan and its
//! record's box use; the writer, the glTF and the plan outline draw its
//! `solid_shape`.
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
//! A wall of which this is not proven keeps its base. When it names a floor
//! with shape-edited corners it carries the reason as `BaseCoverRefusal`: the
//! edits of a floor, roof or ceiling are not read, the wall is not one plain
//! extrusion, no named element drawn as a body covers its plan or two do, or
//! the top of the one that does is at or below the wall's base, at or above
//! its top, or crosses one of them. A named element of which only part covers
//! the wall is not stepped here.

use std::collections::{HashMap, HashSet};

use super::{
    ELEMENT_RECORD_PROPERTY_SET, FLOOR_SHAPE_BODY_SOURCE, WALL_BASE_COVER_BODY_BASE_SOURCE,
    WALL_BASE_REFUSAL_PROPERTY, WALL_BASE_SLOPED_BODY_BASE_SOURCE, WALL_BASE_SOURCE_PROPERTY,
    WALL_TOP_COVER_BODY_HEIGHT_SOURCE, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE,
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

/// The area under which what a floor leaves of a wall's plan, all of it inside
/// other walls, may be a stub that Revit cuts although the floor does not cover
/// it, square feet: the one measured, 0.0243 ft2 at the end of a wall, inside
/// the plan of the wall it abuts, is cut by Revit with the rest of the wall.
/// The walls whose small pieces inside other walls Revit keeps (2984235,
/// 2984610, 2992096 and 1478932 on the BIM4LCA office as built) also have
/// larger parts of their plans outside the floor (RE-320 §4).
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
    if area(&clip_convex(piece, clip)) <= PIECE_ROUNDING_SQUARE_FEET {
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
/// left as it was.
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

/// The convex pieces of the wall's plan that lie above each underside of
/// `levels`: outside the outlines of the elements at that underside and the
/// lower ones, and inside their holes and openings. Each is the part of the
/// plan whose top is higher than that underside. `None` where there are more
/// than [`MAX_RISERS`] pieces, where the plan does not break into convex
/// pieces, or where several elements share a hole or opening the wall
/// crosses.
fn steps(
    wall: &Wall,
    levels: &[(f64, Vec<&Cutter>)],
    neighbours: &[&Wall],
) -> Option<Vec<(f64, Vec<Ring>)>> {
    let alone = levels.len() == 1 && levels[0].1.len() == 1;
    let base = convex_pieces(&wall.plan)?;
    let mut pieces = base.clone();
    let mut out = Vec::with_capacity(levels.len());
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
                    return None;
                }
            }
        }
        let above: Vec<Ring> = join_pieces(pieces.iter().cloned().chain(held).map(tidy).collect())
            .into_iter()
            .filter(|piece| area(piece) >= STEP_TOLERANCE_SQUARE_FEET)
            .collect();
        if above.len() > MAX_RISERS || (out.is_empty() && embedded(&above, neighbours)) {
            return None;
        }
        out.push((*level, above));
    }
    Some(out)
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
    Whole {
        level: f64,
        id: u32,
    },
    /// The undersides, lowest first, each with the pieces of the plan above
    /// it, and the ElementIds of the elements they are cut at.
    Stepped {
        steps: Vec<(f64, Vec<Ring>)>,
        ids: Vec<u32>,
    },
}

/// For each of `pending` (a wall's entity index and the ElementIds its top
/// join and cut pairs name) that is cut: its index and how.
fn decide(entities: &[IfcEntity], pending: &[Pending]) -> Vec<(usize, Cut)> {
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
            cuts.push((*index, Cut::Whole { level, id }));
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
        if found.first().is_some_and(|(_, above)| above.is_empty()) {
            cuts.push((
                *index,
                Cut::Whole {
                    level: found[0].0,
                    id: lowest_id,
                },
            ));
        } else {
            cuts.push((
                *index,
                Cut::Stepped {
                    steps: found,
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
pub(super) fn cut_wall_tops(entities: &mut [IfcEntity], pending: &[Pending]) {
    if pending.is_empty() {
        return;
    }
    let cuts = decide(entities, pending);
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
        let (named, source) = match cut {
            Cut::Whole { level, id } => {
                body.height_feet = level - location[2];
                (vec![id], WALL_TOP_COVER_BODY_HEIGHT_SOURCE)
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
                (ids, WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE)
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
        }
    }
}

/// A wall whose base join names elements, waiting for the pass (#1384).
pub(super) struct PendingBase {
    /// The wall's entity index.
    pub index: usize,
    /// The ElementIds its base join names.
    pub named: Vec<u32>,
}

/// How a wall's base is cut.
enum BaseCut {
    /// At one level across its plan.
    Level { level: f64, id: u32 },
    /// Along the plane of a shape-edited top.
    Plane { plane: PlanPlane, id: u32 },
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
    shapes: &'a HashMap<u32, Option<PlanPlane>>,
}

/// How the base of `wall` (the entity at `index`) is cut at the one element of
/// `named` that covers it (#1384): at that element's level, or along the plane
/// its shape-edited corners lie in, when that top lies between the wall's base
/// and top over the whole of the wall's plan. Left as it was, with the reason,
/// where the wall is not one plain extrusion, where a named element's edits
/// are not read, where no named element drawn as a body covers the whole plan,
/// where two do, or where the top crosses the wall's base or its top. A wall
/// with an edited elevation profile is cut like any other: the openings that
/// profile cuts void the solid as they void the full prism, and Revit raises
/// the base of the Basement walls of the BIM4LCA office that have one to the
/// footing's top as it does the others (RE-336).
fn decide_base(
    context: &BaseContext<'_>,
    cutters: &mut HashMap<u32, Option<Cutter>>,
    index: usize,
    named: &[u32],
) -> BaseDecision {
    let BaseContext {
        entities,
        gathered,
        shapes,
    } = context;
    let Some(wall) = entities.get(index).and_then(build_wall) else {
        return BaseDecision::Left("wall_not_one_plain_extrusion");
    };
    let mut eligible: Vec<BaseCut> = Vec::new();
    let mut built = false;
    let mut crossed = false;
    let mut above = false;
    let mut below = false;
    for &id in named {
        let Some(found) = gathered.pieces.get(&id) else {
            continue;
        };
        let shape = shapes.get(&id);
        if matches!(shape, Some(None)) {
            return BaseDecision::Left("slab_shape_not_read");
        }
        let Some(cutter) = cutters
            .entry(id)
            .or_insert_with(|| build_cutter(entities, found, &gathered.voids, shape.is_some()))
            .as_ref()
        else {
            continue;
        };
        built = true;
        if !covers(cutter, &wall) {
            continue;
        }
        let (low, high, cut) = match shape {
            Some(Some(plane)) => {
                let (low, high) = wall
                    .plan
                    .iter()
                    .map(|&(x, y)| plane[0] * x + plane[1] * y + plane[2])
                    .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), h| {
                        (lo.min(h), hi.max(h))
                    });
                (low, high, BaseCut::Plane { plane: *plane, id })
            }
            _ => (
                cutter.top,
                cutter.top,
                BaseCut::Level {
                    level: cutter.top,
                    id,
                },
            ),
        };
        if low > wall.base + UNDERSIDE_TOLERANCE_FEET && high < wall.top - UNDERSIDE_TOLERANCE_FEET
        {
            eligible.push(cut);
        } else if high > wall.base + UNDERSIDE_TOLERANCE_FEET
            && low < wall.top - UNDERSIDE_TOLERANCE_FEET
        {
            crossed = true;
        } else if high <= wall.base + UNDERSIDE_TOLERANCE_FEET {
            below = true;
        } else {
            above = true;
        }
    }
    match (eligible.pop(), eligible.is_empty()) {
        (Some(cut), true) => BaseDecision::Cut(cut),
        (Some(_), false) => BaseDecision::Left("several_elements_cover_the_wall"),
        (None, _) if crossed => BaseDecision::Left("top_crosses_the_wall_base_or_top"),
        (None, _) if above => BaseDecision::Left("top_at_or_above_the_wall_top"),
        (None, _) if below => BaseDecision::Left("top_at_or_below_the_wall_base"),
        (None, _) if built => BaseDecision::Left("no_element_covers_the_wall_plan"),
        (None, _) => BaseDecision::Left("named_element_body_not_read"),
    }
}

/// The decision for each of `pending` that is cut, or that names an element
/// with shape-edited corners and is left: a wall that names only unedited
/// elements and is not cut was not going to be raised by a sloped top, and
/// carries no reason.
fn decide_bases(
    entities: &[IfcEntity],
    pending: &[PendingBase],
    shapes: &HashMap<u32, Option<PlanPlane>>,
) -> Vec<(usize, BaseDecision)> {
    let context = BaseContext {
        entities,
        gathered: gather(entities),
        shapes,
    };
    let mut cutters: HashMap<u32, Option<Cutter>> = HashMap::new();
    let mut decisions = Vec::new();
    for PendingBase { index, named } in pending {
        let decision = decide_base(&context, &mut cutters, *index, named);
        let edited = named.iter().any(|id| shapes.contains_key(id));
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
    shapes: &HashMap<u32, Option<PlanPlane>>,
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
        let Some((outer, holes)) = extrusion_rings(body) else {
            continue;
        };
        if !holes.is_empty() {
            continue;
        }
        let base = location[2];
        let top = body.height_feet;
        let placement = Placement::new(Some(*location), *rotation_radians);
        let (shape, source, id) = match cut {
            BaseCut::Level { level, id } => {
                let from = level - base;
                (
                    SolidShape::PlacedExtrusion {
                        profile: ProfileDef::ArbitraryClosed { points: outer },
                        origin_feet: [0.0, 0.0, from],
                        axis: [0.0, 0.0, 1.0],
                        ref_direction: [1.0, 0.0, 0.0],
                        depth_feet: top - from,
                    },
                    WALL_BASE_COVER_BODY_BASE_SOURCE,
                    id,
                )
            }
            BaseCut::Plane { plane, id } => {
                let bottom = |p: Point2| {
                    let at = placement.apply([p.0, p.1, 0.0]);
                    plane[0] * at[0] + plane[1] * at[1] + plane[2] - base
                };
                let Some(mesh) = between_surfaces_mesh(&outer, &[], bottom, |_| top) else {
                    continue;
                };
                (
                    SolidShape::FacetedBrep {
                        vertices_feet: mesh.vertices,
                        triangles: mesh
                            .triangles
                            .into_iter()
                            .map(|[a, b, c]| BrepTriangle(a, b, c))
                            .collect(),
                    },
                    WALL_BASE_SLOPED_BODY_BASE_SOURCE,
                    id,
                )
            }
        };
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
