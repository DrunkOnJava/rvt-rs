//! RE-316: the B-rep a room's native record holds, and the plan edges at its
//! floor.
//!
//! FACT under test: a room whose partition faces do not close into an outline
//! on its record box (RE-101) holds, in its native record, the same solid as
//! `Face`, `EdgeLoop` and `Edge` objects, and the edges that lie in the floor's
//! plane, with their ends evaluated on the plane of a face they border, close
//! into the outline.
//!
//! Output, per room that does not close: the classes the record holds, the
//! first `Face`, `Edge`, `EdgeLoop` and `Plane`, how many faces each surface
//! class has, and the plan edges at the box's floor with whether
//! `plan_profile_from_lines` closes them on the box.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re316_room_native_faces -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::plan_profile_from_lines;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_saved_mesh::pointer;
use rvt::research::partition_room_boundaries::{BOX_TOLERANCE_FEET, room_outline};
use rvt::walker::InstanceField;
use serde_json::Value;

const FLOOR_TOLERANCE_FEET: f64 = 1e-3;

fn short(value: &Value) -> String {
    value.to_string().chars().take(1400).collect()
}

fn triple(value: &Value) -> Option<[f64; 3]> {
    let a = value.as_array()?;
    Some([
        a.first()?.as_f64()?,
        a.get(1)?.as_f64()?,
        a.get(2)?.as_f64()?,
    ])
}

fn pair(value: &Value) -> Option<[f64; 2]> {
    let a = value.as_array()?;
    Some([a.first()?.as_f64()?, a.get(1)?.as_f64()?])
}

/// The plane `face` lies on: origin, u direction, v direction.
fn plane_of(g: &ObjectGraph, face: usize) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
    let surface = pointer(g, face, &g.objects.get(face)?.fields["m_pSurf"])
        .ok()
        .flatten()?;
    let plane = g.objects.get(surface)?;
    if plane.class_name != "Plane" {
        return None;
    }
    Some((
        triple(&plane.fields["m_origin"])?,
        triple(&plane.fields["m_xVec"])?,
        triple(&plane.fields["m_yVec"])?,
    ))
}

/// The edge's points on the plane of the face on `side`, model feet.
fn edge_points(g: &ObjectGraph, edge: usize, side: usize) -> Option<Vec<[f64; 3]>> {
    let object = g.objects.get(edge)?;
    let faces = object.fields["m_pFace"].as_array()?;
    let face = pointer(g, edge, faces.get(side)?).ok().flatten()?;
    let (origin, x, y) = plane_of(g, face)?;
    let ends = object.fields["m_firstAndLastEdgePnts"].as_array()?;
    if ends.len() != 2 {
        return None;
    }
    let mut uvs = vec![pair(&ends[0]["uv"][side])?];
    for sample in object.fields["m_interiorEdgePnts"].as_array()? {
        uvs.push(pair(&sample["uv"][side])?);
    }
    uvs.push(pair(&ends[1]["uv"][side])?);
    Some(
        uvs.iter()
            .map(|uv| std::array::from_fn(|k| origin[k] + uv[0] * x[k] + uv[1] * y[k]))
            .collect(),
    )
}

fn describe(g: &ObjectGraph, id: u64, bbox: [f64; 6]) {
    let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
    for object in &g.objects {
        *classes.entry(object.class_name.as_str()).or_default() += 1;
    }
    println!(
        "  native room {id}: objects {} classes {classes:?}",
        g.objects.len()
    );
    for wanted in ["Face", "Edge", "EdgeLoop", "Plane", "RoomElem"] {
        if let Some(object) = g.objects.iter().find(|o| o.class_name == wanted) {
            println!("  first {wanted}: {}", short(&object.fields));
        }
    }
    let mut surfaces: BTreeMap<String, usize> = BTreeMap::new();
    let mut horizontal_planes: Vec<String> = Vec::new();
    for (index, object) in g.objects.iter().enumerate() {
        if object.class_name != "Face" {
            continue;
        }
        let kind = pointer(g, index, &object.fields["m_pSurf"])
            .ok()
            .flatten()
            .and_then(|s| g.objects.get(s))
            .map_or("none".to_string(), |s| s.class_name.clone());
        *surfaces.entry(kind).or_default() += 1;
        if let Some((origin, x, y)) = plane_of(g, index) {
            let nz = x[0] * y[1] - x[1] * y[0];
            if nz.abs() > 0.5 {
                horizontal_planes.push(format!("z {:.4} nz {nz:.3}", origin[2]));
            }
        }
    }
    println!("  faces by surface class {surfaces:?}; horizontal planes {horizontal_planes:?}");
    let floor = bbox[2];
    let mut lines: Vec<[f64; 4]> = Vec::new();
    let (mut edges, mut at_floor, mut unreadable) = (0, 0, 0);
    for (index, object) in g.objects.iter().enumerate() {
        if object.class_name != "Edge" {
            continue;
        }
        edges += 1;
        let Some(points) = (0..2).find_map(|side| edge_points(g, index, side)) else {
            unreadable += 1;
            continue;
        };
        if points
            .iter()
            .all(|p| (p[2] - floor).abs() <= FLOOR_TOLERANCE_FEET)
        {
            at_floor += 1;
            for w in points.windows(2) {
                lines.push([w[0][0], w[0][1], w[1][0], w[1][1]]);
            }
        }
    }
    let mut pieces = lines.clone();
    pieces.retain(|l| (l[0] - l[2]).abs() > 1e-9 || (l[1] - l[3]).abs() > 1e-9);
    println!(
        "  edges {edges}, at the floor {at_floor}, unreadable {unreadable}, plan segments {} ({} not degenerate)",
        lines.len(),
        pieces.len()
    );
    match plan_profile_from_lines(&pieces) {
        Some(profile) => {
            let extent = profile.plan_extent_feet();
            let wanted = [bbox[0], bbox[1], bbox[3], bbox[4]];
            let gap = extent.map(|e| std::array::from_fn::<f64, 4, _>(|k| e[k] - wanted[k]));
            let on_box = gap.is_some_and(|d| d.iter().all(|v| v.abs() <= BOX_TOLERANCE_FEET));
            let ring = |r: &[(f64, f64)]| {
                let n = r.len();
                (0..n)
                    .map(|i| {
                        let (a, b) = (r[i], r[(i + 1) % n]);
                        a.0 * b.1 - b.0 * a.1
                    })
                    .sum::<f64>()
                    .abs()
                    / 2.0
            };
            let holes: f64 = profile.inner_xy.iter().map(|r| ring(r)).sum();
            println!(
                "  FLOOR-EDGES {}: area {:.3} (box {:.3}) holes {} pieces {} extent-minus-box {gap:.4?}",
                if on_box {
                    "CLOSE ON THE BOX"
                } else {
                    "close, extent differs"
                },
                ring(&profile.outer_xy) - holes,
                (bbox[3] - bbox[0]) * (bbox[4] - bbox[1]),
                profile.inner_xy.len(),
                profile.pieces.len() + 1
            );
        }
        None => println!("  FLOOR-EDGES do not close"),
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
    let mut failing: BTreeMap<u64, [f64; 6]> = BTreeMap::new();
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
        if room_outline(&mut rf, &stream, id, bbox)?.is_none() {
            failing.insert(u64::from(id), bbox);
        }
    }
    println!(
        "release {version}: {} rooms, {} do not close from their partition faces",
        rooms.len(),
        failing.len()
    );
    let options = Options {
        selected_ids: failing.keys().copied().collect::<BTreeSet<u64>>(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    let mut seen = 0;
    extract_graphs(&mut rf, &options, |record| {
        let id = record.identity.element_id;
        let Some(&bbox) = failing.get(&id) else {
            return Ok(());
        };
        seen += 1;
        println!(
            "room {id} stream {} class {:?} status {} box {:.3?}",
            record.source.stream, record.class_name, record.status, bbox
        );
        match record.graph.as_ref() {
            Some(g) => describe(g, id, bbox),
            None => println!("  no graph: {:?}", record.diagnostic),
        }
        Ok(())
    })?;
    println!("native records read for {seen} of {} rooms", failing.len());
    Ok(())
}
