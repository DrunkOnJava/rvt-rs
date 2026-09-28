//! Room plan outlines from the faces of each room's stored solid (RE-101).
//!
//! A room's partition holds the room's solid as a list of planar faces,
//! after a header naming the room:
//!
//! ```text
//! u64            the room's ElementId
//! 04 90 08 00    ROOM_SOLID_TAG
//! ...            the solid's box, twice, and its topology
//! ```
//!
//! and then its faces, each within [`FACE_GAP`] bytes of the one before
//! (174 bytes on, or 206 where the face carries more; other records sit
//! between some of them):
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
//! Core Interior keeps a room's record and solid in more than one partition,
//! and the copies can disagree: room 20832's outline spans 10.5 ft in one and
//! 10.4167 ft in the other. The copy kept is the one in the partition of the
//! record rvt-rs exports, and it is kept only where its plan extent equals
//! that record's box within [`BOX_TOLERANCE_FEET`]. On Core Interior all 116
//! rooms then take Revit's own exported area within 0.1%, and 40 of them
//! were not their record box (`reports/element-framing/RE-101-room-solids.md`).

use crate::element_record_plan_profiles::{PlanProfile, plan_profile_from_lines};
use crate::{Result, RevitFile};

/// Value of [`crate::element_record_plan_profiles::PLAN_PROFILE_SOURCE_FIELD`]
/// on a room outline read here.
pub const ROOM_OUTLINE_SOURCE: &str = "partition_room_solid_faces";

/// Releases this layout is measured on.
pub const ROOM_SOLID_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024, 2025];

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

/// How far apart two wall faces' ends may lie and still meet. Where two
/// walls stand on marginally different lines the room's faces are cut from
/// both and leave a step with no face across it: 3e-5 ft on RE1's room
/// 355008.
pub const ROOM_VERTEX_TOLERANCE_FEET: f64 = 1e-4;

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

fn read_f64s<const N: usize>(buf: &[u8], at: usize) -> Option<[f64; N]> {
    let mut out = [0.0; N];
    for (index, value) in out.iter_mut().enumerate() {
        let start = at.checked_add(8 * index)?;
        let bytes = buf.get(start..start.checked_add(8)?)?;
        *value = f64::from_le_bytes(bytes.try_into().ok()?);
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

/// The faces of the solid whose header starts at `header`: the first face
/// within [`ROOM_SOLID_WINDOW`], and each face after it within
/// [`FACE_GAP`] of the one before.
pub fn solid_faces(buf: &[u8], header: usize) -> Vec<RoomFace> {
    let start = header + 12;
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

/// The room's outline from the solid whose header starts at `header`, where
/// its wall faces (vertical, reaching the floor of `bbox`, a record box
/// `[min x, min y, min z, max x, max y, max z]`) close into loops whose plan
/// extent is `bbox`'s within
/// [`BOX_TOLERANCE_FEET`]. A face's origin is its plane's, not a point on
/// the face, so only the plan edges are checked.
pub fn room_outline_at(buf: &[u8], header: usize, bbox: [f64; 6]) -> Option<PlanProfile> {
    let faces = solid_faces(buf, header);
    let mut lines = Vec::new();
    for face in &faces {
        if let Some((edge, bottom)) = face.plan_edge() {
            if (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET {
                lines.push(edge);
            }
        }
    }
    snap_ends(&mut lines);
    let profile = plan_profile_from_lines(&lines)?;
    let extent = profile.plan_extent_feet()?;
    let wanted = [bbox[0], bbox[1], bbox[3], bbox[4]];
    extent
        .iter()
        .zip(wanted)
        .all(|(got, want)| (got - want).abs() <= BOX_TOLERANCE_FEET)
        .then_some(profile)
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
    let mut needle = u64::from(room_id).to_le_bytes().to_vec();
    needle.extend_from_slice(&ROOM_SOLID_TAG);
    let mut found: Option<PlanProfile> = None;
    for at in memchr::memmem::find_iter(buf, &needle) {
        let Some(profile) = room_outline_at(buf, at, bbox) else {
            continue;
        };
        match &found {
            None => found = Some(profile),
            Some(held) if *held == profile => {}
            Some(_) => return Ok(None),
        }
    }
    Ok(found)
}
