//! Wall bodies: the record box is the *untrimmed* wall, the joins cut it.
//!
//! A Revit 2024 `OST_Walls` partition element record carries the
//! wall's model bounding box (`partition_element_records`). Measured
//! against Revit's own IFC export of `2024_Core_Interior.rvt`, that
//! box is **not** the wall's exported body: it is the wall's location
//! line taken to its raw endpoints, inflated by half the wall's
//! thickness across the run. Revit's exported body is the same prism
//! after the joins have cut it back.
//!
//! Two facts make the difference recoverable from the records alone
//! (#215/#227 background; RE-26 has the tables):
//!
//! 1. **The box's thin plan extent is the wall's thickness.** On the
//!    recorded edge the thin extent equals the nominal thickness of
//!    the `IfcWallType` Revit's export assigns on **360 of 360**
//!    walls, once the newest frame of each element is the one read
//!    (`partition_schema_mvp::select_instance_records`).
//! 2. **The trim at an end is half the thickness of the wall whose
//!    centreline lands on that end.** Every non-zero end delta
//!    between the record box and Revit's `Axis` polyline on the
//!    recorded edge is exactly one of `0.25`, `0.3333`, `0.75` ft —
//!    half of the 6", 8" and 18" wall thicknesses on the file — and
//!    zero everywhere else.
//!
//! 3. **The record names the walls it joins.** The counted reference
//!    list at `+0x88` holds, besides the type and the two Levels, the
//!    ElementIds of the other walls this wall is joined to. Requiring
//!    a candidate to be named there removes 14 over-trims on the
//!    recorded edge and costs nothing: 16 of the 360 walls name no
//!    other wall at all, and every one of them is a wall Revit leaves
//!    untrimmed (#238, RE-29 §2).
//!
//! 4. **A candidate only cuts where its own cut-back run reaches.**
//!    A wall whose end has itself been cut back no longer covers the
//!    centreline of a wall beyond that cut, and Revit does not trim
//!    there. The candidate's run is therefore reduced by its own
//!    trims before the span test — ignoring the trims imposed by
//!    walls on the line being resolved, which are the joins this very
//!    end makes and cannot pre-empt itself (RE-29 §3).
//!
//! [`join_trims`] applies (2)–(4) to the wall set recovered from the
//! records. It is a solver over recorded boxes, not a fit: a
//! candidate must be perpendicular, must be named in this record's
//! reference list, must have its centreline exactly on the end being
//! resolved, must span this wall's centreline along its own reduced
//! run, and must overlap it in elevation. When the candidates at one
//! end disagree about their thickness the **whole element** is
//! declined and keeps its record box.
//!
//! # Honesty
//!
//! - The *which walls join* half is read from the file; the *where
//!   the join cuts* half is still modelled. Revit's per-pair
//!   butt/mitre choice is not in the record — the trimmed endpoints
//!   were searched as `f64` over 4 KiB past every wall record on the
//!   recorded edge and no fixed carrier holds them.
//! - Measured end to end on `2024_Core_Interior.rvt`, world
//!   axis-aligned bounding box against Revit's own export: the
//!   trimmed box is exact on **351 of 360** walls, up from 336 for
//!   the geometry-only solver and 39 for the untrimmed box; worst
//!   corner residual 0.3333 ft, mean 0.0220 → 0.0081 ft. Per end,
//!   689 → **711 of 720** correct, with no end that the geometry-only
//!   solver had right made wrong.
//! - The 9 residual ends are one side of a **true L corner** — two
//!   walls whose runs both stop at the other's centreline, with
//!   nothing continuing past the corner. Revit cuts exactly one of
//!   the pair and the file offers no feature that says which: across
//!   the two distinct corners on the recorded edge the survivor is
//!   the thicker wall once and the thinner wall once, the lower
//!   ElementId once and the higher once. They are recorded, not
//!   papered over (#238, RE-29 §4).
//! - Only axis-parallel walls are resolved. Every wall on the
//!   recorded edge is axis-parallel; a wall whose box is square in
//!   plan has no identifiable thin axis and is declined.

use crate::partition_element_records::PartitionElementRecord;
use std::collections::{BTreeMap, BTreeSet};

/// Field carrying which body a recovered wall is emitting.
pub const WALL_BODY_SOURCE_FIELD: &str = "m_wall_body_source";
/// Value of [`WALL_BODY_SOURCE_FIELD`] when the joins were resolved.
pub const WALL_BODY_JOIN_TRIMMED: &str = "partition_element_record_join_trimmed";
/// Field carrying the wall thickness read off the box's thin axis.
pub const WALL_THICKNESS_FIELD: &str = "m_wall_thickness";
/// Field carrying the trim applied at the low end of the wall run.
pub const WALL_TRIM_START_FIELD: &str = "m_wall_trim_start";
/// Field carrying the trim applied at the high end of the wall run.
pub const WALL_TRIM_END_FIELD: &str = "m_wall_trim_end";
/// Value of the thickness-source property for the thin-axis read.
pub const WALL_THICKNESS_SOURCE: &str = "partition_element_record_bbox_thin_axis";

/// Plan tolerance for "this centreline lands on that end", in feet.
///
/// Revit writes these coordinates as exact doubles and the record
/// carries the same bits, so the tolerance only absorbs the last
/// bits of a subtraction.
pub const JOIN_EPS_FEET: f64 = 1e-6;

/// One wall reduced to the run / thickness / elevation the solver needs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallRun {
    /// The wall's own ElementId.
    pub element_id: u32,
    /// Plan axis the wall runs along: `0` = x, `1` = y.
    pub axis: usize,
    /// Thickness in feet — the box's extent across the run.
    pub thickness_feet: f64,
    /// Centreline coordinate across the run, in feet.
    pub centre_feet: f64,
    /// Low end of the untrimmed run, in feet.
    pub start_feet: f64,
    /// High end of the untrimmed run, in feet.
    pub end_feet: f64,
    /// Base of the box, in feet.
    pub base_feet: f64,
    /// Top of the box, in feet.
    pub top_feet: f64,
}

/// The trims a wall's two ends take, in feet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallJoinTrim {
    /// Plan axis the wall runs along: `0` = x, `1` = y.
    pub axis: usize,
    /// Thickness in feet.
    pub thickness_feet: f64,
    /// Trim at the low end.
    pub start_feet: f64,
    /// Trim at the high end.
    pub end_feet: f64,
}

impl WallJoinTrim {
    /// True when either end was actually cut back.
    pub fn is_trimmed(&self) -> bool {
        self.start_feet > 0.0 || self.end_feet > 0.0
    }
}

/// Reduce a record to a [`WallRun`], or `None` when its plan box has
/// no identifiable long axis.
pub fn wall_run(record: &PartitionElementRecord) -> Option<WallRun> {
    let (dx, dy, _) = record.extents_feet();
    if !dx.is_finite() || !dy.is_finite() {
        return None;
    }
    if (dx - dy).abs() <= JOIN_EPS_FEET {
        return None;
    }
    let axis = usize::from(dy > dx);
    let across = 1 - axis;
    let thickness_feet = if axis == 0 { dy } else { dx };
    if thickness_feet <= 0.0 {
        return None;
    }
    Some(WallRun {
        element_id: record.element_id,
        axis,
        thickness_feet,
        centre_feet: (record.bbox_feet[across] + record.bbox_feet[across + 3]) * 0.5,
        start_feet: record.bbox_feet[axis],
        end_feet: record.bbox_feet[axis + 3],
        base_feet: record.bbox_feet[2],
        top_feet: record.bbox_feet[5],
    })
}

/// The other recovered walls a record names in its counted reference
/// list at `+0x88` (#238, RE-29 §2).
///
/// `wall_ids` is the ElementId set of the recovered wall instances,
/// so a slot that names a type, a Level, a column or an id this
/// decoder has not recovered is dropped rather than guessed at. The
/// record's own id is never a join partner and is excluded.
pub fn joined_walls(record: &PartitionElementRecord, wall_ids: &BTreeSet<u32>) -> BTreeSet<u32> {
    record
        .references
        .iter()
        .filter(|slot| **slot <= u64::from(u32::MAX))
        .map(|slot| *slot as u32)
        .filter(|id| *id != record.element_id && wall_ids.contains(id))
        .collect()
}

/// The wall set plus the join partners each record names, which is
/// everything the solver reads.
struct Solver<'a> {
    runs: &'a [WallRun],
    joined: &'a BTreeMap<u32, BTreeSet<u32>>,
}

impl<'a> Solver<'a> {
    fn names(&self, wall: &WallRun, other: &WallRun) -> bool {
        self.joined
            .get(&wall.element_id)
            .is_some_and(|set| set.contains(&other.element_id))
    }

    /// Every wall that is perpendicular to `wall`, named by it, whose
    /// centreline lands on `coord`, whose own run spans `wall`'s
    /// centreline and whose elevation range overlaps it.
    ///
    /// `skip_centre` drops candidates sitting on that plan line; it is
    /// how a candidate's own reduction ignores the joins it makes with
    /// the line currently being resolved.
    fn candidates(&self, wall: &WallRun, coord: f64, skip_centre: Option<f64>) -> Vec<&'a WallRun> {
        self.runs
            .iter()
            .filter(|other| other.element_id != wall.element_id && other.axis != wall.axis)
            .filter(|other| (other.centre_feet - coord).abs() <= JOIN_EPS_FEET)
            .filter(|other| self.names(wall, other))
            .filter(|other| {
                skip_centre.is_none_or(|line| (other.centre_feet - line).abs() > JOIN_EPS_FEET)
            })
            .filter(|other| {
                wall.centre_feet >= other.start_feet - JOIN_EPS_FEET
                    && wall.centre_feet <= other.end_feet + JOIN_EPS_FEET
            })
            .filter(|other| {
                wall.top_feet.min(other.top_feet) - wall.base_feet.max(other.base_feet)
                    > JOIN_EPS_FEET
            })
            .collect()
    }

    /// Half the one thickness the candidates agree on, `Some(0.0)`
    /// when there are none, `None` when they disagree.
    fn half_thickness(candidates: &[&WallRun]) -> Option<f64> {
        let mut thickness: Option<f64> = None;
        for other in candidates {
            match thickness {
                None => thickness = Some(other.thickness_feet),
                Some(held) if (held - other.thickness_feet).abs() <= JOIN_EPS_FEET => {}
                Some(_) => return None,
            }
        }
        Some(thickness.map_or(0.0, |value| value * 0.5))
    }

    /// `candidate`'s own run, cut back by the joins it makes with
    /// walls that are **not** on `line` — the plan line whose trim is
    /// being resolved. Disagreeing candidates reduce nothing, which
    /// keeps the run at its recorded length rather than inventing one.
    fn reduced_run(&self, candidate: &WallRun, line: f64) -> (f64, f64) {
        let start =
            Self::half_thickness(&self.candidates(candidate, candidate.start_feet, Some(line)))
                .unwrap_or(0.0);
        let end = Self::half_thickness(&self.candidates(candidate, candidate.end_feet, Some(line)))
            .unwrap_or(0.0);
        (candidate.start_feet + start, candidate.end_feet - end)
    }

    /// The trim `wall` takes at `coord`, or `None` when the candidates
    /// there disagree about their thickness.
    fn trim_at(&self, wall: &WallRun, coord: f64) -> Option<f64> {
        let reaching: Vec<&WallRun> = self
            .candidates(wall, coord, None)
            .into_iter()
            .filter(|candidate| {
                let (start, end) = self.reduced_run(candidate, wall.centre_feet);
                wall.centre_feet >= start - JOIN_EPS_FEET && wall.centre_feet <= end + JOIN_EPS_FEET
            })
            .collect();
        Self::half_thickness(&reaching)
    }
}

/// The join trim of every wall in `records`, keyed by ElementId.
///
/// Walls that decline — no long axis, disagreeing candidates, or a
/// trim that would collapse the run — are simply absent from the map
/// and keep their record box.
pub fn join_trims(records: &[PartitionElementRecord]) -> BTreeMap<u32, WallJoinTrim> {
    let wall_ids: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();
    let joined: BTreeMap<u32, BTreeSet<u32>> = records
        .iter()
        .map(|record| (record.element_id, joined_walls(record, &wall_ids)))
        .collect();
    let runs: Vec<WallRun> = records.iter().filter_map(wall_run).collect();
    let solver = Solver {
        runs: &runs,
        joined: &joined,
    };
    let mut out = BTreeMap::new();
    for wall in &runs {
        let (Some(start), Some(end)) = (
            solver.trim_at(wall, wall.start_feet),
            solver.trim_at(wall, wall.end_feet),
        ) else {
            continue;
        };
        if wall.end_feet - end - (wall.start_feet + start) <= JOIN_EPS_FEET {
            continue;
        }
        out.insert(
            wall.element_id,
            WallJoinTrim {
                axis: wall.axis,
                thickness_feet: wall.thickness_feet,
                start_feet: start,
                end_feet: end,
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_element_records as per;

    /// A wall record shaped like the ones on the recorded edge: the
    /// counted list at `+0x88` holds the leading `3`, the type, the
    /// two Levels, the joined walls and the record's own id, all in
    /// ascending order.
    fn wall(element_id: u32, bbox_feet: [f64; 6], joins: &[u32]) -> PartitionElementRecord {
        let mut references: Vec<u64> = vec![3, 17328, 20307, 20308];
        references.extend(joins.iter().map(|id| u64::from(*id)));
        references.push(u64::from(element_id));
        references.sort_unstable();
        PartitionElementRecord {
            stream: "Partitions/59".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0121,
            builtin_category: per::OST_WALLS,
            container: per::CONTAINER_NONE,
            placement_kind: per::PLACEMENT_KIND_INSTANCE,
            bbox_feet,
            preceding_reference: None,
            owner_reference: None,
            references,
            id_from_enclosing_record: false,
        }
    }

    /// The Core Interior corner that RE-26 §4 walks through: wall
    /// 20796 runs in x at y = 81 and ends on 20799's centreline at
    /// x = 75, so it is cut back to 74.6667; 20797 runs in y at
    /// x = 48 and ends on 20796's centreline at y = 81, so it is cut
    /// back to 80.6667. Neither is cut at its other end.
    fn core_interior_corner() -> Vec<PartitionElementRecord> {
        vec![
            wall(
                20796,
                [47.75, 80.66667, 76.0, 75.0, 81.33333, 91.0],
                &[20797, 20799],
            ),
            wall(20797, [47.75, 57.66667, 76.0, 48.25, 81.0, 91.0], &[20796]),
            wall(
                20799,
                [74.66667, 58.0, 76.0, 75.33333, 81.33333, 91.0],
                &[20796],
            ),
        ]
    }

    #[test]
    fn a_wall_is_cut_back_by_half_the_wall_it_ends_on() {
        let trims = join_trims(&core_interior_corner());
        let t = trims[&20796];
        assert_eq!(t.axis, 0);
        assert!((t.thickness_feet - 0.66666).abs() < 1e-4);
        assert!((t.start_feet - 0.0).abs() < 1e-9, "47.75 joins nothing");
        assert!((t.end_feet - 0.33333).abs() < 1e-4, "half of 20799");
    }

    #[test]
    fn the_perpendicular_wall_is_cut_by_the_one_it_ends_on() {
        let trims = join_trims(&core_interior_corner());
        let t = trims[&20797];
        assert_eq!(t.axis, 1);
        assert!((t.start_feet - 0.0).abs() < 1e-9);
        assert!((t.end_feet - 0.33333).abs() < 1e-4, "half of 20796");
    }

    #[test]
    fn a_wall_that_ends_short_of_a_centreline_is_not_cut() {
        // 20799's high end sits at 81.3333, which is 20796's face and
        // not its centreline, so nothing joins there.
        let trims = join_trims(&core_interior_corner());
        let t = trims[&20799];
        assert!((t.end_feet - 0.0).abs() < 1e-9);
        assert!((t.start_feet - 0.0).abs() < 1e-9, "58.0 joins nothing");
    }

    #[test]
    fn a_candidate_out_of_elevation_range_does_not_cut() {
        let mut records = core_interior_corner();
        // Push 20799 up a storey: it no longer overlaps 20796.
        records[2].bbox_feet[2] = 106.0;
        records[2].bbox_feet[5] = 121.0;
        let trims = join_trims(&records);
        assert!((trims[&20796].end_feet - 0.0).abs() < 1e-9);
    }

    #[test]
    fn candidates_that_disagree_on_thickness_decline_the_element() {
        let mut records = core_interior_corner();
        // A second wall on the same centreline, different thickness.
        records.push(wall(
            21000,
            [74.75, 58.0, 76.0, 75.25, 81.33333, 91.0],
            &[20796],
        ));
        records[0].references.push(21000);
        records[0].references.sort_unstable();
        let trims = join_trims(&records);
        assert!(!trims.contains_key(&20796), "ambiguous join declines");
        assert!(trims.contains_key(&20797), "other walls are unaffected");
    }

    #[test]
    fn a_square_plan_box_has_no_wall_axis() {
        let records = vec![wall(30000, [0.0, 0.0, 0.0, 2.0, 2.0, 10.0], &[])];
        assert!(join_trims(&records).is_empty());
    }

    #[test]
    fn a_trim_that_would_collapse_the_run_is_declined() {
        // A 1 ft stub between two 2 ft walls: each end would be cut
        // back a full foot, which leaves nothing.
        let records = vec![
            wall(
                31000,
                [0.0, 9.66667, 0.0, 1.0, 10.33333, 10.0],
                &[31001, 31002],
            ),
            wall(31001, [-1.0, 0.0, 0.0, 1.0, 20.0, 10.0], &[31000]),
            wall(31002, [0.0, 0.0, 0.0, 2.0, 20.0, 10.0], &[31000]),
        ];
        let trims = join_trims(&records);
        assert!(!trims.contains_key(&31000));
    }

    /// Wall 20826 of the recorded edge: an 8" wall whose two ends sit
    /// exactly on the centrelines of two 8" walls that pass through
    /// them, and which Revit leaves at full length. Its record names
    /// no other wall at all, and that is the only thing in the file
    /// that says so (#238, RE-29 §2).
    fn unjoined_span() -> Vec<PartitionElementRecord> {
        vec![
            wall(20826, [119.5, 67.41667, 76.0, 129.0, 68.08333, 91.0], &[]),
            wall(20825, [119.16667, 58.0, 76.0, 119.83333, 81.0, 91.0], &[]),
            wall(20804, [128.66667, 58.0, 76.0, 129.33333, 81.0, 91.0], &[]),
        ]
    }

    #[test]
    fn a_wall_that_names_no_join_partner_is_not_trimmed() {
        let trims = join_trims(&unjoined_span());
        let t = trims[&20826];
        assert!((t.start_feet - 0.0).abs() < 1e-9, "119.5 is not cut");
        assert!((t.end_feet - 0.0).abs() < 1e-9, "129.0 is not cut");
    }

    #[test]
    fn naming_the_same_two_walls_restores_the_trim() {
        let mut records = unjoined_span();
        records[0] = wall(
            20826,
            [119.5, 67.41667, 76.0, 129.0, 68.08333, 91.0],
            &[20804, 20825],
        );
        let t = join_trims(&records)[&20826];
        assert!((t.start_feet - 0.33333).abs() < 1e-4);
        assert!((t.end_feet - 0.33333).abs() < 1e-4);
    }

    /// The recorded edge's `x = 137` cluster. 20800 runs in x at
    /// y = 81 and its high end is cut back to 136.9167 by 20816; that
    /// cut puts 20803's centreline (137.1667) past the end of 20800,
    /// so 20803 has nothing to butt into and Revit leaves it at 81.0
    /// (#238, RE-29 §3).
    fn cut_back_candidate() -> Vec<PartitionElementRecord> {
        vec![
            wall(
                20800,
                [85.5, 80.66667, 76.0, 137.25, 81.33333, 91.0],
                &[20803, 20805, 20816],
            ),
            wall(
                20805,
                [85.16667, 58.0, 76.0, 85.83333, 81.33333, 91.0],
                &[20800],
            ),
            wall(
                20816,
                [136.91667, 58.0, 76.0, 137.58333, 81.0, 91.0],
                &[20800],
            ),
            wall(20803, [136.83333, 81.0, 76.0, 137.5, 87.5, 91.0], &[20800]),
        ]
    }

    #[test]
    fn a_candidate_cut_back_short_of_the_line_does_not_trim() {
        let trims = join_trims(&cut_back_candidate());
        assert!(
            (trims[&20800].end_feet - 0.33333).abs() < 1e-4,
            "20800 is still cut by 20816"
        );
        assert!(
            (trims[&20803].start_feet - 0.0).abs() < 1e-9,
            "20800 no longer reaches 137.1667"
        );
    }

    /// The same fixture records the one thing the solver still gets
    /// wrong: 20800 and 20816 form a true L corner and Revit cuts
    /// only 20800. Nothing in the file says which side survives, so
    /// the solver cuts both and this test pins the residual rather
    /// than hiding it (#238, RE-29 §4).
    #[test]
    fn the_surviving_side_of_a_true_l_corner_is_still_over_trimmed() {
        let trims = join_trims(&cut_back_candidate());
        assert!(
            (trims[&20816].end_feet - 0.33333).abs() < 1e-4,
            "Revit leaves 20816 at 81.0; the solver cuts it"
        );
    }

    /// Where the line being resolved is itself what cut the candidate
    /// back, the cut must not disqualify it: 20798's low end is cut
    /// by the `x = 48` wall line, and 20817 — part of that same line —
    /// is still trimmed by 20798 (RE-29 §3).
    #[test]
    fn a_cut_the_resolved_line_imposed_does_not_disqualify_the_candidate() {
        let records = vec![
            wall(
                20798,
                [48.0, 57.66667, 76.0, 100.25, 58.33333, 91.0],
                &[20797, 20817],
            ),
            wall(20817, [47.75, 51.5, 76.0, 48.25, 58.0, 91.0], &[20798]),
            wall(20797, [47.75, 57.66667, 76.0, 48.25, 81.0, 91.0], &[20798]),
        ];
        let trims = join_trims(&records);
        assert!(
            (trims[&20798].start_feet - 0.25).abs() < 1e-4,
            "20798 is cut by the x = 48 line"
        );
        assert!(
            (trims[&20817].end_feet - 0.33333).abs() < 1e-4,
            "and 20817 is still cut by 20798"
        );
    }

    #[test]
    fn joined_walls_keeps_only_recovered_wall_ids() {
        let record = wall(
            20796,
            [47.75, 80.66667, 76.0, 75.0, 81.33333, 91.0],
            &[20797],
        );
        let ids = [20796u32, 20797].into_iter().collect();
        let joined = joined_walls(&record, &ids);
        assert_eq!(joined.iter().copied().collect::<Vec<u32>>(), vec![20797]);
    }
}
