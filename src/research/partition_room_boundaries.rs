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
//! When the faces the walk reads do not close, the faces up to the next
//! solid's tag are read instead ([`solid_faces_to_next_solid`]), each plan
//! edge directed so the room is on its left, and the room's outline is the
//! boundary of the region those edges wind around ([`region_boundary`],
//! RE-316).
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
/// on a room outline read here.
pub const ROOM_OUTLINE_SOURCE: &str = "partition_room_solid_faces";

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

/// How many bytes past a room's solid header the next solid's tag is looked
/// for (RE-316): the largest solid measured, 115 faces, ends 84 KB in.
pub const ROOM_SOLID_SPAN_MAX: usize = 4 << 20;

/// How far a vertical face's span may fall short of a room's box, and a
/// horizontal face lie from its floor or ceiling, and still be it, feet
/// (RE-296).
pub const PRISM_TOLERANCE_FEET: f64 = 1e-3;

/// How far apart two wall faces' ends may lie and still meet. Where two
/// walls stand on marginally different lines the room's faces are cut from
/// both and leave a step with no face across it: 3e-5 ft on RE1's room
/// 355008.
pub const ROOM_VERTEX_TOLERANCE_FEET: f64 = 1e-4;

/// The most plan segments [`region_boundary`] resolves.
pub const BOUNDARY_SEGMENTS_MAX: usize = 4096;

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
    })
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

/// The faces of the solid whose header starts at `header`: the first face
/// within [`ROOM_SOLID_WINDOW`], and each face after it within
/// [`FACE_GAP`] of the one before.
pub fn solid_faces(buf: &[u8], header: usize) -> Vec<RoomFace> {
    let Some(start) = solid_start(buf, header) else {
        return Vec::new();
    };
    let end = buf.len().min(start + ROOM_SOLID_WINDOW);
    let Some(window) = buf.get(start..end) else {
        return Vec::new();
    };
    let Some(first) = memchr::memmem::find_iter(window, &FACE_TAG)
        .map(|offset| start + offset)
        .find(|&at| face_at(buf, at).is_some())
    else {
        return Vec::new();
    };
    let mut faces = Vec::new();
    let mut at = first;
    while let Some(face) = face_at(buf, at) {
        faces.push(face);
        let from = at + FACE_TAG.len();
        let Some(window) = buf.get(from..buf.len().min(at + FACE_GAP)) else {
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
    let FaceKind::Vertical { low, high } = face.kind() else {
        return false;
    };
    if low < bbox[2] - tolerance || high > bbox[5] + tolerance {
        return true;
    }
    face.plan_edge().is_some_and(|(edge, _)| {
        edge[0].min(edge[2]) < bbox[0] - tolerance
            || edge[0].max(edge[2]) > bbox[3] + tolerance
            || edge[1].min(edge[3]) < bbox[1] - tolerance
            || edge[1].max(edge[3]) > bbox[4] + tolerance
    })
}

fn profile_on_box(profile: PlanProfile, bbox: [f64; 6]) -> Option<PlanProfile> {
    let extent = profile.plan_extent_feet()?;
    let wanted = [bbox[0], bbox[1], bbox[3], bbox[4]];
    extent
        .iter()
        .zip(wanted)
        .all(|(got, want)| (got - want).abs() <= BOX_TOLERANCE_FEET)
        .then_some(profile)
}

/// The outline the wall faces among `faces` make, kept where it spans
/// `bbox`. With `resolve` off, the faces' plan edges must close into loops as
/// they are (RE-101). With it on, the outline is the boundary of the region
/// they wind around ([`region_boundary`]), and a vertical face that reaches
/// outside the box rules the solid out.
fn outline_from_faces(
    faces: &[RoomFace],
    bbox: [f64; 6],
    anchor: OutlineAnchor,
    resolve: bool,
) -> Option<PlanProfile> {
    if resolve && faces.iter().any(|face| strays_from_box(face, bbox)) {
        return None;
    }
    let mut lines = Vec::new();
    for face in faces {
        if let Some(edge) = reaches_anchor(face, bbox, anchor) {
            lines.push(edge);
        }
    }
    snap_ends(&mut lines);
    let profile = if resolve {
        plan_profile_from_lines(&region_boundary(&lines)?)?
    } else {
        plan_profile_from_lines(&lines)?
    };
    profile_on_box(profile, bbox)
}

/// The room's outline from the solid whose header starts at `header` and the
/// faces it was made from. Its wall faces (vertical, reaching the floor of
/// `bbox`, a record box `[min x, min y, min z, max x, max y, max z]`) close
/// into loops whose plan extent is `bbox`'s within [`BOX_TOLERANCE_FEET`];
/// where the faces the walk reads do not close, the faces up to the next
/// solid's tag are resolved to a boundary instead (RE-316). A face's origin is
/// its plane's, not a point on the face, so only the plan edges are checked.
fn outline_with_faces(
    buf: &[u8],
    header: usize,
    bbox: [f64; 6],
) -> Option<(PlanProfile, Vec<RoomFace>)> {
    let walked = solid_faces(buf, header);
    if let Some(profile) = outline_from_faces(&walked, bbox, OutlineAnchor::Floor, false) {
        return Some((profile, walked));
    }
    let span = solid_faces_to_next_solid(buf, header);
    let profile = outline_from_faces(&span, bbox, OutlineAnchor::Floor, true)?;
    Some((profile, span))
}

/// The room's outline from the solid whose header starts at `header`: its
/// wall faces' plan edges as they stand where the walk's faces close (RE-101),
/// and resolved to a boundary where the faces up to the next solid's tag do
/// (RE-316). Kept only where it spans `bbox`.
pub fn room_outline_at(buf: &[u8], header: usize, bbox: [f64; 6]) -> Option<PlanProfile> {
    outline_with_faces(buf, header, bbox).map(|(profile, _)| profile)
}

/// The outline the faces up to the next solid's tag make, resolved to a
/// boundary, for walls that reach `anchor` (RE-316). A probe's reading:
/// [`room_outline_at`] uses only [`OutlineAnchor::Floor`].
pub fn span_outline_at(
    buf: &[u8],
    header: usize,
    bbox: [f64; 6],
    anchor: OutlineAnchor,
) -> Option<PlanProfile> {
    outline_from_faces(&solid_faces_to_next_solid(buf, header), bbox, anchor, true)
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
        let Some((_, faces)) = outline_with_faces(buf, at, bbox) else {
            continue;
        };
        if !faces_form_prism(&faces, bbox) {
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

/// Every face of the solid whose header starts at `header`, from its first
/// face to the tag of the next solid in `buf`, whatever the gap between faces
/// (RE-316). Empty where no tag follows within [`ROOM_SOLID_SPAN_MAX`] bytes:
/// without it the solid's end is not known, and the faces past it are another
/// element's.
pub fn solid_faces_to_next_solid(buf: &[u8], header: usize) -> Vec<RoomFace> {
    let Some(start) = solid_start(buf, header) else {
        return Vec::new();
    };
    let window_end = buf.len().min(start.saturating_add(ROOM_SOLID_SPAN_MAX));
    let Some(window) = buf.get(start..window_end) else {
        return Vec::new();
    };
    let Some(end) = memchr::memmem::find(window, &ROOM_SOLID_TAG) else {
        return Vec::new();
    };
    memchr::memmem::find_iter(&window[..end], &FACE_TAG)
        .filter_map(|offset| face_at(buf, start + offset))
        .collect()
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
pub fn region_boundary(segments: &[[f64; 4]]) -> Option<Vec<[f64; 4]>> {
    let count = segments.len();
    if !(3..=BOUNDARY_SEGMENTS_MAX).contains(&count) {
        return None;
    }
    if segments.iter().flatten().any(|v| !v.is_finite()) {
        return None;
    }
    let mut cuts: Vec<Vec<f64>> = vec![Vec::new(); count];
    for first in 0..count {
        for second in first + 1..count {
            let (on_first, on_second) = cut_parameters(segments[first], segments[second]);
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
        return None;
    }
    let mut leaving = vec![0usize; points.len()];
    let mut entering = vec![0usize; points.len()];
    for &(from, to) in &kept {
        leaving[from] += 1;
        entering[to] += 1;
    }
    if leaving
        .iter()
        .zip(&entering)
        .any(|(out, into)| out != into || *out > 1)
    {
        return None;
    }
    Some(
        kept.iter()
            .map(|&(from, to)| [points[from].0, points[from].1, points[to].0, points[to].1])
            .collect(),
    )
}

/// Move every end within [`ROOM_VERTEX_TOLERANCE_FEET`] of an earlier one
/// onto it.
fn snap_ends(lines: &mut [[f64; 4]]) {
    let mut held: Vec<(f64, f64)> = Vec::new();
    for line in lines.iter_mut() {
        for end in [0, 2] {
            let point = (line[end], line[end + 1]);
            let near = held.iter().copied().find(|q| {
                (q.0 - point.0).abs() <= ROOM_VERTEX_TOLERANCE_FEET
                    && (q.1 - point.1).abs() <= ROOM_VERTEX_TOLERANCE_FEET
            });
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
    let mut found: Option<PlanProfile> = None;
    for &at in headers {
        let Some(profile) = room_outline_at(buf, at, bbox) else {
            continue;
        };
        match &found {
            None => found = Some(profile),
            Some(held) if *held == profile => {}
            Some(_) => return None,
        }
    }
    found
}
