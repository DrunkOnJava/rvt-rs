//! RE-389 follow-up (#1630): why a roof stays a level block.
//!
//! FACT under test: #1612 draws a roof of Autodesk's Snowdon Towers
//! Architectural (Revit 2024) from its saved mesh where the mesh decodes whole
//! (10 of the 20), and leaves the other 10 as the level block of their sketch
//! outline, with volumes 4.9% to 133% above Revit's. `saved_meshes::attach`
//! keeps the block where the scene's own record of what it could not read
//! (`partial_saved_graphics`: a decoder diagnostic or an unbounded face) or
//! where the element has no triangle. This lists every `IfcRoof` the export
//! writes, with the record `attach` reads, so each block has its reason.
//!
//! Output, one line per roof:
//!   roof <id> <BodySource> resolved=<ProfileResolved> scene=no
//!   roof <id> <BodySource> resolved=<ProfileResolved> scene=yes <status>
//!     primitives=<n> triangles=<n> unbounded=<n> empty_trim=<n> diagnostics=<n>
//!     volume=<abs signed volume, ft3> <messages>
//! where <messages> is each distinct diagnostic with its digits replaced by
//! `#` and the object prefix dropped, joined by ` | `, or `-`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re389_roof_blocks -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;

fn message_kind(message: &str) -> String {
    let text = message.split_once(": ").map_or(message, |(_, rest)| rest);
    text.chars()
        .map(|c| if c.is_ascii_digit() { '#' } else { c })
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    // (BodySource, ProfileResolved) of each roof, by ElementId.
    let mut roofs: BTreeMap<u64, (String, bool)> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            property_set: Some(set),
            ..
        } = entity
        else {
            continue;
        };
        if !ifc_type.eq_ignore_ascii_case("IFCROOF") {
            continue;
        }
        let Ok(id) = tag.parse::<u64>() else {
            continue;
        };
        let source = set
            .properties
            .iter()
            .find_map(|p| match &p.value {
                PropertyValue::Text(t) if p.name == "BodySource" => Some(t.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        roofs.insert(id, (source, resolved));
    }
    let options = Options {
        selected_ids: roofs.keys().copied().collect::<BTreeSet<u64>>(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let by_id: BTreeMap<u64, _> = scene
        .elements
        .iter()
        .map(|element| (element.identity.element_id, element))
        .collect();
    for (id, (source, resolved)) in &roofs {
        let Some(element) = by_id.get(id) else {
            println!("roof {id} {source} resolved={resolved} scene=no");
            continue;
        };
        let meshes = &element.meshes;
        let mut volume6 = 0.0f64;
        let mut triangles = 0usize;
        for primitive in &meshes.primitives {
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
        let kinds: BTreeSet<String> = meshes.diagnostics.iter().map(|d| message_kind(d)).collect();
        let messages = if kinds.is_empty() {
            "-".to_string()
        } else {
            kinds.into_iter().collect::<Vec<_>>().join(" | ")
        };
        println!(
            "roof {id} {source} resolved={resolved} scene=yes {} primitives={} triangles={triangles} \
             unbounded={} empty_trim={} diagnostics={} volume={:.6} {messages}",
            element.status,
            meshes.primitives.len(),
            meshes.unbounded_faces,
            meshes.empty_trim_faces,
            meshes.diagnostics.len(),
            (volume6 / 6.0).abs(),
        );
    }
    eprintln!(
        "{} roofs, {} saved elements",
        roofs.len(),
        scene.elements.len()
    );
    Ok(())
}
