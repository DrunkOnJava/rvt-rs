//! #1842 (probe): which recovered Levels are building stories, by count.
//!
//! FACT under test: `ifc::apply_partition_level_storeys` keeps a recovered
//! Level as a storey unless its Building Story setting (RE-118) reads
//! `false`, and a Level that is not a building story binds no element. On
//! Autodesk's `rst_basic` sample every recovered Level reads `false` (9 of 9
//! on 2023 and 2024), so the export has no Level storey and falls back to
//! storeys named from record-box elevations.
//!
//! Revit's exporter (revit-ifc, CLEANROOM.md source 5:
//! `Exporter.BeginDocumentExportCommon`, `LevelUtil.FindAllLevels`) walks the
//! Levels by elevation, ties by ElementId. A Level that is not a building
//! story belongs to the nearest building story below it, and one below every
//! building story belongs to none, so its elements are contained in the
//! building. The probe counts, per file, the Levels each part of that rule
//! touches. Counts only, so it can run on held-out models.
//!
//! Output:
//!   levels release=<v> recovered=<n> building_story_true=<n> false=<n>
//!     unread=<n> false_below_first_storey=<n> false_above=<n>
//!     false_at_a_storey_elevation=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1842_level_storeys -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_level_records::recover_partition_levels;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    // Sorted by elevation, then ElementId, as Revit's exporter walks them.
    let levels = recover_partition_levels(&mut rf, version)?;
    let count = |setting: Option<bool>| {
        levels
            .iter()
            .filter(|level| level.building_story == setting)
            .count()
    };
    let first_storey = levels
        .iter()
        .position(|level| level.building_story != Some(false));
    let (mut below, mut above, mut tied) = (0, 0, 0);
    for (index, level) in levels.iter().enumerate() {
        if level.building_story != Some(false) {
            continue;
        }
        match first_storey {
            Some(first) if index > first => above += 1,
            _ => below += 1,
        }
        let elevation = level.elevation_feet.to_bits();
        if levels.iter().any(|other| {
            other.building_story != Some(false) && other.elevation_feet.to_bits() == elevation
        }) {
            tied += 1;
        }
    }
    println!(
        "levels release={version} recovered={} building_story_true={} false={} unread={} \
         false_below_first_storey={below} false_above={above} false_at_a_storey_elevation={tied}",
        levels.len(),
        count(Some(true)),
        count(Some(false)),
        count(None)
    );
    Ok(())
}
