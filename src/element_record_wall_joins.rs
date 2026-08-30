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
//! [`join_trims`] applies (2) to the wall set recovered from the
//! records. It is a solver over recorded boxes, not a fit: a
//! candidate must be perpendicular, must have its centreline exactly
//! on the end being resolved, must span this wall's centreline along
//! its own run, and must overlap it in elevation. When the candidates
//! at one end disagree about their thickness the **whole element** is
//! declined and keeps its record box.
//!
//! # Honesty
//!
//! - This models Revit's join cleanup; it does not read it. Revit
//!   stores the join state per wall pair, and no byte carrying it has
//!   been identified — the trimmed endpoints are not in the record
//!   (searched as `f64` over 4 KiB past every wall record on the
//!   recorded edge; no fixed carrier).
//! - Measured end to end on `2024_Core_Interior.rvt`: the trimmed box
//!   equals Revit's world bounding box on **336 of 360** walls, up
//!   from 39 of 360 for the untrimmed box; worst corner residual
//!   0.75 → 0.3333 ft, mean 0.2657 → 0.0220 ft. Re-measured on the
//!   emitted IFC against `main`, 309 walls improve, 35 are unchanged
//!   and 16 regress.
//! - The 24 misses are all **over-trims** at a junction where Revit
//!   let the wall run on. They are confined to one feature class —
//!   an 8" wall meeting another 8" wall — where the same feature
//!   values also produce a real trim 125 times, so no available byte
//!   separates them. They are recorded, not papered over.
//! - Only axis-parallel walls are resolved. Every wall on the
//!   recorded edge is axis-parallel; a wall whose box is square in
//!   plan has no identifiable thin axis and is declined.

use crate::partition_element_records::PartitionElementRecord;
use std::collections::BTreeMap;

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

/// Half the thickness of the wall whose centreline lands on `coord`,
/// or `Some(0.0)` when nothing joins there.
///
/// `None` means the candidates disagree about their thickness, which
/// declines the element.
fn trim_at(runs: &[WallRun], wall: &WallRun, coord: f64) -> Option<f64> {
    let mut thickness: Option<f64> = None;
    for other in runs {
        if other.element_id == wall.element_id || other.axis == wall.axis {
            continue;
        }
        if (other.centre_feet - coord).abs() > JOIN_EPS_FEET {
            continue;
        }
        if wall.centre_feet < other.start_feet - JOIN_EPS_FEET
            || wall.centre_feet > other.end_feet + JOIN_EPS_FEET
        {
            continue;
        }
        if wall.top_feet.min(other.top_feet) - wall.base_feet.max(other.base_feet) <= JOIN_EPS_FEET
        {
            continue;
        }
        match thickness {
            None => thickness = Some(other.thickness_feet),
            Some(held) if (held - other.thickness_feet).abs() <= JOIN_EPS_FEET => {}
            Some(_) => return None,
        }
    }
    Some(thickness.map_or(0.0, |value| value * 0.5))
}

/// The join trim of every wall in `records`, keyed by ElementId.
///
/// Walls that decline — no long axis, disagreeing candidates, or a
/// trim that would collapse the run — are simply absent from the map
/// and keep their record box.
pub fn join_trims(records: &[PartitionElementRecord]) -> BTreeMap<u32, WallJoinTrim> {
    let runs: Vec<WallRun> = records.iter().filter_map(wall_run).collect();
    let mut out = BTreeMap::new();
    for wall in &runs {
        let (Some(start), Some(end)) = (
            trim_at(&runs, wall, wall.start_feet),
            trim_at(&runs, wall, wall.end_feet),
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

    fn wall(element_id: u32, bbox_feet: [f64; 6]) -> PartitionElementRecord {
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
            references: Vec::new(),
        }
    }

    /// The Core Interior corner that RE-26 §4 walks through: wall
    /// 20796 runs in x at y = 81 and ends on 20799's centreline at
    /// x = 75, so it is cut back to 74.6667; 20797 runs in y at
    /// x = 48 and ends on 20796's centreline at y = 81, so it is cut
    /// back to 80.6667. Neither is cut at its other end.
    fn core_interior_corner() -> Vec<PartitionElementRecord> {
        vec![
            wall(20796, [47.75, 80.66667, 76.0, 75.0, 81.33333, 91.0]),
            wall(20797, [47.75, 57.66667, 76.0, 48.25, 81.0, 91.0]),
            wall(20799, [74.66667, 58.0, 76.0, 75.33333, 81.33333, 91.0]),
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
        records.push(wall(21000, [74.75, 58.0, 76.0, 75.25, 81.33333, 91.0]));
        let trims = join_trims(&records);
        assert!(!trims.contains_key(&20796), "ambiguous join declines");
        assert!(trims.contains_key(&20797), "other walls are unaffected");
    }

    #[test]
    fn a_square_plan_box_has_no_wall_axis() {
        let records = vec![wall(30000, [0.0, 0.0, 0.0, 2.0, 2.0, 10.0])];
        assert!(join_trims(&records).is_empty());
    }

    #[test]
    fn a_trim_that_would_collapse_the_run_is_declined() {
        // A 1 ft stub between two 2 ft walls: each end would be cut
        // back a full foot, which leaves nothing.
        let records = vec![
            wall(31000, [0.0, 9.66667, 0.0, 1.0, 10.33333, 10.0]),
            wall(31001, [-1.0, 0.0, 0.0, 1.0, 20.0, 10.0]),
            wall(31002, [0.0, 0.0, 0.0, 2.0, 20.0, 10.0]),
        ];
        let trims = join_trims(&records);
        assert!(!trims.contains_key(&31000));
    }
}
