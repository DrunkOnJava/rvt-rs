//! RE-96: a sketch line's curve can be an arc, stored in the arc layout RE-75
//! reads for curved walls, but without the `u64` 1 that marks a wall's arc.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, the first
//! curve record (`04 00 08 01`) in an arc sketch line's data holds start and
//! end angles, a unit X and Y axis, a radius and a centre, preceded by the
//! same bytes a straight sketch line's record is. Read as a line it gives
//! ends outside the line's record box, which is why RE-50's recorded ends did
//! not close these sketches. Read as an arc, every point of it lies in the
//! box, and the sketch closes. An outline with an arc is kept only where it
//! spans its element's record box.
//!
//! The probe lists every sketch-line owner that has an arc line: its lines,
//! how many are arcs, and for each arc its radius, centre and ends.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re96_sketch_arcs -- MODEL.rvt

use rvt::RevitFile;
use rvt::element_record_plan_profiles::{VERTEX_EPS_FEET, arc_points};
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let mut owners: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in &records {
        if let Some(owner) = record.owner_reference {
            owners
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    let ids: BTreeSet<u32> = owners.values().flat_map(|l| l.keys().copied()).collect();
    let curves = rvt::partition_beam_axes::scan_sketch_curves(&mut rf, version, &ids)?;
    let eps = VERTEX_EPS_FEET;
    let mut with_arcs = 0;
    for (owner, lines) in &owners {
        let arcs: Vec<(u32, rvt::partition_beam_axes::BoundedArc, bool)> = lines
            .iter()
            .filter_map(|(id, b)| {
                let arc = curves.get(id)?.arc?;
                let inside = arc_points(&arc)?.iter().all(|p| {
                    (0..3).all(|axis| p[axis] >= b[axis] - eps && p[axis] <= b[axis + 3] + eps)
                });
                Some((*id, arc, inside))
            })
            .filter(|(_, _, inside)| *inside)
            .collect();
        if arcs.is_empty() {
            continue;
        }
        with_arcs += 1;
        println!(
            "owner {owner:>8}  {} sketch lines, {} arcs",
            lines.len(),
            arcs.len()
        );
        for (id, arc, _) in &arcs {
            let (a, b) = (arc.point(arc.start_angle), arc.point(arc.end_angle));
            println!(
                "    line {id:>8}  radius {:.4}  centre ({:.4}, {:.4}, {:.4})  ends ({:.4}, {:.4}) ({:.4}, {:.4})",
                arc.radius, arc.centre[0], arc.centre[1], arc.centre[2], a[0], a[1], b[0], b[1]
            );
        }
    }
    println!(
        "{with_arcs} of {} sketch-line owners have an arc in its record box",
        owners.len()
    );
    Ok(())
}
