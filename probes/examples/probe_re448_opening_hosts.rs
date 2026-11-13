//! RE-448: the wall a door or window is set in, as the library binds it and
//! as the element's own native record names it, on Revit 2015 to 2027.
//!
//! FACT under test: on Revit 2015 to 2023 rvt-rs sets a door or window in
//! the nearest wall its partition record's reference list names (#439), and
//! where the list names several that is not always its host: Autodesk's
//! `rac_advanced` 2023 export voids 28 doors and windows in another wall
//! than its 2024 export does. Hypothesis: the element's native record names
//! the host in `m_hostId`, as RE-245 reads on 2024 to 2027, saved as the
//! 4-byte `{"m_id":{"m_id":n}}` before 2024.
//!
//! Output, one record per line:
//!   file <path> release <year>
//!   opening <ElementId> <Door|Window> bound <wall|-> native <host|-> <same|differs|unbound|no_native>
//!   openings <count> bound <count> same <count> differs <count> unbound <count> no_native <count>
//!
//! `bound` is `OPENING_HOST_FIELD` of the element as
//! `recover_partition_schema_mvp` returns it: the host the IFC export voids.
//! `native` is `native_symbol_cuts::instance_hosts`, the first `m_hostId` of
//! the element's native record. Run it on the same project saved by two
//! releases to compare them.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re448_opening_hosts -- MODEL.rvt...

use std::collections::BTreeSet;

use anyhow::Result;
use rvt::RevitFile;
use rvt::research::native_symbol_cuts::instance_hosts;
use rvt::research::partition_schema_mvp::{OPENING_HOST_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};

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
    let hosts = instance_hosts(&mut rf, &ids)?;
    let [
        mut bound_count,
        mut same,
        mut differs,
        mut unbound,
        mut no_native,
    ] = [0usize; 5];
    for (class, element) in &openings {
        let Some(id) = element.id else {
            continue;
        };
        let bound = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == OPENING_HOST_FIELD => Some(*id),
            _ => None,
        });
        let native = hosts.get(&u64::from(id)).copied();
        let verdict = match (bound, native) {
            (None, _) => {
                unbound += 1;
                "unbound"
            }
            (Some(_), None) => {
                no_native += 1;
                "no_native"
            }
            (Some(bound), Some(native)) if u64::from(bound) == native => {
                same += 1;
                "same"
            }
            _ => {
                differs += 1;
                "differs"
            }
        };
        if bound.is_some() {
            bound_count += 1;
        }
        let show = |value: Option<u64>| value.map_or("-".to_string(), |v| v.to_string());
        println!(
            "opening {id} {class} bound {} native {} {verdict}",
            show(bound.map(u64::from)),
            show(native)
        );
    }
    println!(
        "openings {} bound {bound_count} same {same} differs {differs} unbound {unbound} no_native {no_native}",
        openings.len()
    );
    Ok(())
}

fn main() -> Result<()> {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        anyhow::bail!("usage: probe_re448_opening_hosts MODEL.rvt...");
    }
    for path in &paths {
        if let Err(error) = probe(path) {
            println!("file {path} error {error:#}");
        }
    }
    Ok(())
}
