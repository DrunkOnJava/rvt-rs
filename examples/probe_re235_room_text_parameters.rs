//! RE-235 (#35, W7): which text parameter of a room is the covering Revit's
//! export writes on its space.
//!
//! Its output answered it by name: on the BIM4LCA office each room holds
//! project parameters named `Pset_SpaceCommon.FloorCovering`,
//! `Pset_SpaceCommon.WallCovering` and `Pset_SpaceCommon.CeilingCovering`.
//!
//! FACT under test: Revit's export of the BIM4LCA office writes
//! `FloorCovering` and `CeilingCovering` on 206 spaces and `WallCovering` on
//! 234, and rvt-rs reads none: no parameter named for them is found, and
//! RE-153's Floor Finish (`ROOM_FINISH_FLOOR`) is not read on these rooms.
//! Hypothesis: each is one text parameter of the room's native record, a
//! built-in finish or a project parameter.
//!
//! Output, one line per room:
//!   txt <room number> <ElementId> <parameter id>:<name|->=<text> ...
//! every text value of the room's saved parameters (RE-207), spaces as `_`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re235_room_text_parameters -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_element_parameters;
use rvt::research::partition_schema_mvp::{ROOM_NUMBER_FIELD, recover_partition_schema_mvp};
use rvt::walker::{InstanceField, WalkerLimits};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let rooms: Vec<(u32, String)> = mvp
        .rooms
        .iter()
        .filter_map(|room| {
            let number = room.fields.iter().find_map(|(name, value)| match value {
                InstanceField::String(text) if name == ROOM_NUMBER_FIELD => Some(text.clone()),
                _ => None,
            })?;
            Some((room.id?, number))
        })
        .collect();
    let ids: BTreeSet<u64> = rooms.iter().map(|(id, _)| u64::from(*id)).collect();
    let read = native_element_parameters::read(&mut rf, &ids)?;
    for (id, number) in &rooms {
        let texts: Vec<String> = read
            .elements
            .get(&u64::from(*id))
            .into_iter()
            .flatten()
            .filter_map(|parameter| {
                let text = parameter.value.as_str()?;
                Some(format!(
                    "{}:{}={}",
                    parameter.parameter_id,
                    parameter.name.as_deref().unwrap_or("-").replace(' ', "_"),
                    text.replace(' ', "_")
                ))
            })
            .collect();
        println!("txt {} {id} {}", number.replace(' ', "_"), texts.join(" "));
    }
    eprintln!(
        "{} rooms, {} with parameters read",
        rooms.len(),
        read.elements.len()
    );
    Ok(())
}
