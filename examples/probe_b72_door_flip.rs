//! B72 (probe): a door's or window's type GlobalId in Revit's export, the
//! MD5 of `<original symbol's GlobalId>Sub-element:Flipped: <bool>
//! InAssembly: False` (revit-ifc `GUIDUtil.CreateInternal`). For each door and
//! window of Revit's export beside the model, prints which flip reproduces it
//! and the flip revit-ifc's rule (`DoorWindowInfo.CalculateDoorWindowInformation`)
//! gives from rvt-rs's model: `PosHingeSide` is whether the door's box centre
//! lies on the +Y side of its host wall's axis (Y = Z × the wall's direction),
//! and the door is flipped when that differs from whether its own Y axis
//! (its transform's, RE-87) points to that side.
//!
//! Usage: probe_b72_door_flip <model.rvt>

use md5::{Digest, Md5};
use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::partition_room_parameters::data_objects;
use rvt::revit_global_ids::{canonical_guid, compress_ifc_guid, revit_global_ids};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn entities(step: &str) -> BTreeMap<u64, (String, String)> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(rest) = line.strip_prefix('#') else {
            continue;
        };
        let Some((id, body)) = rest.split_once('=') else {
            continue;
        };
        let Some((entity, args)) = body.split_once('(') else {
            continue;
        };
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn refs(args: &str) -> Vec<u64> {
    args.split('#')
        .skip(1)
        .filter_map(|s| {
            s.chars()
                .take_while(char::is_ascii_digit)
                .collect::<String>()
                .parse()
                .ok()
        })
        .collect()
}

fn hashed(key: &str) -> String {
    let digest = Md5::digest(key.as_bytes());
    compress_ifc_guid(canonical_guid(
        digest.as_slice().try_into().expect("16 bytes"),
    ))
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export");
        return Ok(());
    }
    let ents = entities(&std::fs::read_to_string(&reference)?);
    // Each door's and window's Tag -> its type's GlobalId.
    let mut revit: BTreeMap<u32, String> = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELDEFINESBYTYPE" {
            continue;
        }
        let all = refs(args);
        let Some((&ty, related)) = all.split_last() else {
            continue;
        };
        let Some((ty_entity, ty_args)) = ents.get(&ty) else {
            continue;
        };
        if ![
            "IFCDOORSTYLE",
            "IFCDOORTYPE",
            "IFCWINDOWSTYLE",
            "IFCWINDOWTYPE",
        ]
        .contains(&ty_entity.as_str())
        {
            continue;
        }
        let Some(gid) = ty_args.split('\'').nth(1) else {
            continue;
        };
        for element in related.iter().skip(1) {
            if let Some(tag) = ents
                .get(element)
                .and_then(|(_, a)| a.rsplit('\'').nth(1)?.parse().ok())
            {
                revit.insert(tag, gid.to_string());
            }
        }
    }
    if revit.is_empty() {
        println!("no door or window types");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let ids = revit_global_ids(&mut rf)?;
    let classes = rf.schema_classes()?;
    let element_tag = classes
        .classes
        .iter()
        .find(|c| c.name == "GElement")
        .map(|c| u32::from(c.tag));
    let doors: BTreeSet<u32> = revit.keys().copied().collect();
    let mut originals: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (_, object) in data_objects(buf) {
            if Some(object.class & 0xffff) == element_tag && doors.contains(&object.element_id) {
                let at = object.end - 20;
                originals
                    .entry(object.element_id)
                    .or_default()
                    .insert(u32::from_le_bytes(
                        buf[at..at + 4].try_into().expect("4 bytes"),
                    ));
            }
        }
    }
    let transforms =
        rvt::partition_instance_transforms::scan_instance_transforms(&mut rf, version, &doors)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    for (tag, gid) in &revit {
        let original = originals
            .get(tag)
            .and_then(|v| (v.len() == 1).then(|| *v.iter().next().expect("one")));
        let actual = original.and_then(|o| ids.get(&o)).and_then(|symbol| {
            [false, true].into_iter().find(|flip| {
                hashed(&format!(
                    "{symbol}Sub-element:Flipped: {} InAssembly: False",
                    if *flip { "True" } else { "False" }
                )) == *gid
            })
        });
        // rvt-rs's model: the door, its host wall, and its transform.
        let door = model.entities.iter().find_map(|e| match e {
            IfcEntity::BuildingElement {
                type_guid: Some(t),
                location_feet: Some(location),
                host_element_index,
                ..
            } if t == &tag.to_string() => Some((*location, *host_element_index)),
            _ => None,
        });
        let predicted = door.and_then(|(location, host)| {
            let host = host?;
            let IfcEntity::BuildingElement {
                location_feet: Some(wall),
                rotation_radians,
                ..
            } = model.entities.get(host)?
            else {
                return None;
            };
            let r = rotation_radians.unwrap_or(0.0);
            let wall_y = [-r.sin(), r.cos()];
            let offset = (location[0] - wall[0]) * wall_y[0] + (location[1] - wall[1]) * wall_y[1];
            let pos_hinge_side = offset > -1e-9;
            let door_y = transforms.get(tag)?.axes[1];
            let facing = wall_y[0] * door_y[0] + wall_y[1] * door_y[1] > -1e-9;
            Some((pos_hinge_side != facing, offset, facing))
        });
        println!(
            "{tag}: original {original:?}, Revit's flip {actual:?}, rule from rvt-rs {predicted:?}, host {:?}",
            door.map(|(_, h)| h)
        );
    }
    Ok(())
}
