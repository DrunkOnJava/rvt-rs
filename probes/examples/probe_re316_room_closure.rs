//! RE-316: which rooms whose stored solid did not close now give an outline,
//! and whether the file agrees.
//!
//! FACT under test: where the faces RE-101 walks do not close on a room's
//! record box, the faces up to the next solid's tag, each plan edge directed
//! so the room is on its left, wind around a region whose boundary is the room's
//! outline (`partition_room_boundaries::region_boundary`).
//!
//! For each room whose own partition holds a solid and is not left exactly as
//! RE-101 reads it, one line: the room, the
//! outline RE-101 gave (area, or none), the outline now given, the outline of
//! the walls that reach the ceiling instead, the box's area, and for each area
//! whether Revit's cache of computed room areas in the same partition holds it
//! (`u64 · ff×8 · u32 0 · i32 · f64`, RE-101 §4). The last lines count the
//! rooms by what changed, and whether the plan edges of every room RE-101 kept
//! are directed head to tail.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_room_closure -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::{PlanProfile, plan_profile_from_lines};
use rvt::research::partition_room_boundaries::{
    BOX_TOLERANCE_FEET, FaceKind, OutlineAnchor, ROOM_STEP_TOLERANCE_FEET,
    ROOM_VERTEX_TOLERANCE_FEET, ResolvedKind, SpanRead, outline_of_solids, region_boundary,
    resolved_outline_of_solids, solid_faces, solid_headers, solid_span, span_outline_at,
};
use rvt::walker::InstanceField;

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

fn area(profile: &PlanProfile) -> f64 {
    let net = |outer: &[(f64, f64)], inner: &[Vec<(f64, f64)>]| {
        ring_area(outer) - inner.iter().map(|r| ring_area(r)).sum::<f64>()
    };
    net(&profile.outer_xy, &profile.inner_xy)
        + profile
            .pieces
            .iter()
            .map(|p| net(&p.outer_xy, &p.inner_xy))
            .sum::<f64>()
}

/// How many pairs of `lines` cross at a point that is an end of neither, and
/// how many overlap along a line for more than 1e-5 ft.
fn crossing_pairs(lines: &[[f64; 4]]) -> (usize, usize) {
    let (mut crossings, mut overlaps) = (0, 0);
    for (index, a) in lines.iter().enumerate() {
        for b in &lines[index + 1..] {
            let (ax, ay) = (a[2] - a[0], a[3] - a[1]);
            let (bx, by) = (b[2] - b[0], b[3] - b[1]);
            let (alen, blen) = (ax.hypot(ay), bx.hypot(by));
            if alen < 1e-6 || blen < 1e-6 {
                continue;
            }
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let cross = ax * by - ay * bx;
            if cross.abs() > 1e-9 * alen * blen {
                let t = (dx * by - dy * bx) / cross;
                let u = (dx * ay - dy * ax) / cross;
                let (te, ue) = (1e-6 / alen, 1e-6 / blen);
                if t > te && t < 1.0 - te && u > ue && u < 1.0 - ue {
                    crossings += 1;
                }
            } else if ((dx * ay - dy * ax) / alen).abs() <= 1e-5 {
                let along = |x: f64, y: f64| ((x - a[0]) * ax + (y - a[1]) * ay) / alen;
                let (s0, s1) = (along(b[0], b[1]), along(b[2], b[3]));
                let (lo, hi) = (s0.min(s1).max(0.0), s0.max(s1).min(alen));
                if hi - lo > 1e-5 {
                    overlaps += 1;
                }
            }
        }
    }
    (crossings, overlaps)
}

fn snap(lines: &mut [[f64; 4]]) {
    snap_within(lines, ROOM_VERTEX_TOLERANCE_FEET);
}

fn snap_within(lines: &mut [[f64; 4]], tolerance: f64) {
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

/// The areas Revit caches in a partition.
fn cached_areas(buf: &[u8]) -> Vec<f64> {
    let pattern = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0];
    memchr::memmem::find_iter(buf, &pattern)
        .filter_map(|at| {
            let bytes: [u8; 8] = buf.get(at + 16..at + 24)?.try_into().ok()?;
            let value = f64::from_le_bytes(bytes);
            (value.is_finite() && value > 0.0 && value < 1.0e7).then_some(value)
        })
        .collect()
}

fn in_cache(cache: &[f64], value: f64) -> bool {
    cache.iter().any(|c| (c - value).abs() <= 1e-5 * value)
}

fn text(profile: &Option<PlanProfile>, cache: &[f64]) -> String {
    match profile {
        Some(p) => {
            let a = area(p);
            format!(
                "{a:.3} ({})",
                if in_cache(cache, a) {
                    "cached"
                } else {
                    "not cached"
                }
            )
        }
        None => "none".into(),
    }
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
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let (mut oriented_ok, mut oriented_total) = (0, 0);
    let mut cached_new = (0, 0);
    let mut cached_slack = (0, 0);
    let mut kept_cache = (0, 0);
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
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        let headers = solid_headers(buf, id);
        let Some(&header) = headers.first() else {
            continue;
        };
        let mut lines: Vec<[f64; 4]> = solid_faces(buf, header)
            .iter()
            .filter_map(|f| f.plan_edge())
            .filter(|(_, bottom)| (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET)
            .map(|(edge, _)| edge)
            .collect();
        snap(&mut lines);
        let walked = plan_profile_from_lines(&lines).filter(|p| {
            p.plan_extent_feet().is_some_and(|e| {
                e.iter()
                    .zip([bbox[0], bbox[1], bbox[3], bbox[4]])
                    .all(|(got, want)| (got - want).abs() <= BOX_TOLERANCE_FEET)
            })
        });
        if walked.is_some() {
            oriented_total += 1;
            let mut balance: Vec<((i64, i64), i32)> = Vec::new();
            let key = |x: f64, y: f64| ((x * 1e4).round() as i64, (y * 1e4).round() as i64);
            for l in &lines {
                for (point, step) in [(key(l[0], l[1]), 1), (key(l[2], l[3]), -1)] {
                    match balance.iter_mut().find(|(k, _)| *k == point) {
                        Some(entry) => entry.1 += step,
                        None => balance.push((point, step)),
                    }
                }
            }
            if balance.iter().all(|(_, b)| *b == 0) {
                oriented_ok += 1;
            }
        }
        let walk_all = outline_of_solids(buf, &headers, bbox);
        let resolved = if walk_all.is_none() {
            resolved_outline_of_solids(buf, &headers, bbox)
        } else {
            None
        };
        let now = walk_all
            .clone()
            .or_else(|| resolved.as_ref().map(|(profile, _)| profile.clone()));
        let exact = SpanRead {
            arcs: false,
            slack: 0.0,
        };
        let ceiling = span_outline_at(buf, header, bbox, OutlineAnchor::Ceiling, exact);
        let with_arcs = span_outline_at(
            buf,
            header,
            bbox,
            OutlineAnchor::Floor,
            SpanRead {
                arcs: true,
                slack: 0.0,
            },
        );
        let class = match (&walk_all, &resolved) {
            (Some(_), _) => "kept",
            (None, Some((_, ResolvedKind::Faces))) => "faces",
            (None, Some((_, ResolvedKind::Approximate))) => "approximate",
            (None, None) => "still box",
        };
        *tally.entry(class).or_default() += 1;
        if resolved.is_some() {
            let mut edges: Vec<[f64; 4]> = solid_span(buf, header)
                .iter()
                .filter_map(|f| f.plan_edge())
                .filter(|(_, bottom)| (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET)
                .map(|(edge, _)| edge)
                .collect();
            snap_within(&mut edges, ROOM_STEP_TOLERANCE_FEET);
            let (crossings, overlaps) = crossing_pairs(&edges);
            let shape = match plan_profile_from_lines(&edges) {
                Some(p) => format!(
                    "loops close, {} holes, {} further pieces",
                    p.inner_xy.len(),
                    p.pieces.len()
                ),
                None => "loops do not close as the walk reads them".to_string(),
            };
            println!(
                "room {id}: {class}; {} edges, {shape}, {crossings} crossing pairs, {overlaps} overlapping pairs",
                edges.len()
            );
        }
        let cache = cached_areas(buf);
        if let Some(profile) = walk_all
            .as_ref()
            .filter(|p| (area(p) - dx * dy).abs() > 1e-3 * dx * dy || p.outer_xy.len() > 4)
        {
            kept_cache.1 += 1;
            if in_cache(&cache, area(profile)) {
                kept_cache.0 += 1;
            }
        }
        if walked.is_some() && walk_all.is_some() {
            continue;
        }
        let box_area = dx * dy;
        if let Some((profile, kind)) = resolved.as_ref() {
            let slot = if *kind == ResolvedKind::Faces {
                &mut cached_new
            } else {
                &mut cached_slack
            };
            slot.1 += 1;
            if in_cache(&cache, area(profile)) {
                slot.0 += 1;
            }
        }
        if walk_all.is_none() && resolved.is_none() {
            let faces = solid_span(buf, header);
            let tolerance = BOX_TOLERANCE_FEET;
            let strays = faces
                .iter()
                .filter(|f| match f.kind() {
                    FaceKind::Vertical { low, high } => {
                        low < bbox[2] - tolerance
                            || high > bbox[5] + tolerance
                            || f.plan_edge().is_some_and(|(e, _)| {
                                e[0].min(e[2]) < bbox[0] - tolerance
                                    || e[0].max(e[2]) > bbox[3] + tolerance
                                    || e[1].min(e[3]) < bbox[1] - tolerance
                                    || e[1].max(e[3]) > bbox[4] + tolerance
                            })
                    }
                    _ => false,
                })
                .count();
            let cylinders = faces.iter().filter(|f| f.cylinder.is_some()).count();
            let mut floor_lines: Vec<[f64; 4]> = faces
                .iter()
                .filter_map(|f| f.plan_edge())
                .filter(|(_, bottom)| (bottom - bbox[2]).abs() <= BOX_TOLERANCE_FEET)
                .map(|(edge, _)| edge)
                .collect();
            snap_within(&mut floor_lines, ROOM_STEP_TOLERANCE_FEET);
            let why = match region_boundary(&floor_lines) {
                Ok(pieces) => match plan_profile_from_lines(&pieces) {
                    Some(profile) => format!(
                        "boundary of {} pieces, profile area {:.3}, extent minus box {:.4?}",
                        pieces.len(),
                        area(&profile),
                        profile.plan_extent_feet().map(|e| [
                            e[0] - bbox[0],
                            e[1] - bbox[1],
                            e[2] - bbox[3],
                            e[3] - bbox[4]
                        ])
                    ),
                    None => format!("boundary of {} pieces, no profile", pieces.len()),
                },
                Err(failure) => format!("{failure:?}"),
            };
            println!(
                "room {id}: span faces {} (stray {strays}, cylinders {cylinders}), floor edges {}; {}",
                faces.len(),
                floor_lines.len(),
                why.chars().take(600).collect::<String>()
            );
        }
        println!(
            "room {id} {stream} box {box_area:.3} ({}) walked {} now {} ceiling {} with arcs {} [{class}]",
            if in_cache(&cache, box_area) {
                "cached"
            } else {
                "not cached"
            },
            text(&walked, &cache),
            text(&now, &cache),
            text(&ceiling, &cache),
            text(&with_arcs, &cache),
        );
    }
    println!("release {version}: {} rooms, {tally:?}", rooms.len());
    println!(
        "plan edges directed head to tail on {oriented_ok} of the {oriented_total} rooms RE-101 kept"
    );
    println!(
        "Revit's cached room areas (u64 · ff x8 · u32 0 · i32 · f64, RE-101 section 4) hold the outline's area on {} of {} non-rectangular rooms RE-101 kept (the baseline), on {} of {} exact span outlines and on {} of {} approximate ones",
        kept_cache.0, kept_cache.1, cached_new.0, cached_new.1, cached_slack.0, cached_slack.1
    );
    Ok(())
}
