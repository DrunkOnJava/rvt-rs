//! #824: the BIM4LCA office's monolithic landings, for
//! `tools/re/body_forms_list.py` to print Revit's body of each.
//!
//! FACT under test: RE-214 draws a monolithic landing as its outline from
//! its top down its type's thickness, 100.4% to 104.7% of Revit's volume,
//! because Revit's underside follows a flight's soffit where the flight
//! meets the landing. This names three of them (2859208, 2861138, 3202702,
//! RE-214 §3) and every landing record of the file, so the scorer prints the
//! form of Revit's body: an extrusion with a clipped underside, a boolean, or
//! a faceted solid.
//!
//! Output:
//!   elem <id>
//!
//! Usage:
//!   cargo run --profile ci --example probe_landing_bodies -- MODEL.rvt

use rvt::RevitFile;
use rvt::walker::iter_elements;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut ids: Vec<u32> = iter_elements(&mut rf)?
        .filter(|e| e.class == "StairsLanding")
        .filter_map(|e| e.id)
        .collect();
    ids.sort_unstable();
    for id in [2859208, 2861138, 3202702] {
        println!("elem {id}");
    }
    for id in ids
        .into_iter()
        .filter(|id| ![2859208, 2861138, 3202702].contains(id))
    {
        println!("elem {id}");
    }
    Ok(())
}
