//! RE-59: an element record naming two Levels names its base and top
//! constraint, and the base is the one Revit contains it in.
//!
//! FACT: the counted reference list at `+0x88` of a Revit 2024 or 2025
//! partition element record (RE-27) names the Levels that constrain the
//! element. A wall, column or stair names two: its base and its top
//! constraint, in no fixed order.
//!
//! The base constraint is the higher of the two at or below the record's
//! base, else the lower. On every record naming two Levels, it is the
//! storey Revit's own IFC export contains the element in: Snowdon Towers
//! 650 of 650, 2024_Core_Interior 482 of 482, RE1 Architecture 7 of 7.
//!
//! The probe prints one tab-separated row per element record: its id,
//! category, container, placement kind, reference count, box base and top
//! (feet), and each Level it names as `id:name:elevation`. Join the rows to
//! the storey Revit's export contains each element in (by `Tag`), as
//! `tools/re/storeys_vs_ifc.py` does for rvt-rs's own export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re59_base_constraint_levels -- \
//!     MODEL.rvt > records.tsv

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels = rvt::partition_level_records::recover_partition_levels(&mut rf, version)?;
    let by_id: BTreeMap<u32, (String, f64)> = levels
        .iter()
        .map(|level| (level.element_id, (level.name.clone(), level.elevation_feet)))
        .collect();
    let Some(marker) = per::bbox_marker(version) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let level_ids: BTreeSet<u32> = by_id.keys().copied().collect();
    eprintln!("Revit {version}: {} Levels", levels.len());
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut categories: Vec<(i64, &str)> = vec![
        (per::OST_WALLS, "Wall"),
        (per::OST_COLUMNS, "Column"),
        (per::OST_FLOORS, "Floor"),
        (per::OST_ROOFS, "Roof"),
        (per::OST_CEILINGS, "Ceiling"),
        (per::OST_DOORS, "Door"),
        (per::OST_WINDOWS, "Window"),
    ];
    categories.extend(per::PRODUCT_RECORD_CATEGORIES.iter().copied());
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
                let named: Vec<String> =
                    rvt::element_record_level_refs::named_levels(&record.references, &level_ids)
                        .iter()
                        .map(|id| {
                            let (level, elevation) = &by_id[id];
                            format!("{id}:{level}:{elevation:.6}")
                        })
                        .collect();
                println!(
                    "{}\t{name}\t{}\t{:#x}\t{}\t{:.6}\t{:.6}\t{}",
                    record.element_id,
                    record.container,
                    record.placement_kind,
                    record.references.len(),
                    record.bbox_feet[2],
                    record.bbox_feet[5],
                    named.join("|")
                );
            }
        }
    }
    Ok(())
}
