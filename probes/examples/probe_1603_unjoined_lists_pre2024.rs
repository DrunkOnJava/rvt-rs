//! #1603: what the reference lists of the Revit 2015 to 2023 family instances
//! that name no type do hold, for the 68 windows of a Revit 2018 project whose
//! lists name a named id with no record (`probe_1603_instance_types_pre2024`).
//!
//! FACT to check: an instance with no symbol record of its own category in its
//! list (one `PLACEMENT_KIND_SYMBOL` record with a name entry, RE-109) still
//! names its type and family somewhere: an id the file gives a name entry and
//! that no scanned record carries. Such an id is the type, the family, or
//! something every record names (a level, a phase). The probe separates those
//! three by who else holds the id, with counts only:
//!
//! - an id held by records of other categories too is shared (level, phase,
//!   workset), and is not a type or a family of the category;
//! - an id held only by records of this category is a candidate, and the probe
//!   counts how many candidates each unjoined instance holds, how many
//!   instances hold each candidate, how many unjoined instances hold one that
//!   an instance with a symbol record holds too, and how many hold one a
//!   symbol record of the category lists (a family);
//! - the instances that do have a symbol record are counted the same way, so
//!   that the number of candidates a joined instance holds besides its symbol
//!   (its family, say) shows what an unjoined instance's candidates would be.
//!
//! The stages repeat `probe_1603_instance_types_pre2024`'s: the instances are
//! the exported ones of `scan_records`, one result per ElementId, and a frame
//! with a candidate wins over one without.
//!
//! Counts only: each line carries a category's Revit id (a constant of the
//! format) and numbers beside fixed words, never an ElementId or a name, so
//! `tools/ci/scrub-holdout-results.sh` keeps them for a held-out model.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_unjoined_lists_pre2024 -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::{PLACEMENT_KIND_SYMBOL, PartitionElementRecord};
use rvt::research::partition_element_records_2023::scan_records;
use std::collections::{BTreeMap, BTreeSet};

/// Starts every count line, so the held-out scrub can keep them by it.
const TAG: &str = "unjoined lists pre2024:";

/// The named ids with no record in one instance's list, split by who else
/// holds them, and whether the instance has a symbol record of its category.
struct Held {
    category: i64,
    has_symbol: bool,
    free: BTreeSet<u32>,
}

/// `0 a 1 b 2 c 3 or more d` for a list of counts.
fn spread(counts: impl Iterator<Item = usize>) -> String {
    let mut bins = [0usize; 4];
    for count in counts {
        bins[count.min(3)] += 1;
    }
    format!(
        "0 {} 1 {} 2 {} 3 or more {}",
        bins[0], bins[1], bins[2], bins[3]
    )
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: FILE.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let records = scan_records(&mut rf, version);
    let table = rvt::elem_table::parse_records(&mut rf)?;
    let names = rvt::research::partition_names::name_entries_2023(
        &mut rf,
        &rvt::elem_table::declared_ids(&table),
    );
    let recorded: BTreeSet<u32> = records.iter().map(|record| record.element_id).collect();

    // Who holds each named id that has no record: the categories of every
    // scanned record whose list names it.
    let free_in = |record: &PartitionElementRecord| {
        record
            .references
            .iter()
            .filter_map(|&reference| u32::try_from(reference).ok())
            .filter(|id| *id != record.element_id && !recorded.contains(id))
            .filter(|id| names.contains_key(id))
            .collect::<BTreeSet<u32>>()
    };
    let mut holders: BTreeMap<u32, BTreeSet<i64>> = BTreeMap::new();
    for record in &records {
        for id in free_in(record) {
            holders
                .entry(id)
                .or_default()
                .insert(record.builtin_category);
        }
    }

    // The symbol records by category, and the families they list.
    let mut symbols: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut symbol_families: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.placement_kind == PLACEMENT_KIND_SYMBOL)
    {
        symbols
            .entry(record.builtin_category)
            .or_default()
            .insert(record.element_id);
        if names.contains_key(&record.element_id) {
            symbol_families
                .entry(record.builtin_category)
                .or_default()
                .extend(free_in(record));
        }
    }

    // One result per instance id: a frame with a symbol candidate wins.
    let mut per_instance: BTreeMap<u32, Held> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.is_exported_instance())
    {
        let own = symbols.get(&record.builtin_category);
        let has_symbol = record.references.iter().any(|&reference| {
            u32::try_from(reference)
                .is_ok_and(|id| own.is_some_and(|set| set.contains(&id)) && names.contains_key(&id))
        });
        let held = Held {
            category: record.builtin_category,
            has_symbol,
            free: free_in(record),
        };
        match per_instance.get(&record.element_id) {
            Some(kept) if kept.has_symbol && !held.has_symbol => {}
            _ => {
                per_instance.insert(record.element_id, held);
            }
        }
    }

    // The ids only one category's records hold, and how many instances of
    // that category hold each, split by whether the instance has a symbol.
    let only_here = |category: i64, id: &u32| {
        holders
            .get(id)
            .is_some_and(|set| set.len() == 1 && set.contains(&category))
    };
    let categories: BTreeSet<i64> = per_instance
        .values()
        .filter(|held| !held.has_symbol)
        .map(|held| held.category)
        .collect();
    for category in categories {
        let of_category: Vec<&Held> = per_instance
            .values()
            .filter(|held| held.category == category)
            .collect();
        let unjoined: Vec<&Held> = of_category
            .iter()
            .copied()
            .filter(|held| !held.has_symbol)
            .collect();
        let joined: Vec<&Held> = of_category
            .iter()
            .copied()
            .filter(|held| held.has_symbol)
            .collect();
        let candidates = |held: &Held| -> BTreeSet<u32> {
            held.free
                .iter()
                .copied()
                .filter(|id| only_here(category, id))
                .collect()
        };
        let head = format!("{TAG} release {version} category {category}");
        println!(
            "{head} instances {} with a symbol record {} without {}",
            of_category.len(),
            joined.len(),
            unjoined.len()
        );
        println!(
            "{head} unjoined instances hold named ids with no record: {}",
            spread(unjoined.iter().map(|held| held.free.len()))
        );
        println!(
            "{head} unjoined instances hold ids only this category holds: {}",
            spread(unjoined.iter().map(|held| candidates(held).len()))
        );
        println!(
            "{head} joined instances hold ids only this category holds: {}",
            spread(joined.iter().map(|held| candidates(held).len()))
        );

        // How many instances hold each candidate the unjoined instances hold.
        let mut shares: BTreeMap<u32, usize> = BTreeMap::new();
        for held in &unjoined {
            for id in candidates(held) {
                *shares.entry(id).or_default() += 1;
            }
        }
        let (mut one, mut few, mut many) = (0, 0, 0);
        for count in shares.values() {
            match *count {
                1 => one += 1,
                2..=9 => few += 1,
                _ => many += 1,
            }
        }
        println!(
            "{head} ids only this category holds, held by unjoined instances {} held by 1 instance {one} by 2 to 9 {few} by 10 or more {many} most held by one id {}",
            shares.len(),
            shares.values().copied().max().unwrap_or(0)
        );

        // Candidates an instance with a symbol record holds, and the families
        // the category's symbol records list.
        let joined_ids: BTreeSet<u32> = joined.iter().flat_map(|held| candidates(held)).collect();
        let families = symbol_families.get(&category);
        println!(
            "{head} unjoined instances hold an id an instance with a symbol record holds {} hold the family of a symbol record {}",
            unjoined
                .iter()
                .filter(|held| candidates(held).iter().any(|id| joined_ids.contains(id)))
                .count(),
            unjoined
                .iter()
                .filter(|held| held
                    .free
                    .iter()
                    .any(|id| families.is_some_and(|set| set.contains(id))))
                .count()
        );
        println!(
            "{head} unjoined instances hold one id only this category holds {} of them held by 2 or more instances {}",
            unjoined
                .iter()
                .filter(|held| candidates(held).len() == 1)
                .count(),
            unjoined
                .iter()
                .filter(|held| {
                    let ids = candidates(held);
                    ids.len() == 1
                        && ids
                            .iter()
                            .all(|id| shares.get(id).copied().unwrap_or(0) >= 2)
                })
                .count()
        );
    }
    Ok(())
}
