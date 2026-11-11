//! #1694: the batches the saved-mesh pass plans on a file, and the graph objects
//! each one holds.
//!
//! FACT under test: the peak of the saved-mesh pass is the decoded graph of one
//! batch (`native_saved_scene::extract_in_batches_with_control`), and a batch
//! holds far more than the 20,000 graph objects the plan aims at. dhat
//! (Measure 42960663793) puts 78% to 84% of the pass's own peak on six files in
//! the field maps of decoded graph objects, and the owner records alone are
//! small (Measure 42971398698: 18,280 objects in all on `rac_basic` 2023, the
//! largest 1,849), so what the pass holds is the shared symbols the owners name.
//!
//! This reads the owners the pass reads (channel 103, the ids of every building
//! element), then every shared symbol they name and the symbols those name, one
//! level at a time and keeping only counts, and replays the pass's plan on those
//! counts: the sort of the owners, the first batch of 8 symbols, the next
//! batch's size from the graph objects the one before decoded for each, the
//! closure each batch reads. It prints, per batch, the owners, the symbols they
//! name, the shared records read (the closure) and their graph objects, so the
//! batch that sets the peak and the record that sets the batch are named.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1694_batch_plan -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportQualityMode, RvtDocExporter};
use rvt::research::native_document::{self, Record};

// The pass's own constants (`native_saved_scene`), copied: the plan is replayed.
const BATCH_OBJECT_BUDGET: usize = 20_000;
const FIRST_BATCH_SYMBOLS: usize = 8;
const MIN_BATCH_SYMBOLS: usize = 4;
const MAX_BATCH_SYMBOLS: usize = 1024;

/// `native_metadata::identifier`, which is not public.
fn identifier(value: &serde_json::Value) -> Option<i64> {
    if let Some(n) = value.as_i64() {
        return Some(n);
    }
    ["m_id", "m_id64"]
        .into_iter()
        .find_map(|key| value.get(key))
        .and_then(identifier)
}

/// The pass's `instance_symbols`: what the `InstanceInfo` objects of a record's
/// graph name.
fn instance_symbols(record: &Record) -> Vec<u64> {
    let Some(graph) = &record.graph else {
        return Vec::new();
    };
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "InstanceInfo")
        .filter_map(|object| {
            identifier(&object.fields["m_symbolId"])
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id > 0)
        })
        .collect()
}

/// The pass's `add_references`.
fn add_references(
    ids: impl IntoIterator<Item = u64>,
    owner_references: &BTreeMap<u64, Vec<u64>>,
    expanded: &mut BTreeSet<u64>,
    attempted: &BTreeSet<u64>,
    wanted: &mut BTreeSet<u64>,
) {
    let mut pending: Vec<u64> = ids.into_iter().collect();
    while let Some(id) = pending.pop() {
        match owner_references.get(&id) {
            Some(references) => {
                if expanded.insert(id) {
                    pending.extend(references.iter().copied());
                }
            }
            None => {
                if !attempted.contains(&id) {
                    wanted.insert(id);
                }
            }
        }
    }
}

/// What a shared symbol's record holds.
struct Symbol {
    class: String,
    objects: usize,
    body_bytes: usize,
    references: Vec<u64>,
}

/// One planned batch.
struct Batch {
    owners: usize,
    named: usize,
    shared: usize,
    objects: usize,
    largest: (u64, usize),
    limit: usize,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1694_batch_plan MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(path, rvt::cli::open_limits(None, None, false))?;
    let model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                type_guid: Some(tag),
                ..
            } => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut options = native_document::Options {
        selected_ids: ids.clone(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };

    // The owners, as the pass reads them first.
    let mut owner_references = BTreeMap::<u64, Vec<u64>>::new();
    let mut owner_objects = BTreeMap::<u64, usize>::new();
    native_document::extract_graphs(&mut rf, &options, |r| {
        let id = r.identity.element_id;
        owner_objects.insert(id, r.graph.as_ref().map_or(0, |g| g.objects.len()));
        owner_references.insert(id, instance_symbols(&r));
        Ok(())
    })?;

    // Every shared symbol those name, and the ones they name, a level at a time.
    let mut symbols = BTreeMap::<u64, Symbol>::new();
    let mut attempted = BTreeSet::<u64>::new();
    let mut wanted: BTreeSet<u64> = owner_references
        .values()
        .flatten()
        .copied()
        .filter(|symbol| !owner_references.contains_key(symbol))
        .collect();
    let mut owner_links_from_symbols = 0usize;
    for _ in 0..128 {
        if wanted.is_empty() {
            break;
        }
        attempted.extend(&wanted);
        options.selected_ids = std::mem::take(&mut wanted);
        let mut found = Vec::new();
        native_document::extract_graphs(&mut rf, &options, |r| {
            let references = instance_symbols(&r);
            found.extend(references.iter().copied());
            symbols.insert(
                r.identity.element_id,
                Symbol {
                    class: r.class_name.clone().unwrap_or_default(),
                    objects: r.graph.as_ref().map_or(0, |g| g.objects.len()),
                    body_bytes: r.source.body_bytes,
                    references,
                },
            );
            Ok(())
        })?;
        for id in found {
            if owner_references.contains_key(&id) {
                owner_links_from_symbols += 1;
            } else if !attempted.contains(&id) {
                wanted.insert(id);
            }
        }
    }

    // The plan, as `extract_in_batches_with_control` makes it.
    let named_owners: BTreeSet<u64> = owner_references
        .values()
        .flatten()
        .copied()
        .filter(|symbol| owner_references.contains_key(symbol))
        .collect();
    let streamed: BTreeSet<u64> = owner_references
        .iter()
        .filter(|(id, references)| references.is_empty() && !named_owners.contains(*id))
        .map(|(&id, _)| id)
        .collect();
    type Named = (Option<u64>, u64, BTreeSet<u64>);
    let mut named: Vec<Named> = owner_references
        .iter()
        .filter(|(id, _)| !streamed.contains(*id))
        .map(|(&id, references)| {
            let own: BTreeSet<u64> = references
                .iter()
                .copied()
                .filter(|symbol| !owner_references.contains_key(symbol))
                .collect();
            (own.iter().next().copied(), id, own)
        })
        .collect();
    named.sort();
    let mut naming_count = BTreeMap::<u64, usize>::new();
    for (_, _, own) in &named {
        for symbol in own {
            *naming_count.entry(*symbol).or_default() += 1;
        }
    }
    let kept_owner_objects: usize = named
        .iter()
        .map(|(_, id, _)| owner_objects.get(id).copied().unwrap_or(0))
        .sum();

    let mut batches: Vec<Batch> = Vec::new();
    let mut limit = FIRST_BATCH_SYMBOLS;
    let mut next = 0;
    while next < named.len() {
        let mut batch_ids = Vec::new();
        let mut batch_symbols = BTreeSet::<u64>::new();
        let used_limit = limit;
        while let Some((_, id, own)) = named.get(next) {
            let ends_symbol_free = batch_symbols.is_empty() && !own.is_empty();
            if !batch_ids.is_empty()
                && (ends_symbol_free || batch_symbols.union(own).count() > limit)
            {
                break;
            }
            batch_ids.push(*id);
            batch_symbols.extend(own);
            next += 1;
        }
        let mut shared = BTreeSet::<u64>::new();
        let mut attempted = BTreeSet::<u64>::new();
        let mut expanded = BTreeSet::<u64>::new();
        let mut wanted = BTreeSet::<u64>::new();
        add_references(
            batch_ids.iter().copied(),
            &owner_references,
            &mut expanded,
            &attempted,
            &mut wanted,
        );
        for _ in 0..128 {
            if wanted.is_empty() {
                break;
            }
            attempted.extend(&wanted);
            let now = std::mem::take(&mut wanted);
            let mut found = Vec::new();
            for id in now {
                shared.insert(id);
                if let Some(symbol) = symbols.get(&id) {
                    found.extend(symbol.references.iter().copied());
                }
            }
            add_references(
                found,
                &owner_references,
                &mut expanded,
                &attempted,
                &mut wanted,
            );
        }
        let objects: usize = shared
            .iter()
            .filter_map(|id| symbols.get(id))
            .map(|symbol| symbol.objects)
            .sum();
        let largest = shared
            .iter()
            .filter_map(|id| symbols.get(id).map(|symbol| (*id, symbol.objects)))
            .max_by_key(|(_, objects)| *objects)
            .unwrap_or((0, 0));
        if !batch_symbols.is_empty() {
            limit = (BATCH_OBJECT_BUDGET.saturating_mul(shared.len().max(1)) / objects.max(1))
                .clamp(MIN_BATCH_SYMBOLS, MAX_BATCH_SYMBOLS);
        }
        batches.push(Batch {
            owners: batch_ids.len(),
            named: batch_symbols.len(),
            shared: shared.len(),
            objects,
            largest,
            limit: used_limit,
        });
    }

    let symbol_objects: usize = symbols.values().map(|symbol| symbol.objects).sum();
    let symbol_bytes: usize = symbols.values().map(|symbol| symbol.body_bytes).sum();
    println!(
        "owners {} (named {}, streamed {}) owner_objects_kept {} | shared symbol records {} graph_objects {} body_mib {:.1} | symbol->owner links {}",
        owner_references.len(),
        named.len(),
        streamed.len(),
        kept_owner_objects,
        symbols.len(),
        symbol_objects,
        symbol_bytes as f64 / 1_048_576.0,
        owner_links_from_symbols
    );
    let mut sorted: Vec<(&u64, &Symbol)> = symbols.iter().collect();
    sorted.sort_by_key(|(_, symbol)| std::cmp::Reverse(symbol.objects));
    for (label, low, high) in [
        ("over 100000", 100_001, usize::MAX),
        ("20001 to 100000", 20_001, 100_000),
        ("5001 to 20000", 5_001, 20_000),
        ("1001 to 5000", 1_001, 5_000),
        ("101 to 1000", 101, 1_000),
        ("0 to 100", 0, 100),
    ] {
        let count = sorted
            .iter()
            .filter(|(_, symbol)| (low..=high).contains(&symbol.objects))
            .count();
        println!("symbol records with objects {label}: {count}");
    }
    for top in [1usize, 5, 25] {
        let share: usize = sorted.iter().take(top).map(|(_, s)| s.objects).sum();
        println!(
            "largest {top} symbol records: {share} objects, {:.1}% of all",
            if symbol_objects > 0 {
                100.0 * share as f64 / symbol_objects as f64
            } else {
                0.0
            }
        );
    }
    let peak = batches
        .iter()
        .enumerate()
        .max_by_key(|(_, batch)| batch.objects);
    let over_budget = batches
        .iter()
        .filter(|batch| batch.objects > BATCH_OBJECT_BUDGET)
        .count();
    let over_five = batches
        .iter()
        .filter(|batch| batch.objects > 5 * BATCH_OBJECT_BUDGET)
        .count();
    println!(
        "batches {} | over the budget {} | over five times it {} | sum of batch objects {}",
        batches.len(),
        over_budget,
        over_five,
        batches.iter().map(|batch| batch.objects).sum::<usize>()
    );
    if let Some((index, batch)) = peak {
        println!(
            "peak batch #{index}: objects {} (budget {BATCH_OBJECT_BUDGET}), owners {}, symbols named {}, shared records {}, limit {}, largest record {} with {} objects",
            batch.objects,
            batch.owners,
            batch.named,
            batch.shared,
            batch.limit,
            batch.largest.0,
            batch.largest.1
        );
    }
    let mut by_size: Vec<(usize, &Batch)> = batches.iter().enumerate().collect();
    by_size.sort_by_key(|(_, batch)| std::cmp::Reverse(batch.objects));
    for (index, batch) in by_size.iter().take(8) {
        println!(
            "  batch #{index}: objects {} owners {} named {} shared {} limit {} largest {} ({} objects)",
            batch.objects,
            batch.owners,
            batch.named,
            batch.shared,
            batch.limit,
            batch.largest.0,
            batch.largest.1
        );
    }
    for (id, symbol) in sorted.iter().take(12) {
        println!(
            "  symbol {} class {} objects {} body_kib {:.0} named_by {} owners",
            id,
            symbol.class,
            symbol.objects,
            symbol.body_bytes as f64 / 1024.0,
            naming_count.get(*id).copied().unwrap_or(0)
        );
    }
    Ok(())
}
