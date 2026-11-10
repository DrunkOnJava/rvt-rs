//! RE-397 (#1635): what a railing's saved graphics hold when the branches its
//! visibility flags hide are decoded too.
//!
//! FACT under test: the office's `KJO-1` railings keep no mesh in the default
//! decode (0 primitives, 0 diagnostics), and the graph of one railing is a
//! group of four `GInstance`s, each flagged with the bit that hides it in the
//! normal draw mode and naming a symbol by ElementId (`probe_re397_railing_graphics_tree`).
//! RE-318 found the hidden branches of a door symbol carry its overall width.
//! This decodes each railing with the hidden branches included and prints
//! what they give, to set against Revit's body of the same railing.
//!
//! Output, one line per railing:
//!   rail <id> <name> scene=no
//!   rail <id> <name> scene=yes <status> primitives=<n> hidden=<n> triangles=<n>
//!     diagnostics=<n> bbox=<min x,y,z>..<max x,y,z> volume=<abs signed volume>
//!     visible_triangles=<n>
//! The bounding box and the signed volume cover every triangle of the decode,
//! in the document's feet.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_hidden_graphics -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_including_hidden;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut railings: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            name,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        let class = set.properties.iter().find_map(|p| match &p.value {
            PropertyValue::Text(t) if p.name == "RevitClass" => Some(t.as_str()),
            _ => None,
        });
        if class != Some("BaseRailing") {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            railings.insert(id, name.clone());
        }
    }
    let options = Options {
        selected_ids: railings.keys().copied().collect(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_including_hidden(&mut rf, &options)?;
    let by_id: BTreeMap<u64, _> = scene
        .elements
        .iter()
        .map(|element| (element.identity.element_id, element))
        .collect();
    for (id, name) in &railings {
        let Some(element) = by_id.get(id) else {
            println!("rail {id} {name} scene=no");
            continue;
        };
        let meshes = &element.meshes;
        let mut low = [f64::INFINITY; 3];
        let mut high = [f64::NEG_INFINITY; 3];
        let mut volume6 = 0.0;
        let mut triangles = 0usize;
        let mut visible_triangles = 0usize;
        let mut hidden = 0usize;
        for primitive in &meshes.primitives {
            if primitive.hidden {
                hidden += 1;
            } else {
                visible_triangles += primitive.triangles.len();
            }
            for point in &primitive.vertices {
                for (axis, value) in point.iter().enumerate() {
                    low[axis] = low[axis].min(*value);
                    high[axis] = high[axis].max(*value);
                }
            }
            for triangle in &primitive.triangles {
                let [a, b, c] = triangle.map(|i| primitive.vertices.get(i as usize).copied());
                let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                    continue;
                };
                triangles += 1;
                volume6 += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]);
            }
        }
        println!(
            "rail {id} {name} scene=yes {} primitives={} hidden={hidden} triangles={triangles} \
             diagnostics={} bbox={:.4},{:.4},{:.4}..{:.4},{:.4},{:.4} volume={:.6} \
             visible_triangles={visible_triangles}",
            element.status,
            meshes.primitives.len(),
            meshes.diagnostics.len(),
            low[0],
            low[1],
            low[2],
            high[0],
            high[1],
            high[2],
            (volume6 / 6.0).abs(),
        );
    }
    eprintln!(
        "{} railings, {} saved elements in the scene",
        railings.len(),
        scene.elements.len()
    );
    Ok(())
}
