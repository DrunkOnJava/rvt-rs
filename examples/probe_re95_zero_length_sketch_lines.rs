//! RE-95: a sketch line whose record box is a single point is a zero-length
//! segment. It has no line of its own, adds no edge, and left out it lets the
//! other lines' recorded ends close.
//!
//! FACT: on `2024_Core_Interior.rvt` (Revit 2024) each of the 20 plates
//! Revit's IFC4 export writes as `IfcShadingDevice` owns 57 `OST_SketchLines`
//! records. 56 carry a bounded line (RE-50) whose ends close into a 28-vertex
//! outer loop and a 26-vertex void; the 57th has a box of zero extent, no
//! line, and its point is an end of two of the others. Before, that record
//! made every one of them decline.
//!
//! The probe lists, for every element the box solve (RE-25) does not close,
//! its sketch lines, how many are zero-length without a line, and whether
//! the recorded ends close with and without them.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re95_zero_length_sketch_lines -- MODEL.rvt

use rvt::RevitFile;
use rvt::element_record_plan_profiles::{
    VERTEX_EPS_FEET, plan_profile_from_lines, plan_profiles_from_sketch_line_records,
};
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let solved = plan_profiles_from_sketch_line_records(&records);
    let mut owners: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in &records {
        if let Some(owner) = record.owner_reference.filter(|o| !solved.contains_key(o)) {
            owners
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    let ids: BTreeSet<u32> = owners.values().flat_map(|l| l.keys().copied()).collect();
    let lines = rvt::partition_beam_axes::scan_bounded_lines(&mut rf, version, &ids)?;
    let point = |b: &[f64; 6]| (0..3).all(|axis| (b[axis + 3] - b[axis]).abs() <= VERTEX_EPS_FEET);
    println!(
        "release {version}: {} sketch-line owners, {} closed by their boxes, {} not",
        solved.len() + owners.len(),
        solved.len(),
        owners.len()
    );
    let mut census: BTreeMap<&str, usize> = BTreeMap::new();
    for (owner, segments) in &owners {
        let zero: Vec<u32> = segments
            .iter()
            .filter(|(id, b)| point(b) && !lines.contains_key(id))
            .map(|(id, _)| *id)
            .collect();
        let ends = |skip: &[u32]| -> Option<Vec<[f64; 4]>> {
            segments
                .keys()
                .filter(|id| !skip.contains(id))
                .map(|id| {
                    let line = lines.get(id)?;
                    let (a, b) = (line.start(), line.end());
                    Some([a[0], a[1], b[0], b[1]])
                })
                .collect()
        };
        let with = ends(&[])
            .and_then(|l| plan_profile_from_lines(&l))
            .is_some();
        let without = ends(&zero)
            .and_then(|l| plan_profile_from_lines(&l))
            .is_some();
        let outcome = match (with, without, zero.is_empty()) {
            (true, _, _) => "closes from its recorded ends",
            (false, true, false) => "closes once its zero-length lines are left out",
            (false, false, false) => "has zero-length lines and does not close",
            (false, _, true) => "does not close",
        };
        println!(
            "  owner {owner:>8}  {} sketch lines, {} zero-length without a line: {outcome}",
            segments.len(),
            zero.len()
        );
        *census.entry(outcome).or_default() += 1;
    }
    println!("census:");
    for (outcome, count) in &census {
        println!("  {count:>4}  {outcome}");
    }
    Ok(())
}
