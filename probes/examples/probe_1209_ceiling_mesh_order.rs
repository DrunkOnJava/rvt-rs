//! #1209: the pieces of each ceiling's saved mesh in the order the file
//! stores them, to read which piece Revit's export gives `GrossArea`.
//!
//! FACT under test: on a ceiling in pieces, Revit's `GrossArea` is the area of
//! the piece whose extrusion its export creates last (revit-ifc 24.2's
//! `BodyExporter.ExportBody` sets `ScaledArea` from each extrusion in turn;
//! `ceiling_piece_order_vs_ifc` finds it on 78 of the office's 78 ceilings in
//! pieces). If the saved mesh stores the pieces in the order the export makes
//! them, the last piece the mesh stores is Revit's.
//!
//! Output, in feet and the document frame, for each ceiling whose mesh has
//! more than one connected part:
//!   ceiling <id> primitives <n> parts <n>
//!   part <id> primitive <i> first <triangle> z <low> <high> down <ft2> up <ft2>
//! one `part` line per connected part of a primitive (triangles sharing a
//! vertex 1e-6 ft apart), in primitive order and, within one, by the first
//! triangle it holds; `down` and `up` are the areas of its horizontal
//! triangles facing down and up (normal within 0.999 of vertical, outward by
//! the saved normals).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1209_ceiling_mesh_order -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;

const DETAIL: i64 = 3;
const HORIZONTAL: f64 = 0.999;

type Vec3 = [f64; 3];

fn sub(a: Vec3, b: Vec3) -> Vec3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

/// One connected part of a primitive.
struct Part {
    first: usize,
    low: f64,
    high: f64,
    down: f64,
    up: f64,
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1209_ceiling_mesh_order MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let mut ceilings: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_classes: BTreeSet::from(["Ceiling".to_string()]),
            ..Options::default()
        },
        |record| {
            ceilings.insert(record.identity.element_id);
            Ok(())
        },
    )?;
    eprintln!("ceilings {}", ceilings.len());
    if ceilings.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: ceilings,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &options, DETAIL)?;
    let key = |p: Vec3| p.map(|v| (v * 1e6).round() as i64);
    for element in &scene.elements {
        let mut parts: Vec<(usize, Part)> = Vec::new();
        for (index, primitive) in element.meshes.primitives.iter().enumerate() {
            // Each triangle with its index in the primitive, for its normals.
            let triangles: Vec<(usize, [Vec3; 3])> = primitive
                .triangles
                .iter()
                .enumerate()
                .filter_map(|(raw, t)| {
                    let [Some(a), Some(b), Some(c)] =
                        t.map(|i| primitive.vertices.get(i as usize).copied())
                    else {
                        return None;
                    };
                    Some((raw, [a, b, c]))
                })
                .collect();
            let oriented = primitive.normals.len() == primitive.vertices.len();
            let mut parent: Vec<usize> = (0..triangles.len()).collect();
            let mut seen: BTreeMap<[i64; 3], usize> = BTreeMap::new();
            for (t, (_, corners)) in triangles.iter().enumerate() {
                for corner in corners {
                    match seen.get(&key(*corner)) {
                        Some(&other) => {
                            let (a, b) = (root(&mut parent, t), root(&mut parent, other));
                            if a != b {
                                parent[a.max(b)] = a.min(b);
                            }
                        }
                        None => {
                            seen.insert(key(*corner), t);
                        }
                    }
                }
            }
            let mut by_root: BTreeMap<usize, Part> = BTreeMap::new();
            for (t, (raw, [a, b, c])) in triangles.iter().enumerate() {
                let r = root(&mut parent, t);
                let part = by_root.entry(r).or_insert(Part {
                    first: *raw,
                    low: f64::INFINITY,
                    high: f64::NEG_INFINITY,
                    down: 0.0,
                    up: 0.0,
                });
                part.first = part.first.min(*raw);
                for z in [a[2], b[2], c[2]] {
                    part.low = part.low.min(z);
                    part.high = part.high.max(z);
                }
                let mut normal = cross(sub(*b, *a), sub(*c, *a));
                let length = dot(normal, normal).sqrt();
                if length < 1e-12 {
                    continue;
                }
                if oriented {
                    let mut outward = [0.0; 3];
                    for i in &primitive.triangles[*raw] {
                        let n = primitive.normals[*i as usize];
                        outward = [outward[0] + n[0], outward[1] + n[1], outward[2] + n[2]];
                    }
                    if dot(normal, outward) < 0.0 {
                        normal = [-normal[0], -normal[1], -normal[2]];
                    }
                }
                if normal[2].abs() / length < HORIZONTAL {
                    continue;
                }
                if normal[2] > 0.0 {
                    part.up += length / 2.0;
                } else {
                    part.down += length / 2.0;
                }
            }
            let mut ordered: Vec<Part> = by_root.into_values().collect();
            ordered.sort_by_key(|p| p.first);
            parts.extend(ordered.into_iter().map(|p| (index, p)));
        }
        if parts.len() < 2 {
            continue;
        }
        let id = element.id;
        println!(
            "ceiling {id} primitives {} parts {}",
            element.meshes.primitives.len(),
            parts.len()
        );
        for (index, part) in &parts {
            println!(
                "part {id} primitive {index} first {} z {:.4} {:.4} down {:.4} up {:.4}",
                part.first, part.low, part.high, part.down, part.up
            );
        }
    }
    Ok(())
}
