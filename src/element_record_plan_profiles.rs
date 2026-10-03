//! Plan profiles for sketched elements, from `OST_SketchLines`
//! partition element records (#31, RE-25).
//!
//! A Revit floor is a sketch: a set of boundary curves that close one
//! outer loop and zero or more inner loops. On Revit 2024 project
//! partitions each of those curves is framed as its **own** partition
//! element record — the same 88-byte prologue
//! [`crate::partition_element_records`] documents — carrying
//! `BuiltInCategory`
//! [`crate::partition_element_records::OST_SKETCH_LINES`] (-2000045),
//! its own bounding box, and the sketched element's ElementId in
//! [`crate::partition_element_records::PartitionElementRecord::owner_reference`].
//!
//! # The join
//!
//! `owner_reference` — the last slot of the *second* counted
//! reference list at `+0x88` — is an exact ElementId join. It uses no
//! geometry at all: a sketch line belongs to the element the byte
//! names, or to nothing.
//!
//! # The reconstruction
//!
//! A sketch-line record carries its segment's **bounding box**, not
//! its endpoints. For a segment parallel to an axis that is the same
//! thing; for a diagonal it leaves two candidate endpoint pairs (the
//! two diagonals of the box), and a handful of records carry a box
//! that is looser than the segment. Both are resolved by the loop
//! itself, never by the reference export:
//!
//! 1. Every segment whose box is degenerate on exactly one axis
//!    contributes its endpoints outright.
//! 2. Every remaining segment is placed only when exactly **one**
//!    pair of still-open vertices fits its box — first trying the
//!    corner pairs (the box's own diagonals), then, only when no
//!    corner pair is open, a pair that spans the box lengthwise and
//!    is degenerate across it.
//! 3. Placement repeats until nothing is left. A segment that never
//!    has exactly one candidate, a vertex that does not end at degree
//!    2, an unused edge, or a loop shorter than three vertices all
//!    reject the whole element — [`plan_profile_from_segments`]
//!    returns `None` rather than a guessed polygon.
//!
//! Collinear vertices are then merged, which is what makes the
//! recovered loop comparable to an exporter's: Revit splits one
//! straight boundary run into several sketch lines.
//!
//! # Honesty
//!
//! - Nothing here invents a vertex. Every coordinate emitted is a
//!   corner of a recorded bounding box, or (RE-50,
//!   [`plan_profile_from_lines`]) an end of the line a sketch line's own
//!   data records, used only where the boxes do not close and only when
//!   that end lies in the sketch line's box.
//! - The endpoint choice is a *closure* rule, not a fit: it accepts
//!   only when the choice is forced, and rejects the element
//!   otherwise. It is not scored against, or tuned to, the reference
//!   export — RE-25 measures it afterwards.
//! - Loops are ordered by absolute area, largest first, and the
//!   largest is named the outer loop. Containment is not tested; on
//!   the recorded edge every inner loop lies inside the outer one,
//!   and a sketch where it does not would need its own measurement.
//! - What Revit calls the second reference list is not claimed. Only
//!   that its last slot names the sketched element on the recorded
//!   edge.

use crate::partition_element_records::PartitionElementRecord;
use crate::walker::InstanceField;
use std::collections::BTreeMap;

/// Vertex-coincidence tolerance, in feet.
///
/// Recorded plan coordinates carry the floating dust of Revit's own
/// transform (`177.00000000000011` for a nominal 177 ft), so vertices
/// are matched with a tolerance rather than by bit equality.
pub const VERTEX_EPS_FEET: f64 = 1e-6;

/// Relative cross-product below which three vertices are collinear.
pub const COLLINEAR_EPS: f64 = 1e-9;

/// Largest distance a sketch arc's chords may stray from the arc, feet
/// (RE-96), as a curved wall's are drawn (RE-75).
pub const SKETCH_ARC_TOLERANCE_FEET: f64 = 0.0005;

/// A sketch arc drawn as points from its start to its end, each chord
/// within [`SKETCH_ARC_TOLERANCE_FEET`] of the arc (RE-96). `None` for an arc
/// sweeping no angle or more than a full turn.
pub fn arc_points(arc: &crate::partition_beam_axes::BoundedArc) -> Option<Vec<[f64; 3]>> {
    let sweep = arc.end_angle - arc.start_angle;
    if !(sweep.is_finite() && sweep > 0.0 && sweep <= std::f64::consts::TAU) {
        return None;
    }
    let step = 2.0
        * (1.0 - SKETCH_ARC_TOLERANCE_FEET / arc.radius)
            .clamp(-1.0, 1.0)
            .acos();
    let count = ((sweep / step).ceil() as usize).clamp(1, 512);
    Some(
        (0..=count)
            .map(|index| arc.point(arc.start_angle + sweep * index as f64 / count as f64))
            .collect(),
    )
}

/// Value of [`PLAN_PROFILE_SOURCE_FIELD`] for this carrier.
pub const PLAN_PROFILE_SOURCE: &str = "partition_element_record_sketch_lines";

/// Field carrying the recovered outer boundary loop.
pub const PLAN_PROFILE_OUTER_FIELD: &str = "m_plan_profile_outer";
/// Field carrying the recovered inner boundary loops (voids).
pub const PLAN_PROFILE_INNER_FIELD: &str = "m_plan_profile_inner";
/// Field recording where the profile came from.
pub const PLAN_PROFILE_SOURCE_FIELD: &str = "m_plan_profile_source";
/// Field recording how many sketch-line records the profile used.
pub const PLAN_PROFILE_SEGMENTS_FIELD: &str = "m_plan_profile_segments";
/// Field carrying the further pieces of a sketch made of separate loops
/// (#331): each piece an outer loop followed by its voids.
pub const PLAN_PROFILE_PIECES_FIELD: &str = "m_plan_profile_pieces";

/// One further piece of a plan profile whose sketch is several separate
/// loops (#331): an outer loop and the voids inside it.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanPiece {
    /// Outer boundary loop, counter-clockwise.
    pub outer_xy: Vec<(f64, f64)>,
    /// Inner boundary loops (voids), clockwise, largest first.
    pub inner_xy: Vec<Vec<(f64, f64)>>,
}

/// A recovered plan profile: one outer loop and zero or more voids.
///
/// Vertices are in project plan coordinates (feet), in loop order,
/// without a repeated closing vertex. The outer loop is
/// counter-clockwise and every inner loop clockwise.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanProfile {
    /// Outer boundary loop, counter-clockwise.
    pub outer_xy: Vec<(f64, f64)>,
    /// Inner boundary loops (voids), clockwise, largest first.
    pub inner_xy: Vec<Vec<(f64, f64)>>,
    /// ElementIds of the sketch-line records the loops were built
    /// from, ascending.
    pub segment_ids: Vec<u32>,
    /// Further pieces, largest first, when the sketch is several separate
    /// loops rather than one outer loop with voids (#331). Revit's own
    /// export writes each piece as its own element with the element's
    /// `Tag`. Empty for a single-piece sketch.
    pub pieces: Vec<PlanPiece>,
}

impl PlanProfile {
    /// Total vertex count across every loop of every piece.
    pub fn vertex_count(&self) -> usize {
        let loops = |outer: &[(f64, f64)], inner: &[Vec<(f64, f64)>]| {
            outer.len() + inner.iter().map(Vec::len).sum::<usize>()
        };
        loops(&self.outer_xy, &self.inner_xy)
            + self
                .pieces
                .iter()
                .map(|piece| loops(&piece.outer_xy, &piece.inner_xy))
                .sum::<usize>()
    }

    /// The profile's plan bounding box `[min_x, min_y, max_x, max_y]`.
    pub fn plan_bounds_feet(&self) -> Option<[f64; 4]> {
        let mut bounds: Option<[f64; 4]> = None;
        for &(x, y) in &self.outer_xy {
            bounds = Some(match bounds {
                None => [x, y, x, y],
                Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
            });
        }
        bounds
    }

    /// Plan extent of every piece's outer loop, `[min x, min y, max x,
    /// max y]` feet (RE-96).
    pub fn plan_extent_feet(&self) -> Option<[f64; 4]> {
        self.outer_xy
            .iter()
            .chain(self.pieces.iter().flat_map(|piece| piece.outer_xy.iter()))
            .fold(None, |bounds, &(x, y)| {
                Some(match bounds {
                    None => [x, y, x, y],
                    Some(b) => [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)],
                })
            })
    }

    /// The `DecodedElement` fields carrying this profile.
    pub fn fields(&self) -> Vec<(String, InstanceField)> {
        vec![
            (PLAN_PROFILE_OUTER_FIELD.into(), loop_field(&self.outer_xy)),
            (
                PLAN_PROFILE_INNER_FIELD.into(),
                InstanceField::Vector(self.inner_xy.iter().map(|l| loop_field(l)).collect()),
            ),
            (
                PLAN_PROFILE_SOURCE_FIELD.into(),
                InstanceField::String(PLAN_PROFILE_SOURCE.into()),
            ),
            (
                PLAN_PROFILE_SEGMENTS_FIELD.into(),
                InstanceField::Integer {
                    value: self.segment_ids.len() as i64,
                    signed: false,
                    size: 8,
                },
            ),
            (
                PLAN_PROFILE_PIECES_FIELD.into(),
                InstanceField::Vector(
                    self.pieces
                        .iter()
                        .map(|piece| {
                            InstanceField::Vector(vec![
                                loop_field(&piece.outer_xy),
                                InstanceField::Vector(
                                    piece.inner_xy.iter().map(|l| loop_field(l)).collect(),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]
    }
}

/// Read back a profile written by [`PlanProfile::fields`].
///
/// Returns `None` unless [`PLAN_PROFILE_SOURCE_FIELD`] names this
/// carrier and the outer loop decodes to at least three vertices, so
/// a consumer never mistakes some other field set for a profile.
/// `segment_ids` is not round-tripped; the count is.
pub fn plan_profile_from_fields(fields: &[(String, InstanceField)]) -> Option<PlanProfile> {
    let mut source_ok = false;
    let mut outer = None;
    let mut inner = Vec::new();
    let mut segments = 0usize;
    let mut pieces = Vec::new();
    for (name, value) in fields {
        match (name.as_str(), value) {
            (PLAN_PROFILE_SOURCE_FIELD, InstanceField::String(text)) => {
                source_ok = text == PLAN_PROFILE_SOURCE
                    || text == crate::partition_room_boundaries::ROOM_OUTLINE_SOURCE;
            }
            (PLAN_PROFILE_OUTER_FIELD, field) => outer = points_from_field(field),
            (PLAN_PROFILE_INNER_FIELD, InstanceField::Vector(loops)) => {
                inner = loops.iter().filter_map(points_from_field).collect();
            }
            (PLAN_PROFILE_SEGMENTS_FIELD, InstanceField::Integer { value, .. }) => {
                segments = (*value).max(0) as usize;
            }
            (PLAN_PROFILE_PIECES_FIELD, InstanceField::Vector(items)) => {
                pieces = items.iter().filter_map(piece_from_field).collect();
            }
            _ => {}
        }
    }
    if !source_ok {
        return None;
    }
    let outer = outer?;
    if outer.len() < 3 {
        return None;
    }
    Some(PlanProfile {
        outer_xy: outer,
        inner_xy: inner,
        segment_ids: vec![0; segments],
        pieces,
    })
}

fn piece_from_field(field: &InstanceField) -> Option<PlanPiece> {
    let InstanceField::Vector(parts) = field else {
        return None;
    };
    let [outer, InstanceField::Vector(inner)] = parts.as_slice() else {
        return None;
    };
    let outer_xy = points_from_field(outer)?;
    if outer_xy.len() < 3 {
        return None;
    }
    Some(PlanPiece {
        outer_xy,
        inner_xy: inner.iter().filter_map(points_from_field).collect(),
    })
}

fn points_from_field(field: &InstanceField) -> Option<Vec<(f64, f64)>> {
    let InstanceField::Vector(points) = field else {
        return None;
    };
    let mut out = Vec::with_capacity(points.len());
    for point in points {
        let InstanceField::Vector(pair) = point else {
            return None;
        };
        match pair.as_slice() {
            [
                InstanceField::Float { value: x, .. },
                InstanceField::Float { value: y, .. },
            ] => out.push((*x, *y)),
            _ => return None,
        }
    }
    Some(out)
}

fn loop_field(points: &[(f64, f64)]) -> InstanceField {
    InstanceField::Vector(
        points
            .iter()
            .map(|&(x, y)| {
                InstanceField::Vector(vec![
                    InstanceField::Float { value: x, size: 8 },
                    InstanceField::Float { value: y, size: 8 },
                ])
            })
            .collect(),
    )
}

/// Group sketch-line records by the element they name and recover one
/// [`PlanProfile`] per element that closes.
///
/// Records that carry no [`PartitionElementRecord::owner_reference`]
/// are dropped. A single sketch line can be framed in more than one
/// partition stream — on `2024_Core_Interior.rvt` 3106 distinct
/// sketch-line ids are framed 3724 times and every duplicate agrees
/// on both the box and the owner — so records are de-duplicated by
/// ElementId, keeping the first by `(stream, offset)`.
///
/// Elements whose sketch does not close under
/// [`plan_profile_from_segments`] are absent from the result: there
/// is no partial or best-effort profile.
pub fn plan_profiles_from_sketch_line_records(
    records: &[PartitionElementRecord],
) -> BTreeMap<u32, PlanProfile> {
    let mut by_owner: BTreeMap<u32, BTreeMap<u32, PartitionElementRecord>> = BTreeMap::new();
    for record in records {
        let Some(owner) = record.owner_reference else {
            continue;
        };
        let segments = by_owner.entry(owner).or_default();
        let keep = match segments.get(&record.element_id) {
            None => true,
            Some(existing) => {
                (record.stream.as_str(), record.offset)
                    < (existing.stream.as_str(), existing.offset)
            }
        };
        if keep {
            segments.insert(record.element_id, record.clone());
        }
    }
    let mut out = BTreeMap::new();
    for (owner, segments) in by_owner {
        let ids: Vec<u32> = segments.keys().copied().collect();
        let boxes: Vec<[f64; 4]> = segments
            .values()
            .map(|r| {
                [
                    r.bbox_feet[0],
                    r.bbox_feet[1],
                    r.bbox_feet[3],
                    r.bbox_feet[4],
                ]
            })
            .collect();
        if let Some(mut profile) = plan_profile_from_segments(&boxes) {
            profile.segment_ids = ids;
            out.insert(owner, profile);
        }
    }
    out
}

/// Recover one plan profile from a set of segment plan bounding boxes
/// `[min_x, min_y, max_x, max_y]`, or `None` when the set does not
/// close unambiguously.
pub fn plan_profile_from_segments(segments: &[[f64; 4]]) -> Option<PlanProfile> {
    profile_from_loops(solve_loops(segments)?)
}

/// Recover one plan profile from segments whose ends are known exactly,
/// `[x0, y0, x1, y1]` (RE-50: the ends a sketch line's own data carries).
/// Every end must be shared by exactly two segments, so the segments close
/// into loops with no choice left; otherwise `None`.
pub fn plan_profile_from_lines(lines: &[[f64; 4]]) -> Option<PlanProfile> {
    if lines.len() < 3 {
        return None;
    }
    let mut vertices = Vertices::default();
    let mut edges: Vec<(usize, usize)> = Vec::with_capacity(lines.len());
    for &[x0, y0, x1, y1] in lines {
        if ![x0, y0, x1, y1].iter().all(|v| v.is_finite()) {
            return None;
        }
        let (a, b) = (vertices.intern((x0, y0)), vertices.intern((x1, y1)));
        if a == b {
            return None;
        }
        vertices.degree[a] += 1;
        vertices.degree[b] += 1;
        edges.push((a, b));
    }
    if vertices.degree.iter().any(|&d| d != 2) {
        return None;
    }
    let mut at: Vec<Vec<usize>> = vec![Vec::new(); vertices.points.len()];
    for (index, &(a, b)) in edges.iter().enumerate() {
        at[a].push(index);
        at[b].push(index);
    }
    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();
    for first in 0..edges.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let (start, mut current) = edges[first];
        let mut ring = vec![vertices.points[start]];
        while current != start {
            ring.push(vertices.points[current]);
            let next = at[current].iter().copied().find(|&e| !used[e])?;
            used[next] = true;
            let (a, b) = edges[next];
            current = if a == current { b } else { a };
        }
        loops.push(ring);
    }
    profile_from_loops(loops)
}

/// Order loops into pieces and voids: each loop's depth is how many larger
/// loops contain it.
fn profile_from_loops(loops: Vec<Vec<(f64, f64)>>) -> Option<PlanProfile> {
    let mut merged: Vec<Vec<(f64, f64)>> = Vec::with_capacity(loops.len());
    for one in loops {
        let simplified = merge_collinear(&one);
        if simplified.len() < 3 {
            return None;
        }
        merged.push(simplified);
    }
    merged.sort_by(|a, b| {
        signed_area(b)
            .abs()
            .partial_cmp(&signed_area(a).abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    // Each loop's depth is how many larger loops contain it. An even depth
    // starts a piece; an odd depth is a void of the loop it sits in. A loop
    // that is partly inside another is not a region at all.
    let mut container: Vec<Option<usize>> = Vec::with_capacity(merged.len());
    let mut depth: Vec<usize> = Vec::with_capacity(merged.len());
    for (index, one) in merged.iter().enumerate() {
        let mut parent = None;
        for (earlier, candidate) in merged[..index].iter().enumerate() {
            let within = one.iter().filter(|p| inside(candidate, **p)).count();
            if within == one.len() {
                parent = Some(earlier);
            } else if within > 0 {
                return None;
            }
        }
        depth.push(parent.map_or(0, |p| depth[p] + 1));
        container.push(parent);
    }
    let mut pieces: Vec<PlanPiece> = Vec::new();
    let mut piece_of: Vec<Option<usize>> = vec![None; merged.len()];
    for (index, one) in merged.iter().enumerate() {
        let mut ring = one.clone();
        if depth[index].is_multiple_of(2) {
            if signed_area(&ring) < 0.0 {
                ring.reverse();
            }
            piece_of[index] = Some(pieces.len());
            pieces.push(PlanPiece {
                outer_xy: ring,
                inner_xy: Vec::new(),
            });
        } else {
            if signed_area(&ring) > 0.0 {
                ring.reverse();
            }
            let piece = container[index].and_then(|c| piece_of[c])?;
            pieces[piece].inner_xy.push(ring);
        }
    }
    let mut pieces = pieces.into_iter();
    let first = pieces.next()?;
    Some(PlanProfile {
        outer_xy: first.outer_xy,
        inner_xy: first.inner_xy,
        segment_ids: Vec::new(),
        pieces: pieces.collect(),
    })
}

/// Vertex registry with tolerance matching.
#[derive(Default)]
struct Vertices {
    points: Vec<(f64, f64)>,
    degree: Vec<usize>,
}

impl Vertices {
    fn intern(&mut self, point: (f64, f64)) -> usize {
        for (index, held) in self.points.iter().enumerate() {
            if same(*held, point) {
                return index;
            }
        }
        self.points.push(point);
        self.degree.push(0);
        self.points.len() - 1
    }
}

fn same(a: (f64, f64), b: (f64, f64)) -> bool {
    (a.0 - b.0).abs() <= VERTEX_EPS_FEET && (a.1 - b.1).abs() <= VERTEX_EPS_FEET
}

fn solve_loops(segments: &[[f64; 4]]) -> Option<Vec<Vec<(f64, f64)>>> {
    if segments.len() < 3 {
        return None;
    }
    let mut vertices = Vertices::default();
    let mut edges: Vec<(usize, usize)> = Vec::with_capacity(segments.len());
    let mut pending: Vec<[f64; 4]> = Vec::new();
    for span in segments {
        let [x0, y0, x1, y1] = *span;
        if !(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite()) {
            return None;
        }
        let flat_x = (x1 - x0).abs() <= VERTEX_EPS_FEET;
        let flat_y = (y1 - y0).abs() <= VERTEX_EPS_FEET;
        if flat_x && flat_y {
            // A segment with no extent at all is not a boundary line.
            return None;
        } else if flat_x {
            push_edge(&mut vertices, &mut edges, (x0, y0), (x0, y1))?;
        } else if flat_y {
            push_edge(&mut vertices, &mut edges, (x0, y0), (x1, y0))?;
        } else {
            pending.push(*span);
        }
    }
    while !pending.is_empty() {
        let mut placed = None;
        for (index, span) in pending.iter().enumerate() {
            if let Some(pair) = sole_candidate(&vertices, &edges, span) {
                placed = Some((index, pair));
                break;
            }
        }
        let (index, (a, b)) = placed?;
        pending.remove(index);
        push_edge(&mut vertices, &mut edges, a, b)?;
    }
    if vertices.degree.iter().any(|d| *d != 2) {
        return None;
    }
    trace_loops(&vertices, &edges)
}

fn push_edge(
    vertices: &mut Vertices,
    edges: &mut Vec<(usize, usize)>,
    a: (f64, f64),
    b: (f64, f64),
) -> Option<()> {
    let ia = vertices.intern(a);
    let ib = vertices.intern(b);
    if ia == ib {
        return None;
    }
    if edges
        .iter()
        .any(|(p, q)| (*p == ia && *q == ib) || (*p == ib && *q == ia))
    {
        return None;
    }
    vertices.degree[ia] += 1;
    vertices.degree[ib] += 1;
    if vertices.degree[ia] > 2 || vertices.degree[ib] > 2 {
        return None;
    }
    edges.push((ia, ib));
    Some(())
}

/// The one open vertex pair that fits `span`, or `None` when there is
/// no such pair or more than one.
fn sole_candidate(
    vertices: &Vertices,
    edges: &[(usize, usize)],
    span: &[f64; 4],
) -> Option<((f64, f64), (f64, f64))> {
    let [x0, y0, x1, y1] = *span;
    let open: Vec<usize> = (0..vertices.points.len())
        .filter(|index| {
            let (x, y) = vertices.points[*index];
            vertices.degree[*index] < 2
                && x >= x0 - VERTEX_EPS_FEET
                && x <= x1 + VERTEX_EPS_FEET
                && y >= y0 - VERTEX_EPS_FEET
                && y <= y1 + VERTEX_EPS_FEET
        })
        .collect();
    let mut corner: Vec<((f64, f64), (f64, f64))> = Vec::new();
    let mut spanning: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for (slot, ia) in open.iter().enumerate() {
        for ib in open.iter().skip(slot + 1) {
            if edges
                .iter()
                .any(|(p, q)| (p == ia && q == ib) || (p == ib && q == ia))
            {
                continue;
            }
            let a = vertices.points[*ia];
            let b = vertices.points[*ib];
            let fills_x = near(a.0.min(b.0), x0) && near(a.0.max(b.0), x1);
            let fills_y = near(a.1.min(b.1), y0) && near(a.1.max(b.1), y1);
            if fills_x && fills_y {
                corner.push((a, b));
            } else if (fills_x && near(a.1, b.1)) || (fills_y && near(a.0, b.0)) {
                // A straight sub-line that spans the box along one
                // axis and is degenerate across it — the shape of a
                // segment whose recorded box is looser than the line.
                spanning.push((a, b));
            }
        }
    }
    let tier = if corner.is_empty() { spanning } else { corner };
    if tier.len() == 1 {
        return tier.into_iter().next();
    }
    None
}

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() <= VERTEX_EPS_FEET
}

fn trace_loops(vertices: &Vertices, edges: &[(usize, usize)]) -> Option<Vec<Vec<(f64, f64)>>> {
    let mut adjacency: Vec<Vec<(usize, usize)>> = vec![Vec::new(); vertices.points.len()];
    for (index, (a, b)) in edges.iter().enumerate() {
        adjacency[*a].push((index, *b));
        adjacency[*b].push((index, *a));
    }
    let mut used = vec![false; edges.len()];
    let mut loops = Vec::new();
    for start in 0..vertices.points.len() {
        if adjacency[start].iter().all(|(index, _)| used[*index]) {
            continue;
        }
        let mut order = vec![start];
        let mut current = start;
        let mut last: Option<usize> = None;
        loop {
            let step = adjacency[current]
                .iter()
                .find(|(index, _)| !used[*index] && Some(*index) != last)
                .copied();
            let Some((index, next)) = step else {
                break;
            };
            used[index] = true;
            last = Some(index);
            current = next;
            if current == start {
                break;
            }
            order.push(current);
        }
        if order.len() < 3 || current != start {
            return None;
        }
        loops.push(order.into_iter().map(|i| vertices.points[i]).collect());
    }
    if used.iter().any(|done| !done) {
        return None;
    }
    Some(loops)
}

fn merge_collinear(points: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let count = points.len();
    let mut out = Vec::with_capacity(count);
    for index in 0..count {
        let previous = points[(index + count - 1) % count];
        let current = points[index];
        let next = points[(index + 1) % count];
        let first = (current.0 - previous.0, current.1 - previous.1);
        let second = (next.0 - current.0, next.1 - current.1);
        let cross = first.0 * second.1 - first.1 * second.0;
        let scale = first.0.hypot(first.1) * second.0.hypot(second.1);
        if scale > 0.0 && (cross / scale).abs() < COLLINEAR_EPS {
            continue;
        }
        out.push(current);
    }
    out
}

/// Whether `point` lies inside the closed loop `polygon` (even-odd rule).
fn inside(polygon: &[(f64, f64)], point: (f64, f64)) -> bool {
    let mut within = false;
    let mut previous = match polygon.last() {
        Some(last) => *last,
        None => return false,
    };
    for &current in polygon {
        let (x0, y0) = previous;
        let (x1, y1) = current;
        if (y1 > point.1) != (y0 > point.1) {
            let crossing = x1 + (point.1 - y1) * (x0 - x1) / (y0 - y1);
            if point.0 < crossing {
                within = !within;
            }
        }
        previous = current;
    }
    within
}

fn signed_area(points: &[(f64, f64)]) -> f64 {
    let count = points.len();
    let mut total = 0.0;
    for index in 0..count {
        let (x0, y0) = points[index];
        let (x1, y1) = points[(index + 1) % count];
        total += x0 * y1 - x1 * y0;
    }
    total / 2.0
}

/// Offset of the sketched element's `u64` ElementId in a Sketch element's
/// data, from the start of its element-data header (RE-97).
pub const SKETCH_OWNER_OFFSET: usize = 77;
/// How far into a Sketch element's data its curve list is looked for.
pub const SKETCH_DATA_WINDOW: usize = 0x4000;
/// Most curves a sketch's list is read with.
pub const SKETCH_MAX_CURVES: usize = 4096;
/// Fewest curves a sketch's list is read with: a lone id after a count of 1
/// or 2 is too easily met by chance.
pub const SKETCH_MIN_CURVES: usize = 3;

/// The sketch lines of each sketched element, from its Sketch element's
/// data (RE-97): the sketched element's ElementId at
/// [`SKETCH_OWNER_OFFSET`], and a counted list `u32 n · n × (u64 ElementId ·
/// u32 index)` of at least [`SKETCH_MIN_CURVES`] whose indices rise (a
/// sketch edited to drop curves leaves gaps) and whose ids are all in
/// `sketch_lines`. A sketch line's own owner reference names another element
/// where the sketch was edited, so the list is the sketch's membership. An
/// element whose copies disagree, or a line two sketches list, is dropped.
/// Empty for a release this layout is not measured on (Revit 2024 only).
pub fn scan_sketch_curve_lists(
    rf: &mut crate::RevitFile,
    revit_version: u32,
    sketch_lines: &std::collections::BTreeSet<u32>,
) -> BTreeMap<u32, Vec<u32>> {
    if revit_version != 2024 || sketch_lines.is_empty() {
        return BTreeMap::new();
    }
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return BTreeMap::new();
    };
    let mut found: BTreeMap<u32, Option<Vec<u32>>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(SKETCH_DATA_WINDOW));
            let Some(data) = buf.get(hit..end) else {
                continue;
            };
            let Some((owner, curves)) = sketch_curve_list(data, header.len(), sketch_lines) else {
                continue;
            };
            match found.get(&owner) {
                None => {
                    found.insert(owner, Some(curves));
                }
                Some(held) if held.as_ref() != Some(&curves) => {
                    found.insert(owner, None);
                }
                _ => {}
            }
        }
    }
    let lists: BTreeMap<u32, Vec<u32>> = found
        .into_iter()
        .filter_map(|(owner, curves)| Some((owner, curves?)))
        .collect();
    let mut listed: BTreeMap<u32, usize> = BTreeMap::new();
    for curves in lists.values() {
        for id in curves {
            *listed.entry(*id).or_default() += 1;
        }
    }
    lists
        .into_iter()
        .filter(|(_, curves)| curves.iter().all(|id| listed[id] == 1))
        .collect()
}

/// The sketched element and curve list in one element's data, `data`
/// starting at its element-data header of `header_len` bytes.
fn sketch_curve_list(
    data: &[u8],
    header_len: usize,
    sketch_lines: &std::collections::BTreeSet<u32>,
) -> Option<(u32, Vec<u32>)> {
    let u64_at = |at: usize| {
        data.get(at..at.checked_add(8)?)
            .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
    };
    let u32_at = |at: usize| {
        data.get(at..at.checked_add(4)?)
            .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
    };
    let own = u32::try_from(u64_at(header_len)?).ok()?;
    let owner = u32::try_from(u64_at(SKETCH_OWNER_OFFSET)?).ok()?;
    if owner == own || sketch_lines.contains(&owner) {
        return None;
    }
    (header_len + 8..data.len()).find_map(|at| {
        let count = usize::try_from(u32_at(at)?).ok()?;
        if !(SKETCH_MIN_CURVES..=SKETCH_MAX_CURVES).contains(&count) {
            return None;
        }
        let mut last: Option<u32> = None;
        let curves: Option<Vec<u32>> = (0..count)
            .map(|index| {
                let entry = at + 4 + 12 * index;
                let id = u32::try_from(u64_at(entry)?).ok()?;
                let key = u32_at(entry + 8)?;
                if last.is_some_and(|last| key <= last) || !sketch_lines.contains(&id) {
                    return None;
                }
                last = Some(key);
                Some(id)
            })
            .collect();
        Some((owner, curves?))
    })
}

/// Whether an edge of loop `a` touches or crosses an edge of loop `b`.
fn edges_meet(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    let edges = |points: &[(f64, f64)]| -> Vec<((f64, f64), (f64, f64))> {
        (0..points.len())
            .map(|index| (points[index], points[(index + 1) % points.len()]))
            .collect()
    };
    let cross = |o: (f64, f64), p: (f64, f64), q: (f64, f64)| {
        (p.0 - o.0) * (q.1 - o.1) - (p.1 - o.1) * (q.0 - o.0)
    };
    let (edges_a, edges_b) = (edges(a), edges(b));
    edges_a.iter().any(|&(p, q)| {
        edges_b.iter().any(|&(r, t)| {
            cross(p, q, r) * cross(p, q, t) <= 0.0 && cross(r, t, p) * cross(r, t, q) <= 0.0
        })
    })
}

/// Whether the loop `inner` lies strictly inside the loop `outer`: every
/// vertex of `inner` inside `outer`, no vertex of `outer` inside `inner`,
/// and no edge of one touching an edge of the other (RE-99).
pub fn loop_within(outer: &[(f64, f64)], inner: &[(f64, f64)]) -> bool {
    outer.len() >= 3
        && inner.len() >= 3
        && inner.iter().all(|&point| inside(outer, point))
        && !outer.iter().any(|&point| inside(inner, point))
        && !edges_meet(outer, inner)
}

/// Whether loops `a` and `b` are apart: neither has a vertex inside the
/// other and no edges touch (RE-99).
pub fn loops_disjoint(a: &[(f64, f64)], b: &[(f64, f64)]) -> bool {
    !a.iter().any(|&point| inside(b, point))
        && !b.iter().any(|&point| inside(a, point))
        && !edges_meet(a, b)
}

/// Add `voids` to the plan profile `fields` record (RE-99), clockwise, with
/// every void kept largest first. `false`, changing nothing, where the fields
/// hold no profile.
pub fn add_voids_to_fields(
    fields: &mut [(String, InstanceField)],
    voids: &[Vec<(f64, f64)>],
) -> bool {
    let Some(profile) = plan_profile_from_fields(fields) else {
        return false;
    };
    let mut inner = profile.inner_xy;
    for void in voids {
        let mut ring = void.clone();
        if signed_area(&ring) > 0.0 {
            ring.reverse();
        }
        inner.push(ring);
    }
    inner.sort_by(|a, b| signed_area(b).abs().total_cmp(&signed_area(a).abs()));
    let Some((_, field)) = fields
        .iter_mut()
        .find(|(name, _)| name == PLAN_PROFILE_INNER_FIELD)
    else {
        return false;
    };
    *field = InstanceField::Vector(inner.iter().map(|ring| loop_field(ring)).collect());
    true
}
