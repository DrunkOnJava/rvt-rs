//! RE-302: which of a toilet's faces draw surface that Revit's export does
//! not, and which of Revit's vertices our faces miss (#1247).
//!
//! FACT measured (Measure run 41767705968, at d5d26fb4 on
//! `re/1247-per-cell-clip`, the second attempt's readers, three toilets): the
//! BIM4LCA office's `WC-istuin:WC-1` toilets (development data, not held out)
//! are 1.0060 of Revit's volume by `mesh_bodies_vs_ifc` (all 43 within 1%,
//! run 41766414367 at b5d5546f, whose reader code d5d26fb4 shares), our
//! vertices 0.0215 to 0.0222 ft from Revit's surface and Revit's 0.0046 to
//! 0.0050 ft from ours. Face by face, each of the three is
//! 14,471 triangles in 70 faces and 23.0667 sq ft of ours against Revit's
//! 1,668 triangles and 22.9621 sq ft. On the first toilet, 1,127 of our
//! triangles lie more than 0.01 ft from Revit's mesh, all on four surfaces of
//! revolution, and 66 of Revit's 808 vertices lie more than 0.001 ft off our
//! surface (66 of 805 on the third, which is rotated).
//!
//! The scorer (`mesh_bodies_vs_ifc`) judges a whole body; this prints each
//! decoded face on its own, so that `toilet_faces_vs_ifc.py` can say, per face,
//! how far its vertices lie from Revit's mesh of the same toilet and which of
//! our faces lies nearest each of Revit's vertices. Whether an excess is our
//! faces drawing what Revit's export does not, or Revit's export being the
//! coarser polygonisation of the same surfaces, shows in which faces are off
//! and by how much.
//!
//! Output: for each toilet id, one `toilet` line, then per decoded face one
//! `face` line (its index, owner, geometry object, tag, surface class,
//! vertex and triangle counts, area) and one `tri` line per triangle: the
//! face index and the nine coordinates, in document feet, the frame the
//! scorer's meshes are in.
//!
//! The ids are the first toilets the scorer listed (Measure run 41585460305),
//! or those on the command line after the file.
//!
//! Run it at d5d26fb4, on `re/1247-per-cell-clip`, where the readers draw the
//! toilets (Measure 41767705968): on `main` they are boxes, and only the faces
//! that decode are printed. Its result is in `RE-302`, section 6.
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re302_toilet_faces -- model.rvt [ids...]
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::control::WalkerControl;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::pointer;
use rvt::research::native_saved_scene::extract_at_detail_with_control;

const DEFAULT_IDS: [u64; 3] = [3_121_927, 3_110_111, 3_085_154];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        anyhow::bail!("usage: probe_re302_toilet_faces FILE.rvt [ids...]");
    };
    let mut ids: BTreeSet<u64> = args.filter_map(|a| a.parse().ok()).collect();
    if ids.is_empty() {
        ids.extend(DEFAULT_IDS);
    }
    let mut rf = rvt::RevitFile::open(&path)?;
    let options = Options {
        selected_ids: ids.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = extract_at_detail_with_control(&mut rf, &options, 3, &WalkerControl::default())?;
    let owners: BTreeSet<u64> = scene
        .elements
        .iter()
        .flat_map(|e| {
            e.meshes
                .primitives
                .iter()
                .map(move |p| p.source_owner_id.unwrap_or(e.identity.element_id))
        })
        .collect();
    let mut classes: BTreeMap<(u64, i64), String> = BTreeMap::new();
    if !owners.is_empty() {
        let owner_options = Options {
            selected_ids: owners,
            channels: BTreeSet::from([103]),
            ..options.clone()
        };
        extract_graphs(&mut rf, &owner_options, |record| {
            let Some(g) = record.graph.as_ref() else {
                return Ok(());
            };
            for (fi, face) in g.objects.iter().enumerate() {
                if face.class_name != "Face" {
                    continue;
                }
                let Some(tag) = face.fields["m_GInfo"]["m_tag"].as_i64() else {
                    continue;
                };
                let Ok(Some(si)) = pointer(g, fi, &face.fields["m_pSurf"]) else {
                    continue;
                };
                classes.insert(
                    (record.identity.element_id, tag),
                    g.objects[si].class_name.clone(),
                );
            }
            Ok(())
        })?;
    }
    for element in &scene.elements {
        if !ids.contains(&element.identity.element_id) {
            continue;
        }
        let primitives = &element.meshes.primitives;
        println!(
            "toilet {} faces {} diagnostics {}",
            element.identity.element_id,
            primitives.len(),
            element.meshes.diagnostics.len()
        );
        for (k, p) in primitives.iter().enumerate() {
            let owner = p.source_owner_id.unwrap_or(element.identity.element_id);
            let class = classes
                .get(&(owner, p.face_tag))
                .map_or("unknown", String::as_str);
            let mut area = 0.;
            for t in &p.triangles {
                let (a, b, c) = (
                    p.vertices[t[0] as usize],
                    p.vertices[t[1] as usize],
                    p.vertices[t[2] as usize],
                );
                let u: [f64; 3] = std::array::from_fn(|i| b[i] - a[i]);
                let v: [f64; 3] = std::array::from_fn(|i| c[i] - a[i]);
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                area += (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt() / 2.;
            }
            println!(
                "face {k} owner {owner} geometry {} tag {} surface {class} vertices {} triangles {} area {area:.6}",
                p.object_index,
                p.face_tag,
                p.vertices.len(),
                p.triangles.len()
            );
            for t in &p.triangles {
                let (a, b, c) = (
                    p.vertices[t[0] as usize],
                    p.vertices[t[1] as usize],
                    p.vertices[t[2] as usize],
                );
                println!(
                    "tri {k} {:.7} {:.7} {:.7} {:.7} {:.7} {:.7} {:.7} {:.7} {:.7}",
                    a[0], a[1], a[2], b[0], b[1], b[2], c[0], c[1], c[2]
                );
            }
        }
    }
    Ok(())
}
