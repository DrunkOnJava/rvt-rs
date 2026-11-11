//! #90 (RE-410): the three weld passes of RvtIO's room-boundary reader, tried on the
//! edges of a room's stored solid.
//!
//! FACT under test: RvtIO (db6c5053, discussion 1241) reads a stored room shell's
//! boundary points with no welding, then welds points whose x, y and z each differ by
//! no more than 0.0005233832795 ft, then by no more than 1/32 in (0.79375 mm),
//! merging transitively; a segment whose two ends weld to one point is dropped, a loop
//! with fewer than three distinct points is a sliver and dropped, and the shell must
//! then close. A room solid's edges (RE-374) name, for each face, the points its edge
//! ends have on that face: a plane's `origin + u * U + v * V`, a cylinder's along its
//! axis. This welds those points at each of the three tolerances and says, for each
//! solid, whether every face's boundary then closes (each point at an even degree), and
//! which pass is the first to close it. An edge round a cylinder stores two points that
//! are not its ends (RE-374), on the cylinder and on the plane across it alike, so both
//! its faces are not placed: a solid with such a face is `closed where placed` at best,
//! and so is one whose edge has no face that was read.
//!
//! Output, one line for each solid of each room, then the tallies:
//!   weld room <id> <stream> main <walk|faces|approximate|box> solid <k> faces <n> read <r>
//!   cylinders <c> edges <m> unplaced faces <x> | <pass> <closed|closed where placed|open>
//!   split <s> collapsed <c> slivers <f> open [<face indices>] | ... | first <pass|never>
//!   weld tally <release>: <what> <count>
//! `main` is what the walk of RE-101 and the span of RE-316 read, not the edges of RE-374.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_90_room_weld_passes -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_room_boundaries::{
    ResolvedKind, RoomFace, SolidTopology, outline_of_solids, resolved_outline_of_solids,
    solid_headers, solid_topology,
};
use rvt::walker::InstanceField;

/// RvtIO's three passes: the name and the tolerance, feet.
const PASSES: [(&str, f64); 3] = [
    ("none", 0.0),
    ("0.000523ft", 0.0005233832795),
    ("1/32in", 1.0 / 32.0 / 12.0),
];

/// The point of a cylindrical face at (angle, distance along its axis).
fn cylinder_point(face: &RoomFace, uv: [f64; 2]) -> Option<[f64; 3]> {
    let cylinder = face.cylinder?;
    let (sin, cos) = uv[0].sin_cos();
    Some([0, 1, 2].map(|i| {
        face.origin[i]
            + cylinder.radius * (cos * face.u[i] + sin * face.v[i])
            + uv[1] * cylinder.axis[i]
    }))
}

/// The weld of `points`: the cluster each belongs to, where two points are one
/// cluster if each of their coordinates differs by at most `tolerance`, directly or
/// through other points.
fn weld(points: &[[f64; 3]], tolerance: f64) -> Vec<usize> {
    fn root(parent: &mut [usize], mut at: usize) -> usize {
        while parent[at] != at {
            parent[at] = parent[parent[at]];
            at = parent[at];
        }
        at
    }
    let mut parent: Vec<usize> = (0..points.len()).collect();
    let mut order: Vec<usize> = (0..points.len()).collect();
    order.sort_by(|&a, &b| points[a][0].total_cmp(&points[b][0]));
    for (position, &i) in order.iter().enumerate() {
        for &j in &order[position + 1..] {
            if points[j][0] - points[i][0] > tolerance {
                break;
            }
            let near = points[i]
                .iter()
                .zip(&points[j])
                .all(|(p, q)| (p - q).abs() <= tolerance);
            if near {
                let (a, b) = (root(&mut parent, i), root(&mut parent, j));
                if a != b {
                    parent[b] = a;
                }
            }
        }
    }
    (0..points.len()).map(|i| root(&mut parent, i)).collect()
}

/// The ends of every edge of one solid, as the faces place them.
struct Boundary {
    /// Every point an edge places on a face, model feet.
    points: Vec<[f64; 3]>,
    /// For each face, the two point indices of each of its edges.
    segments: BTreeMap<u32, Vec<[usize; 2]>>,
    /// For each edge that both its faces place, the two ends as each face places them.
    twice: Vec<[[usize; 2]; 2]>,
    /// The faces with an edge that neither of its faces places.
    unplaced: BTreeSet<u32>,
}

fn place_edges(topology: &SolidTopology) -> Boundary {
    let mut points: Vec<[f64; 3]> = Vec::new();
    let mut segments: BTreeMap<u32, Vec<[usize; 2]>> = BTreeMap::new();
    let mut twice = Vec::new();
    let mut unplaced = BTreeSet::new();
    for edge in &topology.edges {
        // An edge round a cylinder stores two points that are not its ends (RE-374),
        // on the cylinder and on the plane across it alike.
        let round = (0..2).any(|side| {
            topology
                .faces
                .get(&edge.faces[side])
                .and_then(Option::as_ref)
                .is_some_and(|face| {
                    face.cylinder.is_some()
                        && (edge.uv[side][0] - edge.uv[side + 2][0]).abs() > 1e-9
                })
        });
        if round {
            unplaced.extend(edge.faces);
            continue;
        }
        let mut placed: [Option<[usize; 2]>; 2] = [None, None];
        for (side, slot) in placed.iter_mut().enumerate() {
            let Some(face) = topology
                .faces
                .get(&edge.faces[side])
                .and_then(Option::as_ref)
            else {
                continue;
            };
            let ends = match face.cylinder {
                None => Some([
                    face.plane_point(edge.uv[side]),
                    face.plane_point(edge.uv[side + 2]),
                ]),
                // An edge along the axis stores one angle at both points.
                Some(_) if (edge.uv[side][0] - edge.uv[side + 2][0]).abs() <= 1e-9 => {
                    cylinder_point(face, edge.uv[side])
                        .zip(cylinder_point(face, edge.uv[side + 2]))
                        .map(|(a, b)| [a, b])
                }
                Some(_) => None,
            };
            if let Some([a, b]) = ends {
                points.push(a);
                points.push(b);
                *slot = Some([points.len() - 2, points.len() - 1]);
            }
        }
        if let [Some(a), Some(b)] = placed {
            twice.push([a, b]);
        }
        for (side, face) in edge.faces.iter().enumerate() {
            match placed[side].or(placed[1 - side]) {
                Some(ends) => segments.entry(*face).or_default().push(ends),
                None => {
                    unplaced.insert(*face);
                }
            }
        }
    }
    Boundary {
        points,
        segments,
        twice,
        unplaced,
    }
}

/// What a pass makes of a solid.
struct Verdict {
    /// Edges both faces place whose ends the weld does not take as one point each.
    split: usize,
    /// Segments dropped because both their ends welded to one point.
    collapsed: usize,
    /// Faces dropped for fewer than three distinct points left.
    slivers: usize,
    /// Faces, placed whole, whose boundary has a point of odd degree.
    open: Vec<u32>,
}

fn assess(boundary: &Boundary, tolerance: f64) -> Verdict {
    let cluster = weld(&boundary.points, tolerance);
    let split = boundary
        .twice
        .iter()
        .filter(|[a, b]| cluster[a[0]] != cluster[b[0]] || cluster[a[1]] != cluster[b[1]])
        .count();
    let mut collapsed = 0;
    let mut slivers = 0;
    let mut open = Vec::new();
    for (face, segments) in &boundary.segments {
        if boundary.unplaced.contains(face) {
            continue;
        }
        let mut degree: BTreeMap<usize, usize> = BTreeMap::new();
        for &[a, b] in segments {
            let (p, q) = (cluster[a], cluster[b]);
            if p == q {
                collapsed += 1;
                continue;
            }
            *degree.entry(p).or_default() += 1;
            *degree.entry(q).or_default() += 1;
        }
        if degree.len() < 3 {
            slivers += 1;
        } else if degree.values().any(|&d| d & 1 == 1) {
            open.push(*face);
        }
    }
    Verdict {
        split,
        collapsed,
        slivers,
        open,
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
    let mut tally: BTreeMap<String, usize> = BTreeMap::new();
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
        if headers.is_empty() {
            continue;
        }
        let walked = outline_of_solids(buf, &headers, bbox);
        let resolved = walked
            .is_none()
            .then(|| resolved_outline_of_solids(buf, &headers, bbox))
            .flatten();
        let main_kind = match (&walked, &resolved) {
            (Some(_), _) => "walk",
            (None, Some((_, ResolvedKind::Faces))) => "faces",
            (None, Some((_, ResolvedKind::Approximate))) => "approximate",
            (None, None) => "box",
        };
        for (k, header) in headers.iter().enumerate() {
            let Some(topology) = solid_topology(buf, *header) else {
                println!("weld room {id} {stream} main {main_kind} solid {k} no topology");
                *tally
                    .entry(format!("main {main_kind}: no topology"))
                    .or_default() += 1;
                continue;
            };
            let read = topology.faces.values().flatten().count();
            let cylinders = topology
                .faces
                .values()
                .flatten()
                .filter(|face| face.cylinder.is_some())
                .count();
            let placed = place_edges(&topology);
            let mut cells = Vec::new();
            let mut first = "never";
            for (name, tolerance) in PASSES {
                let verdict = assess(&placed, tolerance);
                let kind = if !verdict.open.is_empty() {
                    "open"
                } else if !placed.unplaced.is_empty() {
                    "closed where placed"
                } else {
                    "closed"
                };
                if kind != "open" && first == "never" {
                    first = name;
                }
                cells.push(format!(
                    "{name} {kind} split {} collapsed {} slivers {} open {:?}",
                    verdict.split, verdict.collapsed, verdict.slivers, verdict.open
                ));
                *tally
                    .entry(format!("main {main_kind}, {name}: {kind}"))
                    .or_default() += 1;
            }
            *tally
                .entry(format!("main {main_kind}, first closing pass: {first}"))
                .or_default() += 1;
            println!(
                "weld room {id} {stream} main {main_kind} solid {k} faces {} read {read} cylinders {cylinders} edges {} unplaced faces {} | {} | first {first}",
                topology.faces.len(),
                topology.edges.len(),
                placed.unplaced.len(),
                cells.join(" | "),
            );
        }
    }
    for (what, count) in &tally {
        println!("weld tally {version}: {what} {count}");
    }
    Ok(())
}
