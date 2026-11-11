//! A family's extrusion and revolution profiles from its own records (#156).
//!
//! A family file's elements are the plain groups of its own partitions,
//! framed as a project's are (RE-280); `native_document` reads a
//! project through an index of creation episodes that refuses every family
//! file ("invalid increment episode range"), so [`open`] and [`walk`] read
//! the groups without it. A loaded family's document inside a project is the
//! same records in keyed blocks (`native_family_forms`).
//!
//! The chain Discussion #112 (STE1200's team) reports, sketch curve records
//! to closed loops to extrusion parameters to solids, reads by member name
//! from the records of an `ExtrusionElem`:
//!
//! - its `ExtrusionElemExtrusionHelper` names the sketch (`m_sketchId`);
//! - the sketch's curves are the `CurveElem` records whose `SketchMembership`
//!   names it, with a driven `GLine` or `GArc` (a line's origin,
//!   direction and parameter range; an arc's centre, axes, radius and angle
//!   range) in the sketch plane's own coordinates;
//! - [`assemble_loops`] chains them, ends to ends, into closed loops, and
//!   [`profile`] sorts the loops into outlines and holes by nesting;
//! - the `SketchPlane` the curves name holds the plane's frame (`Trf`:
//!   `m_3x3` and `m_or`, with `m_flipZ`);
//! - the extrusion's two `Double` parameters, -1001800 and -1001801, are its
//!   extent along the plane's normal.
//!
//! The extrusion record also stores the loops it was last built from
//! (`m_pCurveLoops`), which [`FormDocument::extrusion`] compares with the
//! sketch's curves.
//!
//! [`FormDocument::sketch_curves`] resolves the saved curve-driver links for
//! both forms, excluding auxiliary centre markers. [`FormDocument::revolution`]
//! separates the marked axis, reads the sweep angles, and projects the saved
//! family-coordinate curves into a radial/axial profile (RE-391/RE-392).

mod revolution;
pub use revolution::{Revolution, RevolutionMesh};

use crate::le_bytes;
use crate::native_parameters::{GraphObject, ObjectGraph, pointer_offset};
use crate::{RevitFile, native_document, native_saved_mesh, native_segments};
use anyhow::{Result, ensure};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

/// The extrusion's `Double` parameter that is one bound of its extent along
/// the sketch plane's normal: the lower one on every extrusion measured.
pub const START_PARAMETER: i64 = -1_001_800;
/// The other bound: the upper one on every extrusion measured.
pub const END_PARAMETER: i64 = -1_001_801;
/// The `Int` parameter that shows or hides a form (`IS_VISIBLE_PARAM`).
const VISIBLE_PARAMETER: i64 = -1_006_205;

/// How close two curve ends must be to join, feet.
const TOLERANCE: f64 = 1e-6;
/// How far from its plane a sketch curve may lie, feet.
const PLANE_TOLERANCE: f64 = 1e-5;
/// How far apart the ends of two sketch curves may be to join. The sketches
/// whose chains stayed open at [`TOLERANCE`] have gaps of 1.4e-5 to 6.5e-5.
const JOIN_TOLERANCE: f64 = 1e-4;
/// The most triangles a mesh may have to be compared with solids.
const MAX_MESH_TRIANGLES: usize = 50_000;
/// How many point-to-triangle distances a comparison may take.
const MESH_PAIR_BUDGET: usize = 2_000_000;
/// The sagitta of the chords Revit's saved meshes tessellate arcs with, feet.
const SAGITTA: f64 = 0.001;
/// What [`MeshAgreement::volume_agrees`] allows beyond the chord loss, as a
/// fraction of the mesh's volume: rounding in the mesh's coordinates.
const VOLUME_MARGIN: f64 = 1e-4;
const MAX_LOOPS: usize = 256;
/// The most curves a sketch may have to be chained: the chaining compares
/// every open end with every unused curve.
const MAX_CURVES: usize = 4_096;
const MAX_POLYGON_VERTICES: usize = 20_000;
/// The most curves of a sketch and of its stored loops that are compared with
/// one another, which takes time in their product.
const MAX_COMPARED_CURVES: usize = 256;
/// The largest angle an arc may start or end at, radians: the angles stored
/// are within a few turns.
const MAX_ARC_ANGLE: f64 = 1e6;
/// The angle an arc is drawn as a chord of when a polygon is needed.
const ARC_STEP: f64 = PI / 32.0;

type V2 = [f64; 2];
type V3 = [f64; 3];

/// How many chords an arc from `start` to `end` is drawn as.
fn arc_steps(start: f64, end: f64) -> usize {
    (((end - start).abs() / ARC_STEP).ceil() as usize).clamp(1, 4096)
}

/// What a family file's records are read with.
pub struct Document {
    /// The release the file's layout is, 2015 to 2027.
    pub release: u32,
    /// The file's own `Formats/Latest`.
    pub registry: crate::schema_registry::Registry,
    /// The names of its `Partitions/NN` streams, in name order.
    pub streams: Vec<String>,
}

impl std::fmt::Debug for Document {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Document")
            .field("release", &self.release)
            .field("classes", &self.registry.classes.len())
            .field("streams", &self.streams)
            .finish()
    }
}

/// A record of a plain group, borrowed for the length of the callback.
pub struct Entry<'a> {
    pub id: u64,
    pub channel: u64,
    pub stream: &'a str,
    /// Body start in the inflated partition, when the group is a contiguous
    /// span of it. Reassembled noncontiguous groups have no such offset.
    pub body_offset: Option<usize>,
    /// The group's first marker in the checksum-prepared partition.
    pub group_marker_offset: usize,
    /// Body start relative to the reassembled group, including its framing.
    pub group_body_offset: usize,
    /// The class its body's tag names in the file's schema.
    pub class: Option<&'a str>,
    pub body: &'a [u8],
}

impl std::fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entry")
            .field("id", &self.id)
            .field("channel", &self.channel)
            .field("stream", &self.stream)
            .field("class", &self.class)
            .field("body_bytes", &self.body.len())
            .finish()
    }
}

/// How many groups [`walk`] met.
#[derive(Debug, Default, Clone, Copy)]
pub struct Walked {
    /// Plain groups of the channels asked for.
    pub groups: usize,
    /// Keyed groups (a loaded family's document inside a project), skipped.
    pub keyed_groups: usize,
    /// Plain groups whose records did not frame, skipped.
    pub unframed_groups: usize,
    /// Records handed to the callback.
    pub records: usize,
}

/// The schema and partitions of a family file. Errs outside the releases the
/// native record framing is measured on, or where the schema does not parse.
pub fn open(file: &mut RevitFile) -> Result<Document> {
    let release = file.layout_release()?;
    ensure!(
        native_document::NATIVE_RELEASES.contains(&release),
        "native record framing is unvalidated for Revit {release}"
    );
    let registry =
        crate::schema_registry::parse(&native_document::read_single(file, "Formats/Latest")?)?;
    let mut streams: Vec<String> = file
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
        .cloned()
        .collect();
    streams.sort();
    Ok(Document {
        release,
        registry,
        streams,
    })
}

/// Every record of the plain groups of `channels` in `document`'s
/// partitions, in stream order, copies in several partitions each handed
/// over. A group that does not frame is counted and skipped.
pub fn walk(
    file: &mut RevitFile,
    document: &Document,
    channels: &[u64],
    mut emit: impl FnMut(&Entry) -> Result<()>,
) -> Result<Walked> {
    let mut walked = Walked::default();
    let budget = native_document::group_budget(file, native_document::DEFAULT_MAX_GROUP_BYTES);
    for name in &document.streams {
        let prepared = file.read_prepared_stream(name)?;
        let inflated = file.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            &document.registry,
            native_document::partition_budget(
                budget,
                native_document::DEFAULT_MAX_GROUP_BYTES,
                inflated.as_deref(),
            ),
            |source, bytes| {
                if source.content_key.is_some() {
                    walked.keyed_groups += 1;
                    return Ok(());
                }
                if !channels.contains(&source.channel) {
                    return Ok(());
                }
                walked.groups += 1;
                let Ok(records) = native_document::group_records(source, bytes, document.release)
                else {
                    walked.unframed_groups += 1;
                    return Ok(());
                };
                for (id, _, start, end) in records {
                    let body = &bytes[start..end];
                    let class = le_bytes::u16_at(body, 0)
                        .and_then(|tag| document.registry.class(tag))
                        .map(|class| class.name.as_str());
                    walked.records += 1;
                    emit(&Entry {
                        id,
                        channel: source.channel,
                        stream: name.as_str(),
                        body_offset: source
                            .inflated_offset
                            .and_then(|offset| offset.checked_add(start)),
                        group_marker_offset: source.first_marker_offset,
                        group_body_offset: start,
                        class,
                        body,
                    })?;
                }
                Ok(())
            },
        )?;
    }
    Ok(walked)
}

fn dist2(a: V2, b: V2) -> f64 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

fn dot3(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn orient(a: V2, b: V2, c: V2) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn pair(value: Option<&Value>) -> Option<V2> {
    let items = value?.as_array()?;
    if items.len() != 2 {
        return None;
    }
    let point = [items[0].as_f64()?, items[1].as_f64()?];
    point.iter().all(|n| n.is_finite()).then_some(point)
}

fn triple(value: Option<&Value>) -> Option<V3> {
    let items = value?.as_array()?;
    if items.len() != 3 {
        return None;
    }
    let point = [items[0].as_f64()?, items[1].as_f64()?, items[2].as_f64()?];
    point.iter().all(|n| n.is_finite()).then_some(point)
}

/// An ElementId field as either shape of the schema saves it: `m_id64` from
/// Revit 2024 and a 4-byte `m_id` before (RE-280).
fn identifier(value: &Value) -> Option<i64> {
    let inner = value.get("m_id")?;
    inner.as_i64().or_else(|| {
        inner
            .get("m_id64")
            .or_else(|| inner.get("m_id"))
            .and_then(Value::as_i64)
    })
}

/// A curve of a sketch, in the sketch plane's own coordinates (feet).
#[derive(Clone, Debug, PartialEq)]
pub enum Curve {
    Line {
        from: V2,
        to: V2,
    },
    /// Points `center + radius (cos t x_axis + sin t y_axis)`, `t` running
    /// from `start` to `end` (backwards where `end` is the smaller).
    Arc {
        center: V2,
        x_axis: V2,
        y_axis: V2,
        radius: f64,
        start: f64,
        end: f64,
    },
}

impl Curve {
    fn arc_point(center: V2, x_axis: V2, y_axis: V2, radius: f64, t: f64) -> V2 {
        let (sin, cos) = t.sin_cos();
        [
            center[0] + radius * (cos * x_axis[0] + sin * y_axis[0]),
            center[1] + radius * (cos * x_axis[1] + sin * y_axis[1]),
        ]
    }

    /// The point a fraction `s` of the way from the start to the end.
    pub fn point_at(&self, s: f64) -> V2 {
        match self {
            Curve::Line { from, to } => [
                from[0] + s * (to[0] - from[0]),
                from[1] + s * (to[1] - from[1]),
            ],
            Curve::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => Self::arc_point(
                *center,
                *x_axis,
                *y_axis,
                *radius,
                start + s * (end - start),
            ),
        }
    }

    pub fn start_point(&self) -> V2 {
        self.point_at(0.0)
    }

    pub fn end_point(&self) -> V2 {
        self.point_at(1.0)
    }

    /// The same curve run the other way.
    pub fn reversed(&self) -> Curve {
        let mut curve = self.clone();
        match &mut curve {
            Curve::Line { from, to } => std::mem::swap(from, to),
            Curve::Arc { start, end, .. } => std::mem::swap(start, end),
        }
        curve
    }

    /// How many points [`Curve::polyline`] gives.
    fn polyline_len(&self) -> usize {
        match self {
            Curve::Line { .. } => 1,
            Curve::Arc { start, end, .. } => arc_steps(*start, *end),
        }
    }

    /// Points along the curve from its start, the end left out.
    fn polyline(&self, out: &mut Vec<V2>) {
        match self {
            Curve::Line { from, .. } => out.push(*from),
            Curve::Arc { start, end, .. } => {
                let steps = arc_steps(*start, *end);
                for k in 0..steps {
                    out.push(self.point_at(k as f64 / steps as f64));
                }
            }
        }
    }

    /// Half the integral of `x dy - y dx` along the curve.
    fn area_term(&self) -> f64 {
        match self {
            Curve::Line { from, to } => 0.5 * (from[0] * to[1] - to[0] * from[1]),
            Curve::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let (p0, p1) = (self.start_point(), self.end_point());
                let turn = x_axis[0] * y_axis[1] - x_axis[1] * y_axis[0];
                0.5 * (center[0] * (p1[1] - p0[1]) - center[1] * (p1[0] - p0[0])
                    + radius * radius * turn * (end - start))
            }
        }
    }

    /// The least and greatest of `d . p` over the curve's points `p`.
    fn range_along(&self, d: V2) -> (f64, f64) {
        match self {
            Curve::Line { from, to } => {
                let a = d[0] * from[0] + d[1] * from[1];
                let b = d[0] * to[0] + d[1] * to[1];
                (a.min(b), a.max(b))
            }
            Curve::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let base = d[0] * center[0] + d[1] * center[1];
                let a = radius * (d[0] * x_axis[0] + d[1] * x_axis[1]);
                let b = radius * (d[0] * y_axis[0] + d[1] * y_axis[1]);
                let at = |t: f64| base + a * t.cos() + b * t.sin();
                let (lo, hi) = (start.min(*end), start.max(*end));
                let (mut low, mut high) = (at(lo).min(at(hi)), at(lo).max(at(hi)));
                let peak = b.atan2(a);
                let first = ((lo - peak) / PI).ceil() as i64;
                let last = ((hi - peak) / PI).floor() as i64;
                for k in first..=last.min(first.saturating_add(4)) {
                    let value = at(peak + k as f64 * PI);
                    low = low.min(value);
                    high = high.max(value);
                }
                (low, high)
            }
        }
    }

    /// The distance from `p` to the curve.
    fn distance(&self, p: V2) -> f64 {
        match self {
            Curve::Line { from, to } => {
                let d = [to[0] - from[0], to[1] - from[1]];
                let length2 = d[0] * d[0] + d[1] * d[1];
                let t = if length2 > 0.0 {
                    (((p[0] - from[0]) * d[0] + (p[1] - from[1]) * d[1]) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                dist2(p, [from[0] + t * d[0], from[1] + t * d[1]])
            }
            Curve::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let rel = [p[0] - center[0], p[1] - center[1]];
                let u = rel[0] * x_axis[0] + rel[1] * x_axis[1];
                let v = rel[0] * y_axis[0] + rel[1] * y_axis[1];
                let (lo, hi) = (start.min(*end), start.max(*end));
                if (v.atan2(u) - lo).rem_euclid(TAU) <= hi - lo {
                    (u.hypot(v) - radius).abs()
                } else {
                    dist2(p, self.start_point()).min(dist2(p, self.end_point()))
                }
            }
        }
    }
}

/// A curve of a sketch as the record stores it, in the family's coordinates
/// (feet), lying in the sketch plane.
#[derive(Clone, Debug, PartialEq)]
pub enum Curve3 {
    Line {
        from: V3,
        to: V3,
    },
    /// Points `center + radius (cos t x_axis + sin t y_axis)`, `t` running
    /// from `start` to `end`.
    Arc {
        center: V3,
        x_axis: V3,
        y_axis: V3,
        radius: f64,
        start: f64,
        end: f64,
    },
}

impl Curve3 {
    /// The point a fraction `s` of the way from the start to the end.
    pub fn point_at(&self, s: f64) -> V3 {
        match self {
            Curve3::Line { from, to } => [
                from[0] + s * (to[0] - from[0]),
                from[1] + s * (to[1] - from[1]),
                from[2] + s * (to[2] - from[2]),
            ],
            Curve3::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let (sin, cos) = (start + s * (end - start)).sin_cos();
                [
                    center[0] + radius * (cos * x_axis[0] + sin * y_axis[0]),
                    center[1] + radius * (cos * x_axis[1] + sin * y_axis[1]),
                    center[2] + radius * (cos * x_axis[2] + sin * y_axis[2]),
                ]
            }
        }
    }

    /// The curve in `frame`'s plane, in the plane's own coordinates. Errs
    /// where a point or an axis of it is off the plane.
    fn to_plane(&self, frame: &Frame) -> Result<Curve, String> {
        let [x, y, z] = frame.axes;
        let o = frame.origin;
        let place = |p: V3| {
            let rel = [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
            ([dot3(rel, x), dot3(rel, y)], dot3(rel, z))
        };
        let direction = |v: V3| ([dot3(v, x), dot3(v, y)], dot3(v, z));
        match self {
            Curve3::Line { from, to } => {
                let ((a, wa), (b, wb)) = (place(*from), place(*to));
                if wa.abs() > PLANE_TOLERANCE || wb.abs() > PLANE_TOLERANCE {
                    return Err("line off the sketch plane".into());
                }
                Ok(Curve::Line { from: a, to: b })
            }
            Curve3::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let (c, wc) = place(*center);
                let ((u, wu), (v, wv)) = (direction(*x_axis), direction(*y_axis));
                if wc.abs() > PLANE_TOLERANCE
                    || wu.abs() > PLANE_TOLERANCE
                    || wv.abs() > PLANE_TOLERANCE
                {
                    return Err("arc off the sketch plane".into());
                }
                Ok(Curve::Arc {
                    center: c,
                    x_axis: u,
                    y_axis: v,
                    radius: *radius,
                    start: *start,
                    end: *end,
                })
            }
        }
    }
}

/// The curve an object of a sketch's graph is: a `GLine` or a `GArc`. Any
/// other class (an ellipse, a spline) is refused by name.
fn read_curve(object: &GraphObject) -> Result<Curve3, String> {
    let fields = &object.fields;
    match object.class_name.as_str() {
        "GLine" => {
            let origin = triple(fields.get("m_origin")).ok_or("line origin")?;
            let direction = triple(fields.get("m_dirVec")).ok_or("line direction")?;
            let range = pair(fields.get("m_endParams")).ok_or("line parameter range")?;
            let at = |t: f64| {
                [
                    origin[0] + direction[0] * t,
                    origin[1] + direction[1] * t,
                    origin[2] + direction[2] * t,
                ]
            };
            Ok(Curve3::Line {
                from: at(range[0]),
                to: at(range[1]),
            })
        }
        "GArc" => {
            let center = triple(fields.get("m_center")).ok_or("arc centre")?;
            let x_axis = triple(fields.get("m_xVec")).ok_or("arc x axis")?;
            let y_axis = triple(fields.get("m_yVec")).ok_or("arc y axis")?;
            let radius = fields
                .get("m_radius")
                .and_then(Value::as_f64)
                .filter(|r| r.is_finite() && *r > 0.0)
                .ok_or("arc radius")?;
            let range = pair(fields.get("m_endParams")).ok_or("arc angle range")?;
            // A circle is stored as an arc whose two angles are equal.
            let (start, end) = if (range[1] - range[0]).abs() < 1e-9 {
                (range[0], range[0] + TAU)
            } else {
                (range[0], range[1])
            };
            let reasonable = start.is_finite()
                && end.is_finite()
                && start.abs() <= MAX_ARC_ANGLE
                && end.abs() <= MAX_ARC_ANGLE
                && (end - start).abs() <= TAU + 1e-9;
            if !reasonable {
                return Err("arc angle range".into());
            }
            let unit = |v: V3| (dot3(v, v) - 1.0).abs() < TOLERANCE;
            if !unit(x_axis) || !unit(y_axis) || dot3(x_axis, y_axis).abs() > TOLERANCE {
                return Err("arc axes not orthonormal".into());
            }
            Ok(Curve3::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            })
        }
        other => Err(format!("unsupported curve class {other}")),
    }
}

/// Curves chained end to end, the last ending where the first starts.
#[derive(Clone, Debug)]
pub struct Loop {
    pub curves: Vec<Curve>,
}

impl Loop {
    /// Signed area: positive where the loop runs counter-clockwise.
    pub fn signed_area(&self) -> f64 {
        self.curves.iter().map(Curve::area_term).sum()
    }

    /// The same loop run the other way.
    pub fn reversed(&self) -> Loop {
        Loop {
            curves: self.curves.iter().rev().map(Curve::reversed).collect(),
        }
    }

    fn polygon_len(&self) -> usize {
        self.curves.iter().map(Curve::polyline_len).sum()
    }

    fn polygon(&self) -> Vec<V2> {
        let mut out = Vec::new();
        for curve in &self.curves {
            curve.polyline(&mut out);
        }
        out
    }
}

/// Chains `curves` into closed loops, each curve used once and run the way
/// its neighbours need. A chain whose ends are within 1e-4 ft of
/// each other is closed by a line between them. Errs where a chain cannot
/// close, or where more than one curve could continue it (a branching
/// sketch is not a profile).
pub fn assemble_loops(curves: &[Curve]) -> Result<Vec<Loop>, String> {
    if curves.len() > MAX_CURVES {
        return Err("too many curves".into());
    }
    let mut used = vec![false; curves.len()];
    let mut loops = Vec::new();
    for (first, seed) in curves.iter().enumerate() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let mut chain = vec![seed.clone()];
        let start = seed.start_point();
        let mut end = seed.end_point();
        while dist2(start, end) > JOIN_TOLERANCE {
            let mut found: Vec<(usize, bool)> = Vec::new();
            for (j, curve) in curves.iter().enumerate() {
                if used[j] {
                    continue;
                }
                if dist2(end, curve.start_point()) <= JOIN_TOLERANCE {
                    found.push((j, false));
                } else if dist2(end, curve.end_point()) <= JOIN_TOLERANCE {
                    found.push((j, true));
                }
            }
            let &[(next, flip)] = found.as_slice() else {
                return Err(if found.is_empty() {
                    "open loop".into()
                } else {
                    "branching sketch".into()
                });
            };
            used[next] = true;
            let curve = if flip {
                curves[next].reversed()
            } else {
                curves[next].clone()
            };
            end = curve.end_point();
            chain.push(curve);
        }
        if dist2(start, end) > 1e-9 {
            chain.push(Curve::Line {
                from: end,
                to: start,
            });
        }
        loops.push(Loop { curves: chain });
    }
    Ok(loops)
}

fn contains(polygon: &[V2], p: V2) -> bool {
    let mut inside = false;
    let mut previous = match polygon.last() {
        Some(point) => *point,
        None => return false,
    };
    for &current in polygon {
        if (current[1] > p[1]) != (previous[1] > p[1])
            && p[0]
                < (previous[0] - current[0]) * (p[1] - current[1]) / (previous[1] - current[1])
                    + current[0]
        {
            inside = !inside;
        }
        previous = current;
    }
    inside
}

fn polygon_box(polygon: &[V2]) -> (V2, V2) {
    let mut low = [f64::INFINITY; 2];
    let mut high = [f64::NEG_INFINITY; 2];
    for point in polygon {
        for (axis, value) in point.iter().enumerate() {
            low[axis] = low[axis].min(*value);
            high[axis] = high[axis].max(*value);
        }
    }
    (low, high)
}

fn polygons_cross(a: &[V2], b: &[V2]) -> bool {
    let ((alo, ahi), (blo, bhi)) = (polygon_box(a), polygon_box(b));
    if alo[0] > bhi[0] || blo[0] > ahi[0] || alo[1] > bhi[1] || blo[1] > ahi[1] {
        return false;
    }
    for (i, &p) in a.iter().enumerate() {
        let q = a[(i + 1) % a.len()];
        for (j, &r) in b.iter().enumerate() {
            let s = b[(j + 1) % b.len()];
            if orient(p, q, r) * orient(p, q, s) < 0.0 && orient(r, s, p) * orient(r, s, q) < 0.0 {
                return true;
            }
        }
    }
    false
}

/// Closed loops sorted into outlines and holes: a loop inside an even
/// number of the others is an outline, inside an odd number a hole.
#[derive(Clone, Debug)]
pub struct Profile {
    pub loops: Vec<Loop>,
    polygons: Vec<Vec<V2>>,
    /// How many other loops each loop lies inside.
    pub depths: Vec<usize>,
    /// The area of the outlines less the holes', square feet.
    pub area: f64,
}

/// A [`Profile`] of `loops`. Errs where none has area, where two cross, or
/// where there are more than the reader bounds.
pub fn profile(loops: Vec<Loop>) -> Result<Profile, String> {
    if loops.is_empty() {
        return Err("no loop".into());
    }
    if loops.len() > MAX_LOOPS {
        return Err("too many loops".into());
    }
    if loops.iter().map(Loop::polygon_len).sum::<usize>() > MAX_POLYGON_VERTICES {
        return Err("too many curve pieces".into());
    }
    let polygons: Vec<Vec<V2>> = loops.iter().map(Loop::polygon).collect();
    let areas: Vec<f64> = loops.iter().map(|l| l.signed_area().abs()).collect();
    if areas.iter().any(|a| *a < 1e-9) {
        return Err("loop with no area".into());
    }
    for (i, a) in polygons.iter().enumerate() {
        for b in &polygons[i + 1..] {
            if polygons_cross(a, b) {
                return Err("loops cross".into());
            }
        }
    }
    let depths: Vec<usize> = polygons
        .iter()
        .enumerate()
        .map(|(i, polygon)| {
            polygons
                .iter()
                .enumerate()
                .filter(|(j, other)| *j != i && contains(other, polygon[0]))
                .count()
        })
        .collect();
    let area: f64 = areas
        .iter()
        .zip(&depths)
        .map(|(a, depth)| if *depth & 1 == 0 { *a } else { -*a })
        .sum();
    Ok(Profile {
        loops,
        polygons,
        depths,
        area,
    })
}

impl Profile {
    /// The distance from `p` to the nearest curve, negative where `p` is in
    /// the profile.
    pub fn signed_distance(&self, p: V2) -> f64 {
        let nearest = self
            .loops
            .iter()
            .flat_map(|l| l.curves.iter())
            .map(|c| c.distance(p))
            .fold(f64::INFINITY, f64::min);
        let inside = self.polygons.iter().filter(|g| contains(g, p)).count() % 2 == 1;
        if inside { -nearest } else { nearest }
    }

    /// For each loop, the loop it lies directly inside: the container with
    /// the most containers of its own. `None` for a loop inside no other.
    pub fn parents(&self) -> Vec<Option<usize>> {
        self.polygons
            .iter()
            .enumerate()
            .map(|(i, polygon)| {
                self.polygons
                    .iter()
                    .enumerate()
                    .filter(|(j, other)| *j != i && contains(other, polygon[0]))
                    .max_by_key(|(j, _)| self.depths[*j])
                    .map(|(j, _)| j)
            })
            .collect()
    }

    /// The total length of the profile's arcs, feet.
    pub fn arc_length(&self) -> f64 {
        self.loops
            .iter()
            .flat_map(|l| l.curves.iter())
            .map(|curve| match curve {
                Curve::Arc {
                    radius, start, end, ..
                } => radius * (end - start).abs(),
                Curve::Line { .. } => 0.0,
            })
            .sum()
    }

    /// The least and greatest of `d . p` over the profile's curves.
    fn range_along(&self, d: V2) -> (f64, f64) {
        let mut low = f64::INFINITY;
        let mut high = f64::NEG_INFINITY;
        for curve in self.loops.iter().flat_map(|l| l.curves.iter()) {
            let (a, b) = curve.range_along(d);
            low = low.min(a);
            high = high.max(b);
        }
        (low, high)
    }
}

/// A sketch plane's placement: an origin and an orthonormal x, y and normal
/// axis, in the family's coordinates.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub origin: V3,
    pub axes: [V3; 3],
}

fn frame_of(
    matrix: [V3; 3],
    origin: V3,
    transposed: bool,
    flipped: bool,
    cross_normal: bool,
) -> Result<Frame, String> {
    let axis = |j: usize| -> V3 {
        if transposed {
            matrix[j]
        } else {
            [matrix[0][j], matrix[1][j], matrix[2][j]]
        }
    };
    let (x, y) = (axis(0), axis(1));
    let mut z = axis(2);
    if cross_normal {
        z = [
            x[1] * y[2] - x[2] * y[1],
            x[2] * y[0] - x[0] * y[2],
            x[0] * y[1] - x[1] * y[0],
        ];
    }
    if flipped {
        z = [-z[0], -z[1], -z[2]];
    }
    let unit = [x, y, z].iter().all(|a| (dot3(*a, *a) - 1.0).abs() < 1e-6);
    let square = dot3(x, y).abs() < 1e-6 && dot3(x, z).abs() < 1e-6 && dot3(y, z).abs() < 1e-6;
    if !(unit && square) {
        return Err("sketch plane frame is not orthonormal".into());
    }
    Ok(Frame {
        origin,
        axes: [x, y, z],
    })
}

/// A profile swept along a plane's normal between two offsets.
#[derive(Clone, Debug)]
pub struct ExtrusionSolid {
    pub profile: Profile,
    pub frame: Frame,
    pub low: f64,
    pub high: f64,
}

impl ExtrusionSolid {
    /// Cubic feet.
    pub fn volume(&self) -> f64 {
        self.profile.area * (self.high - self.low)
    }

    /// The point at `p` in the plane and `t` along the normal.
    pub fn world(&self, p: V2, t: f64) -> V3 {
        let [x, y, z] = self.frame.axes;
        let o = self.frame.origin;
        [
            o[0] + p[0] * x[0] + p[1] * y[0] + t * z[0],
            o[1] + p[0] * x[1] + p[1] * y[1] + t * z[1],
            o[2] + p[0] * x[2] + p[1] * y[2] + t * z[2],
        ]
    }

    /// The solid's least and greatest corner.
    pub fn bounding_box(&self) -> (V3, V3) {
        let mut low = [0.0; 3];
        let mut high = [0.0; 3];
        for (k, (low_k, high_k)) in low.iter_mut().zip(high.iter_mut()).enumerate() {
            let d = [self.frame.axes[0][k], self.frame.axes[1][k]];
            let (lo, hi) = self.profile.range_along(d);
            let z = self.frame.axes[2][k];
            let (t0, t1) = (self.low * z, self.high * z);
            *low_k = self.frame.origin[k] + lo + t0.min(t1);
            *high_k = self.frame.origin[k] + hi + t0.max(t1);
        }
        (low, high)
    }

    /// The distance from `p` to the solid's surface, negative inside it.
    pub fn signed_distance(&self, p: V3) -> f64 {
        let [x, y, z] = self.frame.axes;
        let o = self.frame.origin;
        let rel = [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
        let plane = self.profile.signed_distance([dot3(rel, x), dot3(rel, y)]);
        let t = dot3(rel, z);
        let h = (t - (self.low + self.high) / 2.0).abs() - (self.high - self.low) / 2.0;
        plane.max(h).min(0.0) + plane.max(0.0).hypot(h.max(0.0))
    }

    /// Points on the surface: along every curve at the two ends and the
    /// middle of the extent, and on a grid over each end face.
    pub fn surface_samples(&self, per_curve: usize) -> Vec<V3> {
        let per_curve = per_curve.max(1);
        let mut out = Vec::new();
        let middle = (self.low + self.high) / 2.0;
        for curve in self.profile.loops.iter().flat_map(|l| l.curves.iter()) {
            for k in 0..=per_curve {
                let p = curve.point_at(k as f64 / per_curve as f64);
                for t in [self.low, middle, self.high] {
                    out.push(self.world(p, t));
                }
            }
        }
        let (x_lo, x_hi) = self.profile.range_along([1.0, 0.0]);
        let (y_lo, y_hi) = self.profile.range_along([0.0, 1.0]);
        let n = 16;
        for i in 0..n {
            for j in 0..n {
                let p = [
                    x_lo + (x_hi - x_lo) * (i as f64 + 0.5) / n as f64,
                    y_lo + (y_hi - y_lo) * (j as f64 + 0.5) / n as f64,
                ];
                if self.profile.signed_distance(p) < 0.0 {
                    out.push(self.world(p, self.low));
                    out.push(self.world(p, self.high));
                }
            }
        }
        out
    }
}

/// An extrusion as its records describe it.
#[derive(Clone, Debug)]
pub struct Extrusion {
    pub id: u64,
    /// Whether it is a void.
    pub cutting: bool,
    pub sketch_id: u64,
    /// The `CurveElem` records of the sketch.
    pub curve_ids: Vec<u64>,
    /// The sketch's curves, in the family's coordinates, in `curve_ids`'
    /// order.
    pub curves: Vec<Curve3>,
    pub plane_id: u64,
    /// The sketch plane's `m_flipZ`.
    pub plane_flipped: bool,
    pub matrix: [V3; 3],
    pub origin: V3,
    /// The matrix's determinant: 1 for a rotation, -1 for a mirror.
    pub determinant: f64,
    /// The class of the sketch plane's `m_oPlaneRef`, and its `m_mirror`.
    pub plane_ref: Option<(String, Option<bool>)>,
    /// The extrusion's `m_alwaysRefPlaneNorm`.
    pub always_ref_plane_norm: Option<bool>,
    /// Its stored `Visible` (`IS_VISIBLE_PARAM`, -1006205), where it holds
    /// one: 0 hides the form.
    pub visible: Option<i64>,
    /// The sketch's `m_flipXY` and `m_flipZ`, where its `VarSketch` record
    /// decodes.
    pub sketch_flips: Option<(bool, bool)>,
    /// The cross product of the x and y vectors of the `Plane` the sketch's
    /// `VarSketch` record holds.
    pub sketch_normal: Option<V3>,
    /// The value of [`START_PARAMETER`] and of [`END_PARAMETER`].
    pub start: f64,
    pub end: f64,
    /// Whether the loops the extrusion record stores are the sketch's
    /// curves, curve for curve; `None` where it stores none that read.
    pub stored_loops_agree: Option<bool>,
}

/// How an extrusion's sketch plane places its solid. Every field but the
/// first is a reading of the plane's matrix the probes tried against the
/// saved meshes; [`Convention::COLUMNS`] is the one that holds.
#[derive(Clone, Copy, Debug)]
pub struct Convention {
    /// Take the matrix's rows, not its columns, as the plane's axes.
    pub transposed: bool,
    /// Reverse the normal where the sketch plane's `m_flipZ` is set.
    pub apply_flip: bool,
    /// Take the normal as the cross product of the x and y axes, not the
    /// third axis (they differ when the matrix is a mirror).
    pub cross_normal: bool,
    /// Reverse the normal.
    pub reverse_normal: bool,
}

impl Convention {
    /// The matrix's columns are the plane's x, y and normal axes, and a
    /// flipped plane reverses the normal.
    pub const COLUMNS: Convention = Convention {
        transposed: false,
        apply_flip: true,
        cross_normal: false,
        reverse_normal: false,
    };
}

impl Extrusion {
    /// Whether the sketch plane's matrix has its third column pointing
    /// exactly down (-z, within 0.001), the plane is not flipped and the
    /// extrusion's `m_alwaysRefPlaneNorm` reads false: the 79 forms (of the
    /// 6,172 with a complete saved mesh in Autodesk's sample projects) whose
    /// solids the matrix's normal puts on the wrong side of the plane, and
    /// no other form. [`Extrusion::default_solid`] reverses their normal. A
    /// plane tilted off -z is not tested here: it is read as the matrix
    /// gives it, and an export checks the solid against the saved mesh.
    pub fn down_facing_unreferenced(&self) -> bool {
        self.matrix[2][2] < -0.999
            && !self.plane_flipped
            && self.always_ref_plane_norm == Some(false)
    }

    /// The solid between the two offsets, read as [`Convention::COLUMNS`]
    /// with the normal reversed where [`Extrusion::down_facing_unreferenced`].
    /// Errs as [`Extrusion::solid`] does, for a down-facing plane that is
    /// not flipped whose `m_alwaysRefPlaneNorm` is not read (the rule is
    /// measured on the field's values, not on its absence), and for an
    /// up-facing plane that is flipped: of the 165 flipped planes with a
    /// complete saved mesh, the one up-facing flipped plane that has a mesh
    /// at the head matches without its flip, which no other does.
    pub fn default_solid(&self) -> Result<ExtrusionSolid, String> {
        if self.matrix[2][2] > 0.999 && self.plane_flipped {
            return Err("up-facing sketch plane that is flipped".into());
        }
        if self.matrix[2][2] < -0.999 && !self.plane_flipped && self.always_ref_plane_norm.is_none()
        {
            return Err("down-facing sketch plane whose m_alwaysRefPlaneNorm is not read".into());
        }
        self.solid(Convention {
            reverse_normal: self.down_facing_unreferenced(),
            ..Convention::COLUMNS
        })
    }

    /// The solid between the two offsets, the sketch's curves placed in the
    /// sketch plane's frame and chained into a profile. Errs where the frame
    /// is not orthonormal, a curve is off the plane, or the curves do not
    /// make closed, non-crossing loops.
    pub fn solid(&self, convention: Convention) -> Result<ExtrusionSolid, String> {
        let frame = frame_of(
            self.matrix,
            self.origin,
            convention.transposed,
            (convention.apply_flip && self.plane_flipped) != convention.reverse_normal,
            convention.cross_normal,
        )?;
        let planar = self
            .curves
            .iter()
            .map(|curve| curve.to_plane(&frame))
            .collect::<Result<Vec<Curve>, String>>()?;
        Ok(ExtrusionSolid {
            profile: profile(assemble_loops(&planar)?)?,
            frame,
            low: self.start.min(self.end),
            high: self.start.max(self.end),
        })
    }
}

fn linked_object<'a>(
    graph: &'a ObjectGraph,
    source: usize,
    field: &str,
) -> Result<(usize, &'a GraphObject), String> {
    let pointer = graph
        .objects
        .get(source)
        .and_then(|o| o.fields.get(field))
        .ok_or_else(|| format!("missing {field}"))?;
    let offset = pointer_offset(pointer).ok_or("pointer offset")?;
    if graph
        .edges
        .iter()
        .filter(|e| e.source_object_index == source && e.pointer_offset as u64 == offset)
        .count()
        != 1
    {
        return Err(format!("missing or ambiguous edge for {field}"));
    }
    let target = native_saved_mesh::pointer(graph, source, pointer)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("null {field}"))?;
    Ok((
        target,
        graph
            .objects
            .get(target)
            .ok_or("pointer target out of range")?,
    ))
}

fn curve_driver(graph: &ObjectGraph) -> Result<(usize, &GraphObject), String> {
    let (index, driver) = linked_object(graph, 0, "m_pCurveDriver")?;
    if driver.class_name != "CurveElemDriver" {
        return Err("unsupported curve driver".into());
    }
    Ok((index, driver))
}

fn element_curve(graph: &ObjectGraph) -> Result<Curve3, String> {
    let (index, _) = curve_driver(graph)?;
    let (_, curve) = linked_object(graph, index, "m_pCrv")?;
    read_curve(curve)
}

/// A driven sketch curve shared by extrusion and revolution readers.
/// Coordinates are preserved as stored; each form establishes its own frame.
#[derive(Clone, Debug)]
pub struct SketchCurve {
    pub id: u64,
    pub plane_id: u64,
    pub curve: Curve3,
}

/// The curves of the loops an extrusion record stores: its helper's
/// `m_pCurveLoops`, each a `CurveLoop` of pointers to curves.
fn stored_curves(graph: &ObjectGraph, helper: usize) -> Result<Vec<Curve3>, String> {
    let pointers = graph.objects[helper]
        .fields
        .get("m_pCurveLoops")
        .and_then(Value::as_array)
        .ok_or("no stored curve loops")?;
    let mut curves = Vec::new();
    for pointer in pointers {
        let target = native_saved_mesh::pointer(graph, helper, pointer)
            .map_err(|e| e.to_string())?
            .ok_or("null curve loop")?;
        let members = graph
            .objects
            .get(target)
            .and_then(|object| object.fields.get("m_curves"))
            .and_then(Value::as_array)
            .ok_or("curve loop has no curves")?;
        for member in members {
            let at = native_saved_mesh::pointer(graph, target, member)
                .map_err(|e| e.to_string())?
                .ok_or("null curve")?;
            curves.push(read_curve(
                graph.objects.get(at).ok_or("curve out of range")?,
            )?);
        }
    }
    Ok(curves)
}

fn dist3(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn cross3(a: V3, b: V3) -> V3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl Curve3 {
    /// The distance from `p` to the curve.
    fn distance(&self, p: V3) -> f64 {
        match self {
            Curve3::Line { from, to } => {
                let d = [to[0] - from[0], to[1] - from[1], to[2] - from[2]];
                let rel = [p[0] - from[0], p[1] - from[1], p[2] - from[2]];
                let length2 = dot3(d, d);
                let t = if length2 > 0.0 {
                    (dot3(rel, d) / length2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                dist3(
                    p,
                    [from[0] + t * d[0], from[1] + t * d[1], from[2] + t * d[2]],
                )
            }
            Curve3::Arc {
                center,
                x_axis,
                y_axis,
                radius,
                start,
                end,
            } => {
                let rel = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
                let u = dot3(rel, *x_axis);
                let v = dot3(rel, *y_axis);
                let w = dot3(rel, cross3(*x_axis, *y_axis));
                let (lo, hi) = (start.min(*end), start.max(*end));
                if (v.atan2(u) - lo).rem_euclid(TAU) <= hi - lo {
                    (u.hypot(v) - radius).hypot(w)
                } else {
                    dist3(p, self.point_at(0.0)).min(dist3(p, self.point_at(1.0)))
                }
            }
        }
    }
}

/// Whether every sampled point of `a` lies on `b`.
fn covers(a: &[Curve3], b: &[Curve3]) -> bool {
    a.iter().all(|curve| {
        (0..=8u32).all(|k| {
            let p = curve.point_at(f64::from(k) / 8.0);
            b.iter().any(|other| other.distance(p) <= PLANE_TOLERANCE)
        })
    })
}

/// Whether two sets of curves lie on one another, however they are split.
fn same_curves(a: &[Curve3], b: &[Curve3]) -> bool {
    covers(a, b) && covers(b, a)
}

/// The records of a family's document an extrusion is rebuilt from, by
/// ElementId: its `ExtrusionElem`, `CurveElem` and `SketchPlane` records.
#[derive(Default)]
pub struct FormDocument {
    graphs: BTreeMap<u64, ObjectGraph>,
    /// The `CurveElem` records of each sketch, by the sketch's ElementId.
    sketch_curves: BTreeMap<u64, Vec<u64>>,
    repeated: BTreeSet<u64>,
}

impl std::fmt::Debug for FormDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormDocument")
            .field("records", &self.graphs.len())
            .field("sketches", &self.sketch_curves.len())
            .finish()
    }
}

impl FormDocument {
    /// Keeps `graph` where its root is a record the rebuild reads.
    pub fn insert(&mut self, id: u64, graph: ObjectGraph) {
        let Some(root) = graph.objects.first() else {
            return;
        };
        match root.class_name.as_str() {
            "CurveElem" => {
                let sketch = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "SketchMembership")
                    .and_then(|object| object.fields.get("m_groupId"))
                    .and_then(identifier)
                    .and_then(|n| u64::try_from(n).ok());
                if let Some(sketch) = sketch.filter(|_| !self.graphs.contains_key(&id)) {
                    self.sketch_curves.entry(sketch).or_default().push(id);
                }
            }
            "ExtrusionElem" | "RevolutionElem" | "SketchPlane" | "VarSketch" => {}
            _ => return,
        }
        if self.graphs.contains_key(&id) {
            self.repeated.insert(id);
        }
        self.graphs.insert(id, graph);
    }

    /// Read a sketch's driven lines/arcs without applying a coordinate transform.
    /// Auxiliary geometry (including an arc's centre-marker line) is excluded.
    pub fn sketch_curves(&self, sketch_id: u64) -> Result<Vec<SketchCurve>, String> {
        let ids = self
            .sketch_curves
            .get(&sketch_id)
            .ok_or("no curve record names the sketch")?;
        if ids.len() > MAX_CURVES {
            return Err("too many curves".into());
        }
        ids.iter()
            .map(|id| {
                let graph = self.graphs.get(id).ok_or("curve record absent")?;
                let plane_id = graph
                    .objects
                    .first()
                    .and_then(|o| o.fields.get("m_sketchPlaneId"))
                    .and_then(identifier)
                    .and_then(|n| u64::try_from(n).ok())
                    .filter(|n| *n > 0)
                    .ok_or("curve without a sketch plane")?;
                Ok(SketchCurve {
                    id: *id,
                    plane_id,
                    curve: element_curve(graph)?,
                })
            })
            .collect()
    }

    /// The ElementIds of the document's extrusions.
    pub fn extrusion_ids(&self) -> Vec<u64> {
        self.graphs
            .iter()
            .filter(|(_, graph)| {
                graph
                    .objects
                    .first()
                    .is_some_and(|root| root.class_name == "ExtrusionElem")
            })
            .map(|(id, _)| *id)
            .collect()
    }

    /// The extrusion `id` with its sketch's curves and sketch plane. Errs,
    /// naming why, where a record or a field the chain needs is
    /// absent or outside what the reader draws.
    pub fn extrusion(&self, id: u64) -> Result<Extrusion, String> {
        let graph = self.graphs.get(&id).ok_or("no extrusion record")?;
        let root = graph.objects.first().ok_or("empty graph")?;
        if root.class_name != "ExtrusionElem" {
            return Err(format!("{} is not an extrusion", root.class_name));
        }
        let cutting = root
            .fields
            .get("m_cutting")
            .and_then(Value::as_bool)
            .ok_or("no m_cutting")?;
        let (helper_index, helper) = graph
            .objects
            .iter()
            .enumerate()
            .find(|(_, object)| object.class_name == "ExtrusionElemExtrusionHelper")
            .ok_or("no extrusion helper")?;
        let sketch_id = helper
            .fields
            .get("m_sketchId")
            .and_then(identifier)
            .and_then(|n| u64::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or("no sketch id")?;

        let (mut start, mut end) = (None, None);
        for object in graph
            .objects
            .iter()
            .filter(|object| object.class_name == "ParamValueSetDouble")
        {
            let entries = object.fields.get("m_paramSet").and_then(Value::as_array);
            for entry in entries.into_iter().flatten() {
                let value = entry.get("m_value").and_then(Value::as_f64);
                match entry.get("m_paramId").and_then(identifier) {
                    Some(START_PARAMETER) => start = value,
                    Some(END_PARAMETER) => end = value,
                    _ => {}
                }
            }
        }
        let mut visible = None;
        for object in graph
            .objects
            .iter()
            .filter(|object| object.class_name == "ParamValueSetInt")
        {
            let entries = object.fields.get("m_paramSet").and_then(Value::as_array);
            for entry in entries.into_iter().flatten() {
                if entry.get("m_paramId").and_then(identifier) == Some(VISIBLE_PARAMETER) {
                    visible = entry.get("m_value").and_then(Value::as_i64);
                }
            }
        }
        let start = start.filter(|v| v.is_finite()).ok_or("no start offset")?;
        let end = end.filter(|v| v.is_finite()).ok_or("no end offset")?;

        let sketch_curves = self.sketch_curves(sketch_id)?;
        let curve_ids = sketch_curves.iter().map(|c| c.id).collect::<Vec<_>>();
        let curves = sketch_curves
            .iter()
            .map(|c| c.curve.clone())
            .collect::<Vec<_>>();
        let planes = sketch_curves
            .iter()
            .map(|c| c.plane_id)
            .collect::<BTreeSet<_>>();
        if planes.len() != 1 {
            return Err("curves on several sketch planes".into());
        }
        let plane_id = *planes.first().ok_or("no sketch plane id")?;
        let stored_loops_agree = stored_curves(graph, helper_index)
            .ok()
            .filter(|stored| {
                !stored.is_empty()
                    && stored.len() <= MAX_COMPARED_CURVES
                    && curves.len() <= MAX_COMPARED_CURVES
            })
            .map(|stored| same_curves(&stored, &curves));

        let plane = self
            .graphs
            .get(&plane_id)
            .ok_or("sketch plane record absent")?;
        let trf = plane
            .objects
            .iter()
            .find(|object| object.class_name == "Trf")
            .ok_or("sketch plane has no transform")?;
        let rows = trf
            .fields
            .get("m_3x3")
            .and_then(Value::as_array)
            .filter(|rows| rows.len() == 3)
            .ok_or("transform matrix")?;
        let mut matrix = [[0.0; 3]; 3];
        for (row, value) in matrix.iter_mut().zip(rows) {
            *row = triple(Some(value)).ok_or("transform row")?;
        }
        let origin = triple(trf.fields.get("m_or")).ok_or("transform origin")?;
        let determinant = matrix[0][0]
            * (matrix[1][1] * matrix[2][2] - matrix[1][2] * matrix[2][1])
            - matrix[0][1] * (matrix[1][0] * matrix[2][2] - matrix[1][2] * matrix[2][0])
            + matrix[0][2] * (matrix[1][0] * matrix[2][1] - matrix[1][1] * matrix[2][0]);
        let plane_ref = plane.objects.first().and_then(|root| {
            let pointer = root.fields.get("m_oPlaneRef")?;
            let target = native_saved_mesh::pointer(plane, 0, pointer).ok()??;
            let object = plane.objects.get(target)?;
            Some((
                object.class_name.clone(),
                object.fields.get("m_mirror").and_then(Value::as_bool),
            ))
        });
        let always_ref_plane_norm = root
            .fields
            .get("m_alwaysRefPlaneNorm")
            .and_then(Value::as_bool);
        let sketch_flips = self
            .graphs
            .get(&sketch_id)
            .and_then(|sketch| sketch.objects.first())
            .and_then(|root| {
                Some((
                    root.fields.get("m_flipXY")?.as_bool()?,
                    root.fields.get("m_flipZ")?.as_bool()?,
                ))
            });
        let sketch_normal = self
            .graphs
            .get(&sketch_id)
            .and_then(|sketch| {
                sketch
                    .objects
                    .iter()
                    .find(|object| object.class_name == "Plane")
            })
            .and_then(|object| {
                Some(cross3(
                    triple(object.fields.get("m_xVec"))?,
                    triple(object.fields.get("m_yVec"))?,
                ))
            });
        let plane_flipped = plane
            .objects
            .first()
            .and_then(|object| object.fields.get("m_flipZ"))
            .and_then(Value::as_bool)
            .ok_or("sketch plane flip not read")?;
        Ok(Extrusion {
            id,
            cutting,
            sketch_id,
            curve_ids,
            curves,
            plane_id,
            plane_flipped,
            matrix,
            origin,
            determinant,
            plane_ref,
            always_ref_plane_norm,
            visible,
            sketch_flips,
            sketch_normal,
            start,
            end,
            stored_loops_agree,
        })
    }
}

fn sub3(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn norm3(a: V3) -> f64 {
    dot3(a, a).sqrt()
}

fn along(a: V3, b: V3, t: f64) -> V3 {
    [a[0] + t * b[0], a[1] + t * b[1], a[2] + t * b[2]]
}

/// The distance from `p` to triangle `abc`.
fn triangle_distance(p: V3, a: V3, b: V3, c: V3) -> f64 {
    let ab = sub3(b, a);
    let ac = sub3(c, a);
    let ap = sub3(p, a);
    let d1 = dot3(ab, ap);
    let d2 = dot3(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return norm3(ap);
    }
    let bp = sub3(p, b);
    let d3 = dot3(ab, bp);
    let d4 = dot3(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return norm3(bp);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return norm3(sub3(p, along(a, ab, d1 / (d1 - d3))));
    }
    let cp = sub3(p, c);
    let d5 = dot3(ab, cp);
    let d6 = dot3(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return norm3(cp);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return norm3(sub3(p, along(a, ac, d2 / (d2 - d6))));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && d4 - d3 >= 0.0 && d5 - d6 >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return norm3(sub3(p, along(b, sub3(c, b), w)));
    }
    let denominator = 1.0 / (va + vb + vc);
    if !denominator.is_finite() {
        return f64::INFINITY;
    }
    let inside = along(along(a, ab, vb * denominator), ac, vc * denominator);
    norm3(sub3(p, inside))
}

/// How closely a union of solids reproduces a triangle mesh.
#[derive(Clone, Copy, Debug)]
pub struct MeshAgreement {
    /// The solids' volume less the mesh's, cubic feet.
    pub volume_deficit: f64,
    pub mesh_volume: f64,
    /// The most volume a mesh whose arcs are chords 0.001 ft deep
    /// can lack of the solids'.
    pub volume_slack: f64,
    /// The furthest any mesh vertex lies from the nearest solid's surface.
    pub vertex_max: f64,
    /// The furthest any sampled point of the solids' surfaces lies from the
    /// mesh.
    pub surface_max: f64,
}

impl MeshAgreement {
    /// Whether the volumes agree within the arcs' chord loss and 0.01% of the
    /// mesh's: on the 6,014 forms of Autodesk's sample projects that match
    /// their own complete mesh the difference never exceeds the chord loss,
    /// above or below.
    pub fn volume_agrees(&self) -> bool {
        self.volume_deficit.abs() <= VOLUME_MARGIN * self.mesh_volume + self.volume_slack
    }
}

/// Compares `solids`, in the mesh's coordinates, with a triangle mesh:
/// volumes, the mesh's vertices against the solids' surfaces, and samples of
/// the solids' surfaces against the mesh's triangles. `None` where the mesh
/// is empty, indexes outside its vertices or is larger than the reader
/// bounds.
pub fn mesh_agreement(
    solids: &[ExtrusionSolid],
    vertices: &[V3],
    triangles: &[[u32; 3]],
) -> Option<MeshAgreement> {
    if solids.is_empty()
        || vertices.is_empty()
        || triangles.is_empty()
        || triangles.len() > MAX_MESH_TRIANGLES
    {
        return None;
    }
    let corners: Vec<[V3; 3]> = triangles
        .iter()
        .map(|t| {
            let at = |i: u32| vertices.get(usize::try_from(i).ok()?).copied();
            Some([at(t[0])?, at(t[1])?, at(t[2])?])
        })
        .collect::<Option<Vec<_>>>()?;
    let mesh_volume = corners
        .iter()
        .map(|[a, b, c]| dot3(*a, cross3(*b, *c)) / 6.0)
        .sum::<f64>()
        .abs();
    let volume: f64 = solids.iter().map(ExtrusionSolid::volume).sum();
    let volume_slack: f64 = solids
        .iter()
        .map(|s| 2.0 / 3.0 * s.profile.arc_length() * SAGITTA * (s.high - s.low))
        .sum();
    let per_vertex: usize = solids
        .iter()
        .map(|s| {
            s.profile
                .loops
                .iter()
                .map(|l| l.curves.len())
                .sum::<usize>()
                + s.profile.polygons.iter().map(Vec::len).sum::<usize>()
        })
        .sum();
    if vertices.len().saturating_mul(per_vertex.max(1)) > MESH_PAIR_BUDGET {
        return None;
    }
    let vertex_max = vertices
        .iter()
        .map(|v| {
            solids
                .iter()
                .map(|s| s.signed_distance(*v).abs())
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    let samples: Vec<V3> = solids.iter().flat_map(|s| s.surface_samples(12)).collect();
    let budget = (MESH_PAIR_BUDGET / corners.len()).clamp(50, 4000);
    let stride = (samples.len() / budget).max(1);
    let surface_max = samples
        .iter()
        .step_by(stride)
        .map(|p| {
            corners
                .iter()
                .map(|[a, b, c]| triangle_distance(*p, *a, *b, *c))
                .fold(f64::INFINITY, f64::min)
        })
        .fold(0.0, f64::max);
    Some(MeshAgreement {
        volume_deficit: volume - mesh_volume,
        mesh_volume,
        volume_slack,
        vertex_max,
        surface_max,
    })
}
