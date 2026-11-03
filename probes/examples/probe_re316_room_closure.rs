//! RE-316: which rooms whose stored solid did not close now give an outline,
//! and whether the file agrees.
//!
//! FACT under test: where the faces RE-101 walks do not close on a room's
//! record box, the faces up to the next solid's tag, each plan edge directed
//! so the room is on its left, wind around a region whose boundary is the room's
//! outline (`partition_room_boundaries::region_boundary`).
//!
//! For each room whose own partition holds a solid, one line: the room, the
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
    BOX_TOLERANCE_FEET, OutlineAnchor, ROOM_VERTEX_TOLERANCE_FEET, room_outline_at, solid_faces,
    solid_headers, span_outline_at,
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
    let (mut resolved_same, mut resolved_total) = (0, 0);
    let mut cached_new = (0, 0);
    let mut cached_ceiling = (0, 0);
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
        let now = room_outline_at(buf, header, bbox);
        let ceiling = span_outline_at(buf, header, bbox, OutlineAnchor::Ceiling);
        let class = match (&walked, &now) {
            (Some(_), Some(_)) => {
                if walked == now {
                    "kept, unchanged"
                } else {
                    "kept, CHANGED"
                }
            }
            (None, Some(_)) => "newly closed",
            (Some(_), None) => "LOST",
            (None, None) => "still box",
        };
        *tally.entry(class).or_default() += 1;
        if walked.is_some() {
            let resolved = span_outline_at(buf, header, bbox, OutlineAnchor::Floor);
            resolved_total += 1;
            let same = match (&walked, &resolved) {
                (Some(a), Some(b)) => (area(a) - area(b)).abs() <= 1e-6 * area(a),
                _ => false,
            };
            if same {
                resolved_same += 1;
            } else {
                println!(
                    "room {id} {stream}: RE-101 outline {} but the resolved span gives {}",
                    walked.as_ref().map_or(0.0, area),
                    resolved.as_ref().map_or(f64::NAN, area)
                );
            }
        }
        if walked.is_some() && now.is_some() && walked == now {
            continue;
        }
        let cache = cached_areas(buf);
        let box_area = dx * dy;
        if walked.is_none() && now.is_some() {
            cached_new.1 += 1;
            if now.as_ref().is_some_and(|p| in_cache(&cache, area(p))) {
                cached_new.0 += 1;
            }
        }
        if walked.is_none() && now.is_none() && ceiling.is_some() {
            cached_ceiling.1 += 1;
            if ceiling.as_ref().is_some_and(|p| in_cache(&cache, area(p))) {
                cached_ceiling.0 += 1;
            }
        }
        println!(
            "room {id} {stream} box {box_area:.3} ({}) walked {} now {} ceiling {} [{class}]",
            if in_cache(&cache, box_area) {
                "cached"
            } else {
                "not cached"
            },
            text(&walked, &cache),
            text(&now, &cache),
            text(&ceiling, &cache),
        );
    }
    println!("release {version}: {} rooms, {tally:?}", rooms.len());
    println!(
        "plan edges directed head to tail on {oriented_ok} of the {oriented_total} rooms RE-101 kept"
    );
    println!(
        "the resolved span's outline has the area of the walk's on {resolved_same} of the {resolved_total} rooms RE-101 kept"
    );
    println!(
        "newly closed rooms whose area Revit's cache holds: {} of {}; ceiling-anchored outlines of rooms still boxes whose area it holds: {} of {}",
        cached_new.0, cached_new.1, cached_ceiling.0, cached_ceiling.1
    );
    Ok(())
}
