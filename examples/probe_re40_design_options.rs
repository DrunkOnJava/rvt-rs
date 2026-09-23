//! RE-40: a design option set's name entry names its primary option.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, each
//! design option set's partition record lists its options at `+0x56`, and
//! the set's name entry repeats that list and closes with the ElementId of
//! the primary option. Every placed instance of a recovered category whose
//! `+0x2a` slot names another option of the set is missing from Revit's own
//! IFC4 export, and every one in the primary option is in it.
//!
//! The probe prints each set with its name, options and primary, then the
//! placed instances per design option and category, marking the options
//! the exporter leaves out. Compare the ElementIds with the `Tag`s of
//! Revit's own export of the same file.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re40_design_options -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{RECOVERED_CATEGORIES, scan_category_records_multi};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let options = rf.design_options();
    println!(
        "release {version}: {} resolved option sets, {} unresolved",
        options.sets.len(),
        options.unresolved_sets.len()
    );
    for set in options.sets.values() {
        println!(
            "  set {} {:?}: options {:?}, primary {}",
            set.set_id, set.name, set.options, set.primary
        );
    }
    for set in &options.unresolved_sets {
        println!("  set {set}: unresolved");
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let categories: Vec<i64> = RECOVERED_CATEGORIES.iter().map(|(c, _)| *c).collect();
    let records = scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    // (design option, class) -> ElementIds of placed instances.
    let mut by_option: BTreeMap<(u32, &str), BTreeSet<u32>> = BTreeMap::new();
    for record in records.iter().filter(|r| r.is_exported_instance()) {
        let Some(option) = record.design_option else {
            continue;
        };
        let class = RECOVERED_CATEGORIES
            .iter()
            .find(|(c, _)| *c == record.builtin_category)
            .map(|(_, class)| *class)
            .unwrap_or("?");
        by_option
            .entry((option, class))
            .or_default()
            .insert(record.element_id);
    }
    for ((option, class), ids) in by_option {
        let status = if options.is_non_primary(option) {
            "left out"
        } else {
            "exported"
        };
        println!(
            "  option {option} {class:<14} {:>4} {status}  {:?}",
            ids.len(),
            ids.iter().take(6).collect::<Vec<_>>()
        );
    }
    Ok(())
}
