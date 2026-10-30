//! #1117: where a viewport's position on its sheet is stored.
//!
//! QUESTION under test: which value in a file gives a viewport's centre on
//! its sheet. No Revit IFC export carries sheets or viewports, so no
//! reference model can score the answer; the oracle is a comparison with
//! the centres the Revit API reports for the same file. This probe lists
//! the candidates for that comparison.
//!
//! FACT found (Measure runs 41044021502 and 41045152790): every view has a
//! native `Viewport` record onto its own `DBDrawing`, with `m_ownerDBViewId`
//! -1 and `m_offset` zero. A viewport placed on a sheet is a `Viewport`
//! record whose `m_ownerDBViewId` names the sheet (a `DBViewDrafting`
//! carrying `m_sheetNumber` and `m_viewName`) and whose `m_dbViewId` names
//! the view it shows; its `m_offset` is a point on the sheet in feet, with z
//! 0. The BIM4LCA office as built has 21 such viewports on 19 sheets (and
//! 173 per-view records); Core Interior and the RE1 models have none. No
//! sheet viewport has a partition element-record frame. On the office,
//! viewports placed side by side share the coordinate they are aligned on
//! (sheet L13's three plans one x, sheet L07's two one y), as box centres
//! would; whether `m_offset` is the box centre the Revit API reports, or the
//! view's origin on the sheet, is what the comparison has to show.
//!
//! HYPOTHESIS under test (a contributor's comparison on #1117): the centre
//! the Revit API reports (`Viewport.GetBoxCenter()`) is `m_offset` plus the
//! view's crop-box centre scaled to the sheet, `m_offset + m_scale * c`,
//! where `c` is the centre of `m_boundOffset` in the view's `Viewer` record
//! (`m_viewerId` of the view). On one viewport of their model it gives the
//! API's centre to the 5 decimals the API printed. `centre` below is that
//! value, printed only for a viewport that is not turned on its sheet
//! (`m_viewportOrientation` 0) and whose crop box reads as two corners or
//! three ranges; everything else prints `-`.
//!
//! Output, lengths in feet on the sheet:
//!   viewport <id> sheet <id> "<number>" "<name>" view <id> <class> "<name>"
//!            scale <m_scale> offset <x> <y> <z> label <x> <y> <z>
//!            orientation <n> viewer <id> crop <m_boundOffset> centre <x> <y>
//!                      each viewport on a sheet, from its native record
//!   summary <class> <records>
//! and, with `--records`:
//!   record <json>      every record of the four viewport classes
//!                      (`Viewport`, `GCSViewport`, `ViewportAttributes`,
//!                      `ViewportInfo`), in full
//!   view <id> <json>   the root object of the view each viewport shows
//!   viewer <id> <json> every object of that view's `Viewer` record
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1117_viewports -- MODEL.rvt [--records] [--large-model]
//!
//! `--large-model` opens the file with `rvt-ifc --large-model`'s limits and
//! lifts the native reader's own stream and group budgets to match, for a
//! model with a partition stream over 512 MiB.

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const VIEWPORT_CLASSES: [&str; 4] = [
    "Viewport",
    "GCSViewport",
    "ViewportAttributes",
    "ViewportInfo",
];

fn id_of(fields: &Value, member: &str) -> Option<u64> {
    fields.get(member)?.get("m_id")?.get("m_id64")?.as_u64()
}

fn text(fields: Option<&Value>, member: &str) -> String {
    fields
        .and_then(|fields| fields.get(member))
        .and_then(Value::as_str)
        .unwrap_or("-")
        .to_string()
}

fn find<'a>(value: &'a Value, member: &str) -> Option<&'a Value> {
    match value {
        Value::Object(map) => map
            .get(member)
            .or_else(|| map.values().find_map(|value| find(value, member))),
        Value::Array(items) => items.iter().find_map(|value| find(value, member)),
        _ => None,
    }
}

fn rows(value: &Value) -> Option<Vec<Vec<f64>>> {
    value
        .as_array()?
        .iter()
        .map(|row| -> Option<Vec<f64>> { row.as_array()?.iter().map(Value::as_f64).collect() })
        .collect()
}

/// The crop box's centre in the view's own coordinates, from `m_boundOffset`
/// read as two corners of three coordinates or as three ranges of two.
fn crop_centre(bounds: &Value) -> Option<(f64, f64)> {
    let ranges = rows(bounds)?;
    match ranges.as_slice() {
        [a, b] if a.len() == 3 && b.len() == 3 => Some(((a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0)),
        [x, y, z] if x.len() == 2 && y.len() == 2 && z.len() == 2 => {
            Some(((x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0))
        }
        _ => None,
    }
}

fn numbers(fields: &Value, member: &str) -> String {
    match fields.get(member) {
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| item.as_f64().map_or("-".to_string(), |v| v.to_string()))
            .collect::<Vec<_>>()
            .join(" "),
        Some(value) => value.to_string(),
        None => "-".to_string(),
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .expect("usage: MODEL.rvt [--records]");
    let records = std::env::args().any(|arg| arg == "--records");
    let large_model = std::env::args().any(|arg| arg == "--large-model");
    let mut rf =
        RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, large_model))?;
    let budgets = if large_model {
        Options {
            max_stream_bytes: rvt::cli::LARGE_MODEL_MAX_STREAM_BYTES,
            max_group_bytes: rvt::cli::LARGE_MODEL_MAX_INFLATE_BYTES,
            ..Options::default()
        }
    } else {
        Options::default()
    };
    let options = Options {
        selected_classes: VIEWPORT_CLASSES
            .iter()
            .map(|class| class.to_string())
            .collect(),
        ..budgets.clone()
    };
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut placed: BTreeMap<u64, Value> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let class = record.class_name.clone().unwrap_or_default();
        *counts.entry(class.clone()).or_default() += 1;
        let root = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first());
        if let Some(root) = root
            .filter(|root| class == "Viewport" && id_of(&root.fields, "m_ownerDBViewId").is_some())
        {
            placed.insert(record.identity.element_id, root.fields.clone());
        }
        if records {
            println!(
                "record {}",
                serde_json::to_string(&record).unwrap_or_default()
            );
        }
        Ok(())
    })?;

    let wanted: BTreeSet<u64> = placed
        .values()
        .flat_map(|fields| {
            [
                id_of(fields, "m_ownerDBViewId"),
                id_of(fields, "m_dbViewId"),
            ]
        })
        .flatten()
        .collect();
    let mut named: BTreeMap<u64, (String, Value)> = BTreeMap::new();
    if !wanted.is_empty() {
        let options = Options {
            selected_ids: wanted,
            ..budgets.clone()
        };
        extract_graphs(&mut rf, &options, |record| {
            if let Some(root) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
            {
                let class = record.class_name.clone().unwrap_or_default();
                named.insert(record.identity.element_id, (class, root.fields.clone()));
            }
            Ok(())
        })?;
    }
    let viewers: BTreeSet<u64> = placed
        .values()
        .filter_map(|fields| id_of(fields, "m_dbViewId"))
        .filter_map(|view| named.get(&view))
        .filter_map(|(_, fields)| id_of(fields, "m_viewerId"))
        .collect();
    let mut viewer_fields: BTreeMap<u64, Value> = BTreeMap::new();
    if !viewers.is_empty() {
        let options = Options {
            selected_ids: viewers,
            ..budgets
        };
        extract_graphs(&mut rf, &options, |record| {
            if let Some(graph) = record.graph.as_ref() {
                let objects = graph.objects.iter().map(|object| object.fields.clone());
                viewer_fields.insert(record.identity.element_id, Value::Array(objects.collect()));
            }
            Ok(())
        })?;
    }
    let mut views = BTreeSet::new();
    for (id, fields) in &placed {
        let sheet = id_of(fields, "m_ownerDBViewId").unwrap_or(0);
        let view = id_of(fields, "m_dbViewId").unwrap_or(0);
        let sheet_fields = named.get(&sheet).map(|(_, fields)| fields);
        let (view_class, view_fields) = named.get(&view).map_or(("-", None), |(class, fields)| {
            (class.as_str(), Some(fields))
        });
        let viewer = view_fields.and_then(|fields| id_of(fields, "m_viewerId"));
        let bounds = viewer
            .and_then(|viewer| viewer_fields.get(&viewer))
            .and_then(|fields| find(fields, "m_boundOffset"));
        let orientation = fields.get("m_viewportOrientation").and_then(Value::as_i64);
        let offset = fields
            .get("m_offset")
            .and_then(Value::as_array)
            .map(|items| items.iter().filter_map(Value::as_f64).collect::<Vec<_>>());
        let scale = fields.get("m_scale").and_then(Value::as_f64);
        let centre = match (
            orientation,
            offset.as_deref(),
            scale,
            bounds.and_then(crop_centre),
        ) {
            (Some(0), Some([x, y, ..]), Some(scale), Some((cx, cy))) => {
                format!("{} {}", x + scale * cx, y + scale * cy)
            }
            _ => "- -".to_string(),
        };
        println!(
            "viewport {id} sheet {sheet} \"{}\" \"{}\" view {view} {view_class} \"{}\" scale {} offset {} label {} orientation {} viewer {} crop {} centre {centre}",
            text(sheet_fields, "m_sheetNumber"),
            text(sheet_fields, "m_viewName"),
            text(view_fields, "m_viewName"),
            numbers(fields, "m_scale"),
            numbers(fields, "m_offset"),
            numbers(fields, "m_labelOrigin"),
            orientation.map_or("-".to_string(), |n| n.to_string()),
            viewer.map_or("-".to_string(), |viewer| viewer.to_string()),
            bounds.map_or("-".to_string(), Value::to_string),
        );
        views.insert(view);
    }

    for view in views.iter().filter(|_| records) {
        if let Some((_, fields)) = named.get(view) {
            println!(
                "view {view} {}",
                serde_json::to_string(fields).unwrap_or_default()
            );
        }
    }
    for (viewer, objects) in viewer_fields.iter().filter(|_| records) {
        println!(
            "viewer {viewer} {}",
            serde_json::to_string(objects).unwrap_or_default()
        );
    }
    for (class, count) in &counts {
        println!("summary {class} {count}");
    }
    Ok(())
}
