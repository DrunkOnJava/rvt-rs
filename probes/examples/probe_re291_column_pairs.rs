//! RE-291 (#1046, W7): the elements each column's native record pairs it
//! with, beside the column's own body, for finding why Revit's export draws
//! 18 of the BIM4LCA office's joined columns at building permit as a cut
//! mesh and the other 272 as extrusions.
//!
//! FACT under test: a column joined to another element (`CleanPairGStep`,
//! `CutPairGStep`, `m_otherId`) is drawn as a mesh, with no `Length` and a
//! volume under its record box's, where the element lies across part of its
//! height, and as its extrusion where the element only meets its ends
//! (RE-247 §4 found no field of the column's own record that tells them
//! apart, and the kinds of the paired elements alike). `tools/re` scorers
//! join the output with Revit's own bodies by `Tag`.
//!
//! Output, one line per column rvt-rs exports:
//!   col <tag> z <base> box <width> <depth> <height> pairs <step>:<other>:<entity>:<base>:<height> ...
//! where the entity, base and height of an element rvt-rs does not export
//! are `-`, and all lengths are feet.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re291_column_pairs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};

type Info = (String, Option<[f64; 3]>, Option<[f64; 3]>);

fn show(value: Option<f64>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut info: BTreeMap<u64, Info> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            location_feet,
            extrusion,
            ..
        } = entity
        {
            if let Ok(id) = tag.parse::<u64>() {
                info.entry(id).or_insert_with(|| {
                    (
                        ifc_type.to_ascii_uppercase(),
                        *location_feet,
                        extrusion
                            .as_ref()
                            .map(|body| [body.width_feet, body.depth_feet, body.height_feet]),
                    )
                });
            }
        }
    }
    let columns: BTreeSet<u64> = info
        .iter()
        .filter(|(_, (entity, _, _))| entity == "IFCCOLUMN")
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
                    .and_then(serde_json::Value::as_i64)
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
        let Some((_, location, size)) = info.get(id) else {
            continue;
        };
        let listed: Vec<String> = pairs
            .get(id)
            .into_iter()
            .flatten()
            .map(|(step, other)| {
                let (entity, base, height) = match info.get(other) {
                    Some((entity, location, size)) => (
                        entity.clone(),
                        show(location.map(|at| at[2])),
                        show(size.map(|dims| dims[2])),
                    ),
                    None => ("-".to_string(), "-".to_string(), "-".to_string()),
                };
                format!("{step}:{other}:{entity}:{base}:{height}")
            })
            .collect();
        println!(
            "col {id} z {} box {} {} {} pairs {}",
            show(location.map(|at| at[2])),
            show(size.map(|dims| dims[0])),
            show(size.map(|dims| dims[1])),
            show(size.map(|dims| dims[2])),
            listed.join(" "),
        );
    }
    Ok(())
}
