//! RE-397 (#1635): does a railing carry saved graphics of its own?
//!
//! FACT under test: every railing of the BIM4LCA office is written as its
//! record box, about a thousandth of Revit's volume (#1635). RE-224's probe
//! listed 18 of the 41 (the `KAI` types, whose saved mesh keeps a reachable
//! `GBiFlipControl`, RE-303) and none of the 22 `KJO-1` railings, most of them
//! hosted on stairs. This prints, for every `BaseRailing` the export writes,
//! whether the saved scene holds graphics for it and what they report, so
//! the railings with no saved graphics at all can be told from those whose
//! saved mesh is partial.
//!
//! Output, one line per railing:
//!   railing <id> <name> <BodySource> scene=no
//!   railing <id> <name> <BodySource> scene=yes <status> <primitives>
//!     <triangles> <diagnostics> <messages> refs=<shared sources> source=<bytes>
//! where <messages> is each distinct diagnostic with its digits replaced by
//! `#`, joined by ` | `, or `-`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_saved_graphics -- MODEL.rvt

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
    let mut railings: BTreeMap<u64, (String, String)> = BTreeMap::new();
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
        let text = |key: &str| {
            set.properties.iter().find_map(|p| match &p.value {
                PropertyValue::Text(t) if p.name == key => Some(t.clone()),
                _ => None,
            })
        };
        if text("RevitClass").as_deref() != Some("BaseRailing") {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            railings.insert(id, (name.clone(), text("BodySource").unwrap_or_default()));
        }
    }
    let options = Options {
        selected_ids: railings.keys().copied().collect(),
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
    for (id, (name, body)) in &railings {
        let Some(element) = by_id.get(id) else {
            println!("railing {id} {name} {body} scene=no");
            continue;
        };
        let meshes = &element.meshes;
        let triangles: usize = meshes.primitives.iter().map(|p| p.triangles.len()).sum();
        let kinds: BTreeSet<String> = meshes.diagnostics.iter().map(|d| message_kind(d)).collect();
        let messages = if kinds.is_empty() {
            "-".to_string()
        } else {
            kinds.into_iter().collect::<Vec<_>>().join(" | ")
        };
        println!(
            "railing {id} {name} {body} scene=yes {} {} {triangles} {} {messages} refs={} source={}",
            element.status,
            meshes.primitives.len(),
            meshes.diagnostics.len(),
            element.referenced_graph_sources.len(),
            element.source.to_string().len(),
        );
    }
    eprintln!(
        "{} railings, {} saved elements in the scene",
        railings.len(),
        scene.elements.len()
    );
    Ok(())
}
