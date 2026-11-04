//! Room plan outlines from the faces of each room's stored solid (RE-101).
//!
//! A room's partition holds the room's solid as a list of planar faces,
//! after a header naming the room:
//!
//! ```text
//! u64            the room's ElementId
//! ff ff ff ff    on Revit 2023 only
//! 04 90 08 00    ROOM_SOLID_TAG
//! ...            the solid's box, twice, and its topology
//! ```
//!
//! and then its faces, each within [`FACE_GAP`] bytes of the one before
//! (174 bytes on, or 206 where the face carries more, and 170 on Revit
//! 2023; other records sit between some of them):
//!
//! ```text
//! +0    04 00 08 00   FACE_TAG
//! +20   f64 × 4       u min, v min, u max, v max, feet
//! +85   01
//! +86   f64 × 3       origin, model feet
//! +110  f64 × 3       unit u direction
//! +134  f64 × 3       unit v direction
//! ```
//!
//! A wall face of the room is vertical, one direction up or down and the
//! other horizontal, and reaches down to the room's floor. Its plan edge
//! runs from `origin + min · h` to `origin + max · h` along the horizontal
//! direction `h`. The wall faces' edges close into the room's outline, with
//! a loop of its own around each column or shaft the room surrounds. A
//! vertical face that stops short of the floor is a step in the room's
//! ceiling, inside the outline.
//!
//! A record the walk does not read can sit between two faces and be longer
//! than [`FACE_GAP`] (626 bytes in office room 1422302), and a face's box is
//! the box of its plane, not its outline: where a column stands in the room
//! and overlaps a wall, the wall's face runs across the column's footprint.
//! Only where the walk closes none of a room's solids is the solid read again
//! (RE-316): its faces are followed with a gap of up to [`SPAN_FACE_GAP`]
//! bytes, and no more than its list of objects counts ([`solid_span`],
//! [`solid_object_count`]), so the faces of the element that follows it are
//! not read. Where the span's wall edges close as the walk reads them (one
//! outer loop, the loops inside it nested, no edge crossing another; their
//! ends joined within [`ROOM_STEP_TOLERANCE_FEET`], where the walk joins them
//! within 1e-4 ft) that is
//! the outline ([`ROOM_OUTLINE_SOURCE_FACES`]). Where a column overlaps a wall
//! and its loop lies partly inside the wall's, each plan edge is directed so
//! the room is on its left and the outline is the boundary of the region those
//! edges wind around ([`region_boundary`]), the column taken off
//! ([`ROOM_OUTLINE_SOURCE_APPROXIMATE`]: whether Revit's export takes it off is
//! not settled by the file). A room read either way says so in its profile
//! source, never
//! as the walk's. A face whose record is 44 bytes longer than a plane's is a
//! cylinder's (a round column): its axis, a unit vector at right angles to
//! the face's u and v, at +158, and its radius at +182 ([`RoomCylinder`]). Its
//! arc closes the loops of the six `rac_advanced` rooms RE-101 left open, but
//! whether Revit takes a column off a room's area is unmeasured (it does not
//! on four office rooms), so a solid with one is left unresolved.
//!
//! Core Interior keeps a room's record and solid in more than one partition,
//! and the copies can disagree: room 20832's outline spans 10.5 ft in one and
//! 10.4167 ft in the other. The copy kept is the one in the partition of the
//! record rvt-rs exports, and it is kept only where its plan extent equals
//! that record's box within [`BOX_TOLERANCE_FEET`]. On Core Interior all 116
//! rooms then take Revit's own exported area within 0.1%, and 40 of them
//! were not their record box (`reports/element-framing/RE-101-room-solids.md`).

use crate::element_record_plan_profiles::{PlanProfile, plan_profile_from_lines};
use crate::le_bytes;
use crate::{Result, RevitFile};
use std::collections::BTreeMap;

/// Value of [`crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD`]
/// on a room outline read here from the faces the walk reads, closing as they
/// are (RE-101).
pub const ROOM_OUTLINE_SOURCE: &str = "partition_room_solid_faces";

/// Value of the same field on a room outline read from the faces its solid
/// lists, up to the solid's own end, where the walk did not close them and
/// the span's wall edges close as the walk reads them (RE-316). Not the
/// walk's: a closed loop of another element's faces inside the box could still
/// be read into it, so an importer tells the two apart.
pub const ROOM_OUTLINE_SOURCE_FACES: &str = "partition_room_solid_faces_resolved";

/// Value of the same field on a room outline that is the boundary of the region
/// the span's edges wind around, a column overlapping a wall taken off it, or
/// falls up to [`ROOM_SLIVER_FEET`] short of the record box on a side (RE-316):
/// approximate, up to 0.68% under Revit's floor area where its export does not
/// take the column off.
pub const ROOM_OUTLINE_SOURCE_APPROXIMATE: &str = "partition_room_solid_faces_approximate";

/// Whether `source` is a value of the profile source field that a room's
/// outline from its stored solid carries.
pub fn is_room_outline_source(source: &str) -> bool {
    source == ROOM_OUTLINE_SOURCE
        || source == ROOM_OUTLINE_SOURCE_FACES
        || source == ROOM_OUTLINE_SOURCE_APPROXIMATE
}

/// Releases this layout is measured on. On Revit 2026 and 2027 it is the
/// 2025 layout: every room of Autodesk's `rac_basic`, `rac_advanced`,
/// `rme_basic` and `rme_advanced` sample projects (257) gives the same
/// verdict and area in their 2026 and 2027 saves as in 2025, and the same
/// rooms keep their box (RE-314, #1260).
pub const ROOM_SOLID_SUPPORTED_REVIT_VERSIONS: &[u32] = &[
    2008, 2009, 2010, 2011, 2012, 2013, 2014, 2015, 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023,
    2024, 2025, 2026, 2027,
];

/// The bytes after a room's ElementId that open its solid.
pub const ROOM_SOLID_TAG: [u8; 4] = [0x04, 0x90, 0x08, 0x00];

/// The bytes a face starts with.
pub const FACE_TAG: [u8; 4] = [0x04, 0x00, 0x08, 0x00];

/// The most bytes from one face's tag to the next face's.
pub const FACE_GAP: usize = 0x200;

/// How far past a room's solid header its first face is looked for.
pub const ROOM_SOLID_WINDOW: usize = 0x1_0000;

/// How closely an outline's plan extent must equal the room's record box,
/// feet.
pub const BOX_TOLERANCE_FEET: f64 = 1e-3;

/// The most bytes from one face's tag to the next face's within one solid
/// when its list of objects bounds the span (RE-316).
pub const SPAN_FACE_GAP: usize = 0x400;

/// The most faces a solid may have to be read at all (RE-316): the largest
/// solid measured lists 136 objects. A solid with more is left to its box, so
/// the work of snapping and resolving its edges is bounded.
pub const SOLID_FACES_MAX: usize = 512;

/// The most faces a solid's span may hold before it is not resolved (RE-316).
pub const RESOLVE_FACES_MAX: usize = SOLID_FACES_MAX;

/// How far a vertical face's span may fall short of a room's box, and a
/// horizontal face lie from its floor or ceiling, and still be it, feet
/// (RE-296).
pub const PRISM_TOLERANCE_FEET: f64 = 1e-3;

/// How far apart two wall faces' ends may lie and still meet. Where two
/// walls stand on marginally different lines the room's faces are cut from
/// both and leave a step with no face across it: 3e-5 ft on RE1's room
/// 355008.
pub const ROOM_VERTEX_TOLERANCE_FEET: f64 = 1e-4;

/// How far apart two wall faces' ends may lie and still meet where the room's
/// faces are resolved to a boundary (RE-316). Walls on lines 0.0007 to 0.0015
/// ft apart leave a step the room's faces do not cross on six rooms of the
/// BIM4LCA office; Revit's own short-curve tolerance is 1/32 inch, 0.0026 ft.
pub const ROOM_STEP_TOLERANCE_FEET: f64 = 0.0025;

/// How far a chord of a column's arc may stray from the arc, feet (as
/// [`crate::element_record_plan_profiles::SKETCH_ARC_TOLERANCE_FEET`]).
const ARC_CHORD_TOLERANCE_FEET: f64 = 0.0005;

/// How far a resolved outline may fall short of the room's record box on a
/// side and still be a room's outline, labelled approximate, feet (RE-316):
/// the back of a column that overlaps a wall stands 0.0164 ft behind the
/// wall's face on three rooms of the BIM4LCA office, and the box holds it.
pub const ROOM_SLIVER_FEET: f64 = 0.02;

/// The most plan segments [`region_boundary`] resolves.
pub const BOUNDARY_SEGMENTS_MAX: usize = RESOLVE_FACES_MAX;

/// The most pieces the segments of [`region_boundary`] may be cut into: a
/// room's own faces cross a few times, and a crafted grid of them crosses
/// quadratically.
pub const BOUNDARY_PIECES_MAX: usize = 4096;

/// How far apart two points of a resolved boundary may lie and be one,
/// feet.
const BOUNDARY_POINT_EPS: f64 = 1e-6;

/// How far to either side of a boundary piece its winding is read, feet.
const BOUNDARY_SIDE_EPS: f64 = 1e-5;

const FACE_LEN: usize = 158;
const UNIT_EPS: f64 = 1e-6;
const AXIS_EPS: f64 = 1e-9;

/// One planar face of a room's solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomFace {
    /// `[u min, v min, u max, v max]`, feet.
    pub uv_box: [f64; 4],
    /// Origin, model feet.
    pub origin: [f64; 3],
    /// Unit u direction.
    pub u: [f64; 3],
    /// Unit v direction.
    pub v: [f64; 3],
    /// The axis and radius where the face is a cylinder's: its record is 44
    /// bytes longer than a plane's, with the axis (a unit vector at right
    /// angles to u and v) and the radius after the 158 bytes a plane's
    /// has, and its uv box is an angle about the axis and a distance along it
    /// (RE-316).
    pub cylinder: Option<RoomCylinder>,
}

/// The axis and radius of a cylindrical face of a room's solid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomCylinder {
    /// Unit axis.
    pub axis: [f64; 3],
    /// Radius, feet.
    pub radius: f64,
}

impl RoomFace {
    /// The face's plan edge `[x0, y0, x1, y1]` and the height of its lower
    /// end, feet, where it is vertical: one direction up or down, the other
    /// horizontal.
    pub fn plan_edge(&self) -> Option<([f64; 4], f64)> {
        let vertical = |d: [f64; 3]| (d[2].abs() - 1.0).abs() < AXIS_EPS;
        let horizontal = |d: [f64; 3]| d[2].abs() < AXIS_EPS;
        let [u_min, v_min, u_max, v_max] = self.uv_box;
        let ((h, lo, hi), (up, down_lo, down_hi)) = if vertical(self.u) && horizontal(self.v) {
            ((self.v, v_min, v_max), (self.u[2], u_min, u_max))
        } else if vertical(self.v) && horizontal(self.u) {
            ((self.u, u_min, u_max), (self.v[2], v_min, v_max))
        } else {
            return None;
        };
        let [ox, oy, oz] = self.origin;
        let bottom = (oz + up * down_lo).min(oz + up * down_hi);
        Some((
            [
                ox + h[0] * lo,
                oy + h[1] * lo,
                ox + h[0] * hi,
                oy + h[1] * hi,
            ],
            bottom,
        ))
    }
}

/// What a face of a room's solid is, by its normal (RE-296).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FaceKind {
    /// In a horizontal plane at this height, feet.
    Horizontal(f64),
    /// In a vertical plane, within its box between these heights, feet.
    Vertical { low: f64, high: f64 },
    /// In a plane that is neither.
    Other,
}

impl RoomFace {
    /// Whether the face is horizontal, vertical or neither, by the vertical
    /// component of its normal, the cross product of its two directions.
    pub fn kind(&self) -> FaceKind {
        let normal_z = self.u[0] * self.v[1] - self.u[1] * self.v[0];
        if (normal_z.abs() - 1.0).abs() < AXIS_EPS {
            return FaceKind::Horizontal(self.origin[2]);
        }
        if normal_z.abs() >= AXIS_EPS {
            return FaceKind::Other;
        }
        let [u_min, v_min, u_max, v_max] = self.uv_box;
        let heights = [
            (u_min, v_min),
            (u_min, v_max),
            (u_max, v_min),
            (u_max, v_max),
        ]
        .map(|(u, v)| self.origin[2] + self.u[2] * u + self.v[2] * v);
        FaceKind::Vertical {
            low: heights.iter().copied().fold(f64::INFINITY, f64::min),
            high: heights.iter().copied().fold(f64::NEG_INFINITY, f64::max),
        }
    }
}

fn read_f64s<const N: usize>(buf: &[u8], at: usize) -> Option<[f64; N]> {
    let mut out = [0.0; N];
    for (index, value) in out.iter_mut().enumerate() {
        let start = at.checked_add(8 * index)?;
        *value = le_bytes::f64_at(buf, start)?;
        if !value.is_finite() {
            return None;
        }
    }
    Some(out)
}

/// The face whose tag starts at `at`: the `01` byte in place, finite
/// values, unit directions and a uv box with its minimum below its maximum.
pub fn face_at(buf: &[u8], at: usize) -> Option<RoomFace> {
    if buf.get(at..at.checked_add(FACE_LEN)?)?[..4] != FACE_TAG || buf[at + 85] != 1 {
        return None;
    }
    let uv_box: [f64; 4] = read_f64s(buf, at + 20)?;
    let origin = read_f64s(buf, at + 86)?;
    let u = read_f64s(buf, at + 110)?;
    let v = read_f64s(buf, at + 134)?;
    let unit = |d: [f64; 3]| (d.iter().map(|c| c * c).sum::<f64>() - 1.0).abs() < UNIT_EPS;
    if !unit(u) || !unit(v) || uv_box[0] > uv_box[2] || uv_box[1] > uv_box[3] {
        return None;
    }
    Some(RoomFace {
        uv_box,
        origin,
        u,
        v,
        cylinder: cylinder_at(buf, at, u, v),
    })
}

/// The cylinder the face record at `at` describes, where the 24 bytes after
/// its 158 are a unit axis at right angles to `u` and `v` and the 8 after
/// them a positive radius. A plane's record has `ff` bytes there.
fn cylinder_at(buf: &[u8], at: usize, u: [f64; 3], v: [f64; 3]) -> Option<RoomCylinder> {
    let axis: [f64; 3] = read_f64s(buf, at.checked_add(FACE_LEN)?)?;
    let radius = le_bytes::f64_at(buf, at.checked_add(FACE_LEN + 24)?)?;
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let unit = (dot(axis, axis) - 1.0).abs() < UNIT_EPS;
    let square = dot(axis, u).abs() < UNIT_EPS && dot(axis, v).abs() < UNIT_EPS;
    (unit && square && radius.is_finite() && radius > 1e-4 && radius < 1e4)
        .then_some(RoomCylinder { axis, radius })
}

/// Where the solid whose header starts at `header` begins: past its
/// [`ROOM_SOLID_TAG`], directly after the ElementId or, on Revit 2023,
/// after `ff ff ff ff`.
fn solid_start(buf: &[u8], header: usize) -> Option<usize> {
    let after_id = header.checked_add(8)?;
    if buf.get(after_id..after_id + 4)? == ROOM_SOLID_TAG {
        return Some(after_id + 4);
    }
    (buf.get(after_id..after_id + 8)? == [0xff, 0xff, 0xff, 0xff, 0x04, 0x90, 0x08, 0x00])
        .then_some(after_id + 8)
}

/// The first face of the solid that starts at `start`, within
/// [`ROOM_SOLID_WINDOW`].
fn first_face_at(buf: &[u8], start: usize) -> Option<usize> {
    let end = buf.len().min(start.checked_add(ROOM_SOLID_WINDOW)?);
    let window = buf.get(start..end)?;
    memchr::memmem::find_iter(window, &FACE_TAG)
        .map(|offset| start + offset)
        .find(|&at| face_at(buf, at).is_some())
}

/// The faces from the one at `first`, each the first face within `max_gap`
/// bytes of its tag after the one before, up to `limit` of them.
fn follow_faces(buf: &[u8], first: usize, max_gap: usize, limit: usize) -> Vec<RoomFace> {
    let mut faces = Vec::new();
    let mut at = first;
    while let Some(face) = face_at(buf, at) {
        faces.push(face);
        if faces.len() >= limit {
            break;
        }
        let from = at + FACE_TAG.len();
        let Some(window) = buf.get(from..buf.len().min(at + max_gap)) else {
            break;
        };
        let Some(next) = memchr::memmem::find_iter(window, &FACE_TAG)
            .map(|offset| from + offset)
            .find(|&next| face_at(buf, next).is_some())
        else {
            break;
        };
        at = next;
    }
    faces
}

/// The faces of the solid whose header starts at `header`: the first face
/// within [`ROOM_SOLID_WINDOW`], and each face after it within
/// [`FACE_GAP`] of the one before. None if there are more than
/// [`SOLID_FACES_MAX`].
pub fn solid_faces(buf: &[u8], header: usize) -> Vec<RoomFace> {
    let Some(start) = solid_start(buf, header) else {
        return Vec::new();
    };
    let Some(first) = first_face_at(buf, start) else {
        return Vec::new();
    };
    let faces = follow_faces(buf, first, FACE_GAP, SOLID_FACES_MAX + 1);
    if faces.len() > SOLID_FACES_MAX {
        return Vec::new();
    }
    faces
}

/// The bytes that open the list of a solid's objects, after its two boxes.
const SOLID_LIST_TAG: [u8; 4] = [0x04, 0xc0, 0x08, 0x00];

/// How far past its start a solid's list of objects is looked for.
const SOLID_LIST_WINDOW: usize = 0x200;

/// The number of objects the solid that starts at `start` lists (RE-316): the
/// list is `04 c0 08 00 · u32 count · u32 4 · count × (u16 class · u32
/// token)` and counts every face, including those whose record
/// [`face_at`] does not read (a solid's floor and ceiling, usually). None
/// unless the list is there with a count from 1 to [`RESOLVE_FACES_MAX`].
pub fn solid_object_count(buf: &[u8], start: usize) -> Option<usize> {
    let end = buf.len().min(start.checked_add(SOLID_LIST_WINDOW)?);
    let window = buf.get(start..end)?;
    let at = start + memchr::memmem::find(window, &SOLID_LIST_TAG)?;
    let count = le_bytes::u32_at(buf, at + 4)? as usize;
    let lead = le_bytes::u32_at(buf, at + 8)?;
    (lead == 4 && (1..=RESOLVE_FACES_MAX).contains(&count)).then_some(count)
}

impl RoomFace {
    /// The heights, low and high, a cylindrical face with a vertical axis
    /// spans, feet.
    fn vertical_cylinder_heights(&self) -> Option<(f64, f64)> {
        let cylinder = self.cylinder?;
        let horizontal = |d: [f64; 3]| d[2].abs() < AXIS_EPS;
        if (cylinder.axis[2].abs() - 1.0).abs() >= AXIS_EPS
            || !horizontal(self.u)
            || !horizontal(self.v)
        {
            return None;
        }
        let ends = [self.uv_box[1], self.uv_box[3]]
            .map(|distance| self.origin[2] + cylinder.axis[2] * distance);
        Some((ends[0].min(ends[1]), ends[0].max(ends[1])))
    }

    /// The chords of a vertical cylinder's arc as plan segments, run
    /// clockwise around its axis, which is how the walls of a column run
    /// round the room (the room is on their left); within
    /// [`ARC_CHORD_TOLERANCE_FEET`] of the arc.
    fn arc_segments(&self) -> Option<Vec<[f64; 4]>> {
        let cylinder = self.cylinder?;
        self.vertical_cylinder_heights()?;
        let (start, end) = (self.uv_box[0], self.uv_box[2]);
        let sweep = end - start;
        if !(sweep > 0.0 && sweep <= std::f64::consts::TAU + 1e-6) {
            return None;
        }
        let radius = cylinder.radius;
        let largest_chord_angle = 2.0
            * (1.0 - ARC_CHORD_TOLERANCE_FEET / radius)
                .clamp(-1.0, 1.0)
                .acos();
        let chords = ((sweep / largest_chord_angle.max(1e-3)).ceil() as usize).clamp(1, 1024);
        let point = |angle: f64| {
            let (sine, cosine) = angle.sin_cos();
            [
                self.origin[0] + radius * (cosine * self.u[0] + sine * self.v[0]),
                self.origin[1] + radius * (cosine * self.u[1] + sine * self.v[1]),
            ]
        };
        let counter_clockwise = self.u[0] * self.v[1] - self.u[1] * self.v[0] > 0.0;
        let angles: Vec<f64> = (0..=chords)
            .map(|index| start + sweep * index as f64 / chords as f64)
            .collect();
        let ordered: Vec<f64> = if counter_clockwise {
            angles.into_iter().rev().collect()
        } else {
            angles
        };
        Some(
            ordered
                .windows(2)
                .map(|pair| {
                    let (from, to) = (point(pair[0]), point(pair[1]));
                    [from[0], from[1], to[0], to[1]]
                })
                .collect(),
        )
    }
}

/// Which end of the room its wall faces must reach to be a wall of its
/// outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineAnchor {
    /// The room's floor, the bottom of its record box.
    Floor,
    /// The room's ceiling, the top of its record box.
    Ceiling,
}

fn reaches_anchor(face: &RoomFace, bbox: [f64; 6], anchor: OutlineAnchor) -> Option<[f64; 4]> {
    let (edge, bottom) = face.plan_edge()?;
    let reached = match anchor {
        OutlineAnchor::Floor => (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET,
        OutlineAnchor::Ceiling => match face.kind() {
            FaceKind::Vertical { high, .. } => (high - bbox[5]).abs() <= BOX_TOLERANCE_FEET,
            _ => false,
        },
    };
    reached.then_some(edge)
}

/// Whether a vertical `face` reaches outside the room's record box, which
/// holds the whole solid: a face of another element, read because the span
/// ran on past the solid's end.
fn strays_from_box(face: &RoomFace, bbox: [f64; 6]) -> bool {
    let tolerance = BOX_TOLERANCE_FEET;
    let outside = |edge: [f64; 4]| {
        edge[0].min(edge[2]) < bbox[0] - tolerance
            || edge[0].max(edge[2]) > bbox[3] + tolerance
            || edge[1].min(edge[3]) < bbox[1] - tolerance
            || edge[1].max(edge[3]) > bbox[4] + tolerance
    };
    if face.cylinder.is_some() {
        let (Some((low, high)), Some(arc)) =
            (face.vertical_cylinder_heights(), face.arc_segments())
        else {
            return true;
        };
        return low < bbox[2] - tolerance
            || high > bbox[5] + tolerance
            || arc.iter().any(|segment| outside(*segment));
    }
    let FaceKind::Vertical { low, high } = face.kind() else {
        return false;
    };
    if low < bbox[2] - tolerance || high > bbox[5] + tolerance {
        return true;
    }
    face.plan_edge().is_some_and(|(edge, _)| outside(edge))
}

/// `profile` where its plan extent equals `bbox`'s on each side within
/// [`BOX_TOLERANCE_FEET`], or, where `slack` is set, falls short of it by no
/// more than `slack` on any side.
fn profile_on_box(profile: PlanProfile, bbox: [f64; 6], slack: f64) -> Option<PlanProfile> {
    let extent = profile.plan_extent_feet()?;
    let inward = [1.0, 1.0, -1.0, -1.0];
    let wanted = [bbox[0], bbox[1], bbox[3], bbox[4]];
    extent
        .iter()
        .zip(wanted)
        .zip(inward)
        .all(|((got, want), sign)| {
            let short = (got - want) * sign;
            short >= -BOX_TOLERANCE_FEET && short <= slack + BOX_TOLERANCE_FEET
        })
        .then_some(profile)
}

/// How the faces of a span are read into an outline (RE-316).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpanRead {
    /// Read the arcs of round columns too. Off, a solid with a cylinder is
    /// not resolved.
    pub arcs: bool,
    /// How far the outline may fall short of the record box on a side.
    pub slack: f64,
}

/// The plan edges of the wall faces among `faces` that reach `anchor`, and,
/// where `span` reads arcs, of the round columns that do.
fn wall_edges(
    faces: &[RoomFace],
    bbox: [f64; 6],
    anchor: OutlineAnchor,
    span: Option<SpanRead>,
) -> Option<Vec<[f64; 4]>> {
    let mut lines = Vec::new();
    for face in faces {
        if span.is_none() || face.cylinder.is_none() {
            if let Some(edge) = reaches_anchor(face, bbox, anchor) {
                lines.push(edge);
            }
        }
        if span.is_some_and(|read| read.arcs) {
            if let Some((low, high)) = face.vertical_cylinder_heights() {
                let reached = match anchor {
                    OutlineAnchor::Floor => (low - bbox[2]).abs() <= BOX_TOLERANCE_FEET,
                    OutlineAnchor::Ceiling => (high - bbox[5]).abs() <= BOX_TOLERANCE_FEET,
                };
                if reached {
                    lines.extend(face.arc_segments()?);
                }
            }
        }
    }
    Some(lines)
}

/// Whether any two of the plan `segments` cross, or overlap along a line,
/// anywhere but at an end both have.
fn crosses(segments: &[[f64; 4]]) -> bool {
    const END: f64 = 1e-9;
    for (index, first) in segments.iter().enumerate() {
        for second in &segments[index + 1..] {
            let (on_first, on_second) = cut_parameters(*first, *second);
            if on_first
                .iter()
                .chain(&on_second)
                .any(|t| *t > END && *t < 1.0 - END)
            {
                return true;
            }
        }
    }
    false
}

/// The outline the faces of a span make (RE-316): with the plan edges of its
/// walls that reach `anchor`, ends within [`ROOM_STEP_TOLERANCE_FEET`] joined,
/// and a flag, set where those edges close as RE-101's walk reads them (every
/// end shared by two edges, loops that nest, one outer loop, no edge crossing
/// another) and otherwise unset, the outline then being the boundary of the
/// region the edges wind around ([`region_boundary`]), which subtracts a
/// column that overlaps a wall. `None` unless the outline spans
/// `bbox` as `read` allows; a face outside `bbox`, a cylinder not read as an
/// arc and more than [`RESOLVE_FACES_MAX`] faces rule the span out.
fn span_outline(
    faces: &[RoomFace],
    bbox: [f64; 6],
    anchor: OutlineAnchor,
    read: SpanRead,
) -> Option<(PlanProfile, bool)> {
    if faces.len() > RESOLVE_FACES_MAX
        || faces.iter().any(|face| strays_from_box(face, bbox))
        || (!read.arcs && faces.iter().any(|face| face.cylinder.is_some()))
    {
        return None;
    }
    let mut lines = wall_edges(faces, bbox, anchor, Some(read))?;
    snap_ends_within(&mut lines, ROOM_STEP_TOLERANCE_FEET);
    if let Some(profile) = plan_profile_from_lines(&lines) {
        if profile.pieces.is_empty() && !crosses(&lines) {
            return profile_on_box(profile, bbox, read.slack).map(|profile| (profile, true));
        }
    }
    let boundary = region_boundary(&lines).ok()?;
    profile_on_box(plan_profile_from_lines(&boundary)?, bbox, read.slack)
        .map(|profile| (profile, false))
}

/// The outline the wall faces among `faces` make, kept where it spans
/// `bbox`: for the walk of RE-101 (`span` unset) the faces' plan edges must
/// close into loops as they are; for a span they are read by [`span_outline`].
fn outline_from_faces(
    faces: &[RoomFace],
    bbox: [f64; 6],
    anchor: OutlineAnchor,
    span: Option<SpanRead>,
) -> Option<PlanProfile> {
    match span {
        None => {
            let mut lines = wall_edges(faces, bbox, anchor, None)?;
            snap_ends(&mut lines);
            profile_on_box(plan_profile_from_lines(&lines)?, bbox, 0.0)
        }
        Some(read) => span_outline(faces, bbox, anchor, read).map(|(profile, _)| profile),
    }
}

/// The room's outline from the solid whose header starts at `header`, where
/// its wall faces (vertical, reaching the floor of `bbox`, a record box
/// `[min x, min y, min z, max x, max y, max z]`) close into loops whose plan
/// extent is `bbox`'s within [`BOX_TOLERANCE_FEET`]. A face's origin is its
/// plane's, not a point on the face, so only the plan edges are checked.
pub fn room_outline_at(buf: &[u8], header: usize, bbox: [f64; 6]) -> Option<PlanProfile> {
    outline_from_faces(&solid_faces(buf, header), bbox, OutlineAnchor::Floor, None)
}

/// The outline the faces up to the solid's end make, resolved to a boundary,
/// for walls that reach `anchor`, read as `read` says (RE-316). A probe's
/// reading: [`resolved_outline_of_solids`] uses only [`OutlineAnchor::Floor`].
pub fn span_outline_at(
    buf: &[u8],
    header: usize,
    bbox: [f64; 6],
    anchor: OutlineAnchor,
    read: SpanRead,
) -> Option<PlanProfile> {
    outline_from_faces(&solid_span(buf, header), bbox, anchor, Some(read))
}

/// How a resolved outline was read, and so what to trust of its area
/// (RE-316).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedKind {
    /// The wall edges of the span close as the walk of RE-101 reads them:
    /// one outer loop, the loops inside it nested, no edge crossing another,
    /// and the outline spanning the record box to [`BOX_TOLERANCE_FEET`] on
    /// every side. A face of another element would have to close a loop of
    /// its own inside the box, within the gap and the count that bound the
    /// span, to be read.
    Faces,
    /// The outline is the boundary of the region the edges wind around,
    /// because a column overlaps a wall and its loop lies partly inside the
    /// wall's, or an end of one wall edge lies on another, or it falls up to [`ROOM_SLIVER_FEET`] short of the record box
    /// on a side. Revit's export does not always take such a column off the
    /// room's area, so the area is approximate: 0.45% to 0.7% under Revit's
    /// floor area on the four office rooms where it does not.
    Approximate,
}

/// The outline of a room whose solids at `headers` none gives an outline the
/// walk reads ([`outline_of_solids`] is `None`), from the faces of each
/// solid's span (RE-316): [`ResolvedKind::Faces`] where they close as the walk
/// reads them, else [`ResolvedKind::Approximate`], also tried with
/// [`ROOM_SLIVER_FEET`] of slack. Where more than one solid gives one, every
/// one must give the same.
pub fn resolved_outline_of_solids(
    buf: &[u8],
    headers: &[usize],
    bbox: [f64; 6],
) -> Option<(PlanProfile, ResolvedKind)> {
    resolved_outline_of_solids_at(buf, headers, bbox).map(|(profile, kind, _)| (profile, kind))
}

/// [`resolved_outline_of_solids`], also giving the headers of the solids that
/// gave the outline (#1368).
pub fn resolved_outline_of_solids_at(
    buf: &[u8],
    headers: &[usize],
    bbox: [f64; 6],
) -> Option<(PlanProfile, ResolvedKind, Vec<usize>)> {
    let mut found: Option<(PlanProfile, ResolvedKind)> = None;
    let mut used = Vec::new();
    for &at in headers {
        let faces = solid_span(buf, at);
        let exact = SpanRead {
            arcs: false,
            slack: 0.0,
        };
        let slack = SpanRead {
            arcs: false,
            slack: ROOM_SLIVER_FEET,
        };
        let Some(this) = span_outline(&faces, bbox, OutlineAnchor::Floor, exact)
            .map(|(profile, simple)| {
                let kind = if simple {
                    ResolvedKind::Faces
                } else {
                    ResolvedKind::Approximate
                };
                (profile, kind)
            })
            .or_else(|| {
                span_outline(&faces, bbox, OutlineAnchor::Floor, slack)
                    .map(|(profile, _)| (profile, ResolvedKind::Approximate))
            })
        else {
            continue;
        };
        match &found {
            None => found = Some(this),
            Some(held) if *held == this => {}
            Some(_) => return None,
        }
        used.push(at);
    }
    found.map(|(profile, kind)| (profile, kind, used))
}

/// Whether `faces`, the faces of a room's solid, make a prism of the room's
/// plan outline between the floor and ceiling of `bbox` (RE-296): there is a
/// vertical face, every vertical face runs from the floor to the ceiling, and
/// there is no face that is neither vertical nor horizontal and no horizontal
/// face between the two. A vertical face that stops short is a step in the
/// ceiling, and a horizontal face in between is the step's tread: the solid is
/// then more than its outline's area times its height.
pub fn faces_form_prism(faces: &[RoomFace], bbox: [f64; 6]) -> bool {
    let (floor, ceiling) = (bbox[2], bbox[5]);
    let mut walls = 0;
    for face in faces {
        match face.kind() {
            FaceKind::Horizontal(z) => {
                if z > floor + PRISM_TOLERANCE_FEET && z < ceiling - PRISM_TOLERANCE_FEET {
                    return false;
                }
            }
            FaceKind::Vertical { low, high } => {
                if low > floor + PRISM_TOLERANCE_FEET || high < ceiling - PRISM_TOLERANCE_FEET {
                    return false;
                }
                walls += 1;
            }
            FaceKind::Other => return false,
        }
    }
    walls > 0
}

/// Whether the room's solids whose headers are at `headers` that give an
/// outline against `bbox` ([`outline_of_solids`]) are all prisms of it
/// ([`faces_form_prism`]), and at least one gives an outline.
pub fn prism_of_solids(buf: &[u8], headers: &[usize], bbox: [f64; 6]) -> bool {
    let mut found = false;
    for &at in headers {
        if room_outline_at(buf, at, bbox).is_none() {
            continue;
        }
        if !faces_form_prism(&solid_faces(buf, at), bbox) {
            return false;
        }
        found = true;
    }
    found
}

/// Where the solids of room `room_id` start in `buf`: each place its `u64`
/// ElementId is followed by [`ROOM_SOLID_TAG`], directly or after
/// `ff ff ff ff`.
pub fn solid_headers(buf: &[u8], room_id: u32) -> Vec<usize> {
    let id = u64::from(room_id).to_le_bytes();
    memchr::memmem::find_iter(buf, &id)
        .filter(|&at| solid_start(buf, at).is_some())
        .collect()
}

/// [`solid_headers`] for every ElementId at once, in one pass over `buf`
///: each [`ROOM_SOLID_TAG`] with the `u64` before it, or before
/// `ff ff ff ff` before it.
// Tracker: B85.
pub fn solid_headers_by_id(buf: &[u8]) -> BTreeMap<u64, Vec<usize>> {
    let mut out: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for tag in memchr::memmem::find_iter(buf, &ROOM_SOLID_TAG) {
        let direct = tag.checked_sub(8);
        let after_ff = tag
            .checked_sub(12)
            .filter(|&header| buf.get(header + 8..tag) == Some(&[0xff; 4][..]));
        for header in [direct, after_ff].into_iter().flatten() {
            let Some(id) = le_bytes::u64_at(buf, header) else {
                continue;
            };
            out.entry(id).or_default().push(header);
        }
    }
    for headers in out.values_mut() {
        headers.sort_unstable();
    }
    out
}

/// The faces of the solid whose header starts at `header` that the walk
/// stops short of (RE-316): from its first face, each next face within
/// [`SPAN_FACE_GAP`] bytes of the one before, and no more than the solid's
/// list of objects counts ([`solid_object_count`]). A record longer than
/// [`FACE_GAP`] can sit between a solid's own faces (592 to 732 bytes on the
/// BIM4LCA office and Autodesk's samples). On the 47 solids measured whose list
/// counts exactly the faces the walk reads, the next face after the walk, which
/// is another element's, is 6826 bytes or more on. Empty without the list or a
/// first face.
pub fn solid_span(buf: &[u8], header: usize) -> Vec<RoomFace> {
    let Some(start) = solid_start(buf, header) else {
        return Vec::new();
    };
    let (Some(count), Some(first)) = (solid_object_count(buf, start), first_face_at(buf, start))
    else {
        return Vec::new();
    };
    follow_faces(buf, first, SPAN_FACE_GAP, count)
}

/// Where each of two segments is cut by the other, as parameters from 0 at
/// its start to 1 at its end: at a crossing, and, where they overlap along
/// one line, at the other's ends.
fn cut_parameters(a: [f64; 4], b: [f64; 4]) -> (Vec<f64>, Vec<f64>) {
    let (mut on_a, mut on_b) = (Vec::new(), Vec::new());
    let (ax, ay) = (a[2] - a[0], a[3] - a[1]);
    let (bx, by) = (b[2] - b[0], b[3] - b[1]);
    let (alen, blen) = (ax.hypot(ay), bx.hypot(by));
    if alen <= BOUNDARY_POINT_EPS || blen <= BOUNDARY_POINT_EPS {
        return (on_a, on_b);
    }
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let cross = ax * by - ay * bx;
    if cross.abs() > 1e-9 * alen * blen {
        let t = (dx * by - dy * bx) / cross;
        let u = (dx * ay - dy * ax) / cross;
        let (t_slack, u_slack) = (BOUNDARY_POINT_EPS / alen, BOUNDARY_POINT_EPS / blen);
        if t >= -t_slack && t <= 1.0 + t_slack && u >= -u_slack && u <= 1.0 + u_slack {
            on_a.push(t);
            on_b.push(u);
        }
    } else if ((dx * ay - dy * ax) / alen).abs() <= 10.0 * BOUNDARY_POINT_EPS {
        let along_a = |x: f64, y: f64| ((x - a[0]) * ax + (y - a[1]) * ay) / (alen * alen);
        let along_b = |x: f64, y: f64| ((x - b[0]) * bx + (y - b[1]) * by) / (blen * blen);
        on_a.push(along_a(b[0], b[1]));
        on_a.push(along_a(b[2], b[3]));
        on_b.push(along_b(a[0], a[1]));
        on_b.push(along_b(a[2], a[3]));
    }
    (on_a, on_b)
}

/// How many times the oriented `segments` wind around `p`: each upward
/// crossing of the ray to its right with `p` on the segment's left adds one,
/// each downward crossing with `p` on its right takes one off.
fn winding(segments: &[[f64; 4]], p: (f64, f64)) -> i32 {
    let mut total = 0;
    for &[x0, y0, x1, y1] in segments {
        let side = (x1 - x0) * (p.1 - y0) - (p.0 - x0) * (y1 - y0);
        if y0 <= p.1 {
            if y1 > p.1 && side > 0.0 {
                total += 1;
            }
        } else if y1 <= p.1 && side < 0.0 {
            total -= 1;
        }
    }
    total
}

fn intern_point(points: &mut Vec<(f64, f64)>, p: (f64, f64)) -> usize {
    let found = points.iter().position(|q| {
        (q.0 - p.0).abs() <= BOUNDARY_POINT_EPS && (q.1 - p.1).abs() <= BOUNDARY_POINT_EPS
    });
    match found {
        Some(index) => index,
        None => {
            points.push(p);
            points.len() - 1
        }
    }
}

/// Why [`region_boundary`] gave no boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum BoundaryFailure {
    /// There are fewer than three segments, more than
    /// [`BOUNDARY_SEGMENTS_MAX`], or one has an end that is not finite.
    Segments(usize),
    /// The segments cross so often that they would be cut into more than
    /// [`BOUNDARY_PIECES_MAX`] pieces.
    Pieces,
    /// Fewer than three pieces bound the region.
    NoRegion,
    /// The points where the pieces that bound the region do not run one in
    /// and one out.
    Unlinked(Vec<(f64, f64)>),
}

/// The boundary of the region that the oriented plan `segments` wind around
/// at least once, as undirected segments that close into loops (RE-316). A
/// segment's room is on its left, so a wall of the room runs counter-clockwise
/// around it and a column's walls clockwise around the column, and a column
/// that overlaps a wall takes its footprint off the room where they overlap
/// and adds nothing outside it. Every segment is cut where another crosses it
/// or touches it, a piece is kept where the region lies on one side of it
/// only, and `None` is returned unless the kept pieces leave every point with
/// one piece in and one out.
#[allow(clippy::needless_range_loop)]
pub fn region_boundary(segments: &[[f64; 4]]) -> Result<Vec<[f64; 4]>, BoundaryFailure> {
    let count = segments.len();
    if !(3..=BOUNDARY_SEGMENTS_MAX).contains(&count)
        || segments.iter().flatten().any(|v| !v.is_finite())
    {
        return Err(BoundaryFailure::Segments(count));
    }
    let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); count];
    let mut cut_total = 0usize;
    for first in 0..count {
        for second in first + 1..count {
            let (on_first, on_second) = cut_parameters(segments[first], segments[second]);
            cut_total += on_first.len() + on_second.len();
            if cut_total + count > BOUNDARY_PIECES_MAX {
                return Err(BoundaryFailure::Pieces);
            }
            cuts[first].extend(on_first);
            cuts[second].extend(on_second);
        }
    }
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut pieces: Vec<(usize, usize)> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        let length = (segment[2] - segment[0]).hypot(segment[3] - segment[1]);
        if length <= BOUNDARY_POINT_EPS {
            continue;
        }
        let mut params = vec![0.0, 1.0];
        params.extend(cuts[index].iter().copied().filter(|t| {
            t * length > BOUNDARY_POINT_EPS && (1.0 - t) * length > BOUNDARY_POINT_EPS
        }));
        params.sort_by(f64::total_cmp);
        params.dedup_by(|a, b| (*a - *b).abs() * length <= BOUNDARY_POINT_EPS);
        let at = |t: f64| {
            (
                segment[0] + t * (segment[2] - segment[0]),
                segment[1] + t * (segment[3] - segment[1]),
            )
        };
        for pair in params.windows(2) {
            let from = intern_point(&mut points, at(pair[0]));
            let to = intern_point(&mut points, at(pair[1]));
            if from != to {
                pieces.push((from, to));
            }
        }
    }
    let mut kept: Vec<(usize, usize)> = Vec::new();
    for &(from, to) in &pieces {
        let (a, b) = (points[from], points[to]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx.hypot(dy);
        let (nx, ny) = (-dy / length, dx / length);
        let middle = ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0);
        let left = winding(
            segments,
            (
                middle.0 + nx * BOUNDARY_SIDE_EPS,
                middle.1 + ny * BOUNDARY_SIDE_EPS,
            ),
        ) >= 1;
        let right = winding(
            segments,
            (
                middle.0 - nx * BOUNDARY_SIDE_EPS,
                middle.1 - ny * BOUNDARY_SIDE_EPS,
            ),
        ) >= 1;
        match (left, right) {
            (true, false) => kept.push((from, to)),
            (false, true) => kept.push((to, from)),
            _ => {}
        }
    }
    kept.sort_unstable();
    kept.dedup();
    if kept.len() < 3 {
        return Err(BoundaryFailure::NoRegion);
    }
    let mut leaving = vec![0usize; points.len()];
    let mut entering = vec![0usize; points.len()];
    for &(from, to) in &kept {
        leaving[from] += 1;
        entering[to] += 1;
    }
    let unlinked: Vec<(f64, f64)> = leaving
        .iter()
        .zip(&entering)
        .zip(&points)
        .filter(|((out, into), _)| out != into || **out > 1)
        .map(|(_, point)| *point)
        .collect();
    if !unlinked.is_empty() {
        return Err(BoundaryFailure::Unlinked(unlinked));
    }
    Ok(kept
        .iter()
        .map(|&(from, to)| [points[from].0, points[from].1, points[to].0, points[to].1])
        .collect())
}

/// Move every end within [`ROOM_VERTEX_TOLERANCE_FEET`] of an earlier one
/// onto it.
fn snap_ends(lines: &mut [[f64; 4]]) {
    snap_ends_within(lines, ROOM_VERTEX_TOLERANCE_FEET);
}

/// Move every end within `tolerance` of an earlier one onto it.
fn snap_ends_within(lines: &mut [[f64; 4]], tolerance: f64) {
    let mut held: Vec<(f64, f64)> = Vec::new();
    for line in lines.iter_mut() {
        for end in [0, 2] {
            let point = (line[end], line[end + 1]);
            let near = held
                .iter()
                .copied()
                .find(|q| (q.0 - point.0).abs() <= tolerance && (q.1 - point.1).abs() <= tolerance);
            match near {
                Some(q) => (line[end], line[end + 1]) = q,
                None => held.push(point),
            }
        }
    }
}

/// The outline of room `room_id` from its solid in partition `stream`,
/// checked against the room's record box. Where the partition holds more
/// than one solid for the room, every one that passes must give the same
/// outline.
pub fn room_outline(
    rf: &mut RevitFile,
    stream: &str,
    room_id: u32,
    bbox: [f64; 6],
) -> Result<Option<PlanProfile>> {
    let inflated = rf.inflated_partition(stream)?;
    let buf = inflated.bytes();
    Ok(outline_of_solids(buf, &solid_headers(buf, room_id), bbox))
}

/// The outline the solids whose headers are at `headers` give, checked
/// against the room's record box; where more than one passes, every one
/// must give the same outline.
pub fn outline_of_solids(buf: &[u8], headers: &[usize], bbox: [f64; 6]) -> Option<PlanProfile> {
    outline_of_solids_at(buf, headers, bbox).map(|(profile, _)| profile)
}

/// [`outline_of_solids`], also giving the headers of the solids that gave the
/// outline (#1368).
pub fn outline_of_solids_at(
    buf: &[u8],
    headers: &[usize],
    bbox: [f64; 6],
) -> Option<(PlanProfile, Vec<usize>)> {
    let mut found: Option<PlanProfile> = None;
    let mut used = Vec::new();
    for &at in headers {
        let Some(profile) = room_outline_at(buf, at, bbox) else {
            continue;
        };
        match &found {
            None => found = Some(profile),
            Some(held) if *held == profile => {}
            Some(_) => return None,
        }
        used.push(at);
    }
    found.map(|profile| (profile, used))
}
