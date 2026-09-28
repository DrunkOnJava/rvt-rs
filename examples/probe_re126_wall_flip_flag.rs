//! RE-126: a wall's flip is one byte, followed by a second flag.
//!
//! FACT: in a wall's element data, after `WALL_FLIP_ANCHOR`, the location
//! line (`u32`) and the orientation word (`u32`) come one flip byte (0 or
//! 1) and a second flag byte. The flag is 0 followed by `00 00`, or 1
//! followed by `ff ff`. On the flowbim.ee Revit 2026 house 35 of 59 walls
//! set it, and on Core Interior (2024) 4 of 360; RE1 Architecture (2025)
//! and Snowdon Towers (2024) have none. What the flag sets is not measured.
//!
//! The probe prints the six bytes after the orientation word for each
//! wall, counted by pattern, and how many of those walls
//! `scan_wall_orientations` reads.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re126_wall_flip_flag -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_compound_structure as pcs;
use std::collections::{BTreeMap, BTreeSet};

/// How far into a wall's data the anchor is looked for.
const WINDOW: usize = 0x2000;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(layout) = rvt::partition_names::element_data_layout(version) else {
        eprintln!("Revit {version}: element data is not read on this release");
        return Ok(());
    };
    let mvp = rvt::partition_schema_mvp::recover_partition_schema_mvp(
        &mut rf,
        version,
        rvt::walker::WalkerLimits::default(),
    )?;
    let walls: BTreeSet<u32> = mvp.walls.iter().filter_map(|wall| wall.id).collect();
    let oriented = pcs::scan_wall_orientations(&mut rf, version, &walls)?;
    let mut census: BTreeMap<String, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let hits: Vec<usize> = memchr::memmem::find_iter(buf, &layout.header).collect();
        for (index, &hit) in hits.iter().enumerate() {
            let id_at = hit + layout.header.len();
            let Some(id) = layout.id_at(buf, id_at).filter(|id| walls.contains(id)) else {
                continue;
            };
            let end = hits
                .get(index + 1)
                .copied()
                .unwrap_or(buf.len())
                .min(hit.saturating_add(WINDOW))
                .min(buf.len());
            let Some(data) = buf.get(id_at + 8..end) else {
                continue;
            };
            let Some(at) = memchr::memmem::find(data, &pcs::WALL_FLIP_ANCHOR) else {
                continue;
            };
            let at = at + pcs::WALL_FLIP_ANCHOR.len() + 8;
            let bytes: Vec<String> = data
                .get(at..at + 6)
                .unwrap_or(&[])
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            census.entry(bytes.join(" ")).or_default().insert(id);
        }
    }
    println!(
        "Revit {version}: {} walls, {} with an orientation",
        walls.len(),
        oriented.len()
    );
    for (bytes, ids) in &census {
        let read = ids.iter().filter(|id| oriented.contains_key(id)).count();
        println!("  {bytes}: {} walls, {read} read", ids.len());
    }
    Ok(())
}
