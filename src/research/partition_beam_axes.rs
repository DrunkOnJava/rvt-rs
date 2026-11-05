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
//! fail have a record box longer than their line (RE-328 measured all 17
//! against their saved meshes: the box along the line is Revit's section
//! across it; RE-49 took 15 of them, rotated in plan, for beams with oblique
//! end cuts). [`beam_body_in_box`] draws such a level beam whose Cross-Section
//! Rotation is 0 as the record box's own length; the rest keep their box.
//!
//! Measured against the meshes of the same beams in a VIM export of a later
//! edition of the sample (`reports/element-framing/RE-49-beam-axes.md`,
//! `probes/examples/probe_re49_beam_axes.rs`), the section across the line is
//! Revit's on 613 of the 839 beams that edition kept unchanged, and Revit's
//! less the top a floor join cuts away on 179 more. What the line does not
//! carry is where Revit trims a beam at its supports (on 352 of the 839, by
//! a median 0.625 ft at the larger end), so the solid runs the line's full
//! length.

use crate::ifc::element_provenance::{CopyRule, ValueSource};
use crate::le_bytes;
use crate::{Result, RevitFile};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where a beam's location line is measured: 2024, and Revit 2025,
/// 2026 and 2027 against the same Autodesk samples saved by an earlier
/// release (RE-341, RE-345): the beams of `rst_basic` and `rac_advanced` on
/// 2025 to 2027 are their 2024 saves'.
pub const BEAM_AXIS_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Releases where the arc records of a sketch line are read, for the outlines
/// of floors, ceilings and roofs: 2024, and Revit 2025, 2026 and 2027 against
/// the same Autodesk samples saved by an earlier release (RE-341, RE-345). No
/// Revit export checks them on 2025 to 2027: the models with Revit's own 2025
/// exports hold no floor, ceiling or roof with an arc. Its own list, apart
/// from the beams' and the shafts', so that each reader's release is a
/// decision of its own.
pub const SKETCH_CURVE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Releases where a shaft opening's outline, its lines and circles, is read
/// and cut from the floors, ceilings and roofs it passes through (RE-99,
/// RE-100): 2024 only (Griffin, 2026-11-05). Measured there on Autodesk's
/// Snowdon Towers architectural sample, 9 shaft openings of which 7 give a
/// void that equals a shaft's outline. The only shafts measured on a Revit
/// 2025 file, the 2 of rac_advanced, cut nothing, and no file of 2026 or 2027
/// is known to hold one that does, so the same-project check could not catch
/// a bad cut there (RE-345). A shaft on a later release leaves its floor,
/// ceiling or roof whole, and the export's `shaft_opening_not_cut` skipped
/// item counts it.
pub const SHAFT_OUTLINE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

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

/// Whether `revit_version` is a release a sketch line's arcs are read on
/// ([`SKETCH_CURVE_SUPPORTED_REVIT_VERSIONS`]).
pub fn supports_sketch_curves(revit_version: u32) -> bool {
    SKETCH_CURVE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

/// Whether `revit_version` is a release a shaft opening is cut on
/// ([`SHAFT_OUTLINE_SUPPORTED_REVIT_VERSIONS`]).
pub fn supports_shaft_outlines(revit_version: u32) -> bool {
    SHAFT_OUTLINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
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

/// The bounded line whose tag starts at `at`: finite values, a unit
/// direction and a start parameter below the end one.
pub fn bounded_line_at(buf: &[u8], at: usize) -> Option<BoundedLine> {
    if buf.get(at..at.checked_add(4)?)? != BOUNDED_LINE_TAG {
        return None;
    }
    let mut values = [0.0f64; 8];
    for (index, value) in values.iter_mut().enumerate() {
        *value = le_bytes::f64_at(buf, at + 4 + 8 * index)?;
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

/// The first bounded line in `data`. `None` where the data's first curve
/// record is an arc ([`bounded_arc_at`]): its axes would read as a line.
pub fn first_bounded_line(data: &[u8]) -> Option<BoundedLine> {
    let first = memchr::memmem::find(data, &BOUNDED_LINE_TAG)?;
    if bounded_arc_at(data, first).is_some() {
        return None;
    }
    memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG).find_map(|at| bounded_line_at(data, at))
}

/// The most lines [`all_bounded_lines`] returns.
pub const MAX_LINES_PER_ELEMENT: usize = 64;

/// Every bounded line in `data`, in order, up to [`MAX_LINES_PER_ELEMENT`]
/// and without the arcs ([`bounded_arc_at`]); `None` where it holds none
/// (RE-328: whether a beam's data holds another line than its first).
pub fn all_bounded_lines(data: &[u8]) -> Option<Vec<BoundedLine>> {
    let lines: Vec<BoundedLine> = memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG)
        .filter(|&at| bounded_arc_at(data, at).is_none())
        .filter_map(|at| bounded_line_at(data, at))
        .take(MAX_LINES_PER_ELEMENT)
        .collect();
    (!lines.is_empty()).then_some(lines)
}

/// A circular arc record (RE-75): the [`BOUNDED_LINE_TAG`] record preceded
/// by a `u64` 1 where a line's is 0, and laid out
///
/// ```text
/// -8   u64 1         the record is an arc
/// +0   04 00 08 01   BOUNDED_LINE_TAG
/// +4   f64           start angle, radians
/// +12  f64           end angle, radians
/// +20  f64 × 3       unit X axis
/// +44  f64 × 3       unit Y axis, square to X
/// +68  f64           radius, feet
/// +76  f64 × 3       centre, model feet
/// ```
///
/// The arc runs from `centre + radius · (cos a · X + sin a · Y)` at the
/// start angle to the end angle. On Autodesk's Snowdon Towers 2024
/// architectural sample it is the first curve record of 32 walls, and on
/// the 24 of them Revit's IFC4 export draws with an arc axis, its centre and
/// radius are that axis's to 1e-5 ft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundedArc {
    pub start_angle: f64,
    pub end_angle: f64,
    pub x_axis: [f64; 3],
    pub y_axis: [f64; 3],
    pub radius: f64,
    pub centre: [f64; 3],
}

impl BoundedArc {
    /// The point at `angle` on the arc, model feet.
    pub fn point(&self, angle: f64) -> [f64; 3] {
        let (sin, cos) = angle.sin_cos();
        [0, 1, 2].map(|axis| {
            self.centre[axis] + self.radius * (cos * self.x_axis[axis] + sin * self.y_axis[axis])
        })
    }
}

/// The arc whose tag starts at `at`: the `u64` before it is 1, the values
/// are finite, the axes are unit and square to each other, the radius is
/// positive and the start angle is below the end one.
pub fn bounded_arc_at(buf: &[u8], at: usize) -> Option<BoundedArc> {
    if buf.get(at..at.checked_add(4)?)? != BOUNDED_LINE_TAG {
        return None;
    }
    let kind = le_bytes::u64_at(buf, at.checked_sub(8)?)?;
    if kind != 1 {
        return None;
    }
    arc_values_at(buf, at)
}

/// The record at `at` read as an arc ([`bounded_arc_at`]'s layout), whatever
/// the `u64` before it: a sketch line's arc is not marked (RE-96).
pub fn arc_values_at(buf: &[u8], at: usize) -> Option<BoundedArc> {
    if buf.get(at..at.checked_add(4)?)? != BOUNDED_LINE_TAG {
        return None;
    }
    let mut values = [0.0f64; 12];
    for (index, value) in values.iter_mut().enumerate() {
        *value = le_bytes::f64_at(buf, at + 4 + 8 * index)?;
    }
    if !values.iter().all(|v| v.is_finite()) {
        return None;
    }
    let [
        start_angle,
        end_angle,
        xx,
        xy,
        xz,
        yx,
        yy,
        yz,
        radius,
        cx,
        cy,
        cz,
    ] = values;
    let unit = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - 1.0).abs() <= 1e-9;
    let (x_axis, y_axis) = ([xx, xy, xz], [yx, yy, yz]);
    if !unit(x_axis)
        || !unit(y_axis)
        || (xx * yx + xy * yy + xz * yz).abs() > 1e-9
        || radius <= 0.0
        || start_angle >= end_angle
    {
        return None;
    }
    Some(BoundedArc {
        start_angle,
        end_angle,
        x_axis,
        y_axis,
        radius,
        centre: [cx, cy, cz],
    })
}

/// The arc a wall's data stores where its first curve record is one
/// ([`bounded_arc_at`]).
pub fn first_bounded_arc(data: &[u8]) -> Option<BoundedArc> {
    bounded_arc_at(data, memchr::memmem::find(data, &BOUNDED_LINE_TAG)?)
}

/// The tag of a full circle's record (RE-100): [`bounded_arc_at`]'s layout
/// with both angles 0, where a bounded curve's tag ends in 1.
pub const CIRCLE_TAG: [u8; 4] = [0x04, 0x00, 0x08, 0x00];

/// The full circle in `data`: the first [`CIRCLE_TAG`] record whose angles
/// are both 0, axes unit and square to each other, and radius positive,
/// returned as an arc from 0 to a full turn (RE-100).
pub fn first_circle(data: &[u8]) -> Option<BoundedArc> {
    memchr::memmem::find_iter(data, &CIRCLE_TAG).find_map(|at| {
        let mut values = [0.0f64; 12];
        for (index, value) in values.iter_mut().enumerate() {
            *value = le_bytes::f64_at(data, at + 4 + 8 * index)?;
        }
        if !values.iter().all(|v| v.is_finite()) {
            return None;
        }
        let [a0, a1, xx, xy, xz, yx, yy, yz, radius, cx, cy, cz] = values;
        let unit = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - 1.0).abs() <= 1e-9;
        let (x_axis, y_axis) = ([xx, xy, xz], [yx, yy, yz]);
        (a0 == 0.0
            && a1 == 0.0
            && unit(x_axis)
            && unit(y_axis)
            && (xx * yx + xy * yy + xz * yz).abs() <= 1e-9
            && radius > 0.0)
            .then_some(BoundedArc {
                start_angle: 0.0,
                end_angle: std::f64::consts::TAU,
                x_axis,
                y_axis,
                radius,
                centre: [cx, cy, cz],
            })
    })
}

/// The full circle of each sketch line in `ids` ([`first_circle`]), read as
/// [`scan_bounded_lines`] reads lines (RE-100). Only a shaft's sketch is read
/// for circles, so it is empty for a release a shaft is not cut on
/// ([`supports_shaft_outlines`]).
pub fn scan_sketch_circles(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedArc>> {
    if !supports_shaft_outlines(revit_version) {
        return Ok(BTreeMap::new());
    }
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_circle,
        Copies::Current,
        "sketch_circle",
    )
}

/// A sketch line's curve (RE-96): the first [`BOUNDED_LINE_TAG`] record in
/// its data that reads as a line or as an arc, with both readings where it
/// reads as both. A sketch line's record does not say which it is, so the
/// caller keeps the reading its record box holds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SketchCurve {
    /// The record read as a line.
    pub line: Option<BoundedLine>,
    /// The record read as an arc.
    pub arc: Option<BoundedArc>,
}

/// See [`SketchCurve`].
pub fn first_sketch_curve(data: &[u8]) -> Option<SketchCurve> {
    memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG).find_map(|at| {
        let curve = SketchCurve {
            line: bounded_line_at(data, at),
            arc: arc_values_at(data, at),
        };
        (curve.line.is_some() || curve.arc.is_some()).then_some(curve)
    })
}

/// The curve of each sketch line in `ids` ([`first_sketch_curve`]), read as
/// [`scan_bounded_lines`] reads lines (RE-96). Empty for a release this
/// layout is not measured on ([`supports_sketch_curves`]).
pub fn scan_sketch_curves(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, SketchCurve>> {
    if !supports_sketch_curves(revit_version) {
        return Ok(BTreeMap::new());
    }
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_sketch_curve,
        Copies::Current,
        "sketch_curve",
    )
}

/// The first bounded line in the element data of each id in `ids` (a
/// beam's location line, or a sketch line's segment, RE-50), read from every
/// partition. The data runs from its header to the next header, at most
/// [`BEAM_DATA_WINDOW`] bytes. An id whose copies disagree is dropped; the
/// map is empty for a release this layout is not measured on.
pub fn scan_bounded_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedLine>> {
    if !supports_revit_version(revit_version) {
        return Ok(BTreeMap::new());
    }
    scan_first_bounded_lines(rf, revit_version, ids)
}

/// Releases where a sketch line's own data is measured to hold its segment
/// as a bounded line: 2024 (RE-50) and 2025 (RE-199, the roof sketches of
/// #356's `R2025_Models.zip`, each line's ends on Revit's exported outline);
/// Revit 2026 and 2027 against the same Autodesk samples saved by an earlier
/// release (RE-341).
pub const SKETCH_LINE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025, 2026, 2027];

/// Each sketch line's segment, read as [`scan_bounded_lines`] reads a beam's
/// line except that a line whose copies disagree takes its current
/// partition's (RE-216, #824), on the releases in
/// [`SKETCH_LINE_SUPPORTED_REVIT_VERSIONS`]; empty on others.
pub fn scan_sketch_bounded_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedLine>> {
    if !SKETCH_LINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Ok(BTreeMap::new());
    }
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_bounded_line,
        Copies::Current,
        "sketch_line",
    )
}

/// Releases where a sketch line's B-spline ([`BoundedSpline`]) is measured
/// (RE-211).
pub const SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2025];

/// The most control points a [`BoundedSpline`] is read with.
pub const MAX_SPLINE_CONTROL_POINTS: usize = 4096;

/// A B-spline curve record (RE-211): after the [`BOUNDED_LINE_TAG`], as an
/// arc's is (RE-96),
///
/// ```text
/// +4   f64           start parameter
/// +12  f64           end parameter
/// +20  u32           degree p
/// +24  u8 × 2        flags (0 on the spline measured)
/// +26  u32           n, the number of control points
/// +30  n × f64 × 4   each control point's weight, then x, y, z, model feet
///      u32           n + p + 1, the number of knots
///      f64 × (n+p+1) the knots, rising
/// ```
///
/// Measured on the sketch of `11_RoofByExtrusion` in puzzbobb's R2025
/// models: a cubic with 6 control points, every weight 1, whose curve
/// Revit's export of the roof draws its top along.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundedSpline {
    pub start_parameter: f64,
    pub end_parameter: f64,
    pub degree: usize,
    /// Each control point as `[weight, x, y, z]`.
    pub control_points: Vec<[f64; 4]>,
    pub knots: Vec<f64>,
}

impl BoundedSpline {
    /// The point at `parameter`, clamped to the curve's domain (Cox-de Boor,
    /// rational); `None` where the knots and control points do not fit the
    /// degree.
    pub fn point(&self, parameter: f64) -> Option<[f64; 3]> {
        let p = self.degree;
        let n = self.control_points.len();
        if n <= p || self.knots.len() != n + p + 1 {
            return None;
        }
        let knot = |i: usize| self.knots.get(i).copied();
        let (low, high) = (knot(p)?, knot(n)?);
        let u = parameter.clamp(low, high);
        let span = if u >= high {
            n - 1
        } else {
            (p..n).rev().find(|&i| knot(i).is_some_and(|k| k <= u))?
        };
        let mut basis = vec![0.0; p + 1];
        let mut left = vec![0.0; p + 1];
        let mut right = vec![0.0; p + 1];
        basis[0] = 1.0;
        for j in 1..=p {
            left[j] = u - knot(span + 1 - j)?;
            right[j] = knot(span + j)? - u;
            let mut saved = 0.0;
            for r in 0..j {
                let denominator = right[r + 1] + left[j - r];
                let temp = if denominator == 0.0 {
                    0.0
                } else {
                    basis[r] / denominator
                };
                basis[r] = saved + right[r + 1] * temp;
                saved = left[j - r] * temp;
            }
            basis[j] = saved;
        }
        let mut sum = [0.0; 4];
        for (k, b) in basis.iter().enumerate() {
            let [w, x, y, z] = *self.control_points.get(span - p + k)?;
            sum[0] += b * w;
            sum[1] += b * w * x;
            sum[2] += b * w * y;
            sum[3] += b * w * z;
        }
        (sum[0] != 0.0).then(|| [sum[1] / sum[0], sum[2] / sum[0], sum[3] / sum[0]])
    }

    /// The curve from its start to its end parameter as `segments` chords'
    /// `segments + 1` points.
    pub fn points(&self, segments: usize) -> Option<Vec<[f64; 3]>> {
        let segments = segments.max(1);
        (0..=segments)
            .map(|i| {
                let t = i as f64 / segments as f64;
                self.point(self.start_parameter + (self.end_parameter - self.start_parameter) * t)
            })
            .collect()
    }
}

/// The B-spline whose tag starts at `at`, where its values are finite, its
/// degree 1 to 9, its control points more than its degree and at most
/// [`MAX_SPLINE_CONTROL_POINTS`], its weights positive, its knots as many as
/// the control points and degree need and rising, and its parameters within
/// them.
pub fn bounded_spline_at(buf: &[u8], at: usize) -> Option<BoundedSpline> {
    if buf.get(at..at.checked_add(4)?)? != BOUNDED_LINE_TAG {
        return None;
    }
    let start_parameter = le_bytes::f64_at(buf, at.checked_add(4)?)?;
    let end_parameter = le_bytes::f64_at(buf, at.checked_add(12)?)?;
    let degree = usize::try_from(le_bytes::u32_at(buf, at.checked_add(20)?)?).ok()?;
    let count = usize::try_from(le_bytes::u32_at(buf, at.checked_add(26)?)?).ok()?;
    if !(1..=9).contains(&degree) || count <= degree || count > MAX_SPLINE_CONTROL_POINTS {
        return None;
    }
    let points_at = at.checked_add(30)?;
    let control_points = (0..count)
        .map(|i| {
            let base = points_at.checked_add(32 * i)?;
            let mut point = [0.0; 4];
            for (k, value) in point.iter_mut().enumerate() {
                *value = le_bytes::f64_at(buf, base.checked_add(8 * k)?)?;
            }
            (point.iter().all(|v| v.is_finite()) && point[0] > 0.0).then_some(point)
        })
        .collect::<Option<Vec<_>>>()?;
    let knots_at = points_at.checked_add(32 * count)?;
    let knot_count = usize::try_from(le_bytes::u32_at(buf, knots_at)?).ok()?;
    if knot_count != count + degree + 1 {
        return None;
    }
    let knots = (0..knot_count)
        .map(|i| le_bytes::f64_at(buf, knots_at.checked_add(4 + 8 * i)?))
        .collect::<Option<Vec<_>>>()?;
    let rising = knots.windows(2).all(|pair| pair[0] <= pair[1]);
    let (low, high) = (knots[degree], knots[count]);
    if !(knots.iter().all(|k| k.is_finite())
        && rising
        && low < high
        && start_parameter.is_finite()
        && end_parameter.is_finite()
        && low <= start_parameter
        && start_parameter < end_parameter
        && end_parameter <= high)
    {
        return None;
    }
    Some(BoundedSpline {
        start_parameter,
        end_parameter,
        degree,
        control_points,
        knots,
    })
}

/// The first [`BoundedSpline`] in `data`.
pub fn first_bounded_spline(data: &[u8]) -> Option<BoundedSpline> {
    memchr::memmem::find_iter(data, &BOUNDED_LINE_TAG).find_map(|at| bounded_spline_at(data, at))
}

/// Each sketch line's B-spline ([`first_bounded_spline`]), read as
/// [`scan_bounded_lines`] reads lines, on the releases in
/// [`SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS`]; empty on others.
pub fn scan_sketch_splines(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedSpline>> {
    if !SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version) {
        return Ok(BTreeMap::new());
    }
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_bounded_spline,
        Copies::Current,
        "sketch_spline",
    )
}

/// [`scan_bounded_lines`] without its release gate, for callers that
/// measured the layout on other releases (a wall's location line on 2025,
/// RE-55). Empty where the element-data header is not known.
pub fn scan_first_bounded_lines(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedLine>> {
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_bounded_line,
        Copies::Agreeing,
        "bounded_line",
    )
}

/// The arc in the element data of each id in `ids` whose first curve record
/// is one ([`first_bounded_arc`]), read as [`scan_first_bounded_lines`]
/// reads lines (RE-75).
pub fn scan_first_bounded_arcs(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, BoundedArc>> {
    scan_first_records(
        rf,
        revit_version,
        ids,
        first_bounded_arc,
        Copies::Agreeing,
        "bounded_arc",
    )
}

/// Which of an element's data copies [`scan_first_records`] takes where they
/// disagree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Copies {
    /// None: the element is dropped.
    Agreeing,
    /// Its current partition's (RE-216, [`crate::native_document`]): a
    /// sketch line's (#824). Walls and beams keep [`Copies::Agreeing`]: on
    /// Core Interior 7 of the 41 walls whose copies disagree are drawn
    /// 0.333 ft off Revit's ends from their current copy's line, where
    /// their record box draws them exactly.
    Current,
}

/// Every copy of each id's element data in `ids`, from every partition: the
/// partitions whose data names the id, and what `read` finds in each copy.
type Copied<T> = (
    BTreeMap<u32, Vec<(u32, T)>>,
    BTreeMap<u32, BTreeSet<u32>>,
    CopyOffsets,
);

/// Where each copy in [`Copied`] starts, in the same order: the offset of its
/// element-data header in the partition inflated (#1360). Empty unless the
/// caller asks for them.
type CopyOffsets = BTreeMap<u32, Vec<usize>>;

fn collect_copies<T>(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
    read: fn(&[u8]) -> Option<T>,
    with_offsets: bool,
) -> Result<Copied<T>> {
    let Some(layout) = crate::partition_names::element_data_layout(revit_version) else {
        return Ok((BTreeMap::new(), BTreeMap::new(), BTreeMap::new()));
    };
    let header = layout.header;
    let mut found: BTreeMap<u32, Vec<(u32, T)>> = BTreeMap::new();
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    let mut offsets: CopyOffsets = BTreeMap::new();
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
            let Some(id) = layout.id_at(buf, id_at) else {
                continue;
            };
            if !ids.contains(&id) {
                continue;
            }
            holders.entry(id).or_default().insert(partition);
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(BEAM_DATA_WINDOW))
                .min(buf.len());
            let Some(line) = buf.get(id_at + 8..end).and_then(read) else {
                continue;
            };
            found.entry(id).or_default().push((partition, line));
            if with_offsets {
                offsets.entry(id).or_default().push(hit);
            }
        }
    }
    Ok((found, holders, offsets))
}

/// What [`scan_bounded_lines`] finds for each id in `ids`, without choosing
/// among copies: the partitions whose element data names the id, and the
/// line each copy holds, for a probe of why a beam keeps its box (#1247).
/// Empty for a release this layout is not measured on.
pub fn line_copies(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, LineCopies>> {
    if !supports_revit_version(revit_version) {
        return Ok(BTreeMap::new());
    }
    let (mut found, holders, _) =
        collect_copies(rf, revit_version, ids, first_bounded_line, false)?;
    Ok(holders
        .into_iter()
        .map(|(id, partitions)| {
            let lines = found.remove(&id).unwrap_or_default();
            (id, LineCopies { partitions, lines })
        })
        .collect())
}

/// Each copy's partition and every line it holds ([`all_line_copies`]).
pub type AllLineCopies = BTreeMap<u32, Vec<(u32, Vec<BoundedLine>)>>;

/// What [`all_bounded_lines`] finds in each copy of each id's element data in
/// `ids`, without choosing among copies (RE-328). Empty for a release this
/// layout is not measured on.
pub fn all_line_copies(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
) -> Result<AllLineCopies> {
    if !supports_revit_version(revit_version) {
        return Ok(BTreeMap::new());
    }
    let (found, _, _) = collect_copies(rf, revit_version, ids, all_bounded_lines, false)?;
    Ok(found)
}

/// An element's copies of its element data ([`line_copies`]).
#[derive(Debug, Clone)]
pub struct LineCopies {
    /// The partitions whose element data names the element.
    pub partitions: BTreeSet<u32>,
    /// Each copy's partition and the line it holds, where it holds one.
    pub lines: Vec<(u32, BoundedLine)>,
}

/// What `read` finds in the element data of each id in `ids`, from every
/// partition. An id whose copies disagree is dropped, or takes its current
/// partition's copy where `copies` says so; one whose current copy cannot be
/// told, or holds copies that disagree, is dropped. While the handle records
/// value sources (#1360), the copy taken is noted for each id under `reader`:
/// the lowest-numbered partition's where the copies agree, the current
/// partition's where they do not.
fn scan_first_records<T: PartialEq>(
    rf: &mut RevitFile,
    revit_version: u32,
    ids: &BTreeSet<u32>,
    read: fn(&[u8]) -> Option<T>,
    copies: Copies,
    reader: &'static str,
) -> Result<BTreeMap<u32, T>> {
    let recording = rf.recording_value_sources();
    let (found, holders, offsets) = collect_copies(rf, revit_version, ids, read, recording)?;
    // RE-216: where an element's copies in different partitions disagree,
    // the copy taken is its current partition's (#824).
    let disagreeing: BTreeMap<u32, BTreeSet<u32>> = found
        .iter()
        .filter(|(_, found)| found.iter().any(|(_, line)| *line != found[0].1))
        .filter_map(|(id, _)| Some((*id, holders.get(id)?.clone())))
        .collect();
    let current_partitions = match copies {
        Copies::Current => crate::native_document::current_partitions(rf, &disagreeing),
        Copies::Agreeing => BTreeMap::new(),
    };
    let mut noted: Vec<(u32, ValueSource)> = Vec::new();
    let picked: BTreeMap<u32, T> = found
        .into_iter()
        .filter_map(|(id, copies)| {
            let held = holders.get(&id).map_or(0, BTreeSet::len);
            let offset_of = |index: usize| offsets.get(&id).and_then(|at| at.get(index)).copied();
            if !disagreeing.contains_key(&id) {
                let partitions: BTreeSet<u32> = if recording {
                    copies.iter().map(|(partition, _)| *partition).collect()
                } else {
                    BTreeSet::new()
                };
                let (partition, line) = copies.into_iter().next()?;
                if recording {
                    let source = ValueSource::in_partition(
                        reader,
                        CopyRule::Agreeing,
                        held,
                        partition,
                        offset_of(0),
                    )
                    .with_partitions(partitions.into_iter().collect());
                    noted.push((id, source));
                }
                return Some((id, line));
            }
            let current = *current_partitions.get(&id)?;
            let mut in_current = copies
                .into_iter()
                .enumerate()
                .filter(|(_, (p, _))| *p == current);
            let (index, (_, pick)) = in_current.next()?;
            if !in_current.all(|(_, (_, line))| line == pick) {
                return None;
            }
            if recording {
                let source = ValueSource::in_partition(
                    reader,
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

/// How closely a beam's body must be as wide and as deep as its type's I
/// section, and its line lie on its record box's top, feet (RE-103).
pub const I_SECTION_TOLERANCE_FEET: f64 = 0.01;

/// The body of a beam whose type is an I section `section` (RE-103): its
/// body along its line where that is as wide and as deep as the section.
/// A level beam set down from its line has a record box that runs from the
/// line, on the box's top, down to the steel's bottom: where the box is as
/// wide as the section and deeper, and both ends of the line lie on its
/// top, the section sits on the box's bottom. Otherwise `None`.
pub fn i_section_body(
    body: BeamBody,
    section: &crate::partition_type_parameters::ISection,
    bbox: [f64; 6],
    start: [f64; 3],
    end: [f64; 3],
) -> Option<BeamBody> {
    let near = |a: f64, b: f64| (a - b).abs() <= I_SECTION_TOLERANCE_FEET;
    if !near(body.width_feet, section.width_feet) {
        return None;
    }
    if near(body.depth_feet, section.depth_feet) {
        return Some(body);
    }
    let set_down = body.is_horizontal()
        && body.depth_feet > section.depth_feet
        && near(start[2], bbox[5])
        && near(end[2], bbox[5]);
    set_down.then(|| BeamBody {
        centre: [
            body.centre[0],
            body.centre[1],
            bbox[2] + section.depth_feet / 2.0,
        ],
        depth_feet: section.depth_feet,
        ..body
    })
}

/// The box along the line from `start` to `end` whose axis-aligned extent is
/// the record box `bbox` (`[min x, min y, min z, max x, max y, max z]`, model
/// feet), or `None` when the line leaves the box, is too steep for a
/// horizontal width axis, or no such box reproduces the box's extents
/// ([`beam_check`] says which).
pub fn beam_body(bbox: [f64; 6], start: [f64; 3], end: [f64; 3]) -> Option<BeamBody> {
    beam_check(bbox, start, end).ok()
}

/// Why [`beam_check`] declines a line (RE-328), in the order it tests them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeamReject {
    /// A value of the box or the line is not finite.
    NotFinite,
    /// The box's low bound on some axis is above its high one.
    InvertedBox,
    /// An end lies outside the box by more than [`END_TOLERANCE_FEET`]: the
    /// greatest distance past a face on any axis, feet.
    EndOutside { feet: f64 },
    /// The line has no length.
    ZeroLength,
    /// The line leaves less than [`MIN_PLAN_SHARE`] of its length in plan.
    TooSteep,
    /// The solved width or depth is not positive.
    NoSection,
    /// The plan extent the solve does not use is `feet` off the solved box's,
    /// more than [`CLOSURE_TOLERANCE_FEET`].
    OpenClosure { feet: f64 },
}

/// How far `point` lies outside `bbox` on each axis, feet, signed: negative
/// below the box's low bound, positive above its high one, 0 inside it.
pub fn box_overshoot(bbox: [f64; 6], point: [f64; 3]) -> [f64; 3] {
    [0, 1, 2].map(|axis| {
        if point[axis] < bbox[axis] {
            point[axis] - bbox[axis]
        } else if point[axis] > bbox[axis + 3] {
            point[axis] - bbox[axis + 3]
        } else {
            0.0
        }
    })
}

/// The greatest distance `point` lies past a face of `bbox` on any axis,
/// feet; 0 inside the box ([`box_overshoot`]).
pub fn box_overshoot_feet(bbox: [f64; 6], point: [f64; 3]) -> f64 {
    box_overshoot(bbox, point)
        .iter()
        .fold(0.0, |most, v| f64::max(most, v.abs()))
}

/// [`beam_body`], saying why it declines a line (RE-328).
pub fn beam_check(bbox: [f64; 6], start: [f64; 3], end: [f64; 3]) -> Result<BeamBody, BeamReject> {
    if !bbox.iter().chain(&start).chain(&end).all(|v| v.is_finite()) {
        return Err(BeamReject::NotFinite);
    }
    if (0..3).any(|axis| bbox[axis] > bbox[axis + 3]) {
        return Err(BeamReject::InvertedBox);
    }
    let outside = f64::max(
        box_overshoot_feet(bbox, start),
        box_overshoot_feet(bbox, end),
    );
    if outside > END_TOLERANCE_FEET {
        return Err(BeamReject::EndOutside { feet: outside });
    }
    beam_section_fit(bbox, start, end)
}

/// [`beam_check`] without its test that both ends lie in the box: the box
/// along the line whose extents reproduce the record box's, where the line is
/// level enough, has a length and the plan extent the solve does not use
/// closes (RE-328 measures what the ends test alone keeps from a body).
pub fn beam_section_fit(
    bbox: [f64; 6],
    start: [f64; 3],
    end: [f64; 3],
) -> Result<BeamBody, BeamReject> {
    if !bbox.iter().chain(&start).chain(&end).all(|v| v.is_finite()) {
        return Err(BeamReject::NotFinite);
    }
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
    if length <= 0.0 {
        return Err(BeamReject::ZeroLength);
    }
    let d = [delta[0] / length, delta[1] / length, delta[2] / length];
    let plan = d[0].hypot(d[1]);
    if plan < MIN_PLAN_SHARE {
        return Err(BeamReject::TooSteep);
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
    if !(width > 0.0 && depth > 0.0) {
        return Err(BeamReject::NoSection);
    }
    let miss = (closure - extent[check]).abs();
    if miss > CLOSURE_TOLERANCE_FEET {
        return Err(BeamReject::OpenClosure { feet: miss });
    }
    Ok(BeamBody {
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

/// The box along the unit `direction` whose axis-aligned extent is the record
/// box `bbox`, its length free: `[length, width, depth]` in feet (RE-328).
/// [`beam_section_fit`] takes the length from the line and checks one plan
/// extent; this takes all three from the three extents, so nothing checks it:
/// a box that is not a box along the line (end cuts square to something
/// else) gets a section that is not the beam's. `None` for a direction with
/// less than [`MIN_PLAN_SHARE`] of its length in plan, for a singular solve
/// (a level line at 45 degrees to the axes) and for a non-positive result.
pub fn box_along(bbox: [f64; 6], direction: [f64; 3]) -> Option<[f64; 3]> {
    if !bbox.iter().chain(&direction).all(|v| v.is_finite()) {
        return None;
    }
    let d = direction;
    let plan = d[0].hypot(d[1]);
    if plan < MIN_PLAN_SHARE {
        return None;
    }
    let u = [-d[1] / plan, d[0] / plan, 0.0];
    let w = [-d[2] * d[0] / plan, -d[2] * d[1] / plan, plan];
    let a = [
        [d[0].abs(), u[0].abs(), w[0].abs()],
        [d[1].abs(), u[1].abs(), w[1].abs()],
        [d[2].abs(), u[2].abs(), w[2].abs()],
    ];
    let extent = [bbox[3] - bbox[0], bbox[4] - bbox[1], bbox[5] - bbox[2]];
    let det = |m: [[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let whole = det(a);
    if whole.abs() < 1e-6 {
        return None;
    }
    let solved = [0, 1, 2].map(|column| {
        let mut m = a;
        for (row, value) in m.iter_mut().zip(extent) {
            row[column] = value;
        }
        det(m) / whole
    });
    solved
        .iter()
        .all(|v| v.is_finite() && *v > 0.0)
        .then_some(solved)
}

fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The least `|cos 2θ|` of the plan angle θ of a level line that
/// [`beam_body_in_box`] solves, `cos 50°`: a line within 25 degrees of a model
/// axis. The free-length solve ([`box_along`]) divides by it, and it is 0 at
/// 45 degrees. The beams it draws were measured on a model axis and 24.5
/// degrees off one (0.656); wider angles are unmeasured and stay boxes.
pub const MIN_PLAN_CONDITION: f64 = 0.6427876096865394;

/// The body of a level beam whose line lies inside its record box but is
/// shorter than the box, where [`beam_check`] declines it for a section that
/// does not close (RE-328): the box along the line whose extents are the
/// record box's, its length the box's own ([`box_along`]).
///
/// The length is the record box's, not the beam's: the body can run past
/// Revit's beam at its ends. On Autodesk's 2024 samples all 34 beams
/// declined for that reason on a level line are longer in the box than in
/// the line, by at most a foot. On the 25 whose saved mesh decodes whole
/// the section across the line is the mesh's to 0.01 ft on all 25, the
/// mesh's ends are the body's on 16, and on 9 the mesh ends inside the body
/// at one end or both, by more than 0.01 ft and at most 1 ft (RE-328). Only
/// the caller knows the Cross-Section Rotation (`BEAM_ROTATION_FIELD` in
/// `partition_schema_mvp`), which must be exactly 0: a rolled section's
/// extents across the line are wider than the section, and the 16 sloped,
/// rolled beams measured are smaller than the box along their line. `None`
/// for any other line the check declines, for a line that is sloped, for a
/// plan angle more than 25 degrees off a model axis ([`MIN_PLAN_CONDITION`],
/// unmeasured), for a box no longer than the line, and for a line that does
/// not lie inside the body.
pub fn beam_body_in_box(bbox: [f64; 6], start: [f64; 3], end: [f64; 3]) -> Option<BeamBody> {
    if !matches!(
        beam_check(bbox, start, end),
        Err(BeamReject::OpenClosure { .. })
    ) {
        return None;
    }
    let delta = [end[0] - start[0], end[1] - start[1], end[2] - start[2]];
    let length = dot3(delta, delta).sqrt();
    let d = [delta[0] / length, delta[1] / length, delta[2] / length];
    if d[2].abs() > HORIZONTAL_EPS || (d[0] * d[0] - d[1] * d[1]).abs() < MIN_PLAN_CONDITION {
        return None;
    }
    let [box_length, width, depth] = box_along(bbox, d)?;
    if box_length <= length {
        return None;
    }
    let centre = [
        (bbox[0] + bbox[3]) / 2.0,
        (bbox[1] + bbox[4]) / 2.0,
        (bbox[2] + bbox[5]) / 2.0,
    ];
    let across = [-d[1], d[0], 0.0];
    let holds = |point: [f64; 3]| {
        let rel = [
            point[0] - centre[0],
            point[1] - centre[1],
            point[2] - centre[2],
        ];
        dot3(rel, d).abs() <= box_length / 2.0 + CLOSURE_TOLERANCE_FEET
            && dot3(rel, across).abs() <= width / 2.0 + CLOSURE_TOLERANCE_FEET
            && rel[2].abs() <= depth / 2.0 + CLOSURE_TOLERANCE_FEET
    };
    (holds(start) && holds(end)).then_some(BeamBody {
        centre,
        direction: d,
        length_feet: box_length,
        width_feet: width,
        depth_feet: depth,
    })
}
