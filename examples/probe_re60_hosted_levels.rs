//! RE-60: a Level for element records that name none.
//!
//! FACT: on Revit 2024 and 2025, many elements' records name no Level.
//! Two readings recover one:
//!
//! - A railing's record names the stair or ramp it is hosted by. Revit's own
//!   IFC export contains the railing in that host's storey: 73 of 74 on
//!   Autodesk's Snowdon Towers sample.
//! - Other records name objects laid out `01 00 00 00 · u64 id · u64 Level
//!   id` (work planes and views). Where the one Level those objects carry
//!   is also the highest Level at or below the element's base, it is
//!   Revit's storey on 161 of 161 Snowdon elements: light fixtures, generic
//!   models, slab edges and wall sweeps. Taken alone, each reading is wrong
//!   more often (38 and 8 light fixtures).
//!
//! The probe prints one tab-separated row per element record that names no
//! Level:
//! - its id and category;
//! - the one stair or ramp it names, if any;
//! - the Levels the objects it names carry;
//! - the highest Level at or below its base.
//!
//! Join the rows to the storey Revit's export gives each element (by
//! `Tag`).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re60_hosted_levels -- \
//!     MODEL.rvt > rows.tsv

use rvt::RevitFile;
use rvt::element_record_level_refs::{level_at_or_below, named_levels};
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
    let Some(marker) = per::bbox_marker(version) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let objects = rvt::partition_level_records::scan_level_objects(&mut rf, &declared, &level_ids);
    eprintln!(
        "Revit {version}: {} Levels, {} objects naming a Level",
        levels.len(),
        objects.len()
    );
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut categories: Vec<(i64, &str)> = vec![
        (per::OST_WALLS, "Wall"),
        (per::OST_COLUMNS, "Column"),
        (per::OST_FLOORS, "Floor"),
        (per::OST_CEILINGS, "Ceiling"),
        (per::OST_DOORS, "Door"),
        (per::OST_WINDOWS, "Window"),
    ];
    categories.extend(per::PRODUCT_RECORD_CATEGORIES.iter().copied());
    let mut records = Vec::new();
    let mut hosts = BTreeSet::new();
    let mut seen = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let ids = assigned.get(&stream).unwrap_or(&none);
        for (category, name) in &categories {
            for record in per::find_category_records_assigned(
                &stream, buf, *category, &declared, &marker, ids,
            ) {
                if !seen.insert(record.element_id) {
                    continue;
                }
                if matches!(*name, "Stair" | "Ramp") {
                    hosts.insert(record.element_id);
                }
                records.push((*name, record));
            }
        }
    }
    let label = |id: Option<u32>| {
        id.and_then(|id| names.get(&id).copied())
            .unwrap_or("-")
            .to_string()
    };
    for (name, record) in &records {
        if !named_levels(&record.references, &level_ids).is_empty() {
            continue;
        }
        let named: Vec<u32> = record
            .references
            .iter()
            .filter_map(|slot| u32::try_from(*slot).ok())
            .filter(|id| *id != record.element_id)
            .collect();
        let host: Vec<u32> = named
            .iter()
            .copied()
            .filter(|id| hosts.contains(id))
            .collect();
        let carried: BTreeSet<&str> = named
            .iter()
            .filter_map(|id| objects.get(id))
            .filter_map(|level| names.get(level).copied())
            .collect();
        println!(
            "{}\t{name}\t{}\t{}\t{}",
            record.element_id,
            match host.as_slice() {
                [id] => id.to_string(),
                _ => "-".into(),
            },
            carried.into_iter().collect::<Vec<_>>().join("|"),
            label(level_at_or_below(&elevations, record.bbox_feet[2])),
        );
    }
    Ok(())
}
