//! RE-450: the opening a door or window cuts in its host, as the library
//! records it, on Revit 2015 to 2027.
//!
//! FACT under test: on Revit 2015 to 2023 rvt-rs cut a door's or window's
//! opening to the element's record box, its frame and trim included, where
//! on 2024 to 2027 it cuts the outline the symbol that draws it cuts in its
//! host (RE-215, RE-94): on Autodesk's `rac_advanced` door 154488 is
//! 1.067 x 2.21 m on 2023 and 0.915 x 2.134 m on 2024. Hypothesis: the
//! symbol's `m_pCutLoops` and the instance's `InstanceInfo.m_Trf` are saved
//! on 2015 to 2023 as on 2024, and the 4-byte `{"m_id":{"m_id":n}}` ElementId
//! (RE-280) names the symbol.
//!
//! Output, one record per line:
//!   file <path> release <year>
//!   read instances <n> loops <n> transforms <n>
//!   opening <ElementId> <Door|Window> loop <width> <height>|none opening <x> <y> <base> <width> <height>|none
//!   openings <count> loop <count> placed <count>
//!
//! `loop` is the extents, feet, of the symbol's cut loop
//! (`native_symbol_cuts::instance_cuts_and_symbols`); `opening` is the
//! centre, bottom and size, feet, of the opening `recover_partition_schema_mvp`
//! records (`filler_opening_from_fields`), the outline the IFC export cuts.
//! Run it on the same project saved by two releases to compare them.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re450_door_cut_loops -- MODEL.rvt...

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_symbol_cuts::instance_cuts_and_symbols;
use rvt::research::partition_schema_mvp::{
    filler_opening_from_fields, recover_partition_schema_mvp,
};
use rvt::walker::WalkerLimits;

fn probe(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    let mvp = recover_partition_schema_mvp(&mut rf, release, WalkerLimits::default())?;
    let openings: Vec<_> = mvp
        .doors
        .iter()
        .map(|element| ("Door", element))
        .chain(mvp.windows.iter().map(|element| ("Window", element)))
        .collect();
    let ids: BTreeSet<u64> = openings
        .iter()
        .filter_map(|(_, element)| element.id.map(u64::from))
        .collect();
    let cuts = instance_cuts_and_symbols(&mut rf, &ids)?;
    println!(
        "read instances {} loops {} transforms {}",
        ids.len(),
        cuts.loops.len(),
        cuts.transforms.len()
    );
    let [mut with_loop, mut placed] = [0usize; 2];
    for (class, element) in &openings {
        let Some(id) = element.id else {
            continue;
        };
        let cut = cuts
            .loops
            .get(&u64::from(id))
            .map_or("none".to_string(), |cut| {
                with_loop += 1;
                format!("{:.4} {:.4}", cut.width_feet(), cut.height_feet())
            });
        let opening =
            filler_opening_from_fields(&element.fields).map_or("none".to_string(), |opening| {
                placed += 1;
                format!(
                    "{:.4} {:.4} {:.4} {:.4} {:.4}",
                    opening.centre[0],
                    opening.centre[1],
                    opening.base_feet,
                    opening.width_feet,
                    opening.height_feet
                )
            });
        println!("opening {id} {class} loop {cut} opening {opening}");
    }
    println!(
        "openings {} loop {with_loop} placed {placed}",
        openings.len()
    );
    Ok(())
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_re450_door_cut_loops MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
