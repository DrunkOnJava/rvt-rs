//! RE-224 (#255, W6): does the saved scene's own record of what it could not
//! read tell a complete saved mesh from a partial one?
//!
//! FACT under test: RE-78 found saved display meshes exact where they are
//! complete and a fraction of Revit's body elsewhere, with nothing in the
//! file found to tell the two apart, so `--saved-meshes` stays opt-in.
//! `native_saved_scene` already keeps, per element, what the graphics walk
//! dropped: decoder diagnostics (a surface-bearing component reference, a
//! nested detail-family filter, an unsupported instance mode, a failed face),
//! unbounded faces, filters rejected at the detail level, branches hidden by
//! their visibility flags, and curve-only branches; its `status` is
//! `partial_saved_graphics` when any diagnostic or unbounded face is left.
//! `saved_meshes::attach` reads none of it. This prints that record for
//! every element `attach` would replace, so
//! `tools/re/saved_mesh_status_vs_ifc.py` can set it against each body's
//! agreement with Revit's export.
//!
//! Output, one line per element whose record box `attach` would replace and
//! whose saved graphics the scene holds:
//!   mesh <id> <status> <primitives> <triangles> <unbounded faces>
//!     <empty trim faces> <rejected filters> <hidden branches>
//!     <curve-only branches> <diagnostics> <messages>
//! where <messages> is each distinct diagnostic with its digits replaced by
//! `#` and the object prefix dropped, joined by ` | `, or `-`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re224_saved_mesh_status -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene::extract_at_detail;

const RECORD_BOX: &str = "partition_element_record_bbox";

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
    let wholes: BTreeSet<usize> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::Aggregate { whole, .. } => Some(*whole),
            _ => None,
        })
        .collect();
    let mut tags: BTreeMap<u64, usize> = BTreeMap::new();
    for (index, entity) in model.entities.iter().enumerate() {
        if wholes.contains(&index) {
            continue;
        }
        let IfcEntity::BuildingElement {
            type_guid: Some(tag),
            property_set: Some(set),
            location_feet: Some(_),
            ..
        } = entity
        else {
            continue;
        };
        let boxed = set.properties.iter().any(|p| {
            p.name == "BodySource" && matches!(&p.value, PropertyValue::Text(t) if t == RECORD_BOX)
        });
        let resolved = set.properties.iter().any(|p| {
            p.name == "ProfileResolved" && matches!(p.value, PropertyValue::Boolean(true))
        });
        if !boxed || resolved {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            *tags.entry(id).or_default() += 1;
        }
    }
    let single: BTreeSet<u64> = tags
        .iter()
        .filter(|(_, n)| **n == 1)
        .map(|(id, _)| *id)
        .collect();
    let options = Options {
        selected_ids: single.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &options, 3)?;
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    for element in &scene.elements {
        let id = element.identity.element_id;
        if !single.contains(&id) {
            continue;
        }
        let m = &element.meshes;
        if m.primitives.is_empty() {
            continue;
        }
        let triangles: usize = m.primitives.iter().map(|p| p.triangles.len()).sum();
        let kinds: BTreeSet<String> = m.diagnostics.iter().map(|d| message_kind(d)).collect();
        let messages = if kinds.is_empty() {
            "-".to_string()
        } else {
            kinds.into_iter().collect::<Vec<_>>().join(" | ")
        };
        *statuses.entry(element.status.clone()).or_default() += 1;
        println!(
            "mesh {id} {} {} {triangles} {} {} {} {} {} {} {messages}",
            element.status,
            m.primitives.len(),
            m.unbounded_faces,
            m.empty_trim_faces,
            m.rejected_filters,
            m.excluded_visibility_branches,
            m.excluded_non_surface_branches,
            m.diagnostics.len(),
        );
    }
    eprintln!(
        "{} record-box candidates, {} with one entity, {} saved elements; {:?}",
        tags.len(),
        single.len(),
        scene.elements.len(),
        statuses
    );
    Ok(())
}
