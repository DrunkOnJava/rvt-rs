//! RE-97: a Sketch element's data names the element it sketches and lists
//! its curves; a sketch line's own owner reference names another element
//! where the sketch was edited.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a Sketch
//! element's data holds, 77 bytes from its element-data header, the
//! sketched element's `u64` ElementId, and a counted list `u32 n ·
//! n × (u64 ElementId · u32 key)` of its sketch lines, the keys rising (0 to
//! n − 1 on a sketch never edited, with gaps where curves were deleted). On
//! most sketches the list is exactly the lines whose owner
//! reference (the last slot of their record's second reference list,
//! RE-25) names the element. Where it is not, the owner reference names an
//! element the line was sketched for before, and the list is what Revit
//! draws: slab 1164441's top edge is a line whose owner reference names
//! 1506825, which is not exported.
//!
//! The probe lists every sketch whose list differs from the lines naming its
//! element.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re97_sketch_curve_lists -- MODEL.rvt

use rvt::RevitFile;
use rvt::element_record_plan_profiles::scan_sketch_curve_lists;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records =
        per::scan_category_records_multi(&mut rf, version, &[per::OST_SKETCH_LINES], &declared)?;
    let mut owner_of: BTreeMap<u32, u32> = BTreeMap::new();
    let mut named: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
    for record in &records {
        if let Some(owner) = record.owner_reference {
            owner_of.insert(record.element_id, owner);
            named.entry(owner).or_default().insert(record.element_id);
        }
    }
    let ids: BTreeSet<u32> = records.iter().map(|r| r.element_id).collect();
    let lists = scan_sketch_curve_lists(&mut rf, version, &ids);
    let mut differ = 0;
    for (element, curves) in &lists {
        let listed: BTreeSet<u32> = curves.iter().copied().collect();
        if named.get(element) == Some(&listed) {
            continue;
        }
        differ += 1;
        let mut by_owner: BTreeMap<u32, usize> = BTreeMap::new();
        for id in curves {
            *by_owner
                .entry(owner_of.get(id).copied().unwrap_or(0))
                .or_default() += 1;
        }
        println!(
            "  element {element:>8}  {} curves listed, {} lines name it; listed lines' owner references {by_owner:?}",
            curves.len(),
            named.get(element).map_or(0, |set| set.len())
        );
    }
    println!(
        "release {version}: {} sketches list their curves, {} of them differently from the lines naming the element",
        lists.len(),
        differ
    );
    Ok(())
}
