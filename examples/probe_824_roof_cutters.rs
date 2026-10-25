//! #824: which elements name roof 2991742 in their reference lists, the
//! candidates for what cuts the four 600 mm holes Revit's export writes in
//! it, none of which the roof's sketch draws.
//!
//! FACT under test: the holes are cut by elements hosted on the roof (family
//! instances with a void, or openings), whose records name the roof and whose
//! record boxes stand where the holes are.
//!
//! Output, per element record of a category in `BUILTIN_CATEGORY_NAMES`
//! whose first reference list holds an owner in `RVT_PROBE_OWNERS` (default
//! 2991742):
//!   ref <owner> <id> <category> <name> box <min x y z> <max x y z> container <id|none>
//!
//! `tools/re/slab_outlines_list.py` prints the holes of Revit's roof.
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_roof_cutters -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use std::collections::BTreeSet;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let owners: BTreeSet<u64> = std::env::var("RVT_PROBE_OWNERS")
        .unwrap_or_else(|_| "2991742".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for owner in &owners {
        println!("owner {owner}");
    }
    for record in &records {
        for owner in record.references.iter().filter(|r| owners.contains(r)) {
            let name = per::BUILTIN_CATEGORY_NAMES
                .iter()
                .find(|(id, _)| *id == record.builtin_category)
                .map_or("?", |(_, name)| name);
            let b = record.bbox_feet;
            println!(
                "ref {owner} {} {} {name:?} box {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} container {}",
                record.element_id,
                record.builtin_category,
                b[0],
                b[1],
                b[2],
                b[3],
                b[4],
                b[5],
                if record.container == per::CONTAINER_NONE {
                    "none".to_string()
                } else {
                    record.container.to_string()
                }
            );
        }
    }
    Ok(())
}
