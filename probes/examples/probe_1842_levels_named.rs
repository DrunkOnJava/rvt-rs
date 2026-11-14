//! #1842, #1864: how many Levels does an element record's reference list
//! name, by category, and where do they lie against the record's box?
//!
//! FACT under test: a record whose reference list names exactly one Level is
//! bound to it (RE-27) and one that names exactly two is bound to its base
//! constraint (RE-59); a record that names three or more matches neither rule
//! and is left unbound (`rvt::research::element_record_level_refs`). Autodesk's
//! `rst_basic` saved by Revit 2026 holds 12 framing records that name three
//! Levels where the 2025 save names two or one, the third the same Level in
//! all 12 (#1864), and a confidential Revit 2027 structural model reports 104
//! columns, beams, slabs, stair flights and footings on no storey (#1842).
//! Whether those 104 are records that name three or more Levels, or none, is
//! what this counts.
//!
//! The probe reads every element record of the file the way the export does
//! (`partition_element_records`) and counts, per category, the placed
//! instances by the number of Levels their list names, and for the records
//! naming two or more, how many of those Levels lie at or below the record's
//! box base, within the box, and at or above its top. It prints counts only:
//! no name, no ElementId, no elevation. It is for a reporter to run on a file
//! that must stay on their machine.
//!
//! Output:
//!   file revit=<release> levels=<n> records=<n> instances=<n>
//!   levels_named <category> instances=<n> n0=<n> n1=<n> n2=<n> n3plus=<n>
//!     (one line per category that has an instance; `others` below counts
//!     the records of the category that are not placed instances)
//!   levels_pattern <category> named=<2|3plus> at_or_below/within/at_or_above=<a>/<b>/<c> instances=<n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1842_levels_named -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::element_record_level_refs::named_levels;
use rvt::research::partition_element_records as per;

/// How many of `named` lie at or below `base`, strictly within the box, and at
/// or above `top` (feet).
fn placement(elevations: &BTreeMap<u32, f64>, named: &[u32], base: f64, top: f64) -> [usize; 3] {
    let mut out = [0usize; 3];
    for id in named {
        let Some(elevation) = elevations.get(id) else {
            continue;
        };
        let at = if *elevation <= base {
            0
        } else if *elevation >= top {
            2
        } else {
            1
        };
        out[at] += 1;
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1842_levels_named MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let levels =
        rvt::research::partition_level_records::recover_partition_levels(&mut rf, version)?;
    let elevations: BTreeMap<u32, f64> = levels
        .iter()
        .map(|level| (level.element_id, level.elevation_feet))
        .collect();
    let level_ids: BTreeSet<u32> = elevations.keys().copied().collect();
    let categories: Vec<i64> = per::BUILTIN_CATEGORY_NAMES
        .iter()
        .map(|(id, _)| *id)
        .chain(per::PRODUCT_RECORD_CATEGORIES.iter().map(|(id, _)| *id))
        .collect::<BTreeSet<i64>>()
        .into_iter()
        .collect();
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let instances = records
        .iter()
        .filter(|record| record.is_exported_instance())
        .count();
    println!(
        "file revit={version} levels={} records={} instances={instances}",
        levels.len(),
        records.len()
    );
    // Per category: instances by Levels named (0, 1, 2, 3 or more), then the
    // records that are not instances.
    let mut counts: BTreeMap<&str, ([usize; 4], usize)> = BTreeMap::new();
    let mut patterns: BTreeMap<(&str, &str, [usize; 3]), usize> = BTreeMap::new();
    for record in &records {
        let category = per::builtin_category_name(record.builtin_category).unwrap_or("unnamed");
        let entry = counts.entry(category).or_default();
        if !record.is_exported_instance() {
            entry.1 += 1;
            continue;
        }
        let named = named_levels(&record.references, &level_ids);
        entry.0[named.len().min(3)] += 1;
        if named.len() >= 2 {
            let kind = if named.len() == 2 { "2" } else { "3plus" };
            let at = placement(
                &elevations,
                &named,
                record.bbox_feet[2],
                record.bbox_feet[5],
            );
            *patterns.entry((category, kind, at)).or_default() += 1;
        }
    }
    for (category, (by_named, others)) in &counts {
        let total: usize = by_named.iter().sum();
        if total == 0 {
            continue;
        }
        println!(
            "levels_named {category} instances={total} n0={} n1={} n2={} n3plus={} others={others}",
            by_named[0], by_named[1], by_named[2], by_named[3]
        );
    }
    for ((category, kind, at), count) in &patterns {
        println!(
            "levels_pattern {category} named={kind} at_or_below/within/at_or_above={}/{}/{} instances={count}",
            at[0], at[1], at[2]
        );
    }
    Ok(())
}
