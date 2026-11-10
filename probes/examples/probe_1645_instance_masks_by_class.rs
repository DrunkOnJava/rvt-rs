//! #1645: the instances of every element's saved graphics that the walk skips or
//! refuses, by IFC class.
//!
//! FACT under test: `exported.saved_mesh_refusals` counts `instance target
//! restrictions are unsupported` on bodies of several classes (the sanitary
//! terminals of the BIM4LCA housing among them), and RE-397 draws only the pair
//! of visibility 1 and `m_targets` 53246. This lists, for each IFC class, every
//! combination of the instance's visibility (as `graphics_visibility` reads its
//! flags), its `m_forbiddenTarget.m_targets`, `m_bHasScale` and
//! `m_resolveSymInView`, with the instances and the elements that hold it, so the
//! masks that are not 53246, and the classes they belong to, are known.
//!
//! Output, one line per combination, tab separated:
//!   mask <class> vis=<0..3> targets=<n> scaled=<b> viewres=<b> instances=<n> elements=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1645_instance_masks_by_class -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportOptions, run_export};
use rvt::research::native_document::{Options, extract_graphs};

/// `graphics_visibility` of `native_graphics_traversal`: 1 and 3 are the
/// instances the normal draw mode skips.
fn visibility(flags: u64) -> u8 {
    let bits = flags & 0x1e0;
    if bits == 0 {
        0
    } else if bits == 0xe0 {
        3
    } else if bits < 0xe0 {
        if flags & 0x160 == 0x20 { 1 } else { 2 }
    } else if flags & 0x1a0 == 0x100 {
        1
    } else {
        2
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = run_export(&mut rf, &ExportOptions::default())?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        else {
            continue;
        };
        if let Ok(id) = tag.parse::<u64>() {
            classes.entry(id).or_insert_with(|| ifc_type.clone());
        }
    }
    let options = Options {
        selected_ids: classes.keys().copied().collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    type Key = (String, u8, u64, String, String);
    let mut census: BTreeMap<Key, (usize, BTreeSet<u64>)> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let class = classes
            .get(&id)
            .cloned()
            .unwrap_or_else(|| "unmapped".to_string());
        for object in &graph.objects {
            if object.class_name != "GInstance" {
                continue;
            }
            let flags = object.fields["m_GInfo"]["m_flags"].as_u64().unwrap_or(0);
            let targets = object.fields["m_forbiddenTarget"]["m_targets"]
                .as_u64()
                .unwrap_or(u64::MAX);
            let key = (
                class.clone(),
                visibility(flags),
                targets,
                object.fields["m_bHasScale"].to_string(),
                object.fields["m_resolveSymInView"].to_string(),
            );
            let entry = census.entry(key).or_default();
            entry.0 += 1;
            entry.1.insert(id);
        }
        Ok(())
    })?;
    for ((class, vis, targets, scaled, viewres), (instances, elements)) in &census {
        println!(
            "mask\t{class}\tvis={vis}\ttargets={targets}\tscaled={scaled}\tviewres={viewres}\tinstances={instances}\telements={}",
            elements.len()
        );
    }
    Ok(())
}
