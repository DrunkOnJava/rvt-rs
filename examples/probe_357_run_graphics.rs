//! #357: which of a stair run's own boundary solids is the flight Revit's
//! export draws.
//!
//! FACT under test: a Revit 2025 `StairsRun` record holds two `Geometry`
//! objects of planar faces (66 faces on `13_Rooms_Stairs`), and meshing all
//! of them sums to 20 times Revit's flight volume (probe_357_run_faces), so
//! they are more than the flight. This runs the saved-graphics selection
//! (`native_saved_mesh::graphics`, the fine detail) on each run's and
//! landing's own graph and prints every primitive's vertices with the
//! `Geometry` object it comes from, for `tools/re/run_graphics_vs_ifc.py` to
//! match against Revit's body of the same flight.
//!
//! Output:
//!   graphics <id> <class> primitives <n> unbounded <n> diagnostics <json>
//!   primitive <id> <object index> <triangles> <signed volume ft3>
//!   vertex <id> <object index> <x> <y> <z>
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_run_graphics -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::graphics;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: ["StairsRun", "StairsLanding"]
            .iter()
            .map(|c| c.to_string())
            .collect::<BTreeSet<_>>(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let meshes = graphics(graph);
        println!(
            "graphics {id} {class} primitives {} unbounded {} diagnostics {}",
            meshes.primitives.len(),
            meshes.unbounded_faces,
            serde_json::to_string(&meshes.diagnostics)?
        );
        let mut by_object: BTreeMap<usize, BTreeSet<[i64; 3]>> = BTreeMap::new();
        for p in &meshes.primitives {
            let mut volume = 0.0;
            for t in &p.triangles {
                let [a, b, c] = t.map(|i| p.vertices[i as usize]);
                volume += (a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0;
            }
            println!(
                "primitive {id} {} {} {volume:.6}",
                p.object_index,
                p.triangles.len()
            );
            let set = by_object.entry(p.object_index).or_default();
            for v in &p.vertices {
                set.insert(v.map(|c| (c * 1e6).round() as i64));
            }
        }
        for (object, set) in by_object {
            for v in set {
                println!(
                    "vertex {id} {object} {} {} {}",
                    v[0] as f64 / 1e6,
                    v[1] as f64 / 1e6,
                    v[2] as f64 / 1e6
                );
            }
        }
        Ok(())
    })?;
    Ok(())
}
