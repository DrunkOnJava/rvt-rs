//! B63 follow-up: why an element Revit contains in a space stays in rvt-rs's
//! storey. For each element whose container differs between rvt-rs's export
//! and Revit's (the model's `.ifc` beside it), prints rvt-rs's entity (type,
//! RevitClass, host, storey, location) with every space it was tested
//! against, and Revit's entity with its placement chain.
//!
//! Usage: probe_b63_uncontained <model.rvt>

use rvt::RevitFile;
use rvt::element_record_plan_profiles::inside;
use rvt::ifc::entities::{IfcEntity, ProfileDef, PropertyValue};
use rvt::ifc::{RvtDocExporter, write_step};
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
        let args = args.trim_end().trim_end_matches(';');
        let args = args.strip_suffix(')').unwrap_or(args);
        if let Ok(id) = id.trim().parse() {
            out.insert(id, (entity.trim().to_string(), args.to_string()));
        }
    }
    out
}

fn split_args(args: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let (mut quoted, mut depth) = (false, 0usize);
    for c in args.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                current.push(c);
            }
            '(' if !quoted => {
                depth += 1;
                current.push(c);
            }
            ')' if !quoted => {
                depth = depth.saturating_sub(1);
                current.push(c);
            }
            ',' if !quoted && depth == 0 => out.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    out.push(current);
    out
}

fn references(field: &str) -> Vec<u64> {
    field
        .split(|c: char| !(c == '#' || c.is_ascii_digit()))
        .filter_map(|f| f.strip_prefix('#')?.parse().ok())
        .collect()
}

/// `Tag -> (element id, Some(space name) | None)`.
fn containers(ents: &BTreeMap<u64, (String, String)>) -> BTreeMap<String, (u64, Option<String>)> {
    let mut out = BTreeMap::new();
    for (entity, args) in ents.values() {
        if entity != "IFCRELCONTAINEDINSPATIALSTRUCTURE" {
            continue;
        }
        let f = split_args(args);
        let Some((structure, structure_args)) = f
            .get(5)
            .and_then(|s| references(s).first().copied())
            .and_then(|s| ents.get(&s))
        else {
            continue;
        };
        let space = (structure == "IFCSPACE")
            .then(|| split_args(structure_args).get(2).map(|n| n.trim().to_string()).unwrap_or_default());
        for element in references(f.get(4).map(String::as_str).unwrap_or("")) {
            let Some((_, element_args)) = ents.get(&element) else {
                continue;
            };
            let tag = split_args(element_args)
                .get(7)
                .map(|t| t.trim().trim_matches('\'').to_string())
                .unwrap_or_default();
            if !tag.is_empty() && tag.chars().all(|c| c.is_ascii_digit()) {
                out.insert(tag, (element, space.clone()));
            }
        }
    }
    out
}

fn dump(ents: &BTreeMap<u64, (String, String)>, id: u64, depth: usize, budget: &mut usize) {
    if *budget == 0 {
        return;
    }
    *budget -= 1;
    let Some((entity, args)) = ents.get(&id) else {
        return;
    };
    println!("{}#{id}={entity}({args})", "  ".repeat(depth));
    if depth >= 8 || entity == "IFCDIRECTION" {
        return;
    }
    for child in references(args) {
        dump(ents, child, depth + 1, budget);
    }
}

/// Distance, feet, from `q` to the closed loop `ring`.
fn distance(ring: &[(f64, f64)], q: (f64, f64)) -> f64 {
    let mut best = f64::INFINITY;
    for (i, a) in ring.iter().enumerate() {
        let b = ring[(i + 1) % ring.len()];
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let length = dx * dx + dy * dy;
        let s = if length > 0.0 { (((q.0 - a.0) * dx + (q.1 - a.1) * dy) / length).clamp(0.0, 1.0) } else { 0.0 };
        best = best.min((q.0 - a.0 - s * dx).hypot(q.1 - a.1 - s * dy));
    }
    best
}

/// An element's location, feet, from its `IfcLocalPlacement` chain, adding
/// each placement's point (the RE1 parents are all at the origin, unturned).
fn revit_point_feet(ents: &BTreeMap<u64, (String, String)>, placement: u64) -> Option<[f64; 3]> {
    let mut sum = [0.0; 3];
    let mut next = Some(placement);
    while let Some(id) = next {
        let (entity, args) = ents.get(&id)?;
        if entity != "IFCLOCALPLACEMENT" {
            return None;
        }
        let f = split_args(args);
        let axis = references(f.get(1)?).first().copied()?;
        let point = references(&split_args(&ents.get(&axis)?.1)[0]).first().copied()?;
        let coords = ents.get(&point)?.1.trim().trim_start_matches('(').trim_end_matches(')').to_string();
        for (slot, value) in sum.iter_mut().zip(coords.split(',')) {
            *slot += value.trim().parse::<f64>().ok()? / 304.8;
        }
        next = references(f.first()?).first().copied();
    }
    Some(sum)
}

fn place(location: &[f64; 3], rotation: f64, ring: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let (sin, cos) = rotation.sin_cos();
    ring.iter()
        .map(|(x, y)| (location[0] + x * cos - y * sin, location[1] + x * sin + y * cos))
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let reference = path.with_extension("ifc");
    if !reference.exists() {
        println!("no reference export at {}", reference.display());
        return Ok(());
    }
    let theirs_text = std::fs::read_to_string(&reference)?;
    let theirs_ents = entities(&theirs_text);
    let theirs = containers(&theirs_ents);
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ours_text = write_step(&model);
    let ours_ents = entities(&ours_text);
    let ours = containers(&ours_ents);
    for (tag, (their_id, their_space)) in &theirs {
        let Some((our_id, our_space)) = ours.get(tag) else {
            continue;
        };
        if our_space == their_space {
            continue;
        }
        println!(
            "== {tag}: Revit {:?}, rvt-rs {:?}",
            their_space.as_deref().unwrap_or("storey"),
            our_space.as_deref().unwrap_or("storey")
        );
        println!("-- Revit's element and placement:");
        let (_, args) = &theirs_ents[their_id];
        let f = split_args(args);
        println!("#{their_id}={}({args})", theirs_ents[their_id].0);
        let placement = f.get(5).and_then(|p| references(p).first().copied());
        if let Some(placement) = placement {
            dump(&theirs_ents, placement, 1, &mut 40);
        }
        let revit_point = placement.and_then(|p| revit_point_feet(&theirs_ents, p));
        println!("Revit's location, feet (parents' offsets summed, rotations ignored): {revit_point:?}");
        let ids = tag.parse::<u32>().ok().into_iter().collect::<std::collections::BTreeSet<u32>>();
        let transforms = rvt::partition_instance_transforms::scan_instance_transforms(&mut rf, version, &ids)?;
        let transform = transforms.values().next().copied();
        println!("rvt-rs's instance transform: {transform:?}");
        println!("-- rvt-rs's element line and placement:");
        let (_, args) = &ours_ents[our_id];
        println!("#{our_id}={}({args})", ours_ents[our_id].0);
        if let Some(placement) = split_args(args).get(5).and_then(|p| references(p).first().copied()) {
            dump(&ours_ents, placement, 1, &mut 40);
        }
        println!("-- rvt-rs's model entities with this Tag:");
        for (index, entity) in model.entities.iter().enumerate() {
            let IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid,
                storey_index,
                location_feet,
                rotation_radians,
                host_element_index,
                property_set,
                ..
            } = entity
            else {
                continue;
            };
            if type_guid.as_deref() != Some(tag.as_str()) {
                continue;
            }
            let class = property_set.as_ref().and_then(|set| {
                set.properties.iter().find_map(|p| match (&p.name[..], &p.value) {
                    ("RevitClass", PropertyValue::Text(class)) => Some(class.clone()),
                    _ => None,
                })
            });
            let host = host_element_index.and_then(|h| match model.entities.get(h) {
                Some(IfcEntity::BuildingElement { ifc_type, type_guid, .. }) => {
                    Some(format!("{h} {ifc_type} {type_guid:?}"))
                }
                _ => Some(format!("{h} (not a building element)")),
            });
            println!(
                "entity {index}: {ifc_type} {name:?} class {class:?} storey {storey_index:?} location {location_feet:?} rotation {rotation_radians:?} host {host:?}"
            );
            let Some(location) = location_feet else {
                continue;
            };
            let point = (location[0], location[1]);
            for (space_index, space) in model.entities.iter().enumerate() {
                let IfcEntity::BuildingElement {
                    ifc_type,
                    name,
                    storey_index: space_storey,
                    location_feet: space_location,
                    rotation_radians: space_rotation,
                    extrusion,
                    ..
                } = space
                else {
                    continue;
                };
                if ifc_type != "IFCSPACE" {
                    continue;
                }
                let named = their_space.as_deref().is_some_and(|s| s.contains(name.as_str()));
                let (Some(space_location), Some(extrusion)) = (space_location, extrusion) else {
                    if named {
                        println!("  space {space_index} {name:?} storey {space_storey:?}: no location or extrusion");
                    }
                    continue;
                };
                let rotation = space_rotation.unwrap_or(0.0);
                let (outer, voids) = match &extrusion.profile_override {
                    Some(ProfileDef::ArbitraryClosed { points }) => (place(space_location, rotation, points), Vec::new()),
                    Some(ProfileDef::ArbitraryWithVoids { points, voids }) => (
                        place(space_location, rotation, points),
                        voids.iter().map(|ring| place(space_location, rotation, ring)).collect::<Vec<_>>(),
                    ),
                    None => {
                        let (w, d) = (extrusion.width_feet / 2.0, extrusion.depth_feet / 2.0);
                        (place(space_location, rotation, &[(-w, -d), (w, -d), (w, d), (-w, d)]), Vec::new())
                    }
                    Some(other) => {
                        if named {
                            println!("  space {space_index} {name:?}: profile {other:?}");
                        }
                        continue;
                    }
                };
                let within = inside(&outer, point);
                let in_void = voids.iter().any(|ring| inside(ring, point));
                let origin = transform.map(|t| (t.origin[0], t.origin[1]));
                let revit = revit_point.map(|p| (p[0], p[1]));
                let near = |q: Option<(f64, f64)>| q.map(|q| (inside(&outer, q), distance(&outer, q)));
                if named || within || space_storey == storey_index {
                    println!(
                        "  space {space_index} {name:?} storey {space_storey:?}: box point inside {within} at {:.3} ft, in a void {in_void}; transform origin (inside, ft) {:?}; Revit point {:?}; outline {}",
                        distance(&outer, point),
                        near(origin),
                        near(revit),
                        if named { format!("{outer:?}") } else { format!("{} points", outer.len()) }
                    );
                }
            }
        }
        println!();
    }
    Ok(())
}
