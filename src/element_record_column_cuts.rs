//! Column bodies: the record box is the whole prism, the joined walls cut it.
//!
//! A Revit 2024 `OST_Columns` partition element record carries the
//! column's model bounding box, and RE-26 settled that the type
//! symbol's section is exactly that box's plan rectangle on all 256
//! exported columns of `2024_Core_Interior.rvt`. 80 of them still
//! disagreed with Revit's own export, whose bodies are **inset** from
//! the prism by 3", 4" or 7" on one or both plan axes — an inset no
//! section can produce, because it is not a profile, it is a cut
//! (#239).
//!
//! What cuts them is in the record. The counted reference list at
//! `+0x88` names the walls the column is joined to, and Revit
//! subtracts those walls' solids from the column:
//!
//! ```text
//! exported column body = record prism  −  ⋃ (record prism of each
//!                                           named wall)
//! ```
//!
//! Measured on `2024_Core_Interior.rvt` against Revit's own export,
//! world axis-aligned bounding box matched by `Tag`: **256 of 256**
//! exact, worst corner residual **9.4e-13 ft**, up from 176 of 256
//! for the uncut prism (RE-28 §5). The cutters are the walls'
//! *untrimmed* record boxes — using the join-trimmed runs of
//! `element_record_wall_joins` instead scores 190 of 256, so a wall
//! runs its full recorded length into a column it cuts.
//!
//! The 256 split three ways, and only the last group moves:
//!
//! | group | columns | emitted |
//! |---|---:|---|
//! | names no wall that overlaps it | 107 | the record prism |
//! | cut, but the cut is interior — a slot through the middle | 69 | the record prism, whose box is already Revit's |
//! | cut from a face, and the remainder is a rectangular box | 80 | the reduced box |
//!
//! # Honesty
//!
//! - [`column_cut_boxes`] answers with a **box**, and it only answers
//!   at all when the difference *is* a box: the surviving cells are
//!   required to fill their own bounding box exactly, by volume. On
//!   the recorded edge every one of the 80 columns whose bounding box
//!   shrinks passes that test, and every column that fails it has a
//!   bounding box equal to the uncut prism — so declining costs
//!   nothing there and never emits material Revit removed.
//! - A wall is a cutter only when the column's own reference list
//!   names it **and** it is one of the recovered wall instances. A
//!   slot this decoder cannot resolve to a wall cuts nothing.
//! - Nothing here reads a Revit "join order" or "cut priority". That
//!   walls cut columns rather than the other way round is the
//!   measured direction on this file, not a decoded rule; a file
//!   where a column wins the join would fail the box test and keep
//!   its prism.
//! - Rotated or non-axis-parallel geometry is out of scope: the
//!   record carries an axis-aligned box and the cut is computed on
//!   boxes.

use crate::partition_element_records::PartitionElementRecord;
use std::collections::{BTreeMap, BTreeSet};

/// Field carrying which body a recovered column is emitting.
pub const COLUMN_BODY_SOURCE_FIELD: &str = "m_column_body_source";
/// Value of [`COLUMN_BODY_SOURCE_FIELD`] when the cut was resolved.
pub const COLUMN_BODY_JOIN_CUT: &str = "partition_element_record_join_cut";
/// Field carrying how many joined walls cut the column.
pub const COLUMN_CUT_WALL_COUNT_FIELD: &str = "m_column_cut_walls";

/// Plan tolerance for the cut, in feet.
///
/// Revit writes these coordinates as exact doubles and both records
/// carry the same bits, so the tolerance only absorbs the last bits
/// of a subtraction.
pub const CUT_EPS_FEET: f64 = 1e-9;

/// Most cutters one column is allowed, so a misread reference list
/// cannot make the cell sweep explode.
///
/// The largest number of joined walls overlapping one column on
/// `2024_Core_Interior.rvt` is 2.
pub const MAX_CUTTERS: usize = 16;

/// A column body recovered as the record prism minus its joined walls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnCut {
    /// The reduced body, `[min x, min y, min z, max x, max y, max z]`.
    pub bbox_feet: [f64; 6],
    /// How many joined walls took part in the cut.
    pub wall_count: usize,
}

fn overlaps(a: &[f64; 6], b: &[f64; 6]) -> bool {
    (0..3).all(|axis| a[axis + 3].min(b[axis + 3]) - a[axis].max(b[axis]) > CUT_EPS_FEET)
}

fn contains(box_feet: &[f64; 6], point: [f64; 3]) -> bool {
    (0..3).all(|axis| {
        point[axis] >= box_feet[axis] - CUT_EPS_FEET
            && point[axis] <= box_feet[axis + 3] + CUT_EPS_FEET
    })
}

/// The cut planes on one axis: the prism's own two faces plus every
/// cutter face strictly inside it, in order.
fn grid(prism: &[f64; 6], cutters: &[[f64; 6]], axis: usize) -> Vec<f64> {
    let mut values = vec![prism[axis], prism[axis + 3]];
    for cutter in cutters {
        for value in [cutter[axis], cutter[axis + 3]] {
            if value > prism[axis] + CUT_EPS_FEET && value < prism[axis + 3] - CUT_EPS_FEET {
                values.push(value);
            }
        }
    }
    values.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    values.dedup_by(|a, b| (*a - *b).abs() <= CUT_EPS_FEET);
    values
}

/// `prism` minus `cutters`, when the difference is a rectangular box.
///
/// `None` when nothing is cut away, when the difference is empty, or
/// when the surviving material is **not** a box — a slot through the
/// middle, an L, a cross. The caller then keeps the recorded prism,
/// which is what the record states and what Revit's bounding box
/// agrees with on every such column of the recorded edge.
pub fn difference_box(prism: &[f64; 6], cutters: &[[f64; 6]]) -> Option<[f64; 6]> {
    let cutters: Vec<[f64; 6]> = cutters
        .iter()
        .filter(|cutter| overlaps(prism, cutter))
        .copied()
        .collect();
    if cutters.is_empty() || cutters.len() > MAX_CUTTERS {
        return None;
    }
    let grids = [
        grid(prism, &cutters, 0),
        grid(prism, &cutters, 1),
        grid(prism, &cutters, 2),
    ];
    let mut low = [f64::INFINITY; 3];
    let mut high = [f64::NEG_INFINITY; 3];
    let mut volume = 0.0f64;
    for xi in 0..grids[0].len() - 1 {
        for yi in 0..grids[1].len() - 1 {
            for zi in 0..grids[2].len() - 1 {
                let cell = [
                    grids[0][xi],
                    grids[1][yi],
                    grids[2][zi],
                    grids[0][xi + 1],
                    grids[1][yi + 1],
                    grids[2][zi + 1],
                ];
                let centre = [
                    (cell[0] + cell[3]) * 0.5,
                    (cell[1] + cell[4]) * 0.5,
                    (cell[2] + cell[5]) * 0.5,
                ];
                if cutters.iter().any(|cutter| contains(cutter, centre)) {
                    continue;
                }
                volume += (cell[3] - cell[0]) * (cell[4] - cell[1]) * (cell[5] - cell[2]);
                for axis in 0..3 {
                    low[axis] = low[axis].min(cell[axis]);
                    high[axis] = high[axis].max(cell[axis + 3]);
                }
            }
        }
    }
    if !low[0].is_finite() {
        return None;
    }
    let hull = [low[0], low[1], low[2], high[0], high[1], high[2]];
    let hull_volume = (0..3)
        .map(|axis| hull[axis + 3] - hull[axis])
        .product::<f64>();
    if (hull_volume - volume).abs() > CUT_EPS_FEET {
        return None;
    }
    if (0..6).all(|slot| (hull[slot] - prism[slot]).abs() <= CUT_EPS_FEET) {
        return None;
    }
    Some(hull)
}

/// The cut body of every column in `columns` that a joined wall in
/// `walls` cuts back, keyed by ElementId.
///
/// Columns whose difference is not a box, or whose box is unchanged,
/// are simply absent from the map and keep their record prism.
pub fn column_cut_boxes(
    columns: &[PartitionElementRecord],
    walls: &[PartitionElementRecord],
) -> BTreeMap<u32, ColumnCut> {
    let wall_boxes: BTreeMap<u32, [f64; 6]> = walls
        .iter()
        .map(|record| (record.element_id, record.bbox_feet))
        .collect();
    let wall_ids: BTreeSet<u32> = wall_boxes.keys().copied().collect();
    let mut out = BTreeMap::new();
    for column in columns {
        let cutters: Vec<[f64; 6]> = column
            .references
            .iter()
            .filter(|slot| **slot <= u64::from(u32::MAX))
            .map(|slot| *slot as u32)
            .filter(|id| wall_ids.contains(id))
            .filter_map(|id| wall_boxes.get(&id).copied())
            .filter(|cutter| overlaps(&column.bbox_feet, cutter))
            .collect();
        if let Some(bbox_feet) = difference_box(&column.bbox_feet, &cutters) {
            out.insert(
                column.element_id,
                ColumnCut {
                    bbox_feet,
                    wall_count: cutters.len(),
                },
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::partition_element_records as per;

    fn record(
        element_id: u32,
        category: i64,
        bbox_feet: [f64; 6],
        refs: &[u32],
    ) -> PartitionElementRecord {
        let mut references: Vec<u64> = vec![3, 5755, 20307];
        references.extend(refs.iter().map(|id| u64::from(*id)));
        references.push(u64::from(element_id));
        references.sort_unstable();
        PartitionElementRecord {
            stream: "Partitions/59".into(),
            offset: element_id as usize,
            element_id,
            flags: 0x0121,
            builtin_category: category,
            container: per::CONTAINER_NONE,
            placement_kind: per::PLACEMENT_KIND_INSTANCE,
            bbox_feet,
            preceding_reference: None,
            owner_reference: None,
            references,
        }
    }

    fn column(element_id: u32, bbox_feet: [f64; 6], refs: &[u32]) -> PartitionElementRecord {
        record(element_id, per::OST_COLUMNS, bbox_feet, refs)
    }

    fn wall(element_id: u32, bbox_feet: [f64; 6]) -> PartitionElementRecord {
        record(element_id, per::OST_WALLS, bbox_feet, &[])
    }

    /// Column 22807 of the recorded edge: a 2 ft square prism whose
    /// `x` face is overrun by the 18" wall 80743 running in `x` at
    /// `y = 113.25`. Revit's body is inset 0.3333 ft on `y`.
    #[test]
    fn a_wall_that_overruns_one_face_insets_the_column() {
        let columns = vec![column(
            22807,
            [20.0, 112.0, -40.0, 22.0, 114.0, 0.0],
            &[80743],
        )];
        let walls = vec![wall(80743, [20.0, 112.5, -40.0, 166.25, 114.0, 0.0])];
        let cut = column_cut_boxes(&columns, &walls)[&22807];
        assert_eq!(cut.wall_count, 1);
        assert!((cut.bbox_feet[4] - 112.5).abs() < 1e-9, "y max cut back");
        assert!((cut.bbox_feet[1] - 112.0).abs() < 1e-9, "y min unchanged");
        assert!((cut.bbox_feet[0] - 20.0).abs() < 1e-9);
        assert!((cut.bbox_feet[3] - 22.0).abs() < 1e-9);
    }

    /// Column 20376 of the recorded edge: a 6" wall passes through
    /// the middle of the prism for part of its height. The difference
    /// is a slot, not a box, so the recorded prism stands — which is
    /// what Revit's bounding box says too.
    #[test]
    fn an_interior_slot_leaves_the_prism_alone() {
        let columns = vec![column(
            20376,
            [48.0, 109.0, 76.0, 50.0, 111.0, 90.33333],
            &[20811],
        )];
        let walls = vec![wall(20811, [49.0, 100.0, 76.0, 49.5, 120.0, 84.0])];
        assert!(column_cut_boxes(&columns, &walls).is_empty());
    }

    #[test]
    fn a_column_that_names_no_wall_is_not_cut() {
        let columns = vec![column(63298, [0.0, 0.0, 0.0, 2.0, 2.0, 9.0], &[])];
        let walls = vec![wall(90000, [0.0, 0.0, 0.0, 0.5, 2.0, 9.0])];
        assert!(column_cut_boxes(&columns, &walls).is_empty());
    }

    #[test]
    fn a_named_wall_that_does_not_overlap_cuts_nothing() {
        let columns = vec![column(63298, [0.0, 0.0, 0.0, 2.0, 2.0, 9.0], &[90000])];
        let walls = vec![wall(90000, [10.0, 0.0, 0.0, 10.5, 2.0, 9.0])];
        assert!(column_cut_boxes(&columns, &walls).is_empty());
    }

    /// Two walls meeting the same corner take an L out of the prism.
    /// An L is not a box, so the solver declines rather than emit a
    /// bounding box that puts material back where Revit removed it.
    #[test]
    fn a_cut_that_leaves_an_l_is_declined() {
        let columns = vec![column(
            63299,
            [0.0, 0.0, 0.0, 2.0, 2.0, 9.0],
            &[90000, 90001],
        )];
        let walls = vec![
            wall(90000, [0.0, 0.0, 0.0, 0.5, 2.0, 4.0]),
            wall(90001, [0.0, 0.0, 4.0, 2.0, 0.5, 9.0]),
        ];
        assert!(column_cut_boxes(&columns, &walls).is_empty());
    }

    /// Two walls overrunning opposite faces for the full height leave
    /// a narrower box, which is exactly the recorded edge's
    /// 1.4167 ft group.
    #[test]
    fn opposite_faces_leave_a_narrower_box() {
        let columns = vec![column(
            63300,
            [0.0, 0.0, 0.0, 2.0, 2.0, 9.0],
            &[90000, 90001],
        )];
        let walls = vec![
            wall(90000, [-5.0, 0.0, 0.0, 0.33333, 2.0, 9.0]),
            wall(90001, [1.75, 0.0, 0.0, 7.0, 2.0, 9.0]),
        ];
        let cut = column_cut_boxes(&columns, &walls)[&63300];
        assert_eq!(cut.wall_count, 2);
        assert!((cut.bbox_feet[0] - 0.33333).abs() < 1e-9);
        assert!((cut.bbox_feet[3] - 1.75).abs() < 1e-9);
        assert!((cut.bbox_feet[3] - cut.bbox_feet[0] - 1.41667).abs() < 1e-4);
    }

    #[test]
    fn a_wall_that_swallows_the_column_leaves_nothing_and_declines() {
        let columns = vec![column(63301, [0.0, 0.0, 0.0, 2.0, 2.0, 9.0], &[90000])];
        let walls = vec![wall(90000, [-1.0, -1.0, -1.0, 3.0, 3.0, 10.0])];
        assert!(column_cut_boxes(&columns, &walls).is_empty());
    }

    #[test]
    fn difference_box_is_none_without_cutters() {
        assert!(difference_box(&[0.0, 0.0, 0.0, 2.0, 2.0, 9.0], &[]).is_none());
    }
}
