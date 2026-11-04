//! RE-329 (#1144, #1247): which faces of a `Stay_Wall_Mounted_:VAR-1` fitting
//! draw surface that Revit's export does not.
//!
//! FACT measured (Measure run 41787545512, `mesh_bodies_vs_ifc` at 8a24df6e,
//! the RE-305 cylinder-hole reader on `main`): the BIM4LCA office's 30
//! `Stay_Wall_Mounted_:VAR-1` fittings (`IfcFurniture`) are the elements the
//! census (`probe_re329_saved_graphics_census`) finds blocked by one cause, a
//! cylinder face with several trim loops. Read with that reader they come out
//! at 1.1102 to 1.1105 of Revit's volume with the scorer taking volume about
//! each body's own centre, Revit's vertices 0.0047 to 0.0051 ft from our
//! surface and ours up to 0.0118 ft from Revit's. Of five shells, the second is
//! 0.0239 cubic ft of ours against Revit's 0.0185.
//!
//! This prints each decoded face of four of them, in the format of
//! `probe_re302_toilet_faces` (the keyword stays `toilet` so that
//! `tools/re/toilet_faces_vs_ifc.py` reads it unchanged), so the scorer can say,
//! per face, how far its triangles lie from Revit's mesh of the same fitting and
//! which of Revit's vertices our faces miss. Run it at a ref where the reader
//! draws the fittings (8a24df6e): on `main` they are boxes.
//!
//! Output: for each id, one `toilet` line, then per decoded face one `face`
//! line and one `tri` line per triangle (nine coordinates, document feet).
//!
//! ```text
//! cargo run --profile ci -p rvt-probes --example probe_re329_wall_mounted_faces -- model.rvt [ids...]
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::control::WalkerControl;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_mesh::pointer;
use rvt::research::native_saved_scene::extract_at_detail_with_control;

const DEFAULT_IDS: [u64; 4] = [2_794_370, 3_073_846, 3_121_894, 1_832_498];

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let Some(path) = args.next() else {
        anyhow::bail!("usage: probe_re329_wall_mounted_faces FILE.rvt [ids...]");
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
