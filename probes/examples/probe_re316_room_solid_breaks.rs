//! RE-316: why a room's stored solid does not close on its record box.
//!
//! FACT under test: for each room whose own partition holds a solid (RE-101)
//! and whose wall faces do not close into an outline spanning the record box,
//! the probe says what is missing: faces the walk from the first one never
//! reached, vertical faces whose frame is not axis-aligned (no plan edge), faces
//! that stop short of the floor, edges shorter than the vertex tolerance, ends
//! that meet no other end, ends that lie on the inside of another edge, or loops
//! that close on a different extent than the box.
//!
//! Output, one block per room: `room <id> <stream> box ... flags [...]`, the
//! solid's header and tag census, one `face` line per face of the walk, the
//! gaps between faces that are not the usual width, the bytes after the last
//! face, the faces found beyond the walk, and the vertices that do not have
//! exactly two edges. The last line tallies the flag sets.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_room_solid_breaks -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::partition_room_boundaries::{
    BOX_TOLERANCE_FEET, FACE_GAP, FACE_TAG, FaceKind, ROOM_SOLID_TAG, ROOM_SOLID_WINDOW,
    ROOM_VERTEX_TOLERANCE_FEET, RoomFace, face_at, room_outline, solid_headers,
};
use rvt::walker::InstanceField;

const FACE_LEN: usize = 158;
const MAX_FACE_LINES: usize = 60;
const MAX_VERTEX_LINES: usize = 16;
const MAX_ORPHAN_LINES: usize = 12;

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

fn chain(buf: &[u8], start: usize) -> Vec<(usize, RoomFace)> {
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

fn intern(points: &mut Vec<(f64, f64)>, degree: &mut Vec<usize>, p: (f64, f64)) -> usize {
    let found = points.iter().position(|q| {
        (q.0 - p.0).abs() <= ROOM_VERTEX_TOLERANCE_FEET
            && (q.1 - p.1).abs() <= ROOM_VERTEX_TOLERANCE_FEET
    });
    match found {
        Some(index) => index,
        None => {
            points.push(p);
            degree.push(0);
            points.len() - 1
        }
    }
}

fn distance_to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (qx, qy) = (a.0 + t * dx, a.1 + t * dy);
    (((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt(), t)
}

fn kind_text(face: &RoomFace) -> String {
    match face.kind() {
        FaceKind::Horizontal(z) => format!("H z {z:.4}"),
        FaceKind::Vertical { low, high } => match face.plan_edge() {
            Some(([x0, y0, x1, y1], bottom)) => format!(
                "V ({x0:.4},{y0:.4})-({x1:.4},{y1:.4}) low {low:.4} high {high:.4} bottom {bottom:.4}"
            ),
            None => format!(
                "V-NOEDGE low {low:.4} high {high:.4} u {:.4?} v {:.4?}",
                face.u, face.v
            ),
        },
        FaceKind::Other => format!("O u {:.4?} v {:.4?}", face.u, face.v),
    }
}

fn describe(buf: &[u8], header: usize, bbox: [f64; 6]) -> Vec<String> {
    let mut out = Vec::new();
    let Some(start) = solid_start(buf, header) else {
        out.push("  solid header unreadable".into());
        return out;
    };
    let walk = chain(buf, start);
    let window_end = buf.len().min(start + ROOM_SOLID_WINDOW);
    let next_solid = memchr::memmem::find(&buf[start..window_end], &ROOM_SOLID_TAG)
        .map_or(window_end, |offset| start + offset);
    let all_tags: Vec<usize> = memchr::memmem::find_iter(&buf[start..window_end], &FACE_TAG)
        .map(|offset| start + offset)
        .collect();
    let valid_tags: Vec<usize> = all_tags
        .iter()
        .copied()
        .filter(|&at| face_at(buf, at).is_some())
        .collect();
    let first_at = walk.first().map_or(0, |(at, _)| *at);
    let last_at = walk.last().map_or(0, |(at, _)| *at);
    out.push(format!(
        "  solid header {header} start {start} first face +{} last face +{} next solid tag +{} tags {} valid {} walk {}",
        first_at.wrapping_sub(start),
        last_at.wrapping_sub(start),
        next_solid - start,
        all_tags.len(),
        valid_tags.len(),
        walk.len()
    ));
    let mut previous = None;
    for (index, (at, face)) in walk.iter().enumerate() {
        let gap = previous.map_or(0, |p| at - p);
        if index < MAX_FACE_LINES {
            out.push(format!(
                "  face {index} @{} gap {gap} uv {:.4?} {}",
                at - start,
                face.uv_box,
                kind_text(face)
            ));
        }
        if previous.is_some() && ![170, 174, 206].contains(&gap) {
            let from = previous.unwrap_or(0) + FACE_LEN;
            let upto = (*at).min(from + 96);
            if let Some(bytes) = buf.get(from..upto) {
                out.push(format!(
                    "  gap-bytes before face {index} ({gap}): {}",
                    hex(bytes)
                ));
            }
        }
        previous = Some(*at);
    }
    if walk.len() > MAX_FACE_LINES {
        out.push(format!("  ... {} faces more", walk.len() - MAX_FACE_LINES));
    }
    if let Some(bytes) = buf.get(last_at + FACE_LEN..buf.len().min(last_at + FACE_LEN + 112)) {
        out.push(format!("  after last face: {}", hex(bytes)));
    }
    let orphans: Vec<usize> = valid_tags
        .iter()
        .copied()
        .filter(|&at| at > last_at)
        .collect();
    for &at in orphans.iter().take(MAX_ORPHAN_LINES) {
        let kind = face_at(buf, at).map_or_else(String::new, |face| kind_text(&face));
        out.push(format!(
            "  beyond the walk: face @{} ({} past the last) {kind}",
            at - start,
            at - last_at
        ));
    }
    if orphans.len() > MAX_ORPHAN_LINES {
        out.push(format!(
            "  ... {} faces beyond the walk more",
            orphans.len() - MAX_ORPHAN_LINES
        ));
    }
    let floor = bbox[2];
    let mut lines: Vec<[f64; 4]> = Vec::new();
    let (mut short_walls, mut no_edge, mut other, mut horizontal) = (0, 0, 0, 0);
    for (_, face) in &walk {
        match face.kind() {
            FaceKind::Horizontal(_) => horizontal += 1,
            FaceKind::Other => other += 1,
            FaceKind::Vertical { .. } => match face.plan_edge() {
                Some((edge, bottom)) => {
                    if (bottom - floor).abs() <= BOX_TOLERANCE_FEET {
                        lines.push(edge);
                    } else {
                        short_walls += 1;
                    }
                }
                None => no_edge += 1,
            },
        }
    }
    let mut points: Vec<(f64, f64)> = Vec::new();
    let mut degree: Vec<usize> = Vec::new();
    let mut edges: Vec<(usize, usize)> = Vec::new();
    let mut collapsed = 0;
    for &[x0, y0, x1, y1] in &lines {
        let a = intern(&mut points, &mut degree, (x0, y0));
        let b = intern(&mut points, &mut degree, (x1, y1));
        if a == b {
            collapsed += 1;
            continue;
        }
        degree[a] += 1;
        degree[b] += 1;
        edges.push((a, b));
    }
    let mut duplicates = 0;
    for (index, &(a, b)) in edges.iter().enumerate() {
        if edges[..index]
            .iter()
            .any(|&(c, d)| (a, b) == (c, d) || (a, b) == (d, c))
        {
            duplicates += 1;
        }
    }
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for &d in &degree {
        *histogram.entry(d).or_default() += 1;
    }
    let mut open = 0;
    let mut on_edge = 0;
    let mut shown = 0;
    for (index, &d) in degree.iter().enumerate() {
        if d == 2 {
            continue;
        }
        open += 1;
        let p = points[index];
        let mut note = String::new();
        for &(a, b) in &edges {
            if a == index || b == index {
                continue;
            }
            let (distance, t) = distance_to_segment(p, points[a], points[b]);
            if distance <= 1e-3 && t > 0.0 && t < 1.0 {
                on_edge += 1;
                note = format!(
                    " on the inside of edge ({:.4},{:.4})-({:.4},{:.4}) at t {t:.5}",
                    points[a].0, points[a].1, points[b].0, points[b].1
                );
                break;
            }
        }
        if shown < MAX_VERTEX_LINES {
            out.push(format!("  vertex ({:.4},{:.4}) edges {d}{note}", p.0, p.1));
            shown += 1;
        }
    }
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for &(x, y) in &points {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let extent_gap = [
        min_x - bbox[0],
        min_y - bbox[1],
        max_x - bbox[3],
        max_y - bbox[4],
    ];
    let mut flags = Vec::new();
    if lines.is_empty() {
        flags.push("no-wall-edges");
    }
    if walk.is_empty() {
        flags.push("no-faces");
    }
    if no_edge > 0 {
        flags.push("vertical-without-axis-frame");
    }
    if !orphans.is_empty() {
        flags.push("faces-beyond-walk");
    }
    if short_walls > 0 {
        flags.push("walls-short-of-floor");
    }
    if other > 0 {
        flags.push("non-vertical-non-horizontal-faces");
    }
    if collapsed > 0 {
        flags.push("edges-shorter-than-tolerance");
    }
    if duplicates > 0 {
        flags.push("duplicate-edges");
    }
    if open > 0 {
        flags.push("ends-not-paired");
    }
    if on_edge > 0 {
        flags.push("end-inside-another-edge");
    }
    if open == 0 && !lines.is_empty() {
        flags.push("degrees-all-two");
        if extent_gap.iter().any(|g| g.abs() > BOX_TOLERANCE_FEET) {
            flags.push("extent-differs-from-box");
        }
    }
    out.push(format!(
        "  walls {} (short of floor {short_walls}, no axis frame {no_edge}), horizontal {horizontal}, other {other}; edges {}, collapsed {collapsed}, duplicates {duplicates}; degrees {histogram:?}; open {open}, on an edge's inside {on_edge}; extent minus box {:.4?}",
        lines.len(),
        edges.len(),
        extent_gap
    ));
    out.push(format!("  FLAGS {}", flags.join(",")));
    out
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
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
    let mut kept = 0;
    let mut absent = 0;
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
            kept += 1;
            continue;
        }
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        let headers = solid_headers(buf, id);
        if headers.is_empty() {
            absent += 1;
            continue;
        }
        println!(
            "room {id} {stream} box [{:.4}, {:.4}, {:.4}, {:.4}, {:.4}, {:.4}] size {dx:.3} x {dy:.3} x {dz:.3} solids {}",
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            bbox[4],
            bbox[5],
            headers.len()
        );
        let mut first_flags = String::new();
        for (index, &header) in headers.iter().enumerate() {
            let block = describe(buf, header, bbox);
            if index == 0 {
                first_flags = block
                    .iter()
                    .find_map(|line| line.trim().strip_prefix("FLAGS ").map(str::to_string))
                    .unwrap_or_default();
            }
            for line in block {
                println!("{line}");
            }
        }
        *tally.entry(first_flags).or_default() += 1;
    }
    println!(
        "release {version}: {} rooms, {kept} outlines kept, {absent} with no solid in their partition, flag sets of the rest: {tally:#?}",
        rooms.len()
    );
    Ok(())
}
