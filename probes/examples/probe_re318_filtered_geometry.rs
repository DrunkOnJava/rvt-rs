//! RE-318 (#810): does a door's or window's symbol width include the
//! geometry its visibility flags hide and the branches its filters reject?
//!
//! FACT under test: RE-318's first run found the BIM4LCA office's door 1765267
//! (hatch type `LU-3.VSS.700x1200_180`, symbol 1766487) drawn 5.8629 ft wide
//! along the symbol's X, Revit's `OverallWidth`, while door 1756089 (type
//! `LU-3.VSS.700x1200_90`, symbol 2058852), which Revit also gives 5.8629 ft,
//! is drawn 3.1168 ft wide. The family holds two yes/no parameters,
//! `Door_90_degrees` and `Door_180_degrees`, and one leaf form for each: the
//! 90 degree type shows only its own leaf. If Revit measures the symbol's
//! geometry whether or not it is visible (the exporter's
//! `ExporterIFCUtils.GetMinSymbolWidth`, CLEANROOM.md source 5), the hidden
//! 180 degree leaf in the 90 degree symbol's saved graphics gives 5.8629 ft.
//!
//! The first run of `probe_re318_hidden_geometry` (Measure 41720185112) found
//! the hidden branches give Revit's width on every office door and window
//! but door 1722215 (symbol 1727541, 2750 mm against Revit's 2755 mm) and
//! the door Revit sizes 0 by 0. This also decodes the children of a filter
//! whose conditions reject the default context (a detail level, a view
//! direction) or are not read, to see whether 5 mm lies there.
//!
//! For every door and window the export writes, this decodes the saved
//! graphics of the symbol that draws it (`InstanceInfo.m_symbolId`), in the
//! symbol's own frame, with both kinds of branch included and each primitive
//! marked, and prints each primitive's box.
//!
//! Output, one record per line:
//!   inst <id> <class> symbol=<drawn by>
//!   scene <symbol> status=.. prims=<n> hidden=<n> filtered=<n> diag=<first>
//!   prim <symbol> hidden=<0|1> filtered=<0|1> owner=<symbol|-> tag=<face tag> style=<render style>
//!     verts=<n> box=<x0,y0,z0,x1,y1,z1>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re318_filtered_geometry -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_saved_scene;
use serde_json::Value;

fn element_id(value: &Value) -> Option<u64> {
    value
        .pointer("/m_id/m_id64")?
        .as_i64()
        .and_then(|n| u64::try_from(n).ok())
}

fn bounds(vertices: &[[f64; 3]]) -> Option<[f64; 6]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for v in vertices {
        for (k, value) in v.iter().enumerate() {
            b[k] = b[k].min(*value);
            b[k + 3] = b[k + 3].max(*value);
        }
    }
    b.iter().all(|x| x.is_finite()).then_some(b)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            if ifc_type.eq_ignore_ascii_case("IFCDOOR")
                || ifc_type.eq_ignore_ascii_case("IFCWINDOW")
            {
                if let Ok(id) = tag.parse() {
                    classes.insert(id, ifc_type.to_ascii_uppercase());
                }
            }
        }
    }
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let options = Options {
        selected_ids: ids.clone(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        if let Some(symbol) = graph
            .objects
            .iter()
            .find(|object| object.class_name == "InstanceInfo")
            .and_then(|object| object.fields.get("m_symbolId"))
            .and_then(element_id)
        {
            drawn_by.insert(record.identity.element_id, symbol);
        }
        Ok(())
    })?;
    for (id, class) in &classes {
        match drawn_by.get(id) {
            Some(symbol) => println!("inst {id} {class} symbol={symbol}"),
            None => println!("inst {id} {class} -"),
        }
    }
    let symbols: BTreeSet<u64> = drawn_by.values().copied().collect();
    let scene_options = Options {
        selected_ids: symbols.clone(),
        ..Options::default()
    };
    let scene = native_saved_scene::extract_including_filtered(&mut rf, &scene_options)?;
    for element in scene
        .elements
        .iter()
        .filter(|element| symbols.contains(&element.id))
    {
        let meshes = &element.meshes;
        println!(
            "scene {} status={} prims={} hidden={} filtered={} diag={}",
            element.id,
            element.status,
            meshes.primitives.len(),
            meshes.primitives.iter().filter(|p| p.hidden).count(),
            meshes.primitives.iter().filter(|p| p.filtered).count(),
            meshes.diagnostics.first().map_or("-".to_string(), |d| d
                .replace(' ', "_")
                .chars()
                .take(200)
                .collect::<String>()),
        );
        for p in &meshes.primitives {
            let b = bounds(&p.vertices);
            println!(
                "prim {} hidden={} filtered={} owner={} tag={} style={} verts={} box={}",
                element.id,
                u8::from(p.hidden),
                u8::from(p.filtered),
                p.source_owner_id.map_or("-".to_string(), |o| o.to_string()),
                p.face_tag,
                p.render_style_id,
                p.vertices.len(),
                b.map_or("-".to_string(), |b| format!(
                    "{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                    b[0], b[1], b[2], b[3], b[4], b[5]
                )),
            );
        }
    }
    eprintln!(
        "{} doors and windows, {} symbols, {} decoded",
        classes.len(),
        symbols.len(),
        scene.elements.len()
    );
    Ok(())
}
