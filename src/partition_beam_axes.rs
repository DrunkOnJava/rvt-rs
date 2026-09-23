//! Beam location lines from their serialised element data (RE-49).
//!
//! A structural-framing element's data (its element-data header and
//! ElementId, see [`crate::partition_names::element_data_header`]) carries
//! its location line as a bounded line:
//!
//! ```text
//! +0   04 00 08 01   BOUNDED_LINE_TAG
//! +4   f64           start parameter, feet
//! +12  f64           end parameter, feet
//! +20  f64 × 3       origin, model feet
//! +44  f64 × 3       unit direction
//! ```
//!
//! The line runs from `origin + start · direction` to
//! `origin + end · direction`. On Autodesk's Snowdon Towers 2024 structural
//! sample, the data of every one of the 942 structural-framing elements
//! rvt-rs exports has one, and on 940 both ends lie inside the element's
//! record box (the other two lie far outside it and are not read). The same
//! record appears in the data of walls, model lines and sketch lines; only
//! framing is read here.
//!
//! [`beam_body`] turns the line and the record's box into the beam's solid:
//! a box `length × width × depth` along the line. The length is the line's.
//! The section is what the box leaves once the line's own extent is taken
//! out of it: the depth from the vertical extent, the width from one plan
//! extent, and the other plan extent is a check the solve must pass. It
//! passes, to [`CLOSURE_TOLERANCE_FEET`], on 923 of the 940 (802 level beams
//! on a model axis, 111 level beams rotated in plan, 10 sloped). The 17 that
//! fail keep their box: 15 are oblique beams whose end cuts are not square to
//! the line, and 2 have a box longer than their line.
//!
//! Measured against the meshes of the same beams in a VIM export of a later
//! edition of the sample (`reports/element-framing/RE-49-beam-axes.md`,
//! `examples/probe_re49_beam_axes.rs`), the section across the line is
//! Revit's on 613 of the 839 beams that edition kept unchanged, and Revit's
//! less the top a floor join cuts away on 179 more. What the line does not
//! carry is where Revit trims a beam at its supports (on 352 of the 839, by
//! a median 0.625 ft at the larger end), so the solid runs the line's full
//! length.

use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this layout is measured.
pub const BEAM_AXIS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// The bytes a bounded line starts with.
pub const BOUNDED_LINE_TAG: [u8; 4] = [0x04, 0x00, 0x08, 0x01];

/// How far into an element's data a line is looked for, when the next
/// element-data header does not end it first. The first line of a Snowdon
/// beam sits at most `0x64a8` bytes in (a concrete beam's long data); most
/// sit within `0x600`.
pub const BEAM_DATA_WINDOW: usize = 0x1_0000;

/// How far a line's end may lie outside the record's box. Measured ends are
/// inside to the last bit or far outside, never in between.
pub const END_TOLERANCE_FEET: f64 = 1e-6;

/// How closely the plan extent the section solve does not use must be
/// reproduced by the solved box.
pub const CLOSURE_TOLERANCE_FEET: f64 = 0.01;

/// The least horizontal share of a line's direction that still gives the
/// section a horizontal width axis. Steeper members are not solved.
pub const MIN_PLAN_SHARE: f64 = 0.1;

/// Below this vertical share a direction is horizontal.
pub const HORIZONTAL_EPS: f64 = 1e-9;

/// Whether `revit_version` is a release this layout is measured on.
pub fn supports_revit_version(revit_version: u32) -> bool {
    BEAM_AXIS_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

/// A bounded line record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedLine {
    pub start_parameter: f64,
    pub end_parameter: f64,
    pub origin: [f64; 3],
    pub direction: [f64; 3],
}

impl BoundedLine {
    fn point(&self, parameter: f64) -> [f64; 3] {
        [
            self.origin[0] + parameter * self.direction[0],
            self.origin[1] + parameter * self.direction[1],
            self.origin[2] + parameter * self.direction[2],
        ]
    }

    /// The line's first end, model feet.
    pub fn start(&self) -> [f64; 3] {
        self.point(self.start_parameter)
    }

    /// The line's second end, model feet.
    pub fn end(&self) -> [f64; 3] {
        self.point(self.end_parameter)
    }
}

fn read_f64(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|s| f64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// The bounded line whose tag starts at `at`: finite values, a unit
/// direction and a start parameter below the end one.
pub fn bounded_line_at(buf: &[u8], at: usize) -> Option<BoundedLine> {
    if buf.get(at..at.checked_add(4)?)? != BOUNDED_LINE_TAG {
        return None;
    }
    let mut values = [0.0f64; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value = read_f64(buf, at + 4 + 8 * index)?;
    }
    if !values.iter().all(|v| v.is_finite()) {
        return None;
    }
    let [start_parameter, end_parameter, ox, oy, oz, dx, dy, dz] = values;
    if (dx * dx + dy * dy + dz * dz - 1.0).abs() > 1e-9 || start_parameter >= end_parameter {
        return None;
    }
    Some(BoundedLine {
        start_parameter,
        end_parameter,
        origin: [ox, oy, oz],
        direction: [dx, dy, dz],
    })
}

/// The first bounded line in `data`.
pub fn first_bounded_line(data: &[u8]) -> Option<BoundedLine> {
    memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG).find_map(|at| bounded_line_at(data, at))
}

/// The first bounded line in the element data of each id in `beams`, read
/// from every partition. The data runs from its header to the next header,
/// at most [`BEAM_DATA_WINDOW`] bytes. An id whose copies disagree is
/// dropped; the map is empty for a release this layout is not measured on.
pub fn scan_beam_axes(
    rf: &mut RevitFile,
    revit_version: u32,
    beams: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedLine>> {
    let header = match crate::partition_names::element_data_header(revit_version) {
        Some(header) if supports_revit_version(revit_version) => header,
        _ => return Ok(BTreeMap::new()),
    };
    let mut found: BTreeMap<u32, Option<BoundedLine>> = BTreeMap::new();
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
                .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
                .and_then(|id| u32::try_from(id).ok())
            else {
                continue;
            };
            if !beams.contains(&id) {
                continue;
            }
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(BEAM_DATA_WINDOW))
                .min(buf.len());
            let Some(line) = buf.get(id_at + 8..end).and_then(first_bounded_line) else {
                continue;
            };
            match found.get_mut(&id) {
                None => {
                    found.insert(id, Some(line));
                }
                Some(held) => {
                    if held.as_ref() != Some(&line) {
                        *held = None;
                    }
                }
            }
        }
    }
    Ok(found
        .into_iter()
        .filter_map(|(id, line)| line.map(|line| (id, line)))
        .collect())
}

/// A beam's solid: a box along its location line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BeamBody {
    /// Centre of the box, model feet: the centre of the record's box.
    pub centre: [f64; 3],
    /// Unit direction of the line.
    pub direction: [f64; 3],
    /// Extent along the line, feet: the line's length.
    pub length_feet: f64,
    /// Horizontal extent across the line, feet.
    pub width_feet: f64,
    /// Extent across the line in its vertical plane, feet.
    pub depth_feet: f64,
}

impl BeamBody {
    /// Whether the line is level.
    pub fn is_horizontal(&self) -> bool {
        self.direction[2].abs() <= HORIZONTAL_EPS
    }

    /// Plan angle of the line from +X, radians.
    pub fn plan_angle_radians(&self) -> f64 {
        self.direction[1].atan2(self.direction[0])
    }

    /// The ends of the box's centreline, model feet.
    pub fn centreline(&self) -> ([f64; 3], [f64; 3]) {
        let half = self.length_feet / 2.0;
        let at = |sign: f64| {
            [
                self.centre[0] + sign * half * self.direction[0],
                self.centre[1] + sign * half * self.direction[1],
                self.centre[2] + sign * half * self.direction[2],
            ]
        };
        (at(-1.0), at(1.0))
    }
}

/// The box along the line from `start` to `end` whose axis-aligned extent is
/// the record box `bbox` (`[min x, min y, min z, max x, max y, max z]`, model
/// feet), or `None` when the line leaves the box, is too steep for a
/// horizontal width axis, or no such box reproduces the box's extents.
pub fn beam_body(bbox: [f64; 6], start: [f64; 3], end: [f64; 3]) -> Option<BeamBody> {
    if !bbox.iter().chain(&start).chain(&end).all(|v| v.is_finite()) {
        return None;
    }
    for axis in 0..3 {
        let (low, high) = (bbox[axis], bbox[axis + 3]);
        if low > high {
            return None;
        }
        for point in [start, end] {
            if point[axis] < low - END_TOLERANCE_FEET || point[axis] > high + END_TOLERANCE_FEET {
                return None;
            }
        }
    }
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if length <= 0.0 {
        return None;
    }
    let d = [delta[0] / length, delta[1] / length, delta[2] / length];
    let plan = d[0].hypot(d[1]);
    if plan < MIN_PLAN_SHARE {
        return None;
    }
    // Width axis: horizontal, across the line. Depth axis: the line's
    // vertical plane, across the line (d × u).
    let u = [-d[1] / plan, d[0] / plan, 0.0];
    let w = [-d[2] * d[0] / plan, -d[2] * d[1] / plan, plan];
    let extent = [bbox[3] - bbox[0], bbox[4] - bbox[1], bbox[5] - bbox[2]];
    // Each extent is length·|d| + width·|u| + depth·|w| on its axis.
    let depth = (extent[2] - length * d[2].abs()) / w[2];
    let (solve, check) = if u[0].abs() >= u[1].abs() {
        (0, 1)
    } else {
        (1, 0)
    };
    let width = (extent[solve] - length * d[solve].abs() - depth * w[solve].abs()) / u[solve].abs();
    let closure = length * d[check].abs() + width * u[check].abs() + depth * w[check].abs();
    if !(width > 0.0 && depth > 0.0) || (closure - extent[check]).abs() > CLOSURE_TOLERANCE_FEET {
        return None;
    }
    Some(BeamBody {
        centre: [
            (bbox[0] + bbox[3]) / 2.0,
            (bbox[1] + bbox[4]) / 2.0,
            (bbox[2] + bbox[5]) / 2.0,
        ],
        direction: d,
        length_feet: length,
        width_feet: width,
        depth_feet: depth,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line_bytes(values: [f64; 8]) -> Vec<u8> {
        let mut out = BOUNDED_LINE_TAG.to_vec();
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    // Snowdon Towers structural beam 627866 (W18x55), its first line and
    // its record box.
    const SNOWDON_627866_LINE: [f64; 8] = [
        0.405647, 23.015746, 61.767124, 41.951037, 18.333333, 0.130526, -0.991445, 0.0,
    ];
    const SNOWDON_627866_BOX: [f64; 6] = [61.509, 19.091, 16.825, 65.082, 41.59, 18.333333];

    fn unit(v: [f64; 8]) -> [f64; 8] {
        let n = (v[5] * v[5] + v[6] * v[6] + v[7] * v[7]).sqrt();
        [v[0], v[1], v[2], v[3], v[4], v[5] / n, v[6] / n, v[7] / n]
    }

    #[test]
    fn reads_the_bounded_line_after_its_tag() {
        let mut data = vec![0xffu8; 7];
        data.extend(line_bytes(unit(SNOWDON_627866_LINE)));
        let line = first_bounded_line(&data).expect("line");
        assert!((line.start_parameter - 0.405647).abs() < 1e-12);
        assert!((line.end_parameter - 23.015746).abs() < 1e-12);
        assert_eq!(line.origin, [61.767124, 41.951037, 18.333333]);
        let start = line.start();
        assert!((start[0] - (61.767124 + 0.405647 * line.direction[0])).abs() < 1e-12);
    }

    #[test]
    fn rejects_a_line_that_is_not_one() {
        let mut values = unit(SNOWDON_627866_LINE);
        values[5] = 0.5;
        assert!(first_bounded_line(&line_bytes(values)).is_none());
        let mut values = unit(SNOWDON_627866_LINE);
        values[1] = values[0];
        assert!(first_bounded_line(&line_bytes(values)).is_none());
        let mut values = unit(SNOWDON_627866_LINE);
        values[3] = f64::NAN;
        assert!(first_bounded_line(&line_bytes(values)).is_none());
        let short = &line_bytes(unit(SNOWDON_627866_LINE))[..40];
        assert!(first_bounded_line(short).is_none());
    }

    #[test]
    fn solves_a_rotated_beams_section_from_its_box() {
        let line = bounded_line_at(&line_bytes(unit(SNOWDON_627866_LINE)), 0).expect("line");
        let start = line.start();
        let end = line.end();
        // The record box is exactly the box of this solid, rounded to the
        // millimetre above; rebuild it from the solved section to test the
        // solve exactly.
        let body = beam_body(snowdon_box(start, end), start, end).expect("body");
        assert!(body.is_horizontal());
        assert!((body.length_feet - 22.610099).abs() < 1e-6);
        assert!((body.width_feet - 0.6275).abs() < 1e-9);
        assert!((body.depth_feet - 1.508333).abs() < 1e-6);
        assert!((body.plan_angle_radians() - (-82.5f64).to_radians()).abs() < 1e-4);
        let (a, b) = body.centreline();
        assert!((a[1] - b[1]).abs() > 22.0);
        // The box is the record's box to its rounding.
        for (solved, recorded) in snowdon_box(start, end).iter().zip(SNOWDON_627866_BOX) {
            assert!((solved - recorded).abs() < 1e-3);
        }
    }

    fn snowdon_box(start: [f64; 3], end: [f64; 3]) -> [f64; 6] {
        // W18x55: 0.6275 ft flange, 1.508333 ft deep, top of steel on the
        // line.
        let (half_width, depth) = (0.6275 / 2.0, 1.508333);
        let d = [end[0] - start[0], end[1] - start[1]];
        let n = d[0].hypot(d[1]);
        let across = [-d[1] / n * half_width, d[0] / n * half_width];
        let xs = [
            start[0] + across[0],
            start[0] - across[0],
            end[0] + across[0],
            end[0] - across[0],
        ];
        let ys = [
            start[1] + across[1],
            start[1] - across[1],
            end[1] + across[1],
            end[1] - across[1],
        ];
        let min = |v: [f64; 4]| v.iter().copied().fold(f64::INFINITY, f64::min);
        let max = |v: [f64; 4]| v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        [
            min(xs),
            min(ys),
            start[2] - depth,
            max(xs),
            max(ys),
            start[2],
        ]
    }

    #[test]
    fn solves_a_sloped_beam() {
        // 2 degrees down along +Y, 0.25 ft wide, 0.3 ft deep.
        let slope = 2f64.to_radians();
        let d = [0.0, slope.cos(), -slope.sin()];
        let (length, width, depth) = (20.0, 0.25, 0.3);
        let start = [5.0, 1.0, 30.0];
        let end = [
            start[0] + length * d[0],
            start[1] + length * d[1],
            start[2] + length * d[2],
        ];
        // The box of that solid, with the line through the section centre.
        let w_axis = [0.0, slope.sin(), slope.cos()];
        let ex = width;
        let ey = length * d[1] + depth * w_axis[1];
        let ez = length * d[2].abs() + depth * w_axis[2];
        let centre = [
            (start[0] + end[0]) / 2.0,
            (start[1] + end[1]) / 2.0,
            (start[2] + end[2]) / 2.0,
        ];
        let bbox = [
            centre[0] - ex / 2.0,
            centre[1] - ey / 2.0,
            centre[2] - ez / 2.0,
            centre[0] + ex / 2.0,
            centre[1] + ey / 2.0,
            centre[2] + ez / 2.0,
        ];
        let body = beam_body(bbox, start, end).expect("body");
        assert!(!body.is_horizontal());
        assert!((body.length_feet - length).abs() < 1e-9);
        assert!((body.width_feet - width).abs() < 1e-9);
        assert!((body.depth_feet - depth).abs() < 1e-9);
        assert_eq!(body.centre, centre);
    }

    #[test]
    fn declines_what_the_box_does_not_support() {
        let start = [0.0, 0.0, 1.0];
        let end = [10.0, 0.0, 1.0];
        let bbox = [0.0, -0.5, 0.0, 10.0, 0.5, 1.0];
        assert!(beam_body(bbox, start, end).is_some());
        // An end outside the box.
        assert!(beam_body(bbox, start, [10.5, 0.0, 1.0]).is_none());
        // A zero-length line.
        assert!(beam_body(bbox, start, start).is_none());
        // A vertical member has no horizontal width axis.
        assert!(
            beam_body(
                [0.0, 0.0, 0.0, 1.0, 1.0, 10.0],
                [0.5, 0.5, 0.0],
                [0.5, 0.5, 10.0]
            )
            .is_none()
        );
        // A rotated line whose box is too narrow across it to close: the
        // skewed end cuts that keep their box.
        let d = [30f64.to_radians().cos(), 30f64.to_radians().sin()];
        let skew_end = [10.0 * d[0], 10.0 * d[1], 1.0];
        let tight = [0.0, 0.0, 0.0, skew_end[0], skew_end[1], 1.0];
        assert!(beam_body(tight, start, skew_end).is_none());
        // Non-finite input.
        assert!(beam_body([f64::NAN, 0.0, 0.0, 1.0, 1.0, 1.0], start, end).is_none());
    }
}
