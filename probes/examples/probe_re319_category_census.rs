//! RE-319 probe: the placed instances of every `BuiltInCategory` a file's
//! partition records hold, recovered or not, on every release the exporter
//! reads (#1261).
//!
//! FACT (to be measured): on each release, the RE-21 instance rule (a
//! declared or enclosing ElementId, no container reference, the placed
//! kind) selects, in the categories of conduits (-2008132), conduit
//! fittings (-2008128), cable trays (-2008130), cable tray fittings
//! (-2008126) and gutters (-2001391), ElementIds that Revit's own export
//! holds as `Tag`, and no ElementId it does not.
//!
//! Reads a 2008 to 2023 file through `partition_element_records_2023::
//! scan_records` and a 2024 to 2027 file through the product sweep with
//! every category the file's marker frames carry, so a category that is not
//! in `RECOVERED_CATEGORIES` shows too. Prints, per category, three lines
//! that `tools/re/category_census_vs_ifc.py` scores against an export:
//!
//! - `any <id> <name> <count> <ids>`: every ElementId a decoded frame of the
//!   category carries (symbols and container members included);
//! - `rule <id> <name> <count> <ids>`: those that pass the instance rule;
//! - `category <id> <name> <count> <ids>`: those with a 3D body, in the
//!   export phase and not in a non-primary design option, which is what the
//!   exporter writes.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re319_category_census -- model.rvt
//! ```

use rvt::RevitFile;
use rvt::research::partition_element_records::{self as per, PartitionElementRecord};
use rvt::research::partition_element_records_2023 as per32;
use std::collections::{BTreeMap, BTreeSet};

fn i64_at(buf: &[u8], at: usize) -> Option<i64> {
    let end = at.checked_add(8)?;
    buf.get(at..end)
        .map(|b| i64::from_le_bytes(b.try_into().expect("8 bytes")))
}

/// Every frame of the file's marker in the `BuiltInCategory` band, decoded
/// for any category: a 2024 to 2027 file's frames with a declared id or an
/// enclosing record's.
fn records_2024(rf: &mut RevitFile, release: u32) -> Vec<PartitionElementRecord> {
    let Some(marker) = per::file_bbox_marker(rf, release) else {
        println!("the release gives no record marker");
        return Vec::new();
    };
    let Ok(table) = rvt::elem_table::parse_records(rf) else {
        return Vec::new();
    };
    let declared = rvt::elem_table::declared_ids(&table);
    let assigned = per::compute_second_prologue_ids(rf);
    let none = BTreeMap::new();
    let mut out = Vec::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        let mut categories: BTreeSet<i64> = BTreeSet::new();
        for hit in memchr::memmem::find_iter(buf, &marker) {
            let Some(start) = hit.checked_sub(per::BBOX_MARKER_OFFSET) else {
                continue;
            };
            if let Some(category) = i64_at(buf, start + per::CATEGORY_OFFSET) {
                if (per::BUILTIN_CATEGORY_MIN..=per::BUILTIN_CATEGORY_MAX).contains(&category) {
                    categories.insert(category);
                }
            }
        }
        let list: Vec<i64> = categories.into_iter().collect();
        for records in per::find_categories_records_assigned(
            &stream,
            buf,
            &list,
            &declared,
            &marker,
            assigned.get(&stream).unwrap_or(&none),
        ) {
            out.extend(records);
        }
    }
    out
}

fn name_of(category: i64) -> String {
    per::builtin_category_name(category)
        .map(|name| name.replace(' ', "_"))
        .unwrap_or_else(|| "unnamed".to_string())
}

fn print_ids(kind: &str, category: i64, ids: &BTreeSet<u32>) {
    let list: Vec<String> = ids.iter().map(u32::to_string).collect();
    println!(
        "{kind} {category} {} {} {}",
        name_of(category),
        ids.len(),
        list.join(",")
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or("usage: probe_re319_category_census <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let saved = rf.basic_file_info()?.version;
    let release = rf.layout_release()?;
    println!("release saved {saved} layout {release}");
    let records: Vec<PartitionElementRecord> = if per32::is_32bit_release(release) {
        per32::scan_records(&mut rf, release)
    } else if per::supports_revit_version(release) {
        records_2024(&mut rf, release)
    } else {
        println!("a release whose records are not read: nothing to measure");
        return Ok(());
    };
    let filtered = per32::applies_export_filters(release) || per::supports_revit_version(release);
    let (phases, options) = if filtered {
        (Some(rf.phase_filter()), Some(rf.design_options()))
    } else {
        (None, None)
    };
    println!("records {}", records.len());

    let mut any: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut rule: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut selected: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut leftout = (0usize, 0usize, 0usize);
    for record in &records {
        let category = record.builtin_category;
        any.entry(category).or_default().insert(record.element_id);
        if !record.is_exported_instance() {
            continue;
        }
        rule.entry(category).or_default().insert(record.element_id);
        if !record.has_volume() {
            leftout.0 += 1;
            continue;
        }
        if options.as_ref().is_some_and(|o| o.excludes(record)) {
            leftout.1 += 1;
            continue;
        }
        if phases.as_ref().is_some_and(|p| p.excludes(record)) {
            leftout.2 += 1;
            continue;
        }
        selected
            .entry(category)
            .or_default()
            .insert(record.element_id);
    }
    println!(
        "placed instances left out: {} without volume, {} by design option, {} by phase",
        leftout.0, leftout.1, leftout.2
    );
    for category in any.keys() {
        print_ids("any", *category, &any[category]);
        print_ids(
            "rule",
            *category,
            rule.get(category).unwrap_or(&BTreeSet::new()),
        );
        print_ids(
            "category",
            *category,
            selected.get(category).unwrap_or(&BTreeSet::new()),
        );
    }
    Ok(())
}
