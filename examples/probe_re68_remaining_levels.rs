//! RE-68: the storey of elements whose record names no Level.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, after RE-59
//! and RE-60, 102 exported elements are on no storey. Compared with every
//! copy Revit's own IFC export writes of each (a multistory stair and its
//! railings appear once per storey):
//! - 11 railings name a multistory stair whose base sits exactly at L3, one
//!   of the storeys Revit writes the stair and railing on;
//! - 16 light fixtures name the one light fixture (a chandelier) whose
//!   nested parts they are, and Revit puts them on its storey;
//! - 71 slab edges, light fixtures, wall sweeps, generic models, hardscape
//!   and a stair are on the Level their base elevation gives. That is the
//!   Level just above the base when it is at most 0.233 ft below it (5 of
//!   them), and otherwise the Level at or below the base, where the next
//!   Level up is at least 0.333 ft above;
//! - 4 plumbing fixtures follow the Level object they name instead.
//!
//! The probe prints one tab-separated row per record of those categories
//! that names no Level:
//! - its id and class, and its base elevation;
//! - the Level [`rvt::element_record_level_refs::elevation_band_level`]
//!   gives;
//! - the Levels the objects it names carry.
//!
//! Join the rows to Revit's export by `Tag`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re68_remaining_levels -- \
//!     MODEL.rvt > rows.tsv

use rvt::RevitFile;
use rvt::element_record_level_refs::{elevation_band_level, named_levels};
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels = rvt::partition_level_records::recover_partition_levels(&mut rf, version)?;
    let elevations: BTreeMap<u32, f64> = levels
        .iter()
        .map(|level| (level.element_id, level.elevation_feet))
        .collect();
    let names: BTreeMap<u32, &str> = levels
        .iter()
        .map(|level| (level.element_id, level.name.as_str()))
        .collect();
    let level_ids: BTreeSet<u32> = elevations.keys().copied().collect();
    let objects = rvt::partition_level_records::scan_level_objects(&mut rf, &declared, &level_ids);
    let wanted: Vec<(i64, &str)> = per::PRODUCT_RECORD_CATEGORIES
        .iter()
        .copied()
        .filter(|(_, class)| {
            rvt::partition_schema_mvp::ELEVATION_PLACED_CLASSES.contains(class)
                || matches!(*class, "PlumbingFixture" | "Railing")
        })
        .collect();
    let categories: Vec<i64> = wanted.iter().map(|(category, _)| *category).collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    eprintln!(
        "Revit {version}: {} Levels, {} records of the measured categories",
        levels.len(),
        records.len()
    );
    let label = |id: Option<u32>| {
        id.and_then(|id| names.get(&id).copied())
            .unwrap_or("-")
            .to_string()
    };
    let mut seen = BTreeSet::new();
    for record in &records {
        if !seen.insert(record.element_id)
            || !named_levels(&record.references, &level_ids).is_empty()
        {
            continue;
        }
        let class = wanted
            .iter()
            .find(|(category, _)| *category == record.builtin_category)
            .map_or("?", |(_, class)| class);
        let base = record.bbox_feet[2];
        let carried: BTreeSet<&str> = record
            .references
            .iter()
            .filter_map(|slot| u32::try_from(*slot).ok())
            .filter_map(|id| objects.get(&id))
            .filter_map(|level| names.get(level).copied())
            .collect();
        println!(
            "{}\t{class}\t{base:.3}\t{}\t{}",
            record.element_id,
            label(elevation_band_level(&elevations, base)),
            carried.into_iter().collect::<Vec<_>>().join("|"),
        );
    }
    Ok(())
}
