//! #1474: the saved graphics of walls 2056705 and 2056770 on the BIM4LCA
//! office as built.
//!
//! FACT to find: Revit's export ends 2056705 at 179.298 ft over a 0.1601 ft2
//! strip beside wall 1493966's face, the bottom of wall 3490093, which no join
//! list, cut pair or clean pair of either wall names (`probe_1474_cut_pairs`),
//! and at 183.071 ft over 0.0943 ft2. Whether the mesh Revit saved for the wall
//! holds that step says whether the wall's own saved body is Revit's there.
//!
//! Output, in feet and the document frame, as `probe_1385_wall_saved_mesh`:
//!
//!   saved tag:<id> status <status> primitives <n> triangles <n> unbounded <n>
//!   saved tag:<id> volume <ft3> box <z low> <z high> <x min> <x max> <y min> <y max>
//!   saved tag:<id> face <up|down> z <z> area <ft2> box <x min> <x max> <y min> <y max>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1474_wall_saved_mesh -- MODEL.rvt

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;
use std::collections::{BTreeMap, BTreeSet};

const DETAIL: i64 = 3;
const HORIZONTAL: f64 = 0.999;

type Vec3 = [f64; 3];

/// Per level of horizontal faces: area and the box x min, x max, y min, y max.
type Levels = BTreeMap<(i64, u8), [f64; 5]>;

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

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1474_wall_saved_mesh MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let hosts: BTreeSet<u64> = [2056705u64, 2056770].into_iter().collect();
    let options = Options {
        selected_ids: hosts,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &options, DETAIL)?;
    for element in &scene.elements {
        let id = element.id;
        let meshes = &element.meshes;
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        println!(
            "saved tag:{id} status {} primitives {} triangles {triangles} unbounded {}",
            element.status,
            meshes.primitives.len(),
            meshes.unbounded_faces
        );
        for diagnostic in &meshes.diagnostics {
            let text: String = diagnostic.chars().take(200).collect();
            println!("saved tag:{id} diagnostic {text}");
        }
        let mut volume = 0.0;
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        let mut levels: Levels = BTreeMap::new();
        for primitive in &meshes.primitives {
            let oriented = primitive.normals.len() == primitive.vertices.len();
            for triangle in &primitive.triangles {
                let corners = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                let [Some(a), Some(mut b), Some(mut c)] = corners else {
                    continue;
                };
                let mut normal = cross(sub(b, a), sub(c, a));
                let length = dot(normal, normal).sqrt();
                if length < 1e-12 {
                    continue;
                }
                if oriented {
                    let mut outward = [0.0; 3];
                    for i in triangle {
                        let n = primitive.normals[*i as usize];
                        outward = [outward[0] + n[0], outward[1] + n[1], outward[2] + n[2]];
                    }
                    if dot(normal, outward) < 0.0 {
                        std::mem::swap(&mut b, &mut c);
                        normal = [-normal[0], -normal[1], -normal[2]];
                    }
                }
                volume += dot(a, cross(b, c)) / 6.0;
                for p in [a, b, c] {
                    for ((l, h), v) in low.iter_mut().zip(high.iter_mut()).zip(p) {
                        *l = l.min(v);
                        *h = h.max(v);
                    }
                }
                if normal[2].abs() / length < HORIZONTAL {
                    continue;
                }
                let key = ((a[2] * 1000.0).round() as i64, u8::from(normal[2] > 0.0));
                let entry = levels.entry(key).or_insert([
                    0.0,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                ]);
                entry[0] += length / 2.0;
                for p in [a, b, c] {
                    entry[1] = entry[1].min(p[0]);
                    entry[2] = entry[2].max(p[0]);
                    entry[3] = entry[3].min(p[1]);
                    entry[4] = entry[4].max(p[1]);
                }
            }
        }
        if triangles == 0 || !volume.is_finite() {
            continue;
        }
        println!(
            "saved tag:{id} volume {:.4} box {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            volume.abs(),
            low[2],
            high[2],
            low[0],
            high[0],
            low[1],
            high[1]
        );
        for ((z, up), [area, x0, x1, y0, y1]) in &levels {
            let direction = if *up == 1 { "up" } else { "down" };
            println!(
                "saved tag:{id} face {direction} z {:.3} area {area:.4} box {x0:.4} {x1:.4} {y0:.4} {y1:.4}",
                *z as f64 / 1000.0
            );
        }
    }
    Ok(())
}
