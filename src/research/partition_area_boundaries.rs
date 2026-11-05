//! The face a Revit Area occupies among its area boundary lines (RE-317).
//!
//! An Area's boundary is not stored in the Area. The lines that make it are
//! elements of their own, `OST_AreaSchemeLines` (-2000079), each with a
//! bounding-box record (the same frame as any element record), a Level and an
//! area scheme in its reference list, and no more. Revit draws an Area as the
//! region of the plane that its scheme's lines on its Level enclose around the
//! Area's point:
//!
//! - lines meet at their ends, and a line that ends on another, or crosses it,
//!   splits both there;
//! - a line with a free end bounds nothing and is left out, as a line that
//!   leads nowhere;
//! - a region is bounded by a loop of lines, and every loop that lies directly
//!   inside it is a hole of it, whether or not another Area sits in that
//!   hole;
//! - a vertex where two lines continue in a straight line is not a vertex of
//!   the outline, and a vertex where a third line meets is.
//!
//! The lines of Autodesk's `rac_advanced` do not all meet at their ends: a line
//! drawn along a wall stops at the other wall's centre, short of the line that
//! it joins, and two parallel lines drawn on the two faces of a wall's end stop
//! at the same place a wall's thickness apart. An IFC2X3 export of that project
//! from the three.js repository (RE-330) draws such an Area to the lines' joins:
//! a line's free end is carried on to the line it names and that names it
//! back, and the free ends of two parallel lines that name each other are
//! joined by a short connector. The partner comes from the lines' reference
//! lists (`PartitionElementRecord::references`), which is inferred from that
//! one export, and a distance only bounds it ([`MAX_CARRY_FEET`],
//! [`MAX_CONNECTOR_FEET`]): a free end whose line names no such partner, or
//! whose partner is not in reach, stays free, and bounds nothing.
//!
//! This module holds that geometry for straight lines along the plan axes,
//! which is every line of the office. [`Arrangement::new`] returns `None` for
//! a set it cannot read as such, and [`Arrangement::face_at`] returns `None`
//! for a point it cannot place in exactly one region bounded by simple
//! loops, so nothing is drawn that is not proven. The Area's point and the
//! rest come from [`crate::partition_area_records`].

use std::collections::{BTreeMap, BTreeSet};

/// How close two points, or a point and a line, must be to be the same, feet.
pub const NODE_TOLERANCE_FEET: f64 = 1e-5;

/// The most lines one arrangement takes. A set of more is not read.
pub const MAX_LINES: usize = 2048;

/// The most points one arrangement takes, its lines' ends and the places they
/// cross included. Lines that cross in more places are not read.
pub const MAX_NODES: usize = 200_000;

/// The largest coordinate taken, feet.
pub const MAX_COORDINATE_FEET: f64 = 1.0e7;

/// The smallest plan area of a ring that bounds a region, square feet.
pub const MIN_RING_AREA: f64 = 1e-6;

/// The farthest a free end is carried on, feet. The export of RE-330 carries
/// 0.666 ft at most; a line whose partner is farther is left free.
pub const MAX_CARRY_FEET: f64 = 1.0;

/// The widest connector between two parallel lines, feet. The export of RE-330
/// has 0.625 ft; lines farther apart are left free.
pub const MAX_CONNECTOR_FEET: f64 = 1.0;

/// One area boundary line: its ElementId and its two ends in plan, feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryLine {
    /// The line's ElementId.
    pub id: u32,
    /// One end.
    pub from: (f64, f64),
    /// The other end.
    pub to: (f64, f64),
}

/// For each edge of a loop, from a vertex to the next (the last to the first),
/// the middle point of the arc it is, or `None` for a line (RE-333).
pub type ArcMids = Vec<Option<(f64, f64)>>;

/// The region of the plane around a point, bounded by lines.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaFace {
    /// The outer loop, counter-clockwise, without a repeated closing vertex.
    pub outer: Vec<(f64, f64)>,
    /// The arcs of the outer loop. Empty where every edge is a line, as the
    /// arrangement of straight lines gives it; the plan topology's region
    /// (RE-333) may have arcs.
    pub outer_arcs: ArcMids,
    /// The loops directly inside it, clockwise, the largest first.
    pub holes: Vec<Vec<(f64, f64)>>,
    /// The arcs of each hole; empty where every hole's edges are lines.
    pub hole_arcs: Vec<ArcMids>,
    /// The ElementIds of the lines that bound the region, its holes' too.
    pub line_ids: BTreeSet<u32>,
    /// Whether the outline, a hole's too, runs along a stretch of a line that
    /// was carried on past the extent its record gives, or along a connector
    /// (RE-330): a region that needed a join to be drawn.
    pub joined: bool,
}

/// A straight line along an axis.
#[derive(Debug, Clone, Copy)]
struct AxisLine {
    id: u32,
    horizontal: bool,
    fixed: f64,
    low: f64,
    high: f64,
    /// The extent the line's record gives, before any end is carried on.
    stored_low: f64,
    stored_high: f64,
    /// Whether this is a connector between two lines (RE-330), which has no
    /// ElementId of its own.
    connector: bool,
}

impl AxisLine {
    fn new(line: &BoundaryLine) -> Option<AxisLine> {
        let ends = [line.from.0, line.from.1, line.to.0, line.to.1];
        if ends
            .iter()
            .any(|v| !v.is_finite() || v.abs() > MAX_COORDINATE_FEET)
        {
            return None;
        }
        let dx = (line.to.0 - line.from.0).abs();
        let dy = (line.to.1 - line.from.1).abs();
        if dx <= NODE_TOLERANCE_FEET && dy <= NODE_TOLERANCE_FEET {
            return None;
        }
        if dy <= NODE_TOLERANCE_FEET {
            let (low, high) = (line.from.0.min(line.to.0), line.from.0.max(line.to.0));
            Some(AxisLine {
                id: line.id,
                horizontal: true,
                fixed: (line.from.1 + line.to.1) / 2.0,
                low,
                high,
                stored_low: low,
                stored_high: high,
                connector: false,
            })
        } else if dx <= NODE_TOLERANCE_FEET {
            let (low, high) = (line.from.1.min(line.to.1), line.from.1.max(line.to.1));
            Some(AxisLine {
                id: line.id,
                horizontal: false,
                fixed: (line.from.0 + line.to.0) / 2.0,
                low,
                high,
                stored_low: low,
                stored_high: high,
                connector: false,
            })
        } else {
            None
        }
    }

    fn point_at(&self, along: f64) -> (f64, f64) {
        if self.horizontal {
            (along, self.fixed)
        } else {
            (self.fixed, along)
        }
    }

    /// The coordinate along the line of its high or low end.
    fn end(&self, high: bool) -> f64 {
        if high { self.high } else { self.low }
    }

    /// Whether `point` lies on the line, to [`NODE_TOLERANCE_FEET`].
    fn holds(&self, point: (f64, f64)) -> bool {
        let (along, across) = if self.horizontal {
            (point.0, point.1)
        } else {
            (point.1, point.0)
        };
        (across - self.fixed).abs() <= NODE_TOLERANCE_FEET
            && along >= self.low - NODE_TOLERANCE_FEET
            && along <= self.high + NODE_TOLERANCE_FEET
    }
}

/// Whether line `a` names line `b` and `b` names `a`.
fn name_each_other(joined: &BTreeMap<u32, BTreeSet<u32>>, a: u32, b: u32) -> bool {
    joined.get(&a).is_some_and(|names| names.contains(&b))
        && joined.get(&b).is_some_and(|names| names.contains(&a))
}

/// Carry the free ends of `axis` on to the lines that join them, and join the
/// free ends that cannot be carried on (RE-330).
///
/// - A free end is an end that lies on no other line. It is carried on, along
///   its own line, to the farthest line within [`MAX_CARRY_FEET`] that is
///   perpendicular, names this line and is named by it: past any nearer line it
///   crosses, and to the point where the two lines would meet. Both lines of a
///   corner that stops short at both are carried on to it, each by its own end.
///   A carry stands only if the line it ends on, after its own carry, spans the
///   place it reaches; otherwise the end stays free. (Choosing the farthest of
///   several such lines rather than the nearest rests on one line of the
///   export, and is unmeasured elsewhere.)
/// - A free end that had no such line is joined to the free end of a parallel
///   line within [`MAX_CONNECTOR_FEET`] that names it and is named by it, when
///   the two ends lie at the same place along the lines, the two lines run away
///   from it on opposite sides, and each end is the other's only such partner:
///   a connector across the gap, as wide as the two lines are apart.
///
/// An end whose line names no such partner stays free, and is left out with
/// its line wherever it leads nowhere.
fn resolve_joins(axis: &mut Vec<AxisLine>, joined: &BTreeMap<u32, BTreeSet<u32>>) {
    if joined.is_empty() {
        return;
    }
    let original = axis.clone();
    let mut open: Vec<(usize, bool)> = Vec::new();
    for (index, line) in original.iter().enumerate() {
        for high in [false, true] {
            let point = line.point_at(line.end(high));
            let touching = original
                .iter()
                .enumerate()
                .any(|(other, line)| other != index && line.holds(point));
            if !touching {
                open.push((index, high));
            }
        }
    }

    // The line each free end would be carried on to, and where it reaches.
    let mut reach: BTreeMap<(usize, bool), (usize, f64)> = BTreeMap::new();
    for &(index, high) in &open {
        let line = original[index];
        let end = line.end(high);
        let outward = if high { 1.0 } else { -1.0 };
        let mut farthest: Option<(f64, usize)> = None;
        for (target, other) in original.iter().enumerate() {
            if other.horizontal == line.horizontal || !name_each_other(joined, line.id, other.id) {
                continue;
            }
            let distance = (other.fixed - end) * outward;
            if distance < -NODE_TOLERANCE_FEET || distance > MAX_CARRY_FEET {
                continue;
            }
            match farthest {
                Some((held, _)) if held >= distance => {}
                _ => farthest = Some((distance, target)),
            }
        }
        if let Some((_, target)) = farthest {
            reach.insert((index, high), (target, original[target].fixed));
        }
    }
    // An end that had such a line is not offered a connector, whether or not
    // its carry stands.
    let claimed: BTreeSet<(usize, bool)> = reach.keys().copied().collect();
    // A carry stands only where the line it ends on, once that line is carried
    // on as far as its own carries stand, spans the place it reaches. Dropping
    // one can drop another, so this runs until none is dropped.
    loop {
        let mut dropped = false;
        for key in reach.keys().copied().collect::<Vec<_>>() {
            let (target, _) = reach[&key];
            let across = original[key.0].fixed;
            let other = original[target];
            let low = reach
                .get(&(target, false))
                .map_or(other.low, |&(_, to)| other.low.min(to));
            let high = reach
                .get(&(target, true))
                .map_or(other.high, |&(_, to)| other.high.max(to));
            if across < low - NODE_TOLERANCE_FEET || across > high + NODE_TOLERANCE_FEET {
                reach.remove(&key);
                dropped = true;
            }
        }
        if !dropped {
            break;
        }
    }

    let bare: Vec<(usize, bool)> = open
        .iter()
        .copied()
        .filter(|end| !claimed.contains(end))
        .collect();
    let partners = |(index, high): (usize, bool)| -> Vec<(usize, bool)> {
        let line = original[index];
        let along = line.end(high);
        bare.iter()
            .copied()
            .filter(|&(other_index, other_high)| {
                let other = original[other_index];
                let apart = (other.fixed - line.fixed).abs();
                other_index != index
                    && other.horizontal == line.horizontal
                    && apart > NODE_TOLERANCE_FEET
                    && apart <= MAX_CONNECTOR_FEET
                    && other_high != high
                    && name_each_other(joined, line.id, other.id)
                    && (other.end(other_high) - along).abs() <= NODE_TOLERANCE_FEET
            })
            .collect()
    };
    let mut connectors = Vec::new();
    for &end in &bare {
        let found = partners(end);
        let &[(other_index, other_high)] = found.as_slice() else {
            continue;
        };
        if other_index < end.0 || partners((other_index, other_high)) != [end] {
            continue;
        }
        let (line, other) = (original[end.0], original[other_index]);
        let (low, high) = (line.fixed.min(other.fixed), line.fixed.max(other.fixed));
        connectors.push(AxisLine {
            id: 0,
            horizontal: !line.horizontal,
            fixed: line.end(end.1),
            low,
            high,
            stored_low: low,
            stored_high: high,
            connector: true,
        });
    }

    for ((index, high), (_, target)) in reach {
        if high {
            axis[index].high = axis[index].high.max(target);
        } else {
            axis[index].low = axis[index].low.min(target);
        }
    }
    axis.extend(connectors);
}

/// Points of the arrangement, merged where they are within the tolerance.
#[derive(Default)]
struct Nodes {
    points: Vec<(f64, f64)>,
    cells: BTreeMap<(i64, i64), Vec<usize>>,
}

impl Nodes {
    fn cell(point: (f64, f64)) -> (i64, i64) {
        (
            (point.0 / NODE_TOLERANCE_FEET).floor() as i64,
            (point.1 / NODE_TOLERANCE_FEET).floor() as i64,
        )
    }

    fn intern(&mut self, point: (f64, f64)) -> usize {
        let (cx, cy) = Nodes::cell(point);
        for x in cx - 1..=cx + 1 {
            for y in cy - 1..=cy + 1 {
                let Some(held) = self.cells.get(&(x, y)) else {
                    continue;
                };
                for &index in held {
                    let other = self.points[index];
                    if (other.0 - point.0).abs() <= NODE_TOLERANCE_FEET
                        && (other.1 - point.1).abs() <= NODE_TOLERANCE_FEET
                    {
                        return index;
                    }
                }
            }
        }
        let index = self.points.len();
        self.points.push(point);
        self.cells.entry((cx, cy)).or_default().push(index);
        index
    }
}

/// One closed walk along the arrangement's edges with the same region on its
/// left.
#[derive(Debug)]
struct Ring {
    nodes: Vec<usize>,
    polygon: Vec<(f64, f64)>,
    bounds: [f64; 4],
    area: f64,
    ids: BTreeSet<u32>,
    /// Whether an edge of the ring is carried on past its line's record, or a
    /// connector.
    joined: bool,
    component: usize,
    simple: bool,
}

/// The planar arrangement of a set of area boundary lines.
#[derive(Debug)]
pub struct Arrangement {
    points: Vec<(f64, f64)>,
    degree: Vec<usize>,
    rings: Vec<Ring>,
    /// The rings of positive area: the regions.
    bounded: Vec<usize>,
    /// The ring of negative area that walks round each connected set of
    /// lines, by the set.
    outer_of: BTreeMap<usize, usize>,
    /// The sets with more than one such ring.
    unclear: BTreeSet<usize>,
}

fn find(parent: &mut [usize], node: usize) -> usize {
    let mut root = node;
    while parent[root] != root {
        root = parent[root];
    }
    let mut at = node;
    while parent[at] != root {
        let next = parent[at];
        parent[at] = root;
        at = next;
    }
    root
}

fn signed_area(points: &[(f64, f64)]) -> f64 {
    let mut sum = 0.0;
    for (index, p) in points.iter().enumerate() {
        let q = points[(index + 1) % points.len()];
        sum += p.0 * q.1 - q.0 * p.1;
    }
    sum / 2.0
}

/// Whether `point` lies within the tolerance of the segment `a` to `b`.
fn on_segment(a: (f64, f64), b: (f64, f64), point: (f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length_squared = dx * dx + dy * dy;
    let t = if length_squared <= 0.0 {
        0.0
    } else {
        (((point.0 - a.0) * dx + (point.1 - a.1) * dy) / length_squared).clamp(0.0, 1.0)
    };
    let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
    (point.0 - cx).hypot(point.1 - cy) <= NODE_TOLERANCE_FEET
}

/// Whether `point` is inside the loop `ring`: `None` when it is on the loop.
fn inside(ring: &[(f64, f64)], point: (f64, f64)) -> Option<bool> {
    let mut crossings = 0usize;
    for (index, &a) in ring.iter().enumerate() {
        let b = ring[(index + 1) % ring.len()];
        if on_segment(a, b, point) {
            return None;
        }
        if (a.1 > point.1) != (b.1 > point.1) {
            let x = a.0 + (point.1 - a.1) / (b.1 - a.1) * (b.0 - a.0);
            if x > point.0 {
                crossings += 1;
            }
        }
    }
    Some(!crossings.is_multiple_of(2))
}

/// [`inside`] for a ring, with a quick answer for a point outside its box.
fn inside_ring(ring: &Ring, point: (f64, f64)) -> Option<bool> {
    let [min_x, min_y, max_x, max_y] = ring.bounds;
    let reach = NODE_TOLERANCE_FEET * 2.0;
    if point.0 < min_x - reach
        || point.0 > max_x + reach
        || point.1 < min_y - reach
        || point.1 > max_y + reach
    {
        return Some(false);
    }
    inside(&ring.polygon, point)
}

impl Arrangement {
    /// Whether `line` is a straight line along a plan axis, which is all
    /// [`Arrangement::new`] reads.
    pub fn reads(line: &BoundaryLine) -> bool {
        AxisLine::new(line).is_some()
    }

    /// The arrangement of `lines`, or `None` where they are not all straight
    /// lines along the plan axes, are more than [`MAX_LINES`], or overlap.
    ///
    /// `joined` holds, for each line, the ElementIds its record names. A free
    /// end is carried on, within [`MAX_CARRY_FEET`], to a line that names it and
    /// that it names, where that line spans the place reached; and the free
    /// ends of two such parallel lines within [`MAX_CONNECTOR_FEET`], on
    /// opposite sides of the place they stop, are joined by a connector. An
    /// empty `joined` leaves every line as it is.
    pub fn new(
        lines: &[BoundaryLine],
        joined: &BTreeMap<u32, BTreeSet<u32>>,
    ) -> Option<Arrangement> {
        if lines.is_empty() || lines.len() > MAX_LINES {
            return None;
        }
        let mut axis = Vec::with_capacity(lines.len());
        for line in lines {
            axis.push(AxisLine::new(line)?);
        }
        resolve_joins(&mut axis, joined);
        let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); axis.len()];
        for (i, &a) in axis.iter().enumerate() {
            for (j, &b) in axis.iter().enumerate().skip(i + 1) {
                if a.horizontal == b.horizontal {
                    let overlap = a.high.min(b.high) - a.low.max(b.low);
                    if (a.fixed - b.fixed).abs() <= NODE_TOLERANCE_FEET
                        && overlap > NODE_TOLERANCE_FEET
                    {
                        return None;
                    }
                    continue;
                }
                let (h, v, hi, vi) = if a.horizontal {
                    (a, b, i, j)
                } else {
                    (b, a, j, i)
                };
                let (x, y) = (v.fixed, h.fixed);
                if x >= h.low - NODE_TOLERANCE_FEET
                    && x <= h.high + NODE_TOLERANCE_FEET
                    && y >= v.low - NODE_TOLERANCE_FEET
                    && y <= v.high + NODE_TOLERANCE_FEET
                {
                    cuts[hi].push(x);
                    cuts[vi].push(y);
                }
            }
        }

        let cut_count: usize = cuts.iter().map(Vec::len).sum();
        if cut_count.saturating_add(axis.len().saturating_mul(2)) > MAX_NODES {
            return None;
        }
        let mut nodes = Nodes::default();
        let mut edges: BTreeMap<(usize, usize), (Option<u32>, bool)> = BTreeMap::new();
        for (line, cut) in axis.iter().zip(cuts.iter_mut()) {
            cut.push(line.low);
            cut.push(line.high);
            // The ends the record gives are points of their own, so that an
            // edge lies wholly on the stored extent or wholly past it.
            cut.push(line.stored_low.clamp(line.low, line.high));
            cut.push(line.stored_high.clamp(line.low, line.high));
            cut.sort_by(|p, q| p.partial_cmp(q).unwrap_or(std::cmp::Ordering::Equal));
            let mut previous: Option<(usize, f64)> = None;
            for &along in cut.iter() {
                let along = along.clamp(line.low, line.high);
                let node = nodes.intern(line.point_at(along));
                if let Some((last, last_along)) = previous {
                    if last != node {
                        let key = (last.min(node), last.max(node));
                        let source = (!line.connector).then_some(line.id);
                        if let Some(&(held, _)) = edges.get(&key) {
                            if held != source {
                                return None;
                            }
                        }
                        let middle = (last_along + along) / 2.0;
                        let carried = line.connector
                            || middle < line.stored_low - NODE_TOLERANCE_FEET
                            || middle > line.stored_high + NODE_TOLERANCE_FEET;
                        edges.insert(key, (source, carried));
                    }
                }
                previous = Some((node, along));
            }
        }

        let count = nodes.points.len();
        // Leave out the lines with a free end, and then those that the removal
        // frees, by a worklist of points with one line left: each line is
        // removed once, so the pass is linear in the lines.
        // An edge's two node indices, with its source line (`None` for a
        // connector) and whether it was carried.
        type Candidate = ((usize, usize), (Option<u32>, bool));
        let candidates: Vec<Candidate> = edges.iter().map(|(&ends, &edge)| (ends, edge)).collect();
        let mut alive = vec![true; candidates.len()];
        let mut degree = vec![0usize; count];
        let mut incident: Vec<Vec<usize>> = vec![Vec::new(); count];
        for (index, &((u, v), _)) in candidates.iter().enumerate() {
            degree[u] += 1;
            degree[v] += 1;
            incident[u].push(index);
            incident[v].push(index);
        }
        let mut free: Vec<usize> = (0..count).filter(|&node| degree[node] == 1).collect();
        while let Some(node) = free.pop() {
            if degree[node] != 1 {
                continue;
            }
            let Some(&edge) = incident[node].iter().find(|&&index| alive[index]) else {
                continue;
            };
            alive[edge] = false;
            let ((u, v), _) = candidates[edge];
            let other = if u == node { v } else { u };
            degree[node] -= 1;
            degree[other] -= 1;
            if degree[other] == 1 {
                free.push(other);
            }
        }
        let edge_list: Vec<(usize, usize, Option<u32>, bool)> = candidates
            .iter()
            .zip(&alive)
            .filter(|&(_, &kept)| kept)
            .map(|(&((u, v), (id, carried)), _)| (u, v, id, carried))
            .collect();
        let origin = |half: usize| {
            let (u, v, _, _) = edge_list[half / 2];
            if half.is_multiple_of(2) { u } else { v }
        };
        let target = |half: usize| origin(half ^ 1);
        let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); count];
        for half in 0..edge_list.len() * 2 {
            outgoing[origin(half)].push(half);
        }
        let mut slot = vec![0usize; edge_list.len() * 2];
        for (node, halves) in outgoing.iter_mut().enumerate() {
            let here = nodes.points[node];
            halves.sort_by(|&p, &q| {
                let angle = |half: usize| {
                    let there = nodes.points[target(half)];
                    (there.1 - here.1).atan2(there.0 - here.0)
                };
                angle(p)
                    .partial_cmp(&angle(q))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for (index, &half) in halves.iter().enumerate() {
                slot[half] = index;
            }
        }

        let mut parent: Vec<usize> = (0..count).collect();
        for &(u, v, _, _) in &edge_list {
            let ru = find(&mut parent, u);
            let rv = find(&mut parent, v);
            parent[ru] = rv;
        }

        let mut visited = vec![false; edge_list.len() * 2];
        let mut rings = Vec::new();
        for start in 0..edge_list.len() * 2 {
            if visited.get(start).copied().unwrap_or(true) {
                continue;
            }
            let mut walk = Vec::new();
            let mut ids = BTreeSet::new();
            let mut joined = false;
            let mut at = start;
            loop {
                visited[at] = true;
                walk.push(origin(at));
                ids.extend(edge_list[at / 2].2);
                joined |= edge_list[at / 2].3;
                let here = target(at);
                let around = &outgoing[here];
                let back = slot[at ^ 1];
                at = around[(back + around.len() - 1) % around.len()];
                if at == start || walk.len() > edge_list.len() * 2 {
                    break;
                }
            }
            let points: Vec<(f64, f64)> = walk.iter().map(|&n| nodes.points[n]).collect();
            let distinct: BTreeSet<usize> = walk.iter().copied().collect();
            let component = find(&mut parent, walk[0]);
            let bounds = points.iter().fold(
                [
                    f64::INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::NEG_INFINITY,
                ],
                |b, p| [b[0].min(p.0), b[1].min(p.1), b[2].max(p.0), b[3].max(p.1)],
            );
            rings.push(Ring {
                area: signed_area(&points),
                simple: distinct.len() == walk.len(),
                nodes: walk,
                polygon: points,
                bounds,
                ids,
                joined,
                component,
            });
        }

        let degree = outgoing.iter().map(Vec::len).collect();
        let bounded = (0..rings.len())
            .filter(|&i| rings[i].area > MIN_RING_AREA)
            .collect();
        let mut outer_of: BTreeMap<usize, usize> = BTreeMap::new();
        let mut unclear = BTreeSet::new();
        for (index, ring) in rings.iter().enumerate() {
            if ring.area < -MIN_RING_AREA && outer_of.insert(ring.component, index).is_some() {
                unclear.insert(ring.component);
            }
        }
        Some(Arrangement {
            points: nodes.points,
            degree,
            rings,
            bounded,
            outer_of,
            unclear,
        })
    }

    /// The ring's corners: its nodes, less those where two lines continue
    /// straight on.
    fn corners(&self, ring: &Ring) -> Option<Vec<(f64, f64)>> {
        let near = |p: f64, q: f64| (p - q).abs() <= NODE_TOLERANCE_FEET;
        let count = ring.nodes.len();
        let mut out = Vec::with_capacity(count);
        for (index, &node) in ring.nodes.iter().enumerate() {
            let a = self.points[ring.nodes[(index + count - 1) % count]];
            let b = self.points[node];
            let c = self.points[ring.nodes[(index + 1) % count]];
            let straight = (near(a.0, b.0) && near(b.0, c.0)) || (near(a.1, b.1) && near(b.1, c.1));
            if !(straight && self.degree[node] == 2) {
                out.push(b);
            }
        }
        (out.len() >= 3).then_some(out)
    }

    /// The region around `point`: the smallest loop of lines that encloses it,
    /// less the loops directly inside that. `None` where the point is on a
    /// line, in no loop, or in two loops of one size, or where its loop or one
    /// of its holes touches itself.
    pub fn face_at(&self, point: (f64, f64)) -> Option<AreaFace> {
        if !point.0.is_finite() || !point.1.is_finite() {
            return None;
        }
        let mut chosen: Option<usize> = None;
        for &i in &self.bounded {
            if inside_ring(&self.rings[i], point)? {
                match chosen {
                    Some(held) if self.rings[held].area <= self.rings[i].area => {
                        if (self.rings[held].area - self.rings[i].area).abs() <= MIN_RING_AREA {
                            return None;
                        }
                    }
                    _ => chosen = Some(i),
                }
            }
        }
        let face = &self.rings[chosen?];
        if !face.simple {
            return None;
        }

        let mut holes: Vec<&Ring> = Vec::new();
        for (component, &outer_index) in &self.outer_of {
            if *component == face.component {
                continue;
            }
            if self.unclear.contains(component) {
                return None;
            }
            let outer = &self.rings[outer_index];
            let probe = self.points[outer.nodes[0]];
            if !inside_ring(face, probe)? {
                continue;
            }
            let mut parent: Option<&Ring> = None;
            for &i in &self.bounded {
                let ring = &self.rings[i];
                if ring.component == *component {
                    continue;
                }
                if inside_ring(ring, probe)? {
                    match parent {
                        Some(held) if held.area <= ring.area => {}
                        _ => parent = Some(ring),
                    }
                }
            }
            if parent.is_some_and(|p| std::ptr::eq(p, face)) {
                if !outer.simple {
                    return None;
                }
                holes.push(outer);
            }
        }
        holes.sort_by(|p, q| {
            q.area
                .abs()
                .partial_cmp(&p.area.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut line_ids = face.ids.clone();
        let mut hole_corners = Vec::with_capacity(holes.len());
        for hole in &holes {
            line_ids.extend(hole.ids.iter().copied());
            hole_corners.push(self.corners(hole)?);
        }
        let joined = face.joined || holes.iter().any(|hole| hole.joined);
        Some(AreaFace {
            outer: self.corners(face)?,
            outer_arcs: Vec::new(),
            holes: hole_corners,
            hole_arcs: Vec::new(),
            line_ids,
            joined,
        })
    }
}
