//! A column's true z-range from its instance geometry node (RE-173).
//!
//! A column's element record box can start at z = 0.0 while the column starts
//! higher. Revit also stores the instance's geometry node in the inflated
//! `Partitions/*` streams: the column's `u64` ElementId, then `u64 3 · u64
//! 0xffffffffffffffff`, preceded by two boxes of six `f64` (min x, y, z, max
//! x, y, z) at 96 and 48 bytes before the id. Their x and y extents are equal,
//! and the second holds Revit's z-range. The run belongs to the schema classes
//! `GArray` (parent `GInstance`) and `GRep`.
//!
//! Fail closed: only for a column whose record box starts at exactly 0.0, and
//! only from a node whose boxes are finite and whose x and y extents equal the
//! record's to 1e-6 ft, whose top is the record's top (0.01 ft) and whose bottom
//! is not. The column's bottom moves only if every such node gives the same one.
//!
//! The box at id-48 is also the extent after geometry joins: a slab or roof
//! joined to the column cuts it there, and the IFC body follows it (RE-178).
//!
//! See `reports/element-framing/RE-173-column-z-from-geometry-node.md` and
//! `reports/element-framing/RE-178-column-cut-body.md`.

use crate::RevitFile;
use std::collections::BTreeMap;

/// What follows the `u64` ElementId: `u64 3 · u64 -1`.
const AFTER_ID: [u8; 16] = [
    3, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
];
const TOLERANCE_FEET: f64 = 0.01;
const PLAN_TOLERANCE_FEET: f64 = 1e-6;

fn boxes(buf: &[u8], at: usize) -> Option<[f64; 6]> {
    let mut out = [0.0; 6];
    for (i, value) in out.iter_mut().enumerate() {
        let start = at.checked_add(i * 8)?;
        *value = f64::from_le_bytes(buf.get(start..start + 8)?.try_into().ok()?);
    }
    (out.iter().all(|v| v.is_finite() && v.abs() < 1e5)
        && out[3] > out[0]
        && out[4] > out[1]
        && out[5] > out[2])
        .then_some(out)
}

/// A geometry node's box at id-96 and box at id-48.
type NodeBoxes = ([f64; 6], [f64; 6]);

/// Every geometry node of a column in `targets`, from every partition.
fn column_nodes(
    rf: &mut RevitFile,
    targets: &BTreeMap<u32, [f64; 6]>,
) -> BTreeMap<u32, Vec<NodeBoxes>> {
    let mut found: BTreeMap<u32, Vec<NodeBoxes>> = BTreeMap::new();
    if targets.is_empty() {
        return found;
    }
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &AFTER_ID) {
            let Some(id_at) = at.checked_sub(8) else {
                continue;
            };
            if id_at < 96 {
                continue;
            }
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .and_then(|b| u64::from_le_bytes(b.try_into().ok()?).try_into().ok())
            else {
                continue;
            };
            if !targets.contains_key(&id) {
                continue;
            }
            if let (Some(first), Some(second)) = (boxes(buf, id_at - 96), boxes(buf, id_at - 48)) {
                found.entry(id).or_default().push((first, second));
            }
        }
    }
    found
}

fn plan_equal(a: &[f64; 6], b: &[f64; 6], tolerance: f64) -> bool {
    [0, 1, 3, 4]
        .iter()
        .all(|&i| (a[i] - b[i]).abs() < tolerance)
}

/// The bottom, in feet, of each column in `targets` (ElementId -> record box)
/// that a geometry node gives and agrees on.
pub fn column_bottoms(rf: &mut RevitFile, targets: &BTreeMap<u32, [f64; 6]>) -> BTreeMap<u32, f64> {
    column_nodes(rf, targets)
        .into_iter()
        .filter_map(|(id, nodes)| {
            let record = targets.get(&id)?;
            let bottoms: Vec<f64> = nodes
                .iter()
                .filter(|(first, second)| {
                    plan_equal(first, second, 1e-9)
                        && plan_equal(second, record, PLAN_TOLERANCE_FEET)
                        && (second[5] - record[5]).abs() < TOLERANCE_FEET
                        && (second[2] - record[2]).abs() > TOLERANCE_FEET
                })
                .map(|(_, second)| second[2])
                .collect();
            let first = *bottoms.first()?;
            bottoms
                .iter()
                .all(|b| (b - first).abs() < TOLERANCE_FEET)
                .then_some((id, first))
        })
        .collect()
}

/// The z-range, in feet, of each column's body after its geometry joins
/// (RE-178): the box at id-48 of its geometry node, where a slab or roof
/// joined to the column has cut it shorter than its own extent.
///
/// Fail closed: only where every node of the column, in every partition, has
/// x and y extents equal to the record's (`targets`, the column's current
/// extent) and the same z-range at id-48, and that range lies inside the
/// current one and differs from it. A column whose copies disagree keeps
/// its extent.
pub fn column_cut_extents(
    rf: &mut RevitFile,
    targets: &BTreeMap<u32, [f64; 6]>,
) -> BTreeMap<u32, (f64, f64)> {
    column_nodes(rf, targets)
        .into_iter()
        .filter_map(|(id, nodes)| {
            let record = targets.get(&id)?;
            let (_, first) = nodes.first()?;
            let (bottom, top) = (first[2], first[5]);
            let agree = nodes.iter().all(|(own, after)| {
                plan_equal(own, after, 1e-9)
                    && plan_equal(after, record, PLAN_TOLERANCE_FEET)
                    && (after[2] - bottom).abs() < TOLERANCE_FEET
                    && (after[5] - top).abs() < TOLERANCE_FEET
            });
            let inside = bottom > record[2] - TOLERANCE_FEET && top < record[5] + TOLERANCE_FEET;
            let differs = (bottom - record[2]).abs() > TOLERANCE_FEET
                || (top - record[5]).abs() > TOLERANCE_FEET;
            (agree && inside && differs).then_some((id, (bottom, top)))
        })
        .collect()
}
