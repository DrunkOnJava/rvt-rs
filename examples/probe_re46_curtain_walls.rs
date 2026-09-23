//! RE-46: a curtain wall is a wall a curtain-wall mullion names.
//!
//! FACT: a curtain wall owns its grid's mullions, and each mullion's
//! partition element record names its wall in its reference list. On
//! Autodesk's Snowdon Towers 2024 architectural sample the placed walls a
//! placed `OST_CurtainWallMullions` record names are exactly the 42 Revit's
//! IFC4 export writes as `IfcCurtainWall` aggregates, and on RE1 Architecture
//! (Revit 2025) the one. No other wall is named by a mullion. Six Snowdon
//! walls are named only by a panel: basic walls used as a panel's infill,
//! which Revit exports as `IfcWall`.
//!
//! The probe prints each wall a mullion names, with how many panels and
//! mullions name that wall and no other curtain wall (the parts rvt-rs
//! aggregates under it), and each wall named only by panels.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re46_curtain_walls -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{
    OST_CURTAIN_WALL_MULLIONS, OST_CURTAIN_WALL_PANELS, OST_WALLS, PartitionElementRecord,
    scan_category_records_multi,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn named(record: &PartitionElementRecord, among: &BTreeSet<u32>) -> BTreeSet<u32> {
    record
        .references
        .iter()
        .filter_map(|&id| u32::try_from(id).ok())
        .filter(|id| among.contains(id) && *id != record.element_id)
        .collect()
}

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = scan_category_records_multi(
        &mut rf,
        version,
        &[
            OST_WALLS,
            OST_CURTAIN_WALL_PANELS,
            OST_CURTAIN_WALL_MULLIONS,
        ],
        &declared,
    )?;
    let placed: Vec<&PartitionElementRecord> = records
        .iter()
        .filter(|record| record.is_exported_instance())
        .collect();
    let walls: BTreeSet<u32> = placed
        .iter()
        .filter(|record| record.builtin_category == OST_WALLS)
        .map(|record| record.element_id)
        .collect();
    let mut by_mullion: BTreeSet<u32> = BTreeSet::new();
    let mut by_panel: BTreeSet<u32> = BTreeSet::new();
    for record in &placed {
        let hit = named(record, &walls);
        match record.builtin_category {
            OST_CURTAIN_WALL_MULLIONS => by_mullion.extend(hit),
            OST_CURTAIN_WALL_PANELS => by_panel.extend(hit),
            _ => {}
        }
    }
    // Parts: panels and mullions naming exactly one curtain wall, counted
    // once per ElementId.
    let mut parts: BTreeMap<u32, BTreeMap<u32, i64>> = BTreeMap::new();
    for record in &placed {
        if record.builtin_category == OST_WALLS {
            continue;
        }
        let hit = named(record, &by_mullion);
        if hit.len() == 1 {
            let wall = *hit.iter().next().expect("one wall");
            parts
                .entry(wall)
                .or_default()
                .insert(record.element_id, record.builtin_category);
        }
    }
    println!(
        "release {version}: {} placed walls, {} named by a mullion (curtain walls), {} named only by panels",
        walls.len(),
        by_mullion.len(),
        by_panel.difference(&by_mullion).count()
    );
    for wall in &by_mullion {
        let wall_parts = parts.get(wall);
        let count =
            |category| wall_parts.map_or(0, |p| p.values().filter(|&&c| c == category).count());
        println!(
            "  curtain wall {wall:>9}: {:>3} panels, {:>3} mullions",
            count(OST_CURTAIN_WALL_PANELS),
            count(OST_CURTAIN_WALL_MULLIONS)
        );
    }
    for wall in by_panel.difference(&by_mullion) {
        println!("  panel infill {wall:>9}");
    }
    Ok(())
}
