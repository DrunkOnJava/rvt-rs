//! #1603: where the type join of a Revit 2015 to 2023 family instance stops,
//! for the 70 windows and 14 railings of a Revit 2018 project that name no type.
//!
//! FACT to check: `type_and_family_names_2023` gives an instance record a type
//! only when exactly one id in its reference list is a symbol record
//! (`PLACEMENT_KIND_SYMBOL`) of the instance's own category that has a name
//! entry (RE-109), and that symbol's family (the one named id in the symbol
//! record's own list that has no record of its own) resolves. The probe
//! repeats those stages for every category of exported instance and prints how
//! many instances stop at each, so a category whose instances all name no type
//! shows which stage it fails: no symbol of its category in the list, a symbol
//! with no name entry, several symbols, or a type whose family is not found.
//!
//! The counts are the stages of the join alone, with no frame selection, so a
//! category's `instances` can exceed the decode's by the frames it drops;
//! `probe_1603_type_maps_pre2024` prints the decode's own totals beside them.
//!
//! Counts only: the lines carry a category's Revit id (a constant of the
//! format) and numbers beside fixed words, never an ElementId or a name, so
//! `tools/ci/scrub-holdout-results.sh` keeps them for a held-out model.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_instance_types_pre2024 -- FILE.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records::PLACEMENT_KIND_SYMBOL;
use rvt::research::partition_element_records_2023::scan_records;
use std::collections::{BTreeMap, BTreeSet};

/// Starts every count line, so the held-out scrub can keep them by it.
const TAG: &str = "type join pre2024:";

/// How far an instance record's join got.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// No candidate: the list holds no named symbol of the category.
    NoCandidate {
        unnamed: bool,
        other_category: bool,
    },
    Several,
    NoFamily,
    Joined,
}

#[derive(Default)]
struct Counts {
    instances: usize,
    symbols: usize,
    named_symbols: usize,
    no_candidate: usize,
    unnamed: usize,
    other_category: usize,
    several: usize,
    no_family: usize,
    joined: usize,
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

    // The symbol records, by category, and every symbol's category.
    let mut symbols: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    let mut symbol_category: BTreeMap<u32, i64> = BTreeMap::new();
    let mut symbol_records = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.placement_kind == PLACEMENT_KIND_SYMBOL)
    {
        symbols
            .entry(record.builtin_category)
            .or_default()
            .insert(record.element_id);
        symbol_category.insert(record.element_id, record.builtin_category);
        symbol_records.insert(record.element_id, record);
    }
    let family_found = |type_id: u32| {
        let Some(record) = symbol_records.get(&type_id) else {
            return false;
        };
        let mut families = record.references.iter().filter_map(|&reference| {
            let id = u32::try_from(reference).ok()?;
            (id != type_id && !recorded.contains(&id))
                .then(|| names.get(&id))
                .flatten()
        });
        families.next().is_some() && families.next().is_none()
    };

    // One result per instance id: joined if any of its frames joined, else the
    // last frame's stage, as `type_and_family_names_2023` keeps the entries it
    // inserts.
    let mut per_instance: BTreeMap<u32, (i64, Stage)> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|record| record.is_exported_instance())
    {
        let own = symbols.get(&record.builtin_category);
        let candidates: Vec<u32> = record
            .references
            .iter()
            .filter_map(|&reference| {
                let id = u32::try_from(reference).ok()?;
                (own?.contains(&id) && names.contains_key(&id)).then_some(id)
            })
            .collect();
        let stage = match candidates.as_slice() {
            [] => Stage::NoCandidate {
                unnamed: record.references.iter().any(|&reference| {
                    u32::try_from(reference).is_ok_and(|id| {
                        own.is_some_and(|set| set.contains(&id)) && !names.contains_key(&id)
                    })
                }),
                other_category: record.references.iter().any(|&reference| {
                    u32::try_from(reference).is_ok_and(|id| {
                        symbol_category
                            .get(&id)
                            .is_some_and(|category| *category != record.builtin_category)
                    })
                }),
            },
            [_, _, ..] => Stage::Several,
            [type_id] if family_found(*type_id) && names.contains_key(type_id) => Stage::Joined,
            [_] => Stage::NoFamily,
        };
        let entry = per_instance
            .entry(record.element_id)
            .or_insert((record.builtin_category, stage));
        if entry.1 != Stage::Joined {
            *entry = (record.builtin_category, stage);
        }
    }

    let mut by_category: BTreeMap<i64, Counts> = BTreeMap::new();
    for (category, set) in &symbols {
        let counts = by_category.entry(*category).or_default();
        counts.symbols = set.len();
        counts.named_symbols = set.iter().filter(|id| names.contains_key(id)).count();
    }
    for (category, stage) in per_instance.values() {
        let counts = by_category.entry(*category).or_default();
        counts.instances += 1;
        match stage {
            Stage::NoCandidate {
                unnamed,
                other_category,
            } => {
                counts.no_candidate += 1;
                counts.unnamed += usize::from(*unnamed);
                counts.other_category += usize::from(*other_category);
            }
            Stage::Several => counts.several += 1,
            Stage::NoFamily => counts.no_family += 1,
            Stage::Joined => counts.joined += 1,
        }
    }
    for (category, counts) in by_category
        .iter()
        .filter(|(_, counts)| counts.instances > 0)
    {
        println!(
            "{TAG} release {version} category {category} instances {} symbol types {} named types {} \
             no candidate {} of which unnamed symbol {} of which other category symbol {} \
             several {} no family {} joined {}",
            counts.instances,
            counts.symbols,
            counts.named_symbols,
            counts.no_candidate,
            counts.unnamed,
            counts.other_category,
            counts.several,
            counts.no_family,
            counts.joined,
        );
    }
    Ok(())
}
