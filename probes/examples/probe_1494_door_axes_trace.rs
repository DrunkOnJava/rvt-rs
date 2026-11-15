//! #1494 (RE-474): where the stored-host placement rule stops for the doors
//! rvt-rs binds to no plain wall. Research trace, not to land.
//!
//! Output: one `door` line per decoded door with whether it carries a host
//! field, both flips, the placement X axis, the stored-host marker, its stored
//! host (`m_hostId`), and whether that host is a decoded wall with a native
//! direction and flip; then a `summary` line counting each combination.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1494_door_axes_trace -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::{
    OPENING_FLIP_FIELDS, OPENING_HOST_FIELD, OPENING_PLACEMENT_FROM_STORED_HOST_FIELD,
    OPENING_PLACEMENT_X_AXIS_FIELDS, WALL_NATIVE_DIRECTION_FIELDS, WALL_NATIVE_FLIPPED_FIELD,
    recover_partition_schema_mvp,
};
use rvt::walker::{InstanceField, WalkerLimits};

fn has(fields: &[(String, InstanceField)], name: &str) -> bool {
    fields.iter().any(|(n, _)| n == name)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let decoded = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let walls: BTreeMap<u32, (bool, bool, String)> = decoded
        .walls
        .iter()
        .filter_map(|wall| {
            Some((
                wall.id?,
                (
                    has(&wall.fields, WALL_NATIVE_DIRECTION_FIELDS[0]),
                    has(&wall.fields, WALL_NATIVE_FLIPPED_FIELD),
                    wall.class.clone(),
                ),
            ))
        })
        .collect();
    let ids: BTreeSet<u64> = decoded
        .doors
        .iter()
        .filter_map(|door| door.id.map(u64::from))
        .collect();
    let stored = rvt::research::native_symbol_cuts::instance_hosts(&mut rf, &ids)?;
    let mut summary: BTreeMap<String, usize> = BTreeMap::new();
    for door in &decoded.doors {
        let Some(id) = door.id else { continue };
        let host = door.fields.iter().find_map(|(n, v)| match v {
            InstanceField::ElementId { id, .. } if n == OPENING_HOST_FIELD => Some(*id),
            _ => None,
        });
        let flips = OPENING_FLIP_FIELDS.iter().all(|f| has(&door.fields, f));
        let axis = has(&door.fields, OPENING_PLACEMENT_X_AXIS_FIELDS[0]);
        let marked = has(&door.fields, OPENING_PLACEMENT_FROM_STORED_HOST_FIELD);
        let stored_host = stored.get(&u64::from(id)).copied();
        let wall = stored_host
            .and_then(|h| u32::try_from(h).ok())
            .and_then(|h| walls.get(&h));
        let key = format!(
            "bound={} flips={flips} axis={axis} marked={marked} stored_host={} wall_decoded={} wall_dir={} wall_flip={} wall_class={}",
            host.is_some(),
            stored_host.is_some(),
            wall.is_some(),
            wall.is_some_and(|w| w.0),
            wall.is_some_and(|w| w.1),
            wall.map_or("-", |w| w.2.as_str()),
        );
        println!("door {id} host {host:?} stored {stored_host:?} {key}");
        *summary.entry(key).or_default() += 1;
    }
    for (key, n) in summary {
        println!("summary {n} {key}");
    }
    Ok(())
}
