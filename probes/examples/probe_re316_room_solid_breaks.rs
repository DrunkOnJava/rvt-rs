//! RE-316: why a room's stored solid does not close on its record box.
//!
//! FACT under test: for each room whose own partition holds a solid (RE-101)
//! and whose wall faces do not close into an outline spanning the record box,
//! the probe says what is missing. It reads the solid three ways and says for
//! each whether the plan edges close into an outline on the record box:
//!
//! - `chain`: the faces RE-101 reads, each within `FACE_GAP` bytes of the one
//!   before, and the faces that reach the box's floor;
//! - `span-floor`: every face between the first one and the next solid's tag,
//!   whatever the gap, that reaches the floor and lies inside the box;
//! - `span-ceiling`: the same, for the faces that reach the box's ceiling;
//! - `span-any`: every vertical face, whatever its heights.
//!
//! Output, one block per room: the solid's header and tag census, one `face`
//! line per face of the chain, the gaps between faces that are not the usual
//! width, the faces beyond the chain and before the next solid, the vertices
//! that do not have exactly two edges and the loops' containment, then one
//! `variant` line for each reading. The last line tallies the verdicts.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_room_solid_breaks -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::{inside, plan_profile_from_lines};
use rvt::research::partition_room_boundaries::{
    BOX_TOLERANCE_FEET, FACE_GAP, FACE_TAG, FaceKind, ROOM_SOLID_TAG, ROOM_SOLID_WINDOW,
    ROOM_VERTEX_TOLERANCE_FEET, RoomFace, face_at, room_outline, solid_headers,
};
use rvt::walker::InstanceField;

const FACE_LEN: usize = 158;
const MAX_FACE_LINES: usize = 40;
const MAX_VERTEX_LINES: usize = 12;
const MAX_ORPHAN_LINES: usize = 16;

type Faces = Vec<(usize, RoomFace)>;

#[derive(Clone, Copy, PartialEq)]
enum Anchor {
    Floor,
    Ceiling,
    Any,
}

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

fn chain(buf: &[u8], start: usize) -> Faces {
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

/// Every valid face from `start` to the next solid's tag, and where that is.
fn span(buf: &[u8], start: usize) -> (Faces, usize) {
    let window_end = buf.len().min(start + ROOM_SOLID_WINDOW);
    let Some(window) = buf.get(start..window_end) else {
        return (Vec::new(), start);
    };
    let end = memchr::memmem::find(window, &ROOM_SOLID_TAG).map_or(window_end, |o| start + o);
    let faces = memchr::memmem::find_iter(&buf[start..end], &FACE_TAG)
        .map(|offset| start + offset)
        .filter_map(|at| face_at(buf, at).map(|face| (at, face)))
        .collect();
    (faces, end)
}

fn kind_text(face: &RoomFace) -> String {
    match face.kind() {
        FaceKind::Horizontal(z) => format!(
            "H z {z:.4} u {:.3?} v {:.3?} origin {:.3?}",
            face.u, face.v, face.origin
        ),
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

fn snap(lines: &mut [[f64; 4]]) {
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

struct Graph {
    points: Vec<(f64, f64)>,
    degree: Vec<usize>,
    edges: Vec<(usize, usize)>,
    collapsed: usize,
}

fn graph(lines: &[[f64; 4]]) -> Graph {
    let mut g = Graph {
        points: Vec::new(),
        degree: Vec::new(),
        edges: Vec::new(),
        collapsed: 0,
    };
    for &[x0, y0, x1, y1] in lines {
        let mut ends = [0usize; 2];
        for (slot, p) in ends.iter_mut().zip([(x0, y0), (x1, y1)]) {
            *slot = match g.points.iter().position(|q| *q == p) {
                Some(index) => index,
                None => {
                    g.points.push(p);
                    g.degree.push(0);
                    g.points.len() - 1
                }
            };
        }
        if ends[0] == ends[1] {
            g.collapsed += 1;
            continue;
        }
        g.degree[ends[0]] += 1;
        g.degree[ends[1]] += 1;
        g.edges.push((ends[0], ends[1]));
    }
    g
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

#[allow(clippy::needless_range_loop)]
fn loops(g: &Graph) -> Vec<Vec<(f64, f64)>> {
    let mut at: Vec<Vec<usize>> = vec![Vec::new(); g.points.len()];
    for (index, &(a, b)) in g.edges.iter().enumerate() {
        at[a].push(index);
        at[b].push(index);
    }
    let mut used = vec![false; g.edges.len()];
    let mut out = Vec::new();
    for first in 0..g.edges.len() {
        if used[first] {
            continue;
        }
        used[first] = true;
        let (start, mut current) = g.edges[first];
        let mut ring = vec![g.points[start]];
        while current != start {
            ring.push(g.points[current]);
            let Some(next) = at[current].iter().copied().find(|&e| !used[e]) else {
                break;
            };
            used[next] = true;
            let (a, b) = g.edges[next];
            current = if a == current { b } else { a };
        }
        out.push(ring);
    }
    out
}

fn ring_area(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

/// The plan edges of `faces` under `anchor`, inside `bbox`, snapped.
fn lines_of(faces: &Faces, bbox: [f64; 6], anchor: Anchor, inside_box: bool) -> Vec<[f64; 4]> {
    let tol = BOX_TOLERANCE_FEET;
    let mut lines = Vec::new();
    for (_, face) in faces {
        let FaceKind::Vertical { high, .. } = face.kind() else {
            continue;
        };
        let Some((edge, bottom)) = face.plan_edge() else {
            continue;
        };
        let reaches = match anchor {
            Anchor::Floor => (bottom - bbox[2]).abs() <= tol,
            Anchor::Ceiling => (high - bbox[5]).abs() <= tol,
            Anchor::Any => true,
        };
        let within = edge[0].min(edge[2]) >= bbox[0] - tol
            && edge[0].max(edge[2]) <= bbox[3] + tol
            && edge[1].min(edge[3]) >= bbox[1] - tol
            && edge[1].max(edge[3]) <= bbox[4] + tol;
        if reaches && (within || !inside_box) {
            lines.push(edge);
        }
    }
    snap(&mut lines);
    lines
}

/// Whether the lines close into an outline on `bbox`, and what stops them.
fn verdict(lines: &[[f64; 4]], bbox: [f64; 6], detail: &mut Vec<String>) -> String {
    let g = graph(lines);
    let open = g.degree.iter().filter(|&&d| d != 2).count();
    let mut histogram: BTreeMap<usize, usize> = BTreeMap::new();
    for &d in &g.degree {
        *histogram.entry(d).or_default() += 1;
    }
    let profile = plan_profile_from_lines(lines);
    let mut text = format!(
        "edges {} collapsed {} degrees {histogram:?} open {open}",
        g.edges.len(),
        g.collapsed
    );
    if open == 0 && !g.edges.is_empty() {
        let rings = loops(&g);
        text.push_str(&format!(" loops {}", rings.len()));
        for (i, one) in rings.iter().enumerate() {
            for (j, other) in rings.iter().enumerate() {
                if i == j {
                    continue;
                }
                let within = one.iter().filter(|p| inside(other, **p)).count();
                let touching = one
                    .iter()
                    .filter(|p| {
                        other.iter().enumerate().any(|(k, &b)| {
                            distance_to_segment(**p, other[(k + 1) % other.len()], b).0 <= 1e-6
                        })
                    })
                    .count();
                if within > 0 && within < one.len() {
                    detail.push(format!(
                        "    loop {i} ({} points, area {:.3}) is partly inside loop {j} ({} points, area {:.3}): {within} points in, {touching} on its boundary",
                        one.len(),
                        ring_area(one),
                        other.len(),
                        ring_area(other)
                    ));
                }
            }
        }
    }
    match profile {
        Some(profile) => {
            let extent = profile.plan_extent_feet();
            let wanted = [bbox[0], bbox[1], bbox[3], bbox[4]];
            let gap = extent.map(|e| {
                [
                    e[0] - wanted[0],
                    e[1] - wanted[1],
                    e[2] - wanted[2],
                    e[3] - wanted[3],
                ]
            });
            let holes: f64 = profile.inner_xy.iter().map(|r| ring_area(r)).sum();
            let on_box = gap.is_some_and(|d| d.iter().all(|v| v.abs() <= BOX_TOLERANCE_FEET));
            text.push_str(&format!(
                " PROFILE area {:.3} extent-minus-box {:.4?} {}",
                ring_area(&profile.outer_xy) - holes,
                gap,
                if on_box { "CLOSES" } else { "extent-differs" }
            ));
            if on_box {
                format!("CLOSES; {text}")
            } else {
                format!("extent-differs; {text}")
            }
        }
        None if open > 0 => format!("open; {text}"),
        None => format!("no-profile; {text}"),
    }
}

fn describe(buf: &[u8], header: usize, bbox: [f64; 6]) -> (Vec<String>, String) {
    let mut out = Vec::new();
    let Some(start) = solid_start(buf, header) else {
        out.push("  solid header unreadable".into());
        return (out, "unreadable".into());
    };
    let walk = chain(buf, start);
    let (in_span, span_end) = span(buf, start);
    let last_at = walk.last().map_or(0, |(at, _)| *at);
    let first_at = walk.first().map_or(0, |(at, _)| *at);
    out.push(format!(
        "  solid header {header} start {start} first face +{} last face +{} next solid tag +{} faces in the chain {} in the span {}",
        first_at.wrapping_sub(start),
        last_at.wrapping_sub(start),
        span_end - start,
        walk.len(),
        in_span.len()
    ));
    if let Some(bytes) = buf.get(start..start + 160) {
        out.push(format!("  header words: {}", hex(bytes)));
    }
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
        if previous.is_some() && ![170, 174, 206].contains(&gap) && index < MAX_FACE_LINES {
            let from = previous.unwrap_or(0) + FACE_LEN;
            let upto = (*at).min(from + 64);
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
    let beyond: Vec<&(usize, RoomFace)> = in_span.iter().filter(|(at, _)| *at > last_at).collect();
    for (at, face) in beyond.iter().take(MAX_ORPHAN_LINES).copied() {
        out.push(format!(
            "  in the span beyond the chain: face @{} ({} past the last) uv {:.4?} {}",
            at - start,
            at - last_at,
            face.uv_box,
            kind_text(face)
        ));
    }
    if beyond.len() > MAX_ORPHAN_LINES {
        out.push(format!(
            "  ... {} faces more in the span",
            beyond.len() - MAX_ORPHAN_LINES
        ));
    }
    let heights: Vec<f64> = {
        let mut seen: Vec<f64> = Vec::new();
        for (_, face) in &in_span {
            if let Some((_, bottom)) = face.plan_edge() {
                if !seen.iter().any(|h| (h - bottom).abs() <= 1e-3) {
                    seen.push(bottom);
                }
            }
        }
        seen.sort_by(f64::total_cmp);
        seen
    };
    out.push(format!(
        "  vertical faces' bottoms in the span: {heights:.4?}, box floor {:.4} ceiling {:.4}",
        bbox[2], bbox[5]
    ));
    let mut detail = Vec::new();
    let chain_floor = {
        let mut lines = Vec::new();
        for (_, face) in &walk {
            if let Some((edge, bottom)) = face.plan_edge() {
                if (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET {
                    lines.push(edge);
                }
            }
        }
        snap(&mut lines);
        lines
    };
    let a = verdict(&chain_floor, bbox, &mut detail);
    let b = verdict(
        &lines_of(&in_span, bbox, Anchor::Floor, true),
        bbox,
        &mut detail,
    );
    let c = verdict(
        &lines_of(&in_span, bbox, Anchor::Ceiling, true),
        bbox,
        &mut detail,
    );
    let d = verdict(
        &lines_of(&in_span, bbox, Anchor::Any, true),
        bbox,
        &mut detail,
    );
    for line in &detail {
        out.push(line.clone());
    }
    let word = |s: &str| s.split(';').next().unwrap_or("").to_string();
    out.push(format!("  variant chain-floor: {a}"));
    out.push(format!("  variant span-floor: {b}"));
    out.push(format!("  variant span-ceiling: {c}"));
    out.push(format!("  variant span-any: {d}"));
    let summary = format!(
        "chain {} | span-floor {} | span-ceiling {} | span-any {}",
        word(&a),
        word(&b),
        word(&c),
        word(&d)
    );
    let _ = MAX_VERTEX_LINES;
    out.push(format!("  SUMMARY {summary}"));
    (out, summary)
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
        let mut first_summary = String::new();
        for (index, &header) in headers.iter().enumerate() {
            let (block, summary) = describe(buf, header, bbox);
            if index == 0 {
                first_summary = summary;
            }
            for line in block {
                println!("{line}");
            }
        }
        *tally.entry(first_summary).or_default() += 1;
    }
    println!(
        "release {version}: {} rooms, {kept} outlines kept, {absent} with no solid in their partition, verdicts of the rest: {tally:#?}",
        rooms.len()
    );
    Ok(())
}
