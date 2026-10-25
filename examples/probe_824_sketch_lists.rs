//! #824: every copy of a slab's Sketch curve list (RE-97) and the partition
//! each Sketch's current record is routed to (RE-216).
//!
//! FACT under test: the BIM4LCA permit file's slab 1384047 has its Sketch's
//! curve list in several partitions that list different curves (its holes
//! added after its first save), and the current partition's copy lists the
//! 40 lines of Revit's outline and holes.
//!
//! Output, per element in `RVT_PROBE_OWNERS` (default 1384047, 1474132):
//!   copy <owner> sketch <id> partition <p> curves <n> <ids...>
//!   current <owner> sketch <id> holders <partitions> current <p|none>
//!   chosen <owner> <n> | chosen <owner> none
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_sketch_lists -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::{
    scan_sketch_curve_lists, sketch_curve_list_copies,
};
use rvt::research::native_document::current_partitions;
use rvt::research::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let wanted: BTreeSet<u32> = std::env::var("RVT_PROBE_OWNERS")
        .unwrap_or_else(|_| "1384047,1474132".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let lines: BTreeSet<u32> =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();
    let copies = sketch_curve_list_copies(&mut rf, version, &lines);
    let mut holders: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for (owner, list) in copies.iter().filter(|(owner, _)| wanted.contains(owner)) {
        for (sketch, partition, curves) in list {
            let ids: Vec<String> = curves.iter().map(u32::to_string).collect();
            println!(
                "copy {owner} sketch {sketch} partition {partition} curves {} {}",
                curves.len(),
                ids.join(" ")
            );
            holders.entry(*sketch).or_default().insert(*partition);
        }
    }
    let current = current_partitions(&mut rf, &holders);
    for (owner, list) in copies.iter().filter(|(owner, _)| wanted.contains(owner)) {
        let sketches: BTreeSet<u32> = list.iter().map(|(sketch, _, _)| *sketch).collect();
        for sketch in sketches {
            println!(
                "current {owner} sketch {sketch} holders {:?} current {}",
                holders.get(&sketch),
                current
                    .get(&sketch)
                    .map_or("none".to_string(), u32::to_string)
            );
        }
    }
    let chosen = scan_sketch_curve_lists(&mut rf, version, &lines);
    for owner in &wanted {
        match chosen.get(owner) {
            Some(curves) => println!("chosen {owner} {}", curves.len()),
            None => println!("chosen {owner} none"),
        }
    }
    Ok(())
}
