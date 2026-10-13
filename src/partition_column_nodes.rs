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
//! See `reports/element-framing/RE-173-column-z-from-geometry-node.md`.

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

/// The bottom, in feet, of each column in `targets` (ElementId -> record box)
/// that a geometry node gives and agrees on.
pub fn column_bottoms(rf: &mut RevitFile, targets: &BTreeMap<u32, [f64; 6]>) -> BTreeMap<u32, f64> {
    let mut found: BTreeMap<u32, Vec<f64>> = BTreeMap::new();
    if targets.is_empty() {
        return BTreeMap::new();
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
            let Some(record) = targets.get(&id) else {
                continue;
            };
            let (Some(first), Some(second)) = (boxes(buf, id_at - 96), boxes(buf, id_at - 48))
            else {
                continue;
            };
            let same_plan = [0, 1, 3, 4]
                .iter()
                .all(|&i| (first[i] - second[i]).abs() < 1e-9);
            let record_plan = [0, 1, 3, 4]
                .iter()
                .all(|&i| (second[i] - record[i]).abs() < PLAN_TOLERANCE_FEET);
            if same_plan
                && record_plan
                && (second[5] - record[5]).abs() < TOLERANCE_FEET
                && (second[2] - record[2]).abs() > TOLERANCE_FEET
            {
                found.entry(id).or_default().push(second[2]);
            }
        }
    }
    found
        .into_iter()
        .filter_map(|(id, bottoms)| {
            let first = *bottoms.first()?;
            bottoms
                .iter()
                .all(|b| (b - first).abs() < TOLERANCE_FEET)
                .then_some((id, first))
        })
        .collect()
}
