//! RE-397 (#1635): every instance of every railing's saved graphics, with the
//! volume of the symbol it names.
//!
//! FACT under test: the saved mesh of a railing is the meshes of the symbols its
//! `GInstance`s name (`probe_re397_railing_graphics_tree`), and the instances
//! the default draw leaves out are those flagged hidden in the normal draw mode
//! (visibility 1 or 3) or restricted by a non-zero `m_forbiddenTarget`
//! (`native_graphics_traversal` refuses both). If Revit's export draws the
//! restricted ones, a railing's visible plus restricted symbol volumes equal
//! its Revit body's volume, for every railing and not only the one of each type
//! `probe_re397_railing_symbol_meshes` read.
//!
//! Output, one line per instance and one per railing:
//!   inst <railing> <name> idx=<graph index> tag=<m_tag> flags=<m_flags>
//!     vis=<0..3> targets=<m_targets> scaled=<m_bHasScale> viewres=<m_resolveSymInView>
//!     symbol=<id> gram=<largest deviation of the matrix from orthonormal>
//!     <symbol status> triangles=<n> diagnostics=<n> volume=<ft3>
//!   rail <railing> <name> instances=<n> restricted=<n> visible_ft3=<sum over the
//!     instances the default draw keeps> restricted_ft3=<sum over the others>
//!   gsym <railing> no graph
//! where a symbol Revit's file does not give a scene element is `symbol=<id> missing`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re397_railing_instances -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene::extract_at_detail;

/// One `GInstance` of a railing's graph.
struct Instance {
    index: usize,
    tag: i64,
    flags: u64,
    visibility: u8,
    targets: u64,
    scaled: Option<bool>,
    view_resolved: Option<bool>,
    symbol: Option<u64>,
    gram: f64,
}

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

/// An identifier as the saved graphs write it: a number, or one inside
/// `m_id` / `m_id64`.
fn identifier(value: &Fv) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .iter()
        .find_map(|key| value.get(*key))
        .and_then(identifier)
}

/// The largest deviation of `m_Trf.m_3x3 * transpose` from the identity: 0 for a
/// rigid transform (a rotation or a mirrored one), the scale otherwise.
fn gram_deviation(transform: &Fv) -> f64 {
    let Some(rows) = transform["m_3x3"].as_array().filter(|rows| rows.len() == 3) else {
        return f64::NAN;
    };
    let mut matrix = [[0.0f64; 3]; 3];
    for (row, cells) in matrix.iter_mut().zip(rows) {
        let Some(cells) = cells.as_array().filter(|cells| cells.len() == 3) else {
            return f64::NAN;
        };
        for (cell, value) in row.iter_mut().zip(cells) {
            let Some(value) = value.as_f64() else {
                return f64::NAN;
            };
            *cell = value;
        }
    }
    let mut worst = 0.0f64;
    for a in 0..3 {
        for b in 0..3 {
            let dot: f64 = (0..3).map(|k| matrix[a][k] * matrix[b][k]).sum();
            worst = worst.max((dot - f64::from(u8::from(a == b))).abs());
        }
    }
    worst
}

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
        let is_railing = set.properties.iter().any(|p| {
            p.name == "RevitClass"
                && matches!(&p.value, PropertyValue::Text(t) if t == "BaseRailing")
        });
        if !is_railing {
            continue;
        }
        if let Ok(id) = tag.parse::<u64>() {
            railings.insert(id, name.clone());
        }
    }

    // The instances of each railing's channel 103 graph.
    let options = Options {
        selected_ids: railings.keys().copied().collect(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Options::default()
    };
    let mut instances: BTreeMap<u64, Vec<Instance>> = BTreeMap::new();
    let mut no_graph: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record| {
        let id = record.identity.element_id;
        let Some(graph) = record.graph.as_ref() else {
            no_graph.insert(id);
            return Ok(());
        };
        let mut found = Vec::new();
        for (index, object) in graph.objects.iter().enumerate() {
            if object.class_name != "GInstance" {
                continue;
            }
            let info = graph
                .edges
                .iter()
                .filter(|edge| edge.source_object_index == index)
                .filter_map(|edge| graph.objects.get(edge.target_object_index))
                .find(|target| target.class_name == "InstanceInfo");
            let flags = object.fields["m_GInfo"]["m_flags"].as_u64().unwrap_or(0);
            found.push(Instance {
                index,
                tag: object.fields["m_GInfo"]["m_tag"].as_i64().unwrap_or(-1),
                flags,
                visibility: visibility(flags),
                targets: object.fields["m_forbiddenTarget"]["m_targets"]
                    .as_u64()
                    .unwrap_or(u64::MAX),
                scaled: object.fields["m_bHasScale"].as_bool(),
                view_resolved: object.fields["m_resolveSymInView"].as_bool(),
                symbol: info
                    .and_then(|info| identifier(&info.fields["m_symbolId"]))
                    .and_then(|symbol| u64::try_from(symbol).ok()),
                gram: info.map_or(f64::NAN, |info| gram_deviation(&info.fields["m_Trf"])),
            });
        }
        instances.insert(id, found);
        Ok(())
    })?;

    // The volume of every symbol they name, read as an element of its own.
    let symbols: BTreeSet<u64> = instances
        .values()
        .flatten()
        .filter_map(|instance| instance.symbol)
        .collect();
    let symbol_options = Options {
        selected_ids: symbols,
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };
    let scene = extract_at_detail(&mut rf, &symbol_options, 3)?;
    let by_id: BTreeMap<u64, _> = scene
        .elements
        .iter()
        .map(|element| (element.identity.element_id, element))
        .collect();

    for (railing, name) in &railings {
        if no_graph.contains(railing) || !instances.contains_key(railing) {
            println!("gsym {railing} no graph");
            continue;
        }
        let list = &instances[railing];
        let mut visible = 0.0f64;
        let mut restricted = 0.0f64;
        let mut restricted_count = 0usize;
        for instance in list {
            let kept = matches!(instance.visibility, 0 | 2) && instance.targets == 0;
            let symbol_text = match instance.symbol.and_then(|symbol| by_id.get(&symbol)) {
                None => format!("symbol={:?} missing", instance.symbol),
                Some(element) => {
                    let mut volume6 = 0.0f64;
                    let mut triangles = 0usize;
                    for primitive in &element.meshes.primitives {
                        for triangle in &primitive.triangles {
                            let [a, b, c] =
                                triangle.map(|i| primitive.vertices.get(i as usize).copied());
                            let (Some(a), Some(b), Some(c)) = (a, b, c) else {
                                continue;
                            };
                            triangles += 1;
                            volume6 += a[0] * (b[1] * c[2] - b[2] * c[1])
                                - a[1] * (b[0] * c[2] - b[2] * c[0])
                                + a[2] * (b[0] * c[1] - b[1] * c[0]);
                        }
                    }
                    let volume = (volume6 / 6.0).abs();
                    if kept {
                        visible += volume;
                    } else {
                        restricted += volume;
                    }
                    format!(
                        "symbol={} {} triangles={triangles} diagnostics={} volume={volume:.6}",
                        element.id,
                        element.status,
                        element.meshes.diagnostics.len()
                    )
                }
            };
            if !kept {
                restricted_count += 1;
            }
            println!(
                "inst {railing} {name} idx={} tag={} flags={} vis={} targets={} scaled={:?} viewres={:?} gram={:.6} {symbol_text}",
                instance.index,
                instance.tag,
                instance.flags,
                instance.visibility,
                instance.targets,
                instance.scaled,
                instance.view_resolved,
                instance.gram,
            );
        }
        println!(
            "rail {railing} {name} instances={} restricted={restricted_count} visible_ft3={visible:.6} restricted_ft3={restricted:.6}",
            list.len()
        );
    }
    eprintln!("{} railings", railings.len());
    Ok(())
}
