//! RE-374 (#90): a room solid's edges, and the floor outline read from them.
//!
//! FACT under test: a room solid's faces take the indices 4 to 4 + n - 1, n
//! the count of its list of objects, and its edges lie between that list and
//! its first face. Each edge names its two faces at +4 and +8 and gives each
//! face's (u, v) at its ends from +32; where both faces are planes the two
//! give the same points. On a cylinder an edge that runs round it stores
//! values a tenth and a fifth of the face's angle in, not its ends. The floor
//! outline drawn from the edges (`edge_outline_at`) equals the outline the
//! walk of RE-101 or the span of RE-316 reads wherever they read one, and is
//! refused where the solid is not a prism of its floor up to
//! `ROOM_PRISM_HEIGHT_FEET`.
//!
//! Output: one `room` line for each room with a solid. It gives what `main`
//! reads (`walk`, `faces`, `approximate` or `box`), what the edges give (an
//! area, or why not) and whether the two agree. Then `layout` lines tally
//! the checks of the layout over every room's first solid, and a `compare`
//! line tallies the outlines.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re374_room_solid_edges -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::PlanProfile;
use rvt::research::partition_room_boundaries::{
    ResolvedKind, SolidTopology, edge_outline_at, edge_outline_of_solids_at, outline_of_solids,
    resolved_outline_of_solids, solid_headers, solid_topology,
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

/// Tally the layout checks of one solid into `tally`.
fn check_layout(topology: &SolidTopology, tally: &mut BTreeMap<&'static str, usize>) {
    let face = |index: u32| topology.faces.get(&index).and_then(Option::as_ref);
    *tally.entry("faces").or_default() += topology.faces.len();
    *tally.entry("faces read").or_default() +=
        topology.faces.values().filter(|f| f.is_some()).count();
    *tally.entry("edges").or_default() += topology.edges.len();
    for edge in &topology.edges {
        let (a, b) = (face(edge.faces[0]), face(edge.faces[1]));
        let planes = a
            .zip(b)
            .filter(|(a, b)| a.cylinder.is_none() && b.cylinder.is_none());
        if let Some((a, b)) = planes {
            let far = |p: [f64; 3], q: [f64; 3]| (0..3).any(|i| (p[i] - q[i]).abs() > 1e-3);
            let differ = far(a.plane_point(edge.uv[0]), b.plane_point(edge.uv[1]))
                || far(a.plane_point(edge.uv[2]), b.plane_point(edge.uv[3]));
            *tally
                .entry(if differ {
                    "plane pairs DIFFER"
                } else {
                    "plane pairs agree"
                })
                .or_default() += 1;
        }
        for (side, read) in [(0, a), (1, b)] {
            let Some(f) = read.filter(|f| f.cylinder.is_some()) else {
                continue;
            };
            let (lo, hi) = (f.uv_box[0], f.uv_box[2]);
            let span = hi - lo;
            let (u0, u1) = (edge.uv[side][0], edge.uv[side + 2][0]);
            let at = |x: f64, y: f64| (x - y).abs() <= 1e-6;
            let class = if at(u0, u1) && (at(u0, lo) || at(u0, hi)) {
                "cylinder edges along the axis, at an end of the angle"
            } else if at(u0, lo + 0.1 * span) && at(u1, lo + 0.2 * span) {
                "cylinder edges round it, a tenth and a fifth in from the start"
            } else if at(u0, hi - 0.1 * span) && at(u1, hi - 0.2 * span) {
                "cylinder edges round it, a tenth and a fifth in from the end"
            } else {
                "cylinder edges OTHER"
            };
            *tally.entry(class).or_default() += 1;
        }
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
    let mut layout: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut compare: BTreeMap<String, usize> = BTreeMap::new();
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
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let headers = solid_headers(buf, id);
        let Some(&first) = headers.first() else {
            continue;
        };
        match solid_topology(buf, first) {
            Some(topology) => {
                *layout.entry("solids read").or_default() += 1;
                check_layout(&topology, &mut layout);
            }
            None => *layout.entry("solids not read").or_default() += 1,
        }
        let walked = outline_of_solids(buf, &headers, bbox);
        let resolved = walked
            .is_none()
            .then(|| resolved_outline_of_solids(buf, &headers, bbox))
            .flatten();
        let (main_kind, main_profile) = match (&walked, &resolved) {
            (Some(p), _) => ("walk", Some(p)),
            (None, Some((p, ResolvedKind::Faces))) => ("faces", Some(p)),
            (None, Some((p, ResolvedKind::Approximate))) => ("approximate", Some(p)),
            (None, None) => ("box", None),
        };
        let edges = edge_outline_of_solids_at(buf, &headers, bbox, false).map(|(p, _)| p);
        let why = match edge_outline_at(buf, first, bbox) {
            Ok(_) => "read",
            Err(reason) => reason,
        };
        let verdict = match (main_profile, &edges) {
            (Some(m), Some(e)) => {
                let same_extent = match (m.plan_extent_feet(), e.plan_extent_feet()) {
                    (Some(a), Some(b)) => (0..4).all(|i| (a[i] - b[i]).abs() <= 1e-3),
                    _ => false,
                };
                if (area(m) - area(e)).abs() <= 0.01 && same_extent {
                    "agree"
                } else {
                    "DIFFER"
                }
            }
            (Some(_), None) => "edges refused",
            (None, Some(_)) => "edges add",
            (None, None) => "neither",
        };
        *compare
            .entry(format!("{main_kind}: {verdict}"))
            .or_default() += 1;
        println!(
            "room {id} {stream} box {:.3} main {main_kind} {} edges {} ({why}) {verdict}",
            dx * dy,
            main_profile.map_or_else(|| "none".to_string(), |p| format!("{:.3}", area(p))),
            edges
                .as_ref()
                .map_or_else(|| "none".to_string(), |p| format!("{:.3}", area(p))),
        );
    }
    for (name, count) in &layout {
        println!("layout {version}: {name} {count}");
    }
    println!("compare {version}: {} rooms, {compare:?}", rooms.len());
    Ok(())
}
