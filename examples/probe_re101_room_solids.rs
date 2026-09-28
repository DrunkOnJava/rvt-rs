//! RE-101: a room's partition stores the room's solid as planar faces, and
//! its wall faces close into the room's plan outline.
//!
//! FACT: on `2024_Core_Interior.rvt`, the `u64` ElementId of every exported
//! room is followed, in the partition of the room's own record, by
//! `04 90 08 00` and later by a run of faces, each
//! `04 00 08 00 · … · +20 f64×4 uv box · +85 01 · +86 f64×3 origin ·
//! +110 f64×3 u · +134 f64×3 v`. The vertical faces that reach the room's
//! floor have plan edges that close into
//! the outline Revit's IFC4 export gives the room, for all 116 rooms.
//!
//! The probe reports, for each room rvt-rs exports, how many solids name it
//! in its own partition and elsewhere, how many faces and wall faces the
//! first one has, and whether an outline is kept ([`room_outline`]), with
//! its area against the record box's.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re101_room_solids -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_room_boundaries::{ROOM_SOLID_TAG, room_outline, solid_faces};
use std::collections::BTreeMap;

fn area(ring: &[(f64, f64)]) -> f64 {
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

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let rooms: Vec<_> = rvt::partition_schema_mvp::rooms_from_partition_category_records(
        &mut rf,
        version,
        &Default::default(),
    )?;
    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|name| name.starts_with("Partitions/"))
        .collect();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for room in &rooms {
        let Some(id) = room.id else { continue };
        let field = |wanted: &str| {
            room.fields.iter().find_map(|(name, value)| match value {
                rvt::walker::InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let stream = room
            .fields
            .iter()
            .find_map(|(name, value)| match value {
                rvt::walker::InstanceField::String(s) if name == "m_source_stream" => {
                    Some(s.clone())
                }
                _ => None,
            })
            .unwrap_or_default();
        let (cx, cy, z) = (
            field("m_locationX").unwrap(),
            field("m_locationY").unwrap(),
            field("m_locationZ").unwrap(),
        );
        let (dx, dy, dz) = (
            field("m_bboxWidth").unwrap(),
            field("m_bboxDepth").unwrap(),
            field("m_bboxHeight").unwrap(),
        );
        let bbox = [
            cx - dx / 2.0,
            cy - dy / 2.0,
            z,
            cx + dx / 2.0,
            cy + dy / 2.0,
            z + dz,
        ];
        let mut needle = u64::from(id).to_le_bytes().to_vec();
        needle.extend_from_slice(&ROOM_SOLID_TAG);
        let (mut own, mut elsewhere, mut first) = (0, 0, None);
        for name in &streams {
            let inflated = rf.inflated_partition(name)?;
            for at in memchr::memmem::find_iter(inflated.bytes(), &needle) {
                if *name == stream {
                    own += 1;
                    if first.is_none() {
                        let faces = solid_faces(inflated.bytes(), at);
                        let walls = faces
                            .iter()
                            .filter(|f| {
                                f.plan_edge()
                                    .is_some_and(|(_, bottom)| (bottom - bbox[2]).abs() <= 1e-3)
                            })
                            .count();
                        first = Some((faces.len(), walls));
                    }
                } else {
                    elsewhere += 1;
                }
            }
        }
        let outline = room_outline(&mut rf, &stream, id, bbox)?;
        let verdict = match (&outline, own) {
            (Some(_), _) => "outline kept",
            (None, 0) => "no solid in its partition",
            (None, _) => "solid does not close on the box",
        };
        let detail = if outline.is_none() && own > 0 {
            format!("  box {:.3?}", bbox)
        } else {
            String::new()
        };
        *tally.entry(verdict).or_default() += 1;
        let area_text = outline.as_ref().map_or(String::new(), |profile| {
            let holes: f64 = profile.inner_xy.iter().map(|ring| area(ring)).sum();
            format!(
                "  area {:.6} of box {:.6} ft2",
                area(&profile.outer_xy) - holes,
                dx * dy
            )
        });
        println!(
            "room {id:>8} {stream}: solids {own} here, {elsewhere} elsewhere; first {:?} (faces, wall faces); {verdict}{area_text}{detail}",
            first
        );
    }
    println!("release {version}: {} rooms, {tally:?}", rooms.len());
    Ok(())
}
