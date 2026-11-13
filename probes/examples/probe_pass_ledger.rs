//! Every native pass a default export makes, where it is asked for and what it costs.
//!
//! FACT under test: `native_document::record_passes` keeps one entry for each
//! `extract` and `extract_graphs` walk, and each use of the handle's
//! parameter-definitions pass (made, held, replaced by another key's,
//! dropped), with the line that asked for it. The export is the same with the
//! ledger on: it only reads the clock and pushes an entry while a probe has
//! turned it on, so every export elsewhere is unchanged (the Measure run that
//! lands this compares every hash).
//!
//! What it is for: a fold that should save a pass shows as that pass gone, and
//! a pass made twice, or a memo dropped before its reader (#1816's first run),
//! shows here without a counter written for it. #1694, #1816 and #1824 each
//! wrote one.
//!
//! Output, one line for each pass in order, then the totals:
//!   pass <n> <kind> <file>:<line> src=<value source|-> ch=<channels> ids=<n>
//!        classes=<n>[<first three>] uncached=<bool> partitions=<n> inflated=<bytes>
//!        emitted=<n> ms=<ms>
//!   total walks=<n> ms=<ms> inflated=<bytes>
//!   definitions made=<n> held=<n> replaced=<n> dropped=<n>
//!   repeated <times>x ch=<channels> ids=<n> classes=<...> by <file:line>, ...
//!   remade <file>:<line> after dropped at <file>:<line>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_pass_ledger -- MODEL.rvt

use std::collections::BTreeMap;

use rvt::RevitFile;
use rvt::research::native_document::{self, PassEntry};

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

fn at(entry: &PassEntry) -> String {
    format!("{}:{}", entry.caller.file(), entry.caller.line())
}

fn classes(entry: &PassEntry) -> String {
    let shown: Vec<&str> = entry
        .selected_classes
        .iter()
        .take(3)
        .map(String::as_str)
        .collect();
    format!("{}[{}]", entry.selected_classes.len(), shown.join(","))
}

fn channels(entry: &PassEntry) -> String {
    let list: Vec<String> = entry.channels.iter().map(u64::to_string).collect();
    list.join(",")
}

fn main() -> anyhow::Result<()> {
    // Measure passes the samples `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_pass_ledger MODEL.rvt");

    native_document::record_passes();
    let exported = rvt::ifc::run_export_with_limits(
        &mut open(&path)?,
        &rvt::ifc::ExportOptions::default(),
        rvt::walker::WalkerLimits::default(),
    );
    let passes = native_document::take_passes();
    if let Err(error) = exported {
        println!("export failed: {error:#}");
    }

    let mut walk_ms = 0.0;
    let mut inflated = 0u64;
    let mut walks = 0usize;
    let mut kinds: BTreeMap<&str, usize> = BTreeMap::new();
    let mut selections: BTreeMap<u64, Vec<&PassEntry>> = BTreeMap::new();
    let mut dropped: Option<&PassEntry> = None;
    let mut remade = Vec::new();
    for (n, entry) in passes.iter().enumerate() {
        let ms = entry.micros as f64 / 1000.0;
        println!(
            "pass {n} {} {} src={} ch={} ids={} classes={} uncached={} partitions={} inflated={} emitted={} ms={ms:.1}",
            entry.kind,
            at(entry),
            entry.value_source.unwrap_or("-"),
            channels(entry),
            entry.selected_ids,
            classes(entry),
            entry.uncached,
            entry.partitions,
            entry.inflated_bytes,
            entry.emitted,
        );
        *kinds.entry(entry.kind).or_default() += 1;
        match entry.kind {
            "extract" | "extract_graphs" | "definitions made" => {
                walks += 1;
                walk_ms += ms;
                inflated += entry.inflated_bytes;
                if entry.kind != "definitions made" {
                    selections.entry(entry.selection).or_default().push(entry);
                }
            }
            _ => {}
        }
        match entry.kind {
            "definitions dropped" => dropped = Some(entry),
            "definitions made" => {
                if let Some(drop) = dropped.take() {
                    remade.push((at(entry), at(drop)));
                }
            }
            _ => {}
        }
    }
    println!("total walks={walks} ms={walk_ms:.1} inflated={inflated}");
    println!(
        "definitions made={} held={} replaced={} dropped={}",
        kinds.get("definitions made").copied().unwrap_or(0),
        kinds.get("definitions held").copied().unwrap_or(0),
        kinds.get("definitions replaced").copied().unwrap_or(0),
        kinds.get("definitions dropped").copied().unwrap_or(0),
    );
    for entries in selections.values().filter(|entries| entries.len() > 1) {
        let callers: Vec<String> = entries.iter().copied().map(at).collect();
        println!(
            "repeated {}x ch={} ids={} classes={} by {}",
            entries.len(),
            channels(entries[0]),
            entries[0].selected_ids,
            classes(entries[0]),
            callers.join(", ")
        );
    }
    for (made, drop) in remade {
        println!("remade {made} after dropped at {drop}");
    }
    Ok(())
}
