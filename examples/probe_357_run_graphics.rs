//! #357: which of a stair run's own boundary solids is the flight Revit's
//! export draws.
//!
//! FACT under test: a Revit 2025 `StairsRun` record holds two `Geometry`
//! objects of planar faces (66 faces on `13_Rooms_Stairs`), and meshing all
//! of them sums to 20 times Revit's flight volume (probe_357_run_faces), so
//! they are more than the flight. The saved-graphics traversal does not
//! take a StairsRun as its root, so this meshes each `Geometry` object's
//! faces (`m_pFaces`) of each run's and landing's own graph as that
//! traversal meshes a Geometry's, and prints the vertices with the
//! `Geometry` object they come from, for `tools/re/run_graphics_vs_ifc.py` to
//! match against Revit's body of the same flight.
//!
//! Output:
//!   primitive <id> <Geometry object index> <faces> <triangles> <signed volume ft3>
//!   graphics <id> <class> geometries <n> unbounded <n> failed <n>
//!   vertex <id> <object index> <x> <y> <z>
//!
//! Usage:
//!   cargo run --profile ci --example probe_357_run_graphics -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::{face, pointer};

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
        // The saved-graphics traversal does not take a StairsRun as its root,
        // so each Geometry object's faces are meshed here, as that traversal
        // meshes a Geometry's.
        let mut by_object: BTreeMap<usize, BTreeSet<[i64; 3]>> = BTreeMap::new();
        let (mut geometries, mut unbounded, mut failed) = (0, 0, 0);
        for (oi, object) in graph.objects.iter().enumerate() {
            if object.class_name != "Geometry" {
                continue;
            }
            geometries += 1;
            let Some(faces) = object.fields["m_pFaces"].as_array() else {
                continue;
            };
            let (mut volume, mut triangles) = (0.0, 0);
            for ptr in faces {
                let Ok(Some(fi)) = pointer(graph, oi, ptr) else {
                    failed += 1;
                    continue;
                };
                match face(graph, fi) {
                    Ok(Some(mesh)) => {
                        for t in &mesh.triangles {
                            let [a, b, c] = t.map(|i| mesh.vertices[i as usize]);
                            triangles += 1;
                            volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                                - a[1] * (b[0] * c[2] - b[2] * c[0])
                                + a[2] * (b[0] * c[1] - b[1] * c[0]))
                                / 6.0;
                        }
                        let set = by_object.entry(oi).or_default();
                        for v in &mesh.vertices {
                            set.insert(v.map(|c| (c * 1e6).round() as i64));
                        }
                    }
                    Ok(None) => unbounded += 1,
                    Err(_) => failed += 1,
                }
            }
            println!(
                "primitive {id} {oi} {} {triangles} {volume:.6}",
                faces.len()
            );
        }
        println!(
            "graphics {id} {class} geometries {geometries} unbounded {unbounded} failed {failed}"
        );
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
