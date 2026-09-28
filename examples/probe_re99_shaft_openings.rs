//! RE-99: a shaft opening (`OST_ShaftOpening`, −2000996) is sketched as a
//! floor is, and its record box spans its height. Revit's export cuts its
//! outline from the floors, roofs and ceilings within that height.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, 9 element
//! records carry category −2000996. Shaft 759388's box runs from −17.92 ft to
//! 58.83 ft, and its Sketch (RE-97) closes into a 9.17 × 8.25 ft rectangle.
//! That rectangle is missing from Revit's IFC4 outline of every floor within
//! the height that it overlaps. Most shafts' sketches also hold the two
//! diagonals of the X Revit draws across a shaft in plan; they cross each
//! other, and without them the boundary closes (8 of the 9 shafts).
//!
//! The probe lists each shaft with its height and whether its sketch closes,
//! then, in the exported IFC, which elements carry each shaft's outline as a
//! void (run `rvt-ifc` first and pass the output).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re99_shaft_openings -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let shafts = per::scan_category_records(&mut rf, version, per::OST_SHAFT_OPENING, &declared)?;
    let mut seen = std::collections::BTreeSet::new();
    println!("release {version}: shaft opening records");
    for record in shafts.iter().filter(|r| seen.insert(r.element_id)) {
        let b = record.bbox_feet;
        println!(
            "  shaft {:>8}  plan ({:.3}, {:.3})..({:.3}, {:.3})  height {:.3}..{:.3}",
            record.element_id, b[0], b[1], b[3], b[4], b[2], b[5]
        );
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut cut = 0;
    for element in mvp.slabs.iter().chain(mvp.products.iter()) {
        let Some(profile) =
            rvt::element_record_plan_profiles::plan_profile_from_fields(&element.fields)
        else {
            continue;
        };
        for void in &profile.inner_xy {
            let (lo, hi) = void.iter().fold(
                ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
                |(lo, hi), &(x, y)| ((lo.0.min(x), lo.1.min(y)), (hi.0.max(x), hi.1.max(y))),
            );
            let matched = shafts.iter().any(|r| {
                let b = r.bbox_feet;
                (b[0] - lo.0).abs() < 1e-6
                    && (b[1] - lo.1).abs() < 1e-6
                    && (b[3] - hi.0).abs() < 1e-6
                    && (b[4] - hi.1).abs() < 1e-6
            });
            if matched {
                cut += 1;
                println!(
                    "  {} {:?} carries a shaft's outline as a void",
                    element.class, element.id
                );
            }
        }
    }
    println!("{cut} voids are shaft outlines");
    Ok(())
}
