//! Research probe (#31 / RE-25): dump every `OST_SketchLines`
//! partition element record, the element each one names in its second
//! counted reference list, and the plan profile the group closes, so
//! the reconstruction can be scored against a reference IFC export
//! offline.
//!
//! Not part of the shipped decode path — the shipped path is
//! `rvt::element_record_plan_profiles::plan_profiles_from_sketch_line_records`,
//! reached through
//! `rvt::partition_schema_mvp::slabs_from_partition_category_records`.

use rvt::element_record_plan_profiles as profiles;
use rvt::partition_element_records as per;
use rvt::{RevitFile, compression};
use std::collections::{BTreeMap, BTreeSet};

/// Mirror of the constants `partition_schema_mvp`'s private
/// `scan_closed_plan_loops` uses, so the legacy plan-loop inventory
/// in this probe is the same population the shipped scanner sees
/// before its ArcWall / area / dedup filters (RE-25 §2).
const PLAN_LOOP_MIN_POINTS: usize = 4;
const PLAN_LOOP_MAX_POINTS: usize = 8;
const PLAN_LOOP_MIN_SPAN_FEET: f64 = 5.0;
const PLAN_LOOP_CLOSE_EPS_FEET: f64 = 0.05;

fn read_f64(buf: &[u8], off: usize) -> Option<f64> {
    let slice = buf.get(off..off + 8)?;
    let value = f64::from_le_bytes(slice.try_into().ok()?);
    value.is_finite().then_some(value)
}

fn plan_coord(value: f64) -> bool {
    value.is_finite() && value.abs() < 500.0
}

/// `(offset, point count, span_x, span_y, min_x, min_y)` per closed
/// plan-polyline candidate.
fn scan_closed_plan_loops(buf: &[u8]) -> Vec<(usize, usize, f64, f64, f64, f64)> {
    let mut found = Vec::new();
    let step = if buf.len() > 5_000_000 { 64 } else { 16 };
    let limit = buf.len().saturating_sub(PLAN_LOOP_MAX_POINTS * 16 + 16);
    let mut i = 0usize;
    while i < limit {
        for n in PLAN_LOOP_MIN_POINTS..=PLAN_LOOP_MAX_POINTS {
            if i + n * 16 + 16 > buf.len() {
                break;
            }
            let mut pts = Vec::with_capacity(n);
            let mut ok = true;
            for k in 0..n {
                let (Some(x), Some(y)) = (read_f64(buf, i + k * 16), read_f64(buf, i + k * 16 + 8))
                else {
                    ok = false;
                    break;
                };
                if !plan_coord(x) || !plan_coord(y) || (x.abs() < 1e-9 && y.abs() < 1e-9) {
                    ok = false;
                    break;
                }
                pts.push((x, y));
            }
            if !ok {
                continue;
            }
            let (x0, y0) = pts[0];
            let (xn, yn) = pts[pts.len() - 1];
            let mut closed_err = (xn - x0).hypot(yn - y0);
            if let (Some(nx), Some(ny)) = (read_f64(buf, i + n * 16), read_f64(buf, i + n * 16 + 8))
            {
                closed_err = closed_err.min((nx - x0).hypot(ny - y0));
            }
            if closed_err > PLAN_LOOP_CLOSE_EPS_FEET {
                continue;
            }
            let min_x = pts.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
            let max_x = pts.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
            let min_y = pts.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
            let max_y = pts.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
            let (span_x, span_y) = (max_x - min_x, max_y - min_y);
            if span_x < PLAN_LOOP_MIN_SPAN_FEET || span_y < PLAN_LOOP_MIN_SPAN_FEET {
                continue;
            }
            found.push((i, pts.len(), span_x, span_y, min_x, min_y));
            break;
        }
        i += step;
    }
    found
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).expect("usage: probe <file.rvt>");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info().map(|i| i.version).unwrap_or(0);

    let declared: BTreeSet<u32> = rvt::elem_table::parse_records(&mut rf)?
        .into_iter()
        .map(|r| r.id_primary)
        .collect();

    let records = per::scan_category_records_multi(
        &mut rf,
        version,
        &[
            per::OST_SKETCH_LINES,
            per::OST_FLOORS,
            per::OST_BUILDING_PAD,
        ],
        &declared,
    )?;
    let sketch_lines: Vec<per::PartitionElementRecord> = records
        .iter()
        .filter(|r| r.builtin_category == per::OST_SKETCH_LINES)
        .cloned()
        .collect();

    let mut segments: Vec<serde_json::Value> = Vec::new();
    let mut per_owner: BTreeMap<u32, usize> = BTreeMap::new();
    let mut distinct: BTreeSet<u32> = BTreeSet::new();
    for record in &sketch_lines {
        distinct.insert(record.element_id);
        if let Some(owner) = record.owner_reference {
            *per_owner.entry(owner).or_default() += 1;
        }
        segments.push(serde_json::json!({
            "stream": record.stream,
            "offset": record.offset,
            "element_id": record.element_id,
            "owner_reference": record.owner_reference,
            "preceding_reference": record.preceding_reference,
            "bbox": record.bbox_feet,
        }));
    }

    let recovered = profiles::plan_profiles_from_sketch_line_records(&sketch_lines);
    let plates: BTreeSet<u32> = records
        .iter()
        .filter(|r| {
            (r.builtin_category == per::OST_FLOORS || r.builtin_category == per::OST_BUILDING_PAD)
                && r.is_exported_instance()
        })
        .map(|r| r.element_id)
        .collect();

    let mut profile_rows: Vec<serde_json::Value> = Vec::new();
    for (owner, profile) in &recovered {
        profile_rows.push(serde_json::json!({
            "element_id": owner,
            "is_recovered_plate": plates.contains(owner),
            "segments": profile.segment_ids.len(),
            "outer": profile.outer_xy,
            "inner": profile.inner_xy,
            "plan_bounds": profile.plan_bounds_feet(),
        }));
    }
    let unresolved: Vec<u32> = per_owner
        .keys()
        .copied()
        .filter(|owner| !recovered.contains_key(owner))
        .collect();

    // The legacy plan-loop population, for the record: what the
    // pre-RE-25 scanner can see, and how much of it lands on a
    // recovered plate's plan box.
    let mut plate_boxes: Vec<[f64; 4]> = Vec::new();
    for record in &records {
        if (record.builtin_category == per::OST_FLOORS
            || record.builtin_category == per::OST_BUILDING_PAD)
            && record.is_exported_instance()
        {
            plate_boxes.push([
                record.bbox_feet[0],
                record.bbox_feet[1],
                record.bbox_feet[3],
                record.bbox_feet[4],
            ]);
        }
    }
    let mut plan_loops: Vec<serde_json::Value> = Vec::new();
    let mut plan_loop_total = 0usize;
    let mut plan_loop_on_a_plate_box = 0usize;
    let stream_names: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    for stream in stream_names {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let concat: Vec<u8> = compression::inflate_all_chunks_for_stream(&stream, &raw)
            .into_iter()
            .flatten()
            .collect();
        let loops = scan_closed_plan_loops(&concat);
        let mut points: BTreeMap<usize, usize> = BTreeMap::new();
        let mut matched = 0usize;
        for (_, n, span_x, span_y, min_x, min_y) in &loops {
            *points.entry(*n).or_default() += 1;
            if plate_boxes.iter().any(|b| {
                (b[0] - min_x).abs() < 1e-3
                    && (b[1] - min_y).abs() < 1e-3
                    && (b[2] - b[0] - span_x).abs() < 1e-3
                    && (b[3] - b[1] - span_y).abs() < 1e-3
            }) {
                matched += 1;
            }
        }
        plan_loop_total += loops.len();
        plan_loop_on_a_plate_box += matched;
        plan_loops.push(serde_json::json!({
            "stream": stream,
            "inflated_bytes": concat.len(),
            "candidates": loops.len(),
            "point_counts": points,
            "on_a_recovered_plate_plan_box": matched,
            "widest_span": loops
                .iter()
                .map(|(_, _, sx, sy, _, _)| (sx.max(*sy) * 1000.0) as i64)
                .max()
                .map(|v| v as f64 / 1000.0),
        }));
    }

    let out = serde_json::json!({
        "file": path,
        "revit_version": version,
        "declared_ids": declared.len(),
        "sketch_line_records": sketch_lines.len(),
        "distinct_sketch_line_ids": distinct.len(),
        "owners": per_owner.len(),
        "owners_that_are_recovered_plates":
            per_owner.keys().filter(|id| plates.contains(id)).count(),
        "profiles_recovered": recovered.len(),
        "owners_without_a_closed_profile": unresolved,
        "legacy_plan_loops": {
            "total": plan_loop_total,
            "on_a_recovered_plate_plan_box": plan_loop_on_a_plate_box,
            "by_stream": plan_loops,
        },
        "profiles": profile_rows,
        "segments": segments,
    });
    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}
