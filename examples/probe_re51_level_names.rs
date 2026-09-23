//! RE-51: every Level's name and elevation are in its name block, on Revit
//! 2024 and 2025 projects alike.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, the
//! standalone placed Levels declared in `Global/ElemTable` (first- and
//! second-prologue frames) are Revit's 18 storeys plus 2 Levels whose name
//! block names an owner. The 18 read, from their blocks, exactly the names
//! and elevations of the storeys in Revit's own IFC4 export. The same holds
//! on Core Interior (15) and every RE1 model (2).
//!
//! The elevation is the `f64` 55 bytes after `05 00 00 00 · u16` (`0x0248`
//! on 2024, `0x025d` on 2025), confirmed by a second copy behind the same 24
//! bytes.
//!
//! The probe prints the candidate Level ids, the owned ones, and each
//! recovered Level. Compare them with the `IfcBuildingStorey` names and
//! elevations of Revit's export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re51_level_names -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_level_records::{
    elevation_marker, find_name_blocks_with, owned_level_ids, recover_partition_levels,
    scan_partition_level_ids,
};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let candidates = scan_partition_level_ids(&mut rf, version, &declared)?;
    let mut owned = BTreeSet::new();
    let mut blocks = Vec::new();
    let marker = elevation_marker(version);
    for stream in rf.partition_stream_names() {
        if let Ok(inflated) = rf.inflated_partition(&stream) {
            owned.extend(owned_level_ids(inflated.bytes(), &candidates));
            if let Some(marker) = marker {
                blocks.extend(
                    find_name_blocks_with(inflated.bytes(), &declared, &marker)
                        .into_iter()
                        .filter(|block| candidates.contains(&block.element_id)),
                );
            }
        }
    }
    let levels = recover_partition_levels(&mut rf, version)?;
    println!(
        "release {version}: {} standalone Levels, {} owned by another element ({owned:?}), {} recovered",
        candidates.len(),
        owned.len(),
        levels.len()
    );
    if levels.is_empty() {
        // Why the storey set was refused: a candidate with no block, or two
        // sharing an elevation.
        for id in candidates.difference(&owned) {
            let own: Vec<_> = blocks.iter().filter(|b| b.element_id == *id).collect();
            match own.first() {
                None => println!("  {id:>9}  no name block"),
                Some(block) => println!(
                    "  {id:>9}  {:<28} {:>10.4} ft{}",
                    block.name,
                    block.elevation_feet,
                    if own.len() > 1 {
                        " (several blocks)"
                    } else {
                        ""
                    }
                ),
            }
        }
    }
    for level in &levels {
        println!(
            "  {:>9}  {:<28} {:>10.4} ft",
            level.element_id, level.name, level.elevation_feet
        );
    }
    Ok(())
}
