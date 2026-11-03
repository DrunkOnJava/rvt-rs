//! RE-316: the topology a room's solid stores before its first face.
//!
//! FACT under test: between a room's solid header (RE-101) and its first face
//! the solid stores its vertices as three consecutive `f64`, a model-space
//! point inside the room's record box, and the points at the box's floor give
//! the room's true outline where the planar faces' boxes do not.
//!
//! Output, per room whose planar faces do not close on its box: the solid's
//! start and first face, every offset between them where three consecutive
//! `f64` are a point inside the box (with the 24 bytes before it and the gap
//! from the point before), the points at the floor, and the 96 bytes after the
//! header's second box.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_solid_vertices -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_room_boundaries::{
    FACE_TAG, ROOM_SOLID_TAG, ROOM_SOLID_WINDOW, face_at, room_outline, solid_headers,
};
use rvt::walker::InstanceField;

const MAX_ROOMS: usize = 14;
const MAX_POINTS: usize = 90;
const TOLERANCE: f64 = 1e-3;

fn solid_start(buf: &[u8], header: usize) -> Option<usize> {
    let after_id = header.checked_add(8)?;
    if buf.get(after_id..after_id + 4)? == ROOM_SOLID_TAG {
        return Some(after_id + 4);
    }
    (buf.get(after_id..after_id + 8)? == [0xff, 0xff, 0xff, 0xff, 0x04, 0x90, 0x08, 0x00])
        .then_some(after_id + 8)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    let bytes: [u8; 8] = buf.get(at..at + 8)?.try_into().ok()?;
    let value = f64::from_le_bytes(bytes);
    value.is_finite().then_some(value)
}

fn triple_at(buf: &[u8], at: usize) -> Option<[f64; 3]> {
    Some([
        f64_at(buf, at)?,
        f64_at(buf, at + 8)?,
        f64_at(buf, at + 16)?,
    ])
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let rooms = rvt::research::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?
    .rooms;
    let mut shown = 0;
    for room in &rooms {
        let Some(id) = room.id else { continue };
        let float = |wanted: &str| {
            room.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let stream = room
            .fields
            .iter()
            .find_map(|(name, value)| match value {
                InstanceField::String(s) if name == "m_source_stream" => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let (Some(cx), Some(cy), Some(z), Some(dx), Some(dy), Some(dz)) = (
            float("m_locationX"),
            float("m_locationY"),
            float("m_locationZ"),
            float("m_bboxWidth"),
            float("m_bboxDepth"),
            float("m_bboxHeight"),
        ) else {
            continue;
        };
        let bbox = [
            cx - dx / 2.0,
            cy - dy / 2.0,
            z,
            cx + dx / 2.0,
            cy + dy / 2.0,
            z + dz,
        ];
        if room_outline(&mut rf, &stream, id, bbox)?.is_some() {
            continue;
        }
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        let Some(&header) = solid_headers(buf, id).first() else {
            continue;
        };
        let Some(start) = solid_start(buf, header) else {
            continue;
        };
        let window_end = buf.len().min(start + ROOM_SOLID_WINDOW);
        let first_face = memchr::memmem::find_iter(&buf[start..window_end], &FACE_TAG)
            .map(|offset| start + offset)
            .find(|&at| face_at(buf, at).is_some());
        let Some(first_face) = first_face else {
            continue;
        };
        if shown >= MAX_ROOMS {
            break;
        }
        shown += 1;
        println!(
            "room {id} {stream} box [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}] solid start {start} first face +{}",
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            bbox[4],
            bbox[5],
            first_face - start
        );
        if let Some(bytes) = buf.get(start + 8 + 96..start + 8 + 96 + 96) {
            println!("  after the two boxes: {}", hex(bytes));
        }
        let inside = |p: [f64; 3]| {
            p[0] >= bbox[0] - TOLERANCE
                && p[0] <= bbox[3] + TOLERANCE
                && p[1] >= bbox[1] - TOLERANCE
                && p[1] <= bbox[4] + TOLERANCE
                && p[2] >= bbox[2] - TOLERANCE
                && p[2] <= bbox[5] + TOLERANCE
        };
        let mut points: Vec<(usize, [f64; 3])> = Vec::new();
        let mut at = start;
        while at + 24 <= first_face {
            match triple_at(buf, at) {
                Some(p) if inside(p) && p.iter().any(|v| v.abs() > 1e-9) => {
                    points.push((at, p));
                    at += 24;
                }
                _ => at += 1,
            }
        }
        let at_floor: Vec<&(usize, [f64; 3])> = points
            .iter()
            .filter(|(_, p)| (p[2] - bbox[2]).abs() <= TOLERANCE)
            .collect();
        println!(
            "  points inside the box {} (at the floor {})",
            points.len(),
            at_floor.len()
        );
        let mut previous = start;
        for (index, (offset, p)) in points.iter().enumerate().take(MAX_POINTS) {
            let context = buf
                .get(offset.saturating_sub(24)..*offset)
                .map_or(String::new(), hex);
            println!(
                "  point {index} @{} (+{} from the last) [{:.4}, {:.4}, {:.4}] before: {context}",
                offset - start,
                offset - previous,
                p[0],
                p[1],
                p[2]
            );
            previous = *offset + 24;
        }
        if points.len() > MAX_POINTS {
            println!("  ... {} points more", points.len() - MAX_POINTS);
        }
    }
    Ok(())
}
