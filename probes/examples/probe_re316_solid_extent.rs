//! RE-316: where a room's stored solid ends.
//!
//! FACT under test: a room's solid lists its objects after its two boxes as a
//! counted list (`04 c0 08 00 · u32 count · u32 4 · count × (u16 class · u32
//! token)`), and the count, or a gap no solid's own faces exceed, says where
//! the faces of the solid stop and another element's begin.
//!
//! One line per room whose own partition holds a solid: the list's count and
//! its first entries, the faces the walk reads (W) and those up to the next
//! solid's tag (S) with how many of each are horizontal, vertical and
//! cylindrical, the offsets of the first face, the last walked face, the last
//! face of the span and the next solid's tag from the solid's start, the
//! largest gap between faces of the walk and of the span, every gap in the
//! span above 210 bytes, and the gap from the last walked face to the next
//! valid face.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_solid_extent -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_room_boundaries::{
    FACE_GAP, FACE_TAG, FaceKind, ROOM_SOLID_TAG, ROOM_SOLID_WINDOW, RoomFace, face_at,
    room_outline, solid_headers,
};
use rvt::walker::InstanceField;

fn solid_start(buf: &[u8], header: usize) -> Option<usize> {
    let after_id = header.checked_add(8)?;
    if buf.get(after_id..after_id + 4)? == ROOM_SOLID_TAG {
        return Some(after_id + 4);
    }
    (buf.get(after_id..after_id + 8)? == [0xff, 0xff, 0xff, 0xff, 0x04, 0x90, 0x08, 0x00])
        .then_some(after_id + 8)
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?))
}

fn u16_at(buf: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(buf.get(at..at + 2)?.try_into().ok()?))
}

fn walk(buf: &[u8], first: usize) -> Vec<(usize, RoomFace)> {
    let mut out = Vec::new();
    let mut at = first;
    while let Some(face) = face_at(buf, at) {
        out.push((at, face));
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
    out
}

fn tally(faces: &[(usize, RoomFace)]) -> (usize, usize, usize) {
    let (mut h, mut v, mut c) = (0, 0, 0);
    for (_, face) in faces {
        if face.cylinder.is_some() {
            c += 1;
        } else {
            match face.kind() {
                FaceKind::Horizontal(_) => h += 1,
                FaceKind::Vertical { .. } => v += 1,
                FaceKind::Other => {}
            }
        }
    }
    (h, v, c)
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
        let kept = room_outline(&mut rf, &stream, id, bbox)?.is_some();
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        for (index, &header) in solid_headers(buf, id).iter().enumerate() {
            let Some(start) = solid_start(buf, header) else {
                continue;
            };
            let window_end = buf.len().min(start + ROOM_SOLID_WINDOW);
            let Some(window) = buf.get(start..window_end) else {
                continue;
            };
            let tag_at = memchr::memmem::find(window, &ROOM_SOLID_TAG).map(|o| start + o);
            let span_end = tag_at.unwrap_or(window_end);
            let valid: Vec<(usize, RoomFace)> =
                memchr::memmem::find_iter(&buf[start..span_end], &FACE_TAG)
                    .filter_map(|o| face_at(buf, start + o).map(|f| (start + o, f)))
                    .collect();
            let Some(&(first, _)) = valid.first() else {
                continue;
            };
            let walked = walk(buf, first);
            let (wh, wv, wc) = tally(&walked);
            let (sh, sv, sc) = tally(&valid);
            let list = memchr::memmem::find(&buf[start..first.min(start + 400)], &[4, 0xc0, 8, 0])
                .map(|o| start + o);
            let (count, lead, entries) = match list {
                Some(p) => {
                    let n = u32_at(buf, p + 4).unwrap_or(0);
                    let lead = u32_at(buf, p + 8).unwrap_or(0);
                    let mut e = Vec::new();
                    for k in 0..(n as usize).min(3) {
                        let at = p + 12 + 6 * k;
                        e.push(format!(
                            "{:04x}:{}",
                            u16_at(buf, at).unwrap_or(0),
                            u32_at(buf, at + 2).unwrap_or(0)
                        ));
                    }
                    let last_at = p + 12 + 6 * (n as usize).saturating_sub(1);
                    e.push(format!(
                        "last {:04x}:{}",
                        u16_at(buf, last_at).unwrap_or(0),
                        u32_at(buf, last_at + 2).unwrap_or(0)
                    ));
                    (n as i64, lead as i64, e.join(" "))
                }
                None => (-1, -1, String::new()),
            };
            let gaps = |faces: &[(usize, RoomFace)]| -> Vec<usize> {
                faces.windows(2).map(|w| w[1].0 - w[0].0).collect()
            };
            let wgaps = gaps(&walked);
            let sgaps = gaps(&valid);
            let big: Vec<String> = sgaps
                .iter()
                .enumerate()
                .filter(|(_, g)| **g > 210)
                .take(8)
                .map(|(i, g)| format!("{i}:{g}"))
                .collect();
            let last_walk = walked.last().map_or(0, |(a, _)| *a);
            let after = valid
                .iter()
                .map(|(a, _)| *a)
                .find(|a| *a > last_walk)
                .map(|a| a - last_walk);
            let last_span = valid.last().map_or(0, |(a, _)| *a);
            println!(
                "room {id} {stream} hdr {index} {} N {count} lead {lead} [{entries}] W {} (h{wh} v{wv} c{wc}) S {} (h{sh} v{sv} c{sc}) first +{} lastW +{} lastS +{} tag {} maxgapW {} maxgapS {} bigS [{}] afterW {:?} stray_to_end {}",
                if kept { "KEPT" } else { "FAILING" },
                walked.len(),
                valid.len(),
                first - start,
                last_walk - start,
                last_span - start,
                tag_at.map_or("none".to_string(), |t| format!("+{}", t - start)),
                wgaps.iter().copied().max().unwrap_or(0),
                sgaps.iter().copied().max().unwrap_or(0),
                big.join(" "),
                after,
                span_end - start,
            );
        }
    }
    Ok(())
}
