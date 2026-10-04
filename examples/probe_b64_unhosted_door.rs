//! B64: the door rvt-rs writes with no host wall. For each door rvt-rs's
//! export holds with no host, prints rvt-rs's entity (name, RevitClass,
//! location, rotation) and, from Revit's export beside the model, the door's
//! line and every relationship that names it (aggregation, filling, voiding,
//! containment), with the other side's entity.
//!
//! Usage: probe_b64_unhosted_door <model.rvt>

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use std::collections::BTreeMap;
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

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export");
        return Ok(());
    }
    let ents = entities(&std::fs::read_to_string(&reference)?);
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid,
            location_feet,
            rotation_radians,
            host_element_index: None,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        if ifc_type != "IFCDOOR" {
            continue;
        }
        let class = property_set.as_ref().and_then(|set| {
            set.properties
                .iter()
                .find_map(|p| match (&p.name[..], &p.value) {
                    ("RevitClass", PropertyValue::Text(class)) => Some(class.clone()),
                    _ => None,
                })
        });
        println!(
            "== rvt-rs: {name:?} tag {type_guid:?} class {class:?} location {location_feet:?} rotation {rotation_radians:?}"
        );
        let Some(tag) = type_guid else {
            continue;
        };
        let quoted = format!("'{tag}'");
        let Some((&door, (door_entity, door_args))) = ents.iter().find(|(_, (e, a))| {
            e.starts_with("IFCDOOR")
                && (a.contains(&format!(",{quoted},"))
                    || a.trim_end_matches([')', ';', ' ']).ends_with(&quoted))
        }) else {
            println!("   not in Revit's export");
            continue;
        };
        println!("   Revit: #{door}={door_entity}({door_args}");
        for (id, (entity, args)) in &ents {
            if !entity.starts_with("IFCREL") || !refs(args).contains(&door) {
                continue;
            }
            println!("   #{id}={entity}({args}");
            for other in refs(args) {
                if other == door {
                    continue;
                }
                if let Some((e, a)) = ents.get(&other) {
                    if !e.starts_with("IFCOWNERHISTORY") {
                        println!(
                            "      #{other}={e}({})",
                            a.chars().take(200).collect::<String>()
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
