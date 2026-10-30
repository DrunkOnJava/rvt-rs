//! #824: what rvt-rs's decode gives the BIM4LCA office's slabs whose outline
//! still differs from Revit's: their class, their body fields and the plan
//! profile the IFC writer reads back from them.
//!
//! FACT under test: roof 1461758's sketch reads as Revit's outline (its two
//! corner arcs included, `probe_824_slab_arcs`), so where its export
//! differs the outline is lost or replaced after the sketch is read.
//!
//! Output, per element in `RVT_PROBE_OWNERS` (default 1461758, 2991742,
//! 1384047, 3058084):
//!   owner <id> class <class> list <list>
//!   field <id> <name> <short value>      every field whose name holds
//!                                         profile, plan, body, sketch, void or outline
//!   profile <id> outer <n> area <ft2> bounds <x0 y0 x1 y1> voids <n> pieces <n> | profile <id> none
//!   vertex <id> <x> <y>                   the outer loop
//!
//! `tools/re/slab_outlines_list.py` prints Revit's outline of each owner.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_824_slab_decode -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::element_record_plan_profiles::plan_profile_from_fields;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::{DecodedElement, InstanceField, WalkerLimits};
use std::collections::BTreeSet;

fn short(value: &InstanceField) -> String {
    let text = format!("{value:?}");
    if text.len() > 160 {
        format!("{}... ({} chars)", &text[..160], text.len())
    } else {
        text
    }
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let wanted: BTreeSet<u32> = std::env::var("RVT_PROBE_OWNERS")
        .unwrap_or_else(|_| "1461758,2991742,1384047,3058084".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let lists: [(&str, &Vec<DecodedElement>); 2] =
        [("slabs", &mvp.slabs), ("products", &mvp.products)];
    for (list, elements) in lists {
        for element in elements.iter() {
            let Some(id) = element.id.filter(|id| wanted.contains(id)) else {
                continue;
            };
            println!("owner {id} class {} list {list}", element.class);
            for (name, value) in &element.fields {
                let lower = name.to_ascii_lowercase();
                if ["profile", "plan", "body", "sketch", "void", "outline"]
                    .iter()
                    .any(|key| lower.contains(key))
                {
                    println!("field {id} {name} {}", short(value));
                }
            }
            match plan_profile_from_fields(&element.fields) {
                Some(profile) => {
                    let ring = &profile.outer_xy;
                    let n = ring.len();
                    let area = (0..n)
                        .map(|i| {
                            let (a, b) = (ring[i], ring[(i + 1) % n]);
                            a.0 * b.1 - b.0 * a.1
                        })
                        .sum::<f64>()
                        .abs()
                        / 2.0;
                    let bounds = profile.plan_bounds_feet().unwrap_or([0.0; 4]);
                    println!(
                        "profile {id} outer {n} area {area:.4} bounds {:.4} {:.4} {:.4} {:.4} voids {} pieces {}",
                        bounds[0],
                        bounds[1],
                        bounds[2],
                        bounds[3],
                        profile.inner_xy.len(),
                        profile.pieces.len()
                    );
                    for (x, y) in ring {
                        println!("vertex {id} {x:.4} {y:.4}");
                    }
                    for (index, void) in profile.inner_xy.iter().enumerate() {
                        let n = void.len();
                        let area = (0..n)
                            .map(|i| {
                                let (a, b) = (void[i], void[(i + 1) % n]);
                                a.0 * b.1 - b.0 * a.1
                            })
                            .sum::<f64>()
                            .abs()
                            / 2.0;
                        let xs = void.iter().map(|p| p.0);
                        let ys = void.iter().map(|p| p.1);
                        println!(
                            "void {id} {index} {n} vertices area {area:.4} x {:.4}..{:.4} y {:.4}..{:.4}",
                            xs.clone().fold(f64::MAX, f64::min),
                            xs.fold(f64::MIN, f64::max),
                            ys.clone().fold(f64::MAX, f64::min),
                            ys.fold(f64::MIN, f64::max)
                        );
                    }
                }
                None => println!("profile {id} none"),
            }
        }
    }
    Ok(())
}
