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
//!   corner of a recorded bounding box.
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
}

impl PlanProfile {
    /// Total vertex count across every loop.
    pub fn vertex_count(&self) -> usize {
        self.outer_xy.len() + self.inner_xy.iter().map(Vec::len).sum::<usize>()
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
    for (name, value) in fields {
        match (name.as_str(), value) {
            (PLAN_PROFILE_SOURCE_FIELD, InstanceField::String(text)) => {
                source_ok = text == PLAN_PROFILE_SOURCE;
            }
            (PLAN_PROFILE_OUTER_FIELD, field) => outer = points_from_field(field),
            (PLAN_PROFILE_INNER_FIELD, InstanceField::Vector(loops)) => {
                inner = loops.iter().filter_map(points_from_field).collect();
            }
            (PLAN_PROFILE_SEGMENTS_FIELD, InstanceField::Integer { value, .. }) => {
                segments = (*value).max(0) as usize;
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
    let loops = solve_loops(segments)?;
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
    let mut iter = merged.into_iter();
    let mut outer = iter.next()?;
    if signed_area(&outer) < 0.0 {
        outer.reverse();
    }
    let mut inner = Vec::new();
    for mut one in iter {
        if signed_area(&one) > 0.0 {
            one.reverse();
        }
        inner.push(one);
    }
    Some(PlanProfile {
        outer_xy: outer,
        inner_xy: inner,
        segment_ids: Vec::new(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 4]> {
        vec![
            [x0, y0, x0, y1],
            [x0, y1, x1, y1],
            [x1, y0, x1, y1],
            [x0, y0, x1, y0],
        ]
    }

    #[test]
    fn four_axis_aligned_boxes_close_a_rectangle() {
        let profile = plan_profile_from_segments(&rect(20.0, 25.0, 167.0, 114.0)).expect("closes");
        assert!(profile.inner_xy.is_empty());
        assert_eq!(profile.outer_xy.len(), 4);
        assert!(signed_area(&profile.outer_xy) > 0.0, "outer is CCW");
        assert_eq!(profile.plan_bounds_feet(), Some([20.0, 25.0, 167.0, 114.0]));
    }

    #[test]
    fn a_box_looser_than_its_segment_is_closed_by_its_neighbours() {
        // `Partitions/46` frames the top boundary of the 20 by 25 to
        // 167 by 114 floor plate with a box one foot either side of
        // the line (RE-25 §3). The three tight edges pin (20,114) and
        // (167,114), and nothing else in the box is open, so the
        // loose segment is forced.
        let mut segments = rect(20.0, 25.0, 167.0, 114.0);
        segments[1] = [20.0, 113.0, 167.0, 115.0];
        let profile = plan_profile_from_segments(&segments).expect("closes");
        let mut ys: Vec<f64> = profile.outer_xy.iter().map(|p| p.1).collect();
        ys.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(ys, vec![25.0, 25.0, 114.0, 114.0]);
    }

    #[test]
    fn a_sawtooth_resolves_its_diagonals_from_the_open_vertices() {
        // The shape of the Core Interior perimeter run: a flat lead-in,
        // then teeth of "vertical up, diagonal down to the right".
        // Both diagonal boxes admit two corner pairs on their own; only
        // one pair per box has both corners still open.
        let segments = vec![
            [0.0, 0.0, 10.0, 0.0],
            [10.0, 0.0, 10.0, 8.0],
            [10.0, 0.0, 20.0, 8.0],
            [20.0, 0.0, 20.0, 8.0],
            [20.0, 0.0, 30.0, 8.0],
            [30.0, 0.0, 40.0, 0.0],
            [40.0, -10.0, 40.0, 0.0],
            [0.0, -10.0, 40.0, -10.0],
            [0.0, -10.0, 0.0, 0.0],
        ];
        let profile = plan_profile_from_segments(&segments).expect("closes");
        assert_eq!(profile.outer_xy.len(), 9);
        let has = |x: f64, y: f64| profile.outer_xy.iter().any(|p| same(*p, (x, y)));
        assert!(has(10.0, 8.0) && has(20.0, 0.0) && has(20.0, 8.0) && has(30.0, 0.0));
        // The wrong diagonal orientation would put these on the loop.
        assert!(!has(10.0, -8.0) && !has(30.0, 8.0));
    }

    #[test]
    fn two_loops_split_into_outer_and_void() {
        let mut segments = rect(0.0, 0.0, 100.0, 100.0);
        segments.extend(rect(40.0, 40.0, 60.0, 60.0));
        let profile = plan_profile_from_segments(&segments).expect("closes");
        assert_eq!(profile.inner_xy.len(), 1);
        assert_eq!(profile.plan_bounds_feet(), Some([0.0, 0.0, 100.0, 100.0]));
        assert!(signed_area(&profile.outer_xy) > 0.0);
        assert!(signed_area(&profile.inner_xy[0]) < 0.0, "void is CW");
    }

    #[test]
    fn an_open_chain_is_rejected() {
        let mut segments = rect(0.0, 0.0, 10.0, 10.0);
        segments.pop();
        assert!(plan_profile_from_segments(&segments).is_none());
    }

    #[test]
    fn an_ambiguous_diagonal_is_rejected() {
        // A square whose four corners are all open, with one diagonal
        // box: both of its diagonals connect two open corners, so no
        // choice is forced.
        let segments = vec![
            [0.0, 0.0, 0.0, 10.0],
            [10.0, 0.0, 10.0, 10.0],
            [0.0, 0.0, 10.0, 10.0],
            [0.0, 20.0, 10.0, 20.0],
        ];
        assert!(plan_profile_from_segments(&segments).is_none());
    }

    #[test]
    fn a_zero_extent_segment_is_rejected() {
        let mut segments = rect(0.0, 0.0, 10.0, 10.0);
        segments.push([5.0, 5.0, 5.0, 5.0]);
        assert!(plan_profile_from_segments(&segments).is_none());
    }

    #[test]
    fn collinear_splits_merge_into_one_edge() {
        // The top run is split at x = 6, the way Revit splits one
        // straight boundary into several sketch lines.
        let segments = vec![
            [0.0, 0.0, 0.0, 10.0],
            [0.0, 10.0, 6.0, 10.0],
            [6.0, 10.0, 10.0, 10.0],
            [10.0, 0.0, 10.0, 10.0],
            [0.0, 0.0, 10.0, 0.0],
        ];
        let profile = plan_profile_from_segments(&segments).expect("closes");
        assert_eq!(profile.outer_xy.len(), 4);
    }

    #[test]
    fn fields_round_trip_through_the_reader() {
        let mut segments = rect(0.0, 0.0, 100.0, 100.0);
        segments.extend(rect(40.0, 40.0, 60.0, 60.0));
        let profile = plan_profile_from_segments(&segments).expect("closes");
        let read = plan_profile_from_fields(&profile.fields()).expect("reads back");
        assert_eq!(read.outer_xy, profile.outer_xy);
        assert_eq!(read.inner_xy, profile.inner_xy);
    }

    #[test]
    fn foreign_fields_are_not_a_profile() {
        let fields = vec![(
            "m_bboxWidth".to_string(),
            InstanceField::Float {
                value: 4.0,
                size: 8,
            },
        )];
        assert!(plan_profile_from_fields(&fields).is_none());
    }

    #[test]
    fn records_without_an_owner_contribute_nothing() {
        assert!(plan_profiles_from_sketch_line_records(&[]).is_empty());
    }

    #[test]
    fn profile_fields_carry_every_loop() {
        let mut segments = rect(0.0, 0.0, 100.0, 100.0);
        segments.extend(rect(40.0, 40.0, 60.0, 60.0));
        let profile = plan_profile_from_segments(&segments).expect("closes");
        assert_eq!(profile.vertex_count(), 8);
        let fields = profile.fields();
        let outer = fields
            .iter()
            .find(|(name, _)| name == PLAN_PROFILE_OUTER_FIELD)
            .map(|(_, value)| value)
            .expect("outer field");
        match outer {
            InstanceField::Vector(points) => assert_eq!(points.len(), 4),
            other => panic!("outer is not a vector: {other:?}"),
        }
        let inner = fields
            .iter()
            .find(|(name, _)| name == PLAN_PROFILE_INNER_FIELD)
            .map(|(_, value)| value)
            .expect("inner field");
        match inner {
            InstanceField::Vector(loops) => assert_eq!(loops.len(), 1),
            other => panic!("inner is not a vector: {other:?}"),
        }
    }
}
