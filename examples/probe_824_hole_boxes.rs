//! #824: which element records stand where Revit's export cuts roof
//! 2991742's four 600 mm holes, of any category this module names.
//!
//! FACT under test: something whose record box covers a hole in plan and
//! reaches the roof's height (a family instance with a void, an opening, a
//! duct or pipe) cuts it, though its record does not name the roof
//! (`probe_824_roof_cutters`).
//!
//! Output, per record whose plan box overlaps a hole in `RVT_PROBE_HOLES`
//! (default the four as-built holes, `x0,y0,x1,y1;...` feet) and whose
//! height reaches 252 to 256 ft:
//!   hole <index> <id> <category> <name> box <min x y z> <max x y z> refs <first 8 references>
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_hole_boxes -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let holes: Vec<[f64; 4]> = std::env::var("RVT_PROBE_HOLES")
        .unwrap_or_else(|_| {
            "151.527,45.8079,153.4955,47.7764;151.527,38.0897,153.4955,40.0582;\
             309.2414,37.7591,311.2099,39.7276;309.2414,45.4773,311.2099,47.4458"
                .into()
        })
        .split(';')
        .filter_map(|hole| {
            let v: Vec<f64> = hole
                .split(',')
                .filter_map(|x| x.trim().parse().ok())
                .collect();
            match v.as_slice() {
                [a, b, c, d] => Some([*a, *b, *c, *d]),
                _ => None,
            }
        })
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let mut categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .collect();
    for extra in [
        per::OST_DUCT_CURVES,
        per::OST_DUCT_FITTING,
        per::OST_PIPE_CURVES,
        per::OST_PIPE_FITTING,
        per::OST_SHAFT_OPENING,
        -2_000_999, // OST_RoofOpening
        -2_000_997, // OST_FloorOpening
        -2_000_151, // OST_GenericModel
        -2_001_140, // OST_MechanicalEquipment
    ] {
        if !categories.contains(&extra) {
            categories.push(extra);
        }
    }
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for record in &records {
        let b = record.bbox_feet;
        if b[5] < 252.0 || b[2] > 256.0 {
            continue;
        }
        for (index, h) in holes.iter().enumerate() {
            let overlaps = b[0] < h[2] && b[3] > h[0] && b[1] < h[3] && b[4] > h[1];
            if !overlaps {
                continue;
            }
            let name = per::BUILTIN_CATEGORY_NAMES
                .iter()
                .find(|(id, _)| *id == record.builtin_category)
                .map_or("?", |(_, name)| name);
            let refs: Vec<String> = record
                .references
                .iter()
                .take(8)
                .map(|r| r.to_string())
                .collect();
            println!(
                "hole {index} {} {} {name:?} box {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} refs {}",
                record.element_id,
                record.builtin_category,
                b[0],
                b[1],
                b[2],
                b[3],
                b[4],
                b[5],
                refs.join(",")
            );
        }
    }
    Ok(())
}
