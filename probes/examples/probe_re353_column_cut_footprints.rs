//! RE-353 (#1203, W7): each column's plan placement and the plan footprint
//! of every element its pair steps name, for reproducing the cut mesh Revit's
//! export draws where a slab lies across the top of a column.
//!
//! FACT under test: where a slab's body lies across the top of a column and
//! the column's `CutPairGStep` names it, Revit's export draws the column as
//! its box minus the part of the slab inside it, a mesh whose `NetVolume` is
//! that mesh's volume and whose `CrossSectionArea` is half its surface area
//! (measured on Revit's own export, `tools/re/qtos_vs_ifc.py`). The part of
//! the slab inside the column is the slab's plan footprint clipped to the
//! column's, from the column's top down to the slab's base. This prints the
//! two footprints for an offline join with Revit's bodies by `Tag`.
//!
//! Output, one line per column rvt-rs exports:
//!   col <tag> at <x> <y> <z> rot <radians|-> prof <profile> h <height> pairs <step>:<other>:<entity>:<x>:<y>:<z>:<rot>:<profile>:<height> ...
//! where a profile is `r_<width>_<depth>` (a rectangle centred on its
//! origin), `p_<x>,<y>;<x>,<y>...` (a closed polyline in the element's own
//! frame) or `other`, an element rvt-rs does not export is `-`, and all
//! lengths are feet.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re353_column_cut_footprints -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::{CurveSegment, Extrusion, IfcEntity, ProfileDef};
use rvt::research::native_document::{Options, extract_graphs};

type Info = (String, [f64; 3], Option<f64>, String, f64);

fn profile(body: &Extrusion) -> String {
    match &body.profile_override {
        None => format!("r_{}_{}", body.width_feet, body.depth_feet),
        Some(ProfileDef::Rectangle {
            width_feet,
            depth_feet,
        }) => format!("r_{width_feet}_{depth_feet}"),
        Some(ProfileDef::ArbitraryClosed { points }) => {
            let listed: Vec<String> = points.iter().map(|(x, y)| format!("{x},{y}")).collect();
            format!("p_{}", listed.join(";"))
        }
        Some(ProfileDef::ArbitraryWithVoids { points, voids }) => {
            let outer: Vec<String> = points.iter().map(|(x, y)| format!("{x},{y}")).collect();
            let holes: Vec<String> = voids
                .iter()
                .map(|hole| {
                    let listed: Vec<String> =
                        hole.iter().map(|(x, y)| format!("{x},{y}")).collect();
                    listed.join(";")
                })
                .collect();
            format!("w_{}/{}", outer.join(";"), holes.join("/"))
        }
        Some(ProfileDef::IndexedPolyCurve { points, segments }) => {
            let listed: Vec<String> = points.iter().map(|(x, y)| format!("{x},{y}")).collect();
            let arcs = segments
                .iter()
                .filter(|segment| matches!(segment, CurveSegment::Arc(_)))
                .count();
            format!("i{arcs}_{}", listed.join(";"))
        }
        Some(_) => "other".to_string(),
    }
}

fn describe(info: &Info) -> String {
    let (entity, at, rotation, shape, height) = info;
    format!(
        "{entity}:{}:{}:{}:{}:{shape}:{height}",
        at[0],
        at[1],
        at[2],
        rotation.map_or_else(|| "-".to_string(), |r| r.to_string()),
    )
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut info: BTreeMap<u64, Vec<Info>> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            location_feet: Some(at),
            rotation_radians,
            extrusion: Some(body),
            solid_shape: None,
            representation_map_index: None,
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                info.entry(id).or_default().push((
                    ifc_type.to_ascii_uppercase(),
                    *at,
                    *rotation_radians,
                    profile(body),
                    body.height_feet,
                ));
            }
        }
    }
    let columns: BTreeSet<u64> = info
        .iter()
        .filter(|(_, held)| held.iter().any(|piece| piece.0 == "IFCCOLUMN"))
        .map(|(id, _)| *id)
        .collect();
    let mut pairs: BTreeMap<u64, Vec<(String, u64)>> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: columns.clone(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            for object in &graph.objects {
                if !object.class_name.ends_with("PairGStep") {
                    continue;
                }
                let other = object
                    .fields
                    .get("m_otherId")
                    .and_then(|value| value.pointer("/m_id/m_id64"))
                    .and_then(rvt::research::graph_value::Fv::as_i64)
                    .and_then(|id| u64::try_from(id).ok());
                if let Some(other) = other {
                    pairs
                        .entry(record.identity.element_id)
                        .or_default()
                        .push((object.class_name.clone(), other));
                }
            }
            Ok(())
        },
    )?;
    for id in &columns {
        let Some(held) = info.get(id).and_then(|pieces| pieces.first()) else {
            continue;
        };
        let listed: Vec<String> = pairs
            .get(id)
            .into_iter()
            .flatten()
            .map(|(step, other)| match info.get(other) {
                Some(pieces) => {
                    let listed: Vec<String> = pieces.iter().map(describe).collect();
                    format!("{step}:{other}:{}", listed.join("|"))
                }
                None => format!("{step}:{other}:-"),
            })
            .collect();
        println!(
            "col {id} at {} {} {} rot {} prof {} h {} pairs {}",
            held.1[0],
            held.1[1],
            held.1[2],
            held.2.map_or_else(|| "-".to_string(), |r| r.to_string()),
            held.3,
            held.4,
            listed.join(" "),
        );
    }
    Ok(())
}
