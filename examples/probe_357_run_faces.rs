//! #357: whether a stair run's own native record holds its solid, as the
//! boundary faces Revit's export draws.
//!
//! FACT under test: on the R2025 stair models a `StairsRun` record's graph
//! holds `Face`, `Plane`, `EdgeLoop`, `Edge` and `GLine` objects (66 faces on
//! `13_Rooms_Stairs`), the boundary representation the saved-graphics
//! reader tessellates (`native_saved_mesh`). This meshes every face of each
//! run's and landing's graph with `native_saved_mesh::face` and prints the
//! mesh's volume, unique vertices and box, for comparison with Revit's body
//! of the same flight (`tools/re/stair_bodies_vs_ifc.py` prints Revit's
//! volume and vertex count).
//!
//! Output:
//!   run <id> <class> faces <n> meshed <n> unbounded <n> failed <n> triangles <n> vertices <n> volume <ft3> box <min xyz> <max xyz>
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_run_faces -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::face;

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
        let (mut faces, mut meshed, mut unbounded, mut failed, mut triangles) = (0, 0, 0, 0, 0);
        let mut volume = 0.0;
        let mut vertices: BTreeSet<[i64; 3]> = BTreeSet::new();
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for (index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "Face" {
                continue;
            }
            faces += 1;
            match face(graph, index) {
                Ok(Some(mesh)) => {
                    meshed += 1;
                    for v in &mesh.vertices {
                        vertices.insert([
                            (v[0] * 1e5).round() as i64,
                            (v[1] * 1e5).round() as i64,
                            (v[2] * 1e5).round() as i64,
                        ]);
                        for k in 0..3 {
                            lo[k] = lo[k].min(v[k]);
                            hi[k] = hi[k].max(v[k]);
                        }
                    }
                    for t in &mesh.triangles {
                        let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                        triangles += 1;
                        volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                            - a[1] * (b[0] * c[2] - b[2] * c[0])
                            + a[2] * (b[0] * c[1] - b[1] * c[0]))
                            / 6.0;
                    }
                }
                Ok(None) => unbounded += 1,
                Err(_) => failed += 1,
            }
        }
        println!(
            "run {} {class} faces {faces} meshed {meshed} unbounded {unbounded} failed {failed} triangles {triangles} vertices {} volume {:.6} box {:?} {:?}",
            record.identity.element_id,
            vertices.len(),
            volume.abs(),
            lo,
            hi
        );
        Ok(())
    })?;
    Ok(())
}
