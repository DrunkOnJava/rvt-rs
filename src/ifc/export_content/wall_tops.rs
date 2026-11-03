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
//! wall of which any of this is not known is left at its full height.
//! Several elements may qualify; the wall is cut at the lowest underside,
//! since a wall cannot rise through a floor above one that has cut it.
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
//! prism of its own plan up to the lowest underside, and, over each part of
//! the plan not covered at or below an underside, a prism from that underside
//! to the next one (the wall's top for the last), one item each in the one
//! body representation. The parts are convex pieces: the plan (itself where
//! it is convex, else its ear-clipped triangles joined where two make a convex
//! piece) less each triangle of an outline in turn, with the pieces that share
//! an edge and make a convex piece together joined. The holes and openings of
//! an element count as uncovered, where one element alone is named. An element
//! that only a cut pair names, and never the top join, cuts the top only where
//! it reaches the wall's top: a Join Geometry cut takes the volume the two
//! share, which is the wall's top only where the element rises to it (RE-320
//! §5). A wall
//! whose pieces pass [`MAX_RISERS`], whose plan does not break into convex
//! pieces, or whose several elements share a hole or opening that lies over
//! it, or with a piece between [`KEPT_SLIVER_FEET`] and [`KEPT_PIECE_FEET`]
//! wide, is left as it was. The wall's `extrusion` stays the full prism, which
//! the passes that read a wall's plan and its record's box use; the writer,
//! the glTF and the plan outline draw its `solid_shape`.

use std::collections::{HashMap, HashSet};

use super::{
    ELEMENT_RECORD_PROPERTY_SET, WALL_TOP_COVER_BODY_HEIGHT_SOURCE,
    WALL_TOP_STEPPED_BODY_HEIGHT_SOURCE,
};
use crate::ifc::body_geometry::{
    Body, Placement, Point2, Ring, extrusion_rings, plan_outline, section,
};
use crate::ifc::entities::{Extrusion, IfcEntity, ProfileDef, Property, PropertyValue, SolidShape};

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

/// The widest piece of a wall's plan outside its floor that Revit is measured
/// to leave at the wall's top among the slivers: the three 0.032 to 0.033 ft
/// pieces past a floor's edge on the BIM4LCA office as built (RE-320 §4), feet.
const KEPT_SLIVER_FEET: f64 = 0.04;

/// The narrowest wider piece measured to stay at the wall's top, feet: 0.098 on
/// the same file (an appendage of a layered wall's plan). Revit cuts the one
/// piece in between, a 0.085 ft stub past a floor's edge inside an abutting
/// wall, and nothing separates it from the pieces Revit keeps (pieces inside
/// another wall's plan are kept too), so a wall with a piece between the two
/// widths is left as it was (RE-320 §4).
const KEPT_PIECE_FEET: f64 = 0.09;

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

/// The least width of the convex counter-clockwise `ring`: the smallest, over
/// its edges, of the greatest distance of a corner from the edge's line, feet.
fn least_width(ring: &[Point2]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            let length = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
            ring.iter()
                .map(|&p| side_of(a, b, p).abs() / length)
                .fold(0.0, f64::max)
        })
        .fold(f64::INFINITY, f64::min)
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
fn steps(wall: &Wall, levels: &[(f64, Vec<&Cutter>)]) -> Option<Vec<(f64, Vec<Ring>)>> {
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
        if above.len() > MAX_RISERS
            || above.iter().any(|piece| {
                let width = least_width(piece);
                width > KEPT_SLIVER_FEET && width < KEPT_PIECE_FEET
            })
        {
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
/// extrusion whose outline triangulates, and they stand at one height.
fn build_cutter(
    entities: &[IfcEntity],
    pieces: &[usize],
    voids: &HashMap<usize, Vec<&[Point2]>>,
) -> Option<Cutter> {
    let mut heights: Option<(f64, f64)> = None;
    let mut top = f64::NEG_INFINITY;
    let mut outline = Vec::new();
    let mut cutouts = Vec::new();
    for &index in pieces {
        let IfcEntity::BuildingElement {
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            location_feet: Some(location),
            rotation_radians,
            ..
        } = entities.get(index)?
        else {
            return None;
        };
        if !(body.height_feet.is_finite() && body.height_feet > 0.0)
            || !location.iter().all(|v| v.is_finite())
        {
            return None;
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
                .or_insert_with(|| build_cutter(entities, found, &voids))
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
        let Some(found) = steps(&wall, &levels) else {
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
