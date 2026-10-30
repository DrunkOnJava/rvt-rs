//! RE-296 (#1016, W7): which rooms are right prisms, and so have a volume
//! their stored solid's faces give.
//!
//! FACT under test: a room whose stored solid (RE-101) has exactly two
//! horizontal planes, the room's floor and ceiling, and whose every other
//! face is vertical and runs the whole way between them, is a prism of its
//! plan outline: Revit's `Qto_SpaceBaseQuantities.NetVolume` is that outline's
//! area, its holes taken off, times the height between the two planes.
//!
//! Output, one line per room whose own partition holds a solid:
//! `room <ElementId> <Name> hdr <n> box <z0> <z1> outline <net area> <outer area>
//! <holes>` and, below it, one `face` line per face of the first solid:
//! `face <ElementId> H <z> <u extent> <v extent>`, `face <ElementId> V <x0> <y0>
//! <x1> <y1> <z low> <z high>` or `face <ElementId> O <nx> <ny> <nz> <z>`, with
//! `Name` the `IfcSpace` name rvt-rs gives the room (its number, else
//! `<name>:<ElementId>`), which keys a space in the quantity census.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re296_space_prisms -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::export_content::{ROOM_NAME_PROPERTY, ROOM_NUMBER_PROPERTY};
use rvt::research::partition_room_boundaries::{room_outline, solid_faces, solid_headers};
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

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let names: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                property_set,
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCSPACE") => {
                let text = |key: &str| {
                    property_set
                        .as_ref()?
                        .properties
                        .iter()
                        .find_map(|p| match &p.value {
                            PropertyValue::Text(text)
                                if p.name == key && !text.trim().is_empty() =>
                            {
                                Some(text.clone())
                            }
                            _ => None,
                        })
                };
                let written = text(ROOM_NUMBER_PROPERTY)
                    .or_else(|| text(ROOM_NAME_PROPERTY).map(|room| format!("{room}:{tag}")))
                    .unwrap_or_else(|| name.clone());
                Some((tag.parse().ok()?, written))
            }
            _ => None,
        })
        .collect();
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
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let headers = solid_headers(inflated.bytes(), id);
        let Some(&first) = headers.first() else {
            continue;
        };
        let faces = solid_faces(inflated.bytes(), first);
        let name = names
            .get(&u64::from(id))
            .cloned()
            .unwrap_or_else(|| "-".into());
        let outline = room_outline(&mut rf, &stream, id, bbox)?;
        let outline_text = outline.as_ref().map_or("none".to_string(), |profile| {
            let holes: f64 = profile.inner_xy.iter().map(|ring| ring_area(ring)).sum();
            let outer = ring_area(&profile.outer_xy);
            format!("{} {outer} {}", outer - holes, profile.inner_xy.len())
        });
        println!(
            "room {id} {} hdr {} box {} {} outline {outline_text}",
            name.replace(' ', "_"),
            headers.len(),
            bbox[2],
            bbox[5]
        );
        for face in &faces {
            let [u_min, v_min, u_max, v_max] = face.uv_box;
            let normal = [
                face.u[1] * face.v[2] - face.u[2] * face.v[1],
                face.u[2] * face.v[0] - face.u[0] * face.v[2],
                face.u[0] * face.v[1] - face.u[1] * face.v[0],
            ];
            let heights: Vec<f64> = [
                (u_min, v_min),
                (u_min, v_max),
                (u_max, v_min),
                (u_max, v_max),
            ]
            .iter()
            .map(|(u, v)| face.origin[2] + face.u[2] * u + face.v[2] * v)
            .collect();
            let low = heights.iter().copied().fold(f64::INFINITY, f64::min);
            let high = heights.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            if (normal[2].abs() - 1.0).abs() < 1e-9 {
                println!(
                    "face {id} H {} {} {}",
                    face.origin[2],
                    u_max - u_min,
                    v_max - v_min
                );
            } else if normal[2].abs() < 1e-9 {
                let edge = face.plan_edge().map_or("- - - -".to_string(), |(e, _)| {
                    format!("{} {} {} {}", e[0], e[1], e[2], e[3])
                });
                println!("face {id} V {edge} {low} {high}");
            } else {
                println!(
                    "face {id} O {} {} {} {}",
                    normal[0], normal[1], normal[2], face.origin[2]
                );
            }
        }
    }
    Ok(())
}
