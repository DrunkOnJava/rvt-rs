//! #1474: every wall's saved mesh, for a census against Revit's own bodies.
//!
//! FACT under test: walls 2056705 and 2056770 of the BIM4LCA office as built
//! save a mesh that is Revit's body, 30.0496 ft3 with its two upper faces
//! (`probe_1474_wall_saved_mesh`), where rvt-rs's body is not. Before any wall
//! is drawn from its saved mesh, this says for every wall of a model what its
//! mesh is: decoded whole or not, how many faces gave no triangles, whether
//! its triangles close (every edge shared by exactly two triangles once
//! vertices 1e-6 ft apart are one), and its volume and box. The probe scorer
//! `wall_mesh_census_vs_ifc` compares each with Revit's body of the same Tag.
//!
//! Output, in feet and the document frame, one line per wall with graphics:
//!   wall <id> status <status> diagnostics <n> unbounded <n> triangles <n>
//!     open_edges <n> volume <ft3> box <x0> <y0> <z0> <x1> <y1> <z1>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1474_wall_mesh_census -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result};
use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;

const DETAIL: i64 = 3;
const WALLS: [&str; 2] = ["SWall", "ArcWall"];

type Key = [i64; 3];

fn key(p: [f64; 3]) -> Key {
    p.map(|v| (v * 1e6).round() as i64)
}

fn main() -> Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .context("usage: probe_1474_wall_mesh_census MODEL.rvt")?;
    let mut rf = RevitFile::open(&path)?;
    let mut walls: BTreeSet<u64> = BTreeSet::new();
    let classes = Options {
        selected_classes: WALLS.iter().map(|name| (*name).to_string()).collect(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &classes, |record| {
        if record
            .class_name
            .as_deref()
            .is_some_and(|class| WALLS.contains(&class))
        {
            walls.insert(record.identity.element_id);
        }
        Ok(())
    })?;
    eprintln!("walls {}", walls.len());
    if walls.is_empty() {
        return Ok(());
    }
    let options = Options {
        selected_ids: walls,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail(&mut rf, &options, DETAIL)?;
    for element in &scene.elements {
        let meshes = &element.meshes;
        let mut volume = 0.0;
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        let mut edges: BTreeMap<(Key, Key), usize> = BTreeMap::new();
        let mut triangles = 0usize;
        for primitive in &meshes.primitives {
            for triangle in &primitive.triangles {
                let corners = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                let [Some(a), Some(b), Some(c)] = corners else {
                    continue;
                };
                triangles += 1;
                volume += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0;
                for p in [a, b, c] {
                    for i in 0..3 {
                        low[i] = low[i].min(p[i]);
                        high[i] = high[i].max(p[i]);
                    }
                }
                let ks = [key(a), key(b), key(c)];
                for (u, v) in [(0, 1), (1, 2), (2, 0)] {
                    if ks[u] == ks[v] {
                        continue;
                    }
                    let edge = if ks[u] < ks[v] {
                        (ks[u], ks[v])
                    } else {
                        (ks[v], ks[u])
                    };
                    *edges.entry(edge).or_insert(0) += 1;
                }
            }
        }
        if triangles == 0 {
            println!(
                "wall {} status {} diagnostics {} unbounded {} triangles 0",
                element.id,
                element.status,
                meshes.diagnostics.len(),
                meshes.unbounded_faces
            );
            continue;
        }
        let open = edges.values().filter(|count| **count != 2).count();
        println!(
            "wall {} status {} diagnostics {} unbounded {} triangles {triangles} open_edges {open} volume {:.4} box {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
            element.id,
            element.status,
            meshes.diagnostics.len(),
            meshes.unbounded_faces,
            volume.abs(),
            low[0],
            low[1],
            low[2],
            high[0],
            high[1],
            high[2]
        );
    }
    Ok(())
}
