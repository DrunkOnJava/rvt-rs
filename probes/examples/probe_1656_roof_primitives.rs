//! #1656: the saved graphics of one element, primitive by primitive.
//!
//! FACT under test: the roof 1627602 of the BIM4LCA housing (`KA-1`) is drawn from its saved mesh at
//! 31.6778 ft3 where Revit's export holds 31.6136 (two shells, 22.3155 and 9.2981 ft3 over their
//! distinct triangles), with every vertex of Revit's body on the mesh's surface and the mesh's
//! farthest vertex 0.033 ft from Revit's. The extra 0.0642 ft3 (0.2%) is in a primitive of the
//! saved mesh that Revit's body does not hold, or in one tessellated otherwise. This prints every
//! primitive of the element's saved graphics (a face tag, a style, a material, its owner, and its
//! vertices in internal feet and triangles) as one JSON line, so that
//! `tools/re/roof_primitives_vs_ifc.py` can set each against Revit's body in the same run, which
//! holds the export.
//!
//! Output, tab separated:
//!   element <id> primitives=<n> diagnostics=<the decoder's diagnostics joined by " | ">
//!   prim <index> <JSON of the primitive>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1656_roof_primitives -- MODEL.rvt [ELEMENT_ID]

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract;

/// The roof of the BIM4LCA housing that #1656 names.
const ROOF: u64 = 1_627_602;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: MODEL.rvt [ELEMENT_ID]");
    let id: u64 = args
        .next()
        .map_or(ROOF, |text| text.parse().expect("an ElementId"));
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_ids: BTreeSet::from([id]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        value_source: Some("saved_graphics"),
        ..Options::default()
    };
    let scene = extract(&mut rf, &options)?;
    for element in scene.elements.iter().filter(|element| element.id == id) {
        let primitives = &element.meshes.primitives;
        println!(
            "element\t{id}\tprimitives={}\tdiagnostics={}",
            primitives.len(),
            element.meshes.diagnostics.join(" | ")
        );
        for (index, primitive) in primitives.iter().enumerate() {
            let line = serde_json::json!({
                "face_tag": primitive.face_tag,
                "render_style_id": primitive.render_style_id,
                "material_id": primitive.material_id,
                "source_owner_id": primitive.source_owner_id,
                "object_index": primitive.object_index,
                "vertices": primitive.vertices,
                "triangles": primitive.triangles,
            });
            println!("prim\t{index}\t{line}");
        }
    }
    Ok(())
}
