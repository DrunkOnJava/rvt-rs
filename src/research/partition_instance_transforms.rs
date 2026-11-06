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

use crate::ifc::element_provenance::{CopyRule, ValueSource};
use crate::le_bytes;
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

/// The nine values at `at` as a rotation, rows first; `None` unless they
/// are orthonormal with determinant 1.
fn rotation_at(data: &[u8], at: usize) -> Option<[[f64; 3]; 3]> {
    let mut rows = [[0.0; 3]; 3];
    for (index, value) in rows.iter_mut().flatten().enumerate() {
        let v = le_bytes::f64_at(data, at + 8 * index)?;
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
            le_bytes::f64_at(data, at + 72)?,
            le_bytes::f64_at(data, at + 80)?,
            le_bytes::f64_at(data, at + 88)?,
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

/// Each instance in `ids` whose data holds a transform, by ElementId. Where
/// an instance's copies in different partitions disagree, the copy taken is
/// the one in the partition its current record is in: the partition its
/// stored revision routes to, or the latest one holding a copy where that
/// one holds none, as `native_document` reads records (RE-145, #548); a
/// partition holds a copy wherever its element data names the instance,
/// whether or not a transform is read there. On Core Interior 8 stacked
/// doors have a copy one inch off across their host in an earlier partition;
/// the current copy places Revit's opening exactly (#227). An instance whose
/// current copy cannot be told, or holds no transform that is read, is
/// dropped. Empty on a release without a measured element-data header.
pub fn scan_instance_transforms(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, InstanceTransform>> {
    if crate::partition_names::element_data_header(revit_version).is_none() {
        return Ok(BTreeMap::new());
    }
    scan_transforms_in_layout(rf, revit_version, ids)
}

/// [`scan_instance_transforms`] on any release with an element-data layout,
/// the 32-bit ones too, for a reader measured on them (RE-366).
pub fn scan_transforms_in_layout(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, InstanceTransform>> {
    let Some(layout) = crate::partition_names::element_data_layout(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let header = layout.header;
    let mut found: BTreeMap<u32, Vec<(u32, InstanceTransform)>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    // Where each copy's element-data header starts, in the order of `found`,
    // kept only while the handle records value sources (#1360).
    let recording = rf.recording_value_sources();
    let mut offsets: BTreeMap<u32, Vec<usize>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Some(partition) = stream
            .strip_prefix("Partitions/")
            .and_then(|n| n.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let Ok(hits) = rf.header_hits(&stream, &header) else {
            continue;
        };
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| ids.contains(id)) else {
                continue;
            };
            holders.entry(id).or_default().insert(partition);
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(id_at + 8 + INSTANCE_TRANSFORM_WINDOW)
                .min(buf.len());
            let Some(transform) = buf.get(id_at + 8..end).and_then(instance_transform) else {
                continue;
            };
            found.entry(id).or_default().push((partition, transform));
            if recording {
                offsets.entry(id).or_default().push(hit);
            }
        }
    }
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = found
        .iter()
        .filter(|(_, copies)| copies.iter().any(|(_, t)| *t != copies[0].1))
        .filter_map(|(id, _)| Some((*id, holders.get(id)?.clone())))
        .collect();
    let current_partitions = crate::native_document::current_partitions(rf, &disagreeing);
    let mut noted: Vec<(u32, ValueSource)> = Vec::new();
    let picked: BTreeMap<u32, InstanceTransform> = found
        .into_iter()
        .filter_map(|(id, copies)| {
            let held = holders.get(&id).map_or(0, BTreeSet::len);
            let offset_of = |index: usize| offsets.get(&id).and_then(|at| at.get(index)).copied();
            let first = copies[0].1;
            if copies.iter().all(|(_, t)| *t == first) {
                if recording {
                    let partitions: BTreeSet<u32> =
                        copies.iter().map(|(partition, _)| *partition).collect();
                    let source = ValueSource::in_partition(
                        READER,
                        CopyRule::Agreeing,
                        held,
                        copies[0].0,
                        offset_of(0),
                    )
                    .with_partitions(partitions.into_iter().collect());
                    noted.push((id, source));
                }
                return Some((id, first));
            }
            let current = *current_partitions.get(&id)?;
            let mut in_current = copies
                .iter()
                .enumerate()
                .filter(|(_, (p, _))| *p == current)
                .map(|(index, (_, t))| (index, *t));
            let (index, pick) = in_current.next()?;
            if !in_current.all(|(_, t)| t == pick) {
                return None;
            }
            if recording {
                let source = ValueSource::in_partition(
                    READER,
                    CopyRule::Current,
                    held,
                    current,
                    offset_of(index),
                );
                noted.push((id, source));
            }
            Some((id, pick))
        })
        .collect();
    for (id, source) in noted {
        rf.note_value_source(id, source);
    }
    Ok(picked)
}

/// The reader name a value source of [`scan_instance_transforms`] carries.
const READER: &str = "instance_transform";
