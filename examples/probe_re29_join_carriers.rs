//! Research probe (#238 / #239 / #240, RE-29): what the `+0x88`
//! reference list says about joins.
//!
//! Three questions, one sweep of `OST_Walls` + `OST_Columns`:
//!
//! 1. Which walls does a wall record name, and does requiring that
//!    membership change the join trim it takes (#238)?
//! 2. Which walls does a column record name, and what is left of the
//!    column prism once those walls are subtracted (#239)?
//! 3. What does the 18" Basement wall record hold in the slot RE-26
//!    read as the type (#240)? The list is dumped verbatim, which is
//!    what shows that `1851` and `3897` are both in it; the wall-type
//!    join itself is RE-28's (`probe_re28_walltype_layers`).
//!
//! The emitted JSON carries the recovered geometry so it can be
//! scored against a reference IFC export offline
//! (`tools/re/ifc_world_aabb.py`). Not part of the shipped decode
//! path; the shipped paths are
//! `rvt::element_record_wall_joins::join_trims` and
//! `rvt::element_record_column_cuts::column_cut_boxes`.

use rvt::element_record_column_cuts as cuts;
use rvt::element_record_wall_joins as joins;
use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);
    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    let mut walls: Vec<per::PartitionElementRecord> = Vec::new();
    let mut columns: Vec<per::PartitionElementRecord> = Vec::new();
    for stream in &streams {
        let Ok(raw) = rf.read_stream(stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks_for_stream(stream, &raw)
            .into_iter()
            .flatten()
            .collect();
        walls.extend(per::find_category_records(
            stream,
            &concat,
            per::OST_WALLS,
            &declared,
        ));
        columns.extend(per::find_category_records(
            stream,
            &concat,
            per::OST_COLUMNS,
            &declared,
        ));
    }

    // Newest frame per ElementId, as `partition_schema_mvp` selects.
    let newest = |records: Vec<per::PartitionElementRecord>| {
        let mut by_id: BTreeMap<u32, per::PartitionElementRecord> = BTreeMap::new();
        for record in records {
            if !record.is_exported_instance() {
                continue;
            }
            let key = |r: &per::PartitionElementRecord| {
                let (_, _, dz) = r.extents_feet();
                ((dz * 10_000.0).round() as i64, r.stream.clone(), r.offset)
            };
            let better = match by_id.get(&record.element_id) {
                None => true,
                Some(existing) => key(&record) > key(existing),
            };
            if better {
                by_id.insert(record.element_id, record);
            }
        }
        by_id.into_values().collect::<Vec<_>>()
    };
    let wall_instances = newest(walls);
    let column_instances = newest(columns);
    let wall_ids: BTreeSet<u32> = wall_instances.iter().map(|r| r.element_id).collect();

    let trims = joins::join_trims(&wall_instances);
    let cut_boxes = cuts::column_cut_boxes(&column_instances, &wall_instances);

    let mut wall_rows = Vec::new();
    let mut names_none = 0usize;
    for record in &wall_instances {
        let named = joins::joined_walls(record, &wall_ids);
        if named.is_empty() {
            names_none += 1;
        }
        let trim = trims.get(&record.element_id);
        wall_rows.push(serde_json::json!({
            "element_id": record.element_id,
            "stream": record.stream,
            "offset": record.offset,
            "bbox": record.bbox_feet,
            "references": record.references,
            "joined_walls": named,
            "axis": trim.map(|t| t.axis),
            "thickness_feet": trim.map(|t| t.thickness_feet),
            "trim_start_feet": trim.map(|t| t.start_feet),
            "trim_end_feet": trim.map(|t| t.end_feet),
        }));
    }

    let mut column_rows = Vec::new();
    let mut cut = 0usize;
    for record in &column_instances {
        let named: Vec<u32> = record
            .references
            .iter()
            .filter(|slot| **slot <= u64::from(u32::MAX))
            .map(|slot| *slot as u32)
            .filter(|id| wall_ids.contains(id))
            .collect();
        let cut_box = cut_boxes.get(&record.element_id);
        if cut_box.is_some() {
            cut += 1;
        }
        column_rows.push(serde_json::json!({
            "element_id": record.element_id,
            "bbox": record.bbox_feet,
            "references": record.references,
            "named_walls": named,
            "cut_bbox": cut_box.map(|c| c.bbox_feet),
            "cut_wall_count": cut_box.map(|c| c.wall_count),
        }));
    }

    eprintln!(
        "walls {} ({names_none} name no other wall)",
        wall_rows.len()
    );
    eprintln!(
        "columns {} ({cut} cut back by a named wall)",
        column_rows.len()
    );

    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "walls": wall_rows,
        "columns": column_rows,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
