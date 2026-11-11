//! #1694: how many graph objects each record the saved-mesh pass decodes holds.
//!
//! FACT under test: the saved-mesh pass's peak on a small Revit 2023 file is
//! decoded graph objects, not meshes. dhat (Measure 42960663793,
//! `probe_1075_heap_sites`) puts 497.9 of the pass's 630.7 MiB on
//! `rac_basic` 2023 in the nodes of the maps of decoded objects' fields
//! (825,000 nodes of 633 bytes, which is a `serde_json::Map<String, Value>` leaf
//! node), and 78% to 84% of it on five other files. A budget of 20,000 graph
//! objects bounds a buffer of owners and a batch of symbols, so 825,000 nodes
//! are either a few records far over the budget (one record is decoded whole)
//! or many records the pass holds at once. This probe says which: it reads the
//! records the pass reads (channel 103, the ids of every building element) one
//! at a time, keeps nothing of them, and prints, per record, its graph objects,
//! its body bytes and the symbols it names.
//!
//! It prints the totals, a histogram of the records by objects, the share of
//! the largest 1, 5 and 25 records, and the 12 largest records by Tag and class.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1694_owner_sizes -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportQualityMode, RvtDocExporter};
use rvt::research::native_document;

struct Row {
    id: u64,
    class: String,
    objects: usize,
    body_bytes: usize,
    symbols: usize,
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1694_owner_sizes MODEL.rvt"))?;
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
    let options = native_document::Options {
        selected_ids: ids.clone(),
        channels: BTreeSet::from([103]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        record_extras: true,
        ..Default::default()
    };
    let mut rows: Vec<Row> = Vec::new();
    native_document::extract_graphs(&mut rf, &options, |record| {
        let (objects, symbols) = record.graph.as_ref().map_or((0, 0), |graph| {
            (
                graph.objects.len(),
                graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "InstanceInfo")
                    .count(),
            )
        });
        rows.push(Row {
            id: record.identity.element_id,
            class: record.class_name.clone().unwrap_or_default(),
            objects,
            body_bytes: record.source.body_bytes,
            symbols,
        });
        Ok(())
    })?;
    rows.sort_by_key(|row| std::cmp::Reverse(row.objects));
    let total: usize = rows.iter().map(|row| row.objects).sum();
    let bytes: usize = rows.iter().map(|row| row.body_bytes).sum();
    let named = rows.iter().filter(|row| row.symbols > 0).count();
    println!(
        "ids {} records {} named {} streamed {} graph_objects {} body_mib {:.1}",
        ids.len(),
        rows.len(),
        named,
        rows.len() - named,
        total,
        bytes as f64 / 1_048_576.0
    );
    for (label, low, high) in [
        ("over 100000", 100_001, usize::MAX),
        ("20001 to 100000", 20_001, 100_000),
        ("1001 to 20000", 1_001, 20_000),
        ("101 to 1000", 101, 1_000),
        ("1 to 100", 1, 100),
        ("none", 0, 0),
    ] {
        let count = rows
            .iter()
            .filter(|row| (low..=high).contains(&row.objects))
            .count();
        println!("records with objects {label}: {count}");
    }
    for top in [1usize, 5, 25] {
        let share: usize = rows.iter().take(top).map(|row| row.objects).sum();
        println!(
            "largest {top} records: {share} objects, {:.1}% of all",
            if total > 0 {
                100.0 * share as f64 / total as f64
            } else {
                0.0
            }
        );
    }
    for row in rows.iter().take(12) {
        println!(
            "  tag {} class {} objects {} body_kib {:.0} symbols {}",
            row.id,
            row.class,
            row.objects,
            row.body_bytes as f64 / 1024.0,
            row.symbols
        );
    }
    Ok(())
}
