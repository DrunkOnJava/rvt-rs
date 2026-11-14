//! #1209 (probe): for each ceiling whose sketch is several separate loops,
//! the piece that holds the first and the last curve of its Sketch's stored
//! curve list (RE-97) and the piece that holds its lowest and highest sketch
//! line ElementId, with every piece's plan area.
//!
//! FACT under test: Revit's `Qto_CoveringBaseQuantities.GrossArea` of a
//! ceiling in pieces is the area of one piece, and neither the order rvt-rs
//! writes the pieces in nor their sizes says which (#1209, Measure
//! 43552644831). RE-355 found that Revit starts a void's loop at its lowest
//! ElementId line, the first drawn. This prints what each of four rules picks,
//! so `tools/re/ceiling_piece_lines_vs_ifc.py` can say which one, if any, is
//! Revit's piece on every ceiling.
//!
//! Output:
//!
//!   file <path> release <release>
//!   json {"ceiling": id, "lines": n, "listed": n|null,
//!         "areas_sqft": [outer piece, further pieces...],
//!         "gross_sqft": [the same pieces without their holes taken off],
//!         "first_listed": i|null, "last_listed": i|null,
//!         "lowest_id": i|null, "highest_id": i|null}
//!
//! A piece index is the piece whose outer loop's edge the line's box centre
//! lies on; `null` where the line is on no outer loop (a hole's) or the
//! ceiling has no stored list.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1209_ceiling_piece_lines -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles as erpp;
use rvt::research::partition_element_records as per;

/// How far a line's box centre may lie from a loop's edge to be on it, feet.
const ON_EDGE_FEET: f64 = 1e-3;

fn area(ring: &[(f64, f64)]) -> f64 {
    let n = ring.len();
    (0..n)
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum::<f64>()
        .abs()
        / 2.0
}

fn on_ring(ring: &[(f64, f64)], p: (f64, f64)) -> bool {
    let n = ring.len();
    (0..n).any(|i| {
        let (a, b) = (ring[i], ring[(i + 1) % n]);
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len2 = dx * dx + dy * dy;
        if len2 == 0.0 {
            return false;
        }
        let t = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len2).clamp(0.0, 1.0);
        let (cx, cy) = (a.0 + t * dx, a.1 + t * dy);
        ((p.0 - cx).powi(2) + (p.1 - cy).powi(2)).sqrt() <= ON_EDGE_FEET
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!("file {path} release {version}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let ceilings: BTreeSet<u32> =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_CEILINGS], &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();
    let lines =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let line_ids: BTreeSet<u32> = lines.iter().map(|record| record.element_id).collect();
    let lists: BTreeMap<u32, Vec<u32>> = erpp::scan_sketch_curve_lists(&mut rf, version, &line_ids)
        .into_iter()
        .filter(|(owner, _)| ceilings.contains(owner))
        .collect();
    let listed: BTreeMap<u32, u32> = lists
        .iter()
        .flat_map(|(owner, curves)| curves.iter().map(move |id| (*id, *owner)))
        .collect();
    // As the export assigns them: a listed line to its list's owner, any other to the
    // element its own record names, unless that element has a list.
    let assigned: Vec<per::PartitionElementRecord> = lines
        .iter()
        .filter_map(|record| {
            let owner = match listed.get(&record.element_id) {
                Some(owner) => *owner,
                None => record
                    .owner_reference
                    .filter(|owner| !lists.contains_key(owner))?,
            };
            ceilings.contains(&owner).then(|| {
                let mut record = record.clone();
                record.owner_reference = Some(owner);
                record
            })
        })
        .collect();
    let centre: BTreeMap<u32, (f64, f64)> = assigned
        .iter()
        .map(|r| {
            (
                r.element_id,
                (
                    (r.bbox_feet[0] + r.bbox_feet[3]) / 2.0,
                    (r.bbox_feet[1] + r.bbox_feet[4]) / 2.0,
                ),
            )
        })
        .collect();
    let detail: BTreeSet<u32> = std::env::var("RVT_PROBE_CEILINGS")
        .unwrap_or_else(|_| "2094020,2094444,2636549,3191264".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut several = 0;
    for (ceiling, profile) in erpp::plan_profiles_from_sketch_line_records(&assigned) {
        if profile.pieces.is_empty() {
            continue;
        }
        several += 1;
        let outers: Vec<&Vec<(f64, f64)>> = std::iter::once(&profile.outer_xy)
            .chain(profile.pieces.iter().map(|piece| &piece.outer_xy))
            .collect();
        let inners: Vec<&Vec<Vec<(f64, f64)>>> = std::iter::once(&profile.inner_xy)
            .chain(profile.pieces.iter().map(|piece| &piece.inner_xy))
            .collect();
        let gross: Vec<f64> = outers.iter().map(|ring| area(ring)).collect();
        let net: Vec<f64> = gross
            .iter()
            .zip(&inners)
            .map(|(outer, holes)| outer - holes.iter().map(|ring| area(ring)).sum::<f64>())
            .collect();
        let piece_of = |line: Option<&u32>| -> Option<usize> {
            let point = centre.get(line?)?;
            let hits: Vec<usize> = outers
                .iter()
                .enumerate()
                .filter(|(_, ring)| on_ring(ring, *point))
                .map(|(i, _)| i)
                .collect();
            (hits.len() == 1).then(|| hits[0])
        };
        let list = lists.get(&ceiling);
        let ids = &profile.segment_ids;
        // The ceilings in RVT_PROBE_CEILINGS (default 2094020, the one the rules miss, and the three more whose pieces miss Revit's area on the timber permit file): each piece's
        // outer loop, and each line in its list's order with every piece whose edge holds it.
        if detail.contains(&ceiling) {
            for (i, ring) in outers.iter().enumerate() {
                let points: Vec<String> =
                    ring.iter().map(|(x, y)| format!("{x:.4},{y:.4}")).collect();
                println!(
                    "piece {ceiling} {i} area {:.4} loop {}",
                    net[i],
                    points.join(" ")
                );
            }
            let order: Vec<u32> = match list {
                Some(list) => list.clone(),
                None => ids.clone(),
            };
            for (position, line) in order.iter().enumerate() {
                let on: Vec<usize> = centre
                    .get(line)
                    .map(|point| {
                        outers
                            .iter()
                            .enumerate()
                            .filter(|(_, ring)| on_ring(ring, *point))
                            .map(|(i, _)| i)
                            .collect()
                    })
                    .unwrap_or_default();
                let b = assigned
                    .iter()
                    .find(|r| r.element_id == *line)
                    .map(|r| r.bbox_feet);
                println!(
                    "line {ceiling} position {position} id {line} pieces {on:?} box {}",
                    b.map(|b| format!("{:.4},{:.4} {:.4},{:.4}", b[0], b[1], b[3], b[4]))
                        .unwrap_or_else(|| "-".into())
                );
            }
        }
        println!(
            "json {}",
            serde_json::json!({
                "ceiling": ceiling,
                "lines": ids.len(),
                "listed": list.map(Vec::len),
                "areas_sqft": net,
                "gross_sqft": gross,
                "first_listed": piece_of(list.and_then(|l| l.first())),
                "last_listed": piece_of(list.and_then(|l| l.last())),
                "lowest_id": piece_of(ids.iter().min()),
                "highest_id": piece_of(ids.iter().max()),
            })
        );
    }
    println!(
        "summary ceilings {} with_list {} in_pieces {several}",
        ceilings.len(),
        lists.len()
    );
    Ok(())
}
