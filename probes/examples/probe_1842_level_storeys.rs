//! #1842 (probe): which recovered Levels the export makes storeys.
//!
//! FACT under test: `ifc::apply_partition_level_storeys` keeps a recovered
//! Level as a storey unless its Building Story setting (RE-118) reads
//! `false`. On Autodesk's `rst_basic` sample the Levels are recovered (9 on
//! 2024) but the export writes storeys named from record-box elevations
//! instead, so every Level would read as not a building story. The probe
//! prints each recovered Level's setting, and how many of each.
//!
//! Output:
//!   levels release=<v> recovered=<n> building_story_true=<n> false=<n> unread=<n>
//!   level <ElementId> <true|false|unread> <elevation ft> <name>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1842_level_storeys -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_level_records::recover_partition_levels;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let levels = recover_partition_levels(&mut rf, version)?;
    let count = |setting: Option<bool>| {
        levels
            .iter()
            .filter(|level| level.building_story == setting)
            .count()
    };
    println!(
        "levels release={version} recovered={} building_story_true={} false={} unread={}",
        levels.len(),
        count(Some(true)),
        count(Some(false)),
        count(None)
    );
    for level in &levels {
        let setting = match level.building_story {
            Some(true) => "true",
            Some(false) => "false",
            None => "unread",
        };
        println!(
            "level {} {setting} {:.4} {}",
            level.element_id, level.elevation_feet, level.name
        );
    }
    Ok(())
}
