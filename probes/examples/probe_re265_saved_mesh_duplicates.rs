//! RE-265 (#1103): do the saved meshes whose surfaces coincide with Revit's
//! bodies but enclose a quarter more volume repeat a shell?
//!
//! On the BIM4LCA office four doors drawn from their saved mesh (1382481,
//! 1623922, 3448563, 3444654) have every vertex within 2e-06 ft of Revit's
//! body, both ways, and 1.25 times its volume. Two coincident copies of one
//! shell, or a shell with its faces reversed, would do that.
//!
//! For every element `saved_meshes::attach` would draw from its saved mesh,
//! the probe keys each primitive by its triangles (vertex coordinates rounded
//! to 1e-6 ft, each triangle turned to start at its least vertex) and each
//! triangle the same way, and counts primitives that repeat another, and
//! triangles that repeat one of the same element with the same or the
//! opposite winding. It prints one line per element with any repeat, with
//! the repeated primitives' object index, face tag and owner, then the
//! totals.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re265_saved_mesh_duplicates -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;

const RECORD_BOX: &str = "partition_element_record_bbox";

type Point = [i64; 3];

fn point(p: [f64; 3]) -> Point {
    p.map(|c| (c * 1e6).round() as i64)
}

/// The triangle turned to start at its least vertex, keeping its winding.
fn turned(t: [Point; 3]) -> [Point; 3] {
    let first = (0..3).min_by_key(|&i| t[i]).unwrap_or(0);
    [t[first], t[(first + 1) % 3], t[(first + 2) % 3]]
}

fn signed_volume(vertices: &[[f64; 3]], triangles: &[[u32; 3]]) -> f64 {
    triangles
        .iter()
        .filter_map(|t| {
            let [a, b, c] = t.map(|i| vertices.get(i as usize).copied());
            let (a, b, c) = (a?, b?, c?);
            let cross = [
                b[1] * c[2] - b[2] * c[1],
                b[2] * c[0] - b[0] * c[2],
                b[0] * c[1] - b[1] * c[0],
            ];
            Some((a[0] * cross[0] + a[1] * cross[1] + a[2] * cross[2]) / 6.0)
        })
        .sum()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut tags: BTreeMap<u64, (usize, String)> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        let boxed = set.properties.iter().any(|p| {
            p.name == "BodySource" && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BOX)
        });
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        if !boxed || resolved {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            let entry = tags.entry(id).or_insert((0, ifc_type.clone()));
            entry.0 += 1;
        }
    }
    let single: BTreeSet<u64> = tags
        .iter()
        .filter(|(_, (n, _))| *n == 1)
        .map(|(id, _)| *id)
        .collect();
    let options = Options {
        selected_ids: single.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let (mut scanned, mut with_shell, mut with_same, mut with_reversed) = (0, 0, 0, 0);
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !single.contains(&id) || element.meshes.primitives.is_empty() {
            continue;
        }
        scanned += 1;
        let primitives = &element.meshes.primitives;
        let mut shells: BTreeMap<Vec<[Point; 3]>, Vec<usize>> = BTreeMap::new();
        let mut triangles: BTreeMap<[Point; 3], usize> = BTreeMap::new();
        for (index, primitive) in primitives.iter().enumerate() {
            let mut keys: Vec<[Point; 3]> = primitive
                .triangles
                .iter()
                .filter_map(|t| {
                    let [a, b, c] = t.map(|i| primitive.vertices.get(i as usize).copied());
                    Some(turned([point(a?), point(b?), point(c?)]))
                })
                .collect();
            for key in &keys {
                *triangles.entry(*key).or_default() += 1;
            }
            keys.sort();
            shells.entry(keys).or_default().push(index);
        }
        let repeated_shells: Vec<&Vec<usize>> = shells
            .values()
            .filter(|indices| indices.len() > 1)
            .collect();
        let same: usize = triangles.values().map(|n| n - 1).sum();
        let reversed = triangles
            .keys()
            .filter(|t| {
                let back = turned([t[0], t[2], t[1]]);
                back != **t && triangles.contains_key(&back)
            })
            .count()
            / 2;
        if repeated_shells.is_empty() && same == 0 && reversed == 0 {
            continue;
        }
        with_shell += usize::from(!repeated_shells.is_empty());
        with_same += usize::from(same > 0);
        with_reversed += usize::from(reversed > 0);
        let volume: f64 = primitives
            .iter()
            .map(|p| signed_volume(&p.vertices, &p.triangles))
            .sum();
        let class = tags.get(&id).map_or("-", |(_, class)| class.as_str());
        println!(
            "repeat {id} {class} primitives {} repeated_shells {} same_triangles {same} reversed_pairs {reversed} signed_volume {volume:.6}",
            primitives.len(),
            repeated_shells.len()
        );
        for indices in repeated_shells {
            let parts: Vec<String> = indices
                .iter()
                .map(|&i| {
                    let p = &primitives[i];
                    format!(
                        "object {} face {} owner {:?} triangles {} volume {:.6}",
                        p.object_index,
                        p.face_tag,
                        p.source_owner_id,
                        p.triangles.len(),
                        signed_volume(&p.vertices, &p.triangles)
                    )
                })
                .collect();
            println!("  shell {}", parts.join(" = "));
        }
    }
    println!(
        "{scanned} saved meshes scanned; {with_shell} repeat a primitive, {with_same} repeat a triangle, {with_reversed} hold a triangle and its reverse"
    );
    Ok(())
}
