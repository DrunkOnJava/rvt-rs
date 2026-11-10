//! #1046: do the visibility and filter flags of a member's saved graphics mark the end faces
//! Revit's NetSurfaceArea leaves out?
//!
//! FACT under test: Revit's NetSurfaceArea of a curtain mullion is the sum of the faces of the
//! geometry it holds (revit-ifc's NetSurfaceAreaCalculator, CLEANROOM.md source 5), which leaves
//! out the end faces its joins hide (RE-268), while the saved mesh rvt-rs draws keeps them.
//! Hypothesis: those end faces are the primitives reached through a hidden or filtered branch.
//!
//! Output, for every IfcMember the export writes:
//!   member tag:<id> status <s> prims <n> hidden <n> filtered <n>
//!   prim tag:<id> hidden <0|1> filtered <0|1> area <ft2> normal <nx> <ny> <nz>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1046_member_flags -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut ids: BTreeSet<u64> = BTreeSet::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if ifc_type.eq_ignore_ascii_case("IFCMEMBER") {
                if let Ok(id) = tag.parse() {
                    ids.insert(id);
                }
            }
        }
    }
    println!("members {}", ids.len());
    let options = Options {
        selected_ids: ids.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let scene = native_saved_scene::extract_including_filtered(&mut rf, &options)?;
    for element in scene.elements.iter().filter(|e| ids.contains(&e.id)) {
        let meshes = &element.meshes;
        println!(
            "member tag:{} status {} prims {} hidden {} filtered {}",
            element.id,
            element.status,
            meshes.primitives.len(),
            meshes.primitives.iter().filter(|p| p.hidden).count(),
            meshes.primitives.iter().filter(|p| p.filtered).count()
        );
        for p in &meshes.primitives {
            let mut area = 0.0;
            let mut normal = [0.0f64; 3];
            for t in &p.triangles {
                let c = t.map(|i| p.vertices.get(i as usize).copied());
                let [Some(a), Some(b), Some(d)] = c else {
                    continue;
                };
                let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                let v = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                area += len / 2.0;
                if normal == [0.0; 3] && len > 1e-12 {
                    normal = [n[0] / len, n[1] / len, n[2] / len];
                }
            }
            println!(
                "prim tag:{} hidden {} filtered {} area {area:.6} normal {:.4} {:.4} {:.4}",
                element.id,
                u8::from(p.hidden),
                u8::from(p.filtered),
                normal[0],
                normal[1],
                normal[2]
            );
        }
    }
    Ok(())
}
