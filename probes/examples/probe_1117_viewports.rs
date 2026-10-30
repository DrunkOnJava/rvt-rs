//! #1117: where a viewport's position on its sheet is stored.
//!
//! QUESTION under test: which value in a file gives a viewport's centre on
//! its sheet. No Revit IFC export carries sheets or viewports, so no
//! reference model can score the answer; the oracle is a comparison with
//! the centres the Revit API reports for the same file. This probe lists
//! the candidates for that comparison.
//!
//! FACT found with its first version (Measure run 41044021502): every view
//! has a native `Viewport` record onto its own `DBDrawing`, with
//! `m_ownerDBViewId` -1 and `m_offset` zero. A viewport placed on a sheet
//! is a `Viewport` record whose `m_ownerDBViewId` names the sheet (a
//! `DBViewDrafting` carrying `m_sheetNumber`) and whose `m_dbViewId` names
//! the view it shows; its `m_offset` is not zero. The BIM4LCA office as
//! built has 21 (and 173 per-view records); Core Interior and the RE1
//! models have none.
//!
//! Output, lengths in feet on the sheet:
//!   viewport <id> sheet <id> "<number>" "<name>" view <id> <class> "<name>"
//!            scale <m_scale> offset <x> <y> label <x> <y> <z>
//!                      each viewport on a sheet, from its native record
//!   frame <id> <category> bbox <x0> <y0> <z0> <x1> <y1> <z1> centre <x> <y>
//!                      the viewport's partition element record, where one
//!                      decodes: its BuiltInCategory and box
//!   view <id> <json>   the root object of the view each viewport shows
//!                      (its crop region is among its members)
//!   record <json>      every record of the four viewport classes
//!                      (`Viewport`, `GCSViewport`, `ViewportAttributes`,
//!                      `ViewportInfo`), in full
//!   summary <class> <records>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1117_viewports -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_element_records::{
    BBOX_MARKER_OFFSET, compute_second_prologue_ids, decode_frame_as, file_bbox_marker,
};
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
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: VIEWPORT_CLASSES
            .iter()
            .map(|class| class.to_string())
            .collect(),
        ..Options::default()
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
        println!(
            "record {}",
            serde_json::to_string(&record).unwrap_or_default()
        );
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
            ..Options::default()
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
    let mut views = BTreeSet::new();
    for (id, fields) in &placed {
        let sheet = id_of(fields, "m_ownerDBViewId").unwrap_or(0);
        let view = id_of(fields, "m_dbViewId").unwrap_or(0);
        let sheet_fields = named.get(&sheet).map(|(_, fields)| fields);
        let (view_class, view_fields) = named.get(&view).map_or(("-", None), |(class, fields)| {
            (class.as_str(), Some(fields))
        });
        println!(
            "viewport {id} sheet {sheet} \"{}\" \"{}\" view {view} {view_class} \"{}\" scale {} offset {} label {}",
            text(sheet_fields, "m_sheetNumber"),
            text(sheet_fields, "m_viewName"),
            text(view_fields, "m_viewName"),
            numbers(fields, "m_scale"),
            numbers(fields, "m_offset"),
            numbers(fields, "m_labelOrigin"),
        );
        views.insert(view);
    }

    let version = rf.basic_file_info()?.version;
    if let Some(marker) = file_bbox_marker(&mut rf, version) {
        let ids: BTreeSet<u64> = placed.keys().copied().collect();
        let assigned = compute_second_prologue_ids(&mut rf);
        let mut frames = BTreeMap::new();
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            let buf = inflated.bytes();
            for at in memchr::memmem::find_iter(buf, &marker) {
                let Some(start) = at.checked_sub(BBOX_MARKER_OFFSET) else {
                    continue;
                };
                let Some(raw) = buf.get(start..start + 8) else {
                    continue;
                };
                let id = u64::from_le_bytes(raw.try_into()?);
                if !ids.contains(&id) {
                    continue;
                }
                if let Some(record) = decode_frame_as(&stream, buf, start, id as u32, &marker) {
                    frames.entry(id).or_insert(record);
                }
            }
            for (&offset, &id) in assigned.get(&stream).into_iter().flatten() {
                if !ids.contains(&u64::from(id)) {
                    continue;
                }
                if let Some(record) = decode_frame_as(&stream, buf, offset, id, &marker) {
                    frames.entry(u64::from(id)).or_insert(record);
                }
            }
        }
        for (id, record) in &frames {
            let b = record.bbox_feet;
            println!(
                "frame {id} {} bbox {} {} {} {} {} {} centre {} {}",
                record.builtin_category,
                b[0],
                b[1],
                b[2],
                b[3],
                b[4],
                b[5],
                (b[0] + b[3]) / 2.0,
                (b[1] + b[4]) / 2.0,
            );
        }
    }

    for view in &views {
        if let Some((_, fields)) = named.get(view) {
            println!(
                "view {view} {}",
                serde_json::to_string(fields).unwrap_or_default()
            );
        }
    }
    for (class, records) in &counts {
        println!("summary {class} {records}");
    }
    Ok(())
}
