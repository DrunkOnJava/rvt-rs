//! #824 (probe): why the BIM4LCA office's triangular and notched slabs are
//! drawn as their record box.
//!
//! For each slab #824 lists, this prints what production makes of it (the
//! plan profile its fields carry, if any), the sketch lines whose owner
//! reference names it (RE-25) with their plan boxes, the curve list its
//! sketch element holds (RE-97), and whether
//! `element_record_plan_profiles::plan_profile_from_segments` closes a loop
//! from the named lines' boxes and from the listed lines' boxes.
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_slab_sketches -- MODEL.rvt

use rvt::RevitFile;
use rvt::element_record_plan_profiles as erp;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

const SLABS: &[u32] = &[
    // Twice the area: Revit's top is a right triangle.
    1994321, 1994605, 1994706, 1994714, 2057322, 2057673, 2058504, 2070694,
    // +0.35%: Revit's rectangle has a corner notch.
    2859208, 2859212, 2861138, 2861142, 2862243, 2862247, 2863245, 2863249, 2867926, 2867930,
    2869613, 2869617, 2870922, 2870926, 2871738, 2871742, 2872558, 2872562, 2897997, 2898001,
    // Arcs, holes, and the permit file's slab without its 6 holes.
    1461758, 3058084, 2991742, 1384047,
];

fn boxes_of(records: &[&per::PartitionElementRecord]) -> Vec<[f64; 4]> {
    records
        .iter()
        .map(|r| {
            [
                r.bbox_feet[0],
                r.bbox_feet[1],
                r.bbox_feet[3],
                r.bbox_feet[4],
            ]
        })
        .collect()
}

fn outcome(boxes: &[[f64; 4]]) -> String {
    match erp::plan_profile_from_segments(boxes) {
        Some(p) => format!(
            "closes: outer {} vertices, {} inner, {} pieces",
            p.outer_xy.len(),
            p.inner_xy.len(),
            p.pieces.len()
        ),
        None => "does not close".into(),
    }
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let wanted: BTreeSet<u32> = SLABS.iter().copied().collect();

    let mut production: BTreeMap<u32, String> = BTreeMap::new();
    for e in rvt::walker::iter_elements(&mut rf)? {
        let Some(id) = e.id.filter(|id| wanted.contains(id)) else {
            continue;
        };
        let profile =
            erp::plan_profile_from_fields(&e.fields).map_or("no plan profile".into(), |p| {
                let outer: Vec<String> = p
                    .outer_xy
                    .iter()
                    .map(|(x, y)| format!("({x:.3},{y:.3})"))
                    .collect();
                format!(
                    "plan profile: outer {} [{}], {} inner, {} pieces",
                    p.outer_xy.len(),
                    outer.join(" "),
                    p.inner_xy.len(),
                    p.pieces.len()
                )
            });
        production.insert(id, format!("{} {}", e.class, profile));
    }

    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let mut by_id: BTreeMap<u32, &per::PartitionElementRecord> = BTreeMap::new();
    for record in &records {
        by_id.entry(record.element_id).or_insert(record);
    }
    let ids: BTreeSet<u32> = by_id.keys().copied().collect();
    let lists = erp::scan_sketch_curve_lists(&mut rf, version, &ids);

    for slab in SLABS {
        println!(
            "slab {slab}: {}",
            production
                .get(slab)
                .map_or("no production row", String::as_str)
        );
        let named: Vec<&per::PartitionElementRecord> = by_id
            .values()
            .copied()
            .filter(|r| r.owner_reference == Some(*slab))
            .collect();
        println!(
            "  named by {} sketch lines: {}",
            named.len(),
            outcome(&boxes_of(&named))
        );
        for r in &named {
            let b = r.bbox_feet;
            println!(
                "    line {} box ({:.3},{:.3})-({:.3},{:.3}) z {:.3}..{:.3}",
                r.element_id, b[0], b[1], b[3], b[4], b[2], b[5]
            );
        }
        match lists.get(slab) {
            Some(curves) => {
                let listed: Vec<&per::PartitionElementRecord> = curves
                    .iter()
                    .filter_map(|id| by_id.get(id).copied())
                    .collect();
                println!(
                    "  sketch lists {} curves, {} with records: {}",
                    curves.len(),
                    listed.len(),
                    outcome(&boxes_of(&listed))
                );
                let missing: Vec<String> = curves
                    .iter()
                    .filter(|id| !by_id.contains_key(id))
                    .map(u32::to_string)
                    .collect();
                if !missing.is_empty() {
                    println!("    listed without a record: {}", missing.join(" "));
                }
            }
            None => println!("  no sketch curve list names it"),
        }
    }
    Ok(())
}
