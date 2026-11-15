//! #1209: the horizontal faces of every ceiling's saved mesh, to read a cut
//! ceiling's area where its saved parameters hold none.
//!
//! FACT under test: Revit's export gives a ceiling Revit cuts a `GrossArea`
//! 0.7 to 1.1% below its sketch's (RE-297): its computed area, which revit-ifc
//! falls back to when the body is not one extrusion, and which no saved
//! parameter holds (`probe_1209_ceiling_params`, negative). If the mesh Revit
//! saved for a cut ceiling holds the cut body, its upper or lower face is that
//! area. The probe scorer `ceiling_mesh_areas_vs_ifc` compares it with Revit's
//! `GrossArea`, for cut and uncut ceilings apart.
//!
//! Output, in feet and the document frame:
//!   ceiling <id> cut <true|false> status <status> diagnostics <n> unbounded <n> triangles <n>
//!   face <id> <up|down> z <z> area <ft2>
//! one `face` line for each height at which horizontal faces of one direction
//! lie (0.001 ft), the direction from the triangles' winding made outward by
//! the saved normals.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1209_ceiling_mesh_faces -- MODEL.rvt

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

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1209_ceiling_mesh_faces MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: BTreeSet::from(["Ceiling".to_string()]),
        ..Options::default()
    };
    let mut cut: BTreeMap<u64, bool> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let pair = graph
            .objects
            .iter()
            .any(|object| object.class_name == "CutPairGStep");
        cut.insert(record.identity.element_id, pair);
        Ok(())
    })?;
    eprintln!("ceilings {}", cut.len());
    if cut.is_empty() {
        return Ok(());
    }
    let scene_options = Options {
        selected_ids: cut.keys().copied().collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &scene_options, DETAIL)?;
    for element in &scene.elements {
        let id = element.id;
        let meshes = &element.meshes;
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        println!(
            "ceiling {id} cut {} status {} diagnostics {} unbounded {} triangles {triangles}",
            cut.get(&id).copied().unwrap_or(false),
            element.status,
            meshes.diagnostics.len(),
            meshes.unbounded_faces
        );
        let mut levels: BTreeMap<(i64, u8), f64> = BTreeMap::new();
        for primitive in &meshes.primitives {
            let oriented = primitive.normals.len() == primitive.vertices.len();
            for triangle in &primitive.triangles {
                let corners = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                let [Some(a), Some(b), Some(c)] = corners else {
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
                        normal = [-normal[0], -normal[1], -normal[2]];
                    }
                }
                if normal[2].abs() / length < HORIZONTAL {
                    continue;
                }
                let key = ((a[2] * 1000.0).round() as i64, u8::from(normal[2] > 0.0));
                *levels.entry(key).or_insert(0.0) += length / 2.0;
            }
        }
        for ((z, up), area) in &levels {
            let direction = if *up == 1 { "up" } else { "down" };
            println!(
                "face {id} {direction} z {:.3} area {area:.4}",
                *z as f64 / 1000.0
            );
        }
    }
    Ok(())
}
