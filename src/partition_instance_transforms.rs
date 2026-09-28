//! A family instance's transform, from its own element data (RE-87).
//!
//! A family instance's data holds its placement as twelve `f64`: a 3 × 3
//! rotation stored row by row, then the origin in model feet. The rotation's
//! columns are the instance's X, Y and Z axes. The reader takes the first
//! place in the data where nine `f64` form a rotation.
//!
//! On Autodesk's Snowdon Towers 2024 architectural sample, 1,956 elements of
//! Revit's IFC4 export hold one whose Z axis is the model's. On 1,951 its X
//! axis is Revit's placement X, its reverse, or a right angle from it. The
//! other 5 are walls, not family instances. All 49 on RE1 Architecture
//! (Revit 2025) agree.
//!
//! ```text
//! f64 Xx · f64 Yx · f64 Zx · f64 Xy · f64 Yy · f64 Zy · f64 Xz · f64 Yz · f64 Zz · f64 ox · f64 oy · f64 oz
//! ```

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// How far into an instance's data its transform is looked for. On Snowdon
/// Towers it starts 354 to 462 bytes in.
pub const INSTANCE_TRANSFORM_WINDOW: usize = 0x1000;

/// How closely the nine values must form a rotation.
pub const ROTATION_TOLERANCE: f64 = 1e-9;

/// A family instance's placement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstanceTransform {
    /// Unit X, Y and Z axes, in model coordinates.
    pub axes: [[f64; 3]; 3],
    /// Model feet.
    pub origin: [f64; 3],
}

impl InstanceTransform {
    /// Whether the instance's Z axis is the model's.
    pub fn is_upright(&self) -> bool {
        let z = self.axes[2];
        z[0].abs() <= ROTATION_TOLERANCE
            && z[1].abs() <= ROTATION_TOLERANCE
            && (z[2] - 1.0).abs() <= ROTATION_TOLERANCE
    }

    /// The unit plan direction of the instance's first horizontal axis,
    /// where one of its axes is vertical (up or down): the X axis of an
    /// upright instance, and of one hosted on a ceiling or a wall, the axis
    /// that lies flat. `None` for an instance tilted off the vertical.
    pub fn plan_axis(&self) -> Option<[f64; 2]> {
        let flat = |axis: &[f64; 3]| axis[2].abs() <= ROTATION_TOLERANCE;
        if !self
            .axes
            .iter()
            .any(|axis| (axis[2].abs() - 1.0).abs() <= ROTATION_TOLERANCE)
        {
            return None;
        }
        let axis = self.axes.iter().find(|axis| flat(axis))?;
        let length = axis[0].hypot(axis[1]);
        (length > 0.5).then(|| [axis[0] / length, axis[1] / length])
    }

    /// The plan angle of the X axis, radians.
    pub fn plan_angle(&self) -> f64 {
        self.axes[0][1].atan2(self.axes[0][0])
    }
}

fn f64_at(data: &[u8], at: usize) -> Option<f64> {
    data.get(at..at.checked_add(8)?)
        .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// The nine values at `at` as a rotation, rows first; `None` unless they
/// are orthonormal with determinant 1.
fn rotation_at(data: &[u8], at: usize) -> Option<[[f64; 3]; 3]> {
    let mut rows = [[0.0; 3]; 3];
    for (index, value) in rows.iter_mut().flatten().enumerate() {
        let v = f64_at(data, at + 8 * index)?;
        if !(v.is_finite() && v.abs() <= 1.0 + ROTATION_TOLERANCE) {
            return None;
        }
        *value = v;
    }
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    for i in 0..3 {
        for j in 0..3 {
            let want = if i == j { 1.0 } else { 0.0 };
            if (dot(rows[i], rows[j]) - want).abs() > ROTATION_TOLERANCE {
                return None;
            }
        }
    }
    let [a, b, c] = rows;
    let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    ((det - 1.0).abs() <= ROTATION_TOLERANCE).then_some(rows)
}

/// The first transform in an instance's `data`, within
/// [`INSTANCE_TRANSFORM_WINDOW`].
pub fn instance_transform(data: &[u8]) -> Option<InstanceTransform> {
    let end = data.len().min(INSTANCE_TRANSFORM_WINDOW);
    (0..end.saturating_sub(96)).find_map(|at| {
        let rows = rotation_at(data, at)?;
        let origin = [
            f64_at(data, at + 72)?,
            f64_at(data, at + 80)?,
            f64_at(data, at + 88)?,
        ];
        if !origin.iter().all(|v| v.is_finite()) {
            return None;
        }
        let column = |j: usize| [rows[0][j], rows[1][j], rows[2][j]];
        Some(InstanceTransform {
            axes: [column(0), column(1), column(2)],
            origin,
        })
    })
}

/// Each instance in `ids` whose data holds a transform, by ElementId; an
/// instance whose copies disagree is dropped. Empty on a release without a
/// measured element-data header.
pub fn scan_instance_transforms(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, InstanceTransform>> {
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let mut found: BTreeMap<u32, Option<InstanceTransform>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = buf
                .get(id_at..id_at + 8)
                .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| ids.contains(id))
            else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(id_at + 8 + INSTANCE_TRANSFORM_WINDOW)
                .min(buf.len());
            let Some(transform) = buf.get(id_at + 8..end).and_then(instance_transform) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(transform));
                }
                Some(held) => {
                    if *held != Some(transform) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, transform)| transform.map(|t| (id, t)))
        .collect())
}
