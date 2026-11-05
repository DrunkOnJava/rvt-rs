//! #1075: the saved graphics read a batch of symbols at a time make the same
//! elements as the read that decodes every symbol first.
//!
//! FACT under test: `native_saved_scene::extract_in_batches_with_control` gives,
//! for every owner, the `SavedElement` that `extract_at_detail_with_control`
//! gives: the same meshes, diagnostics, status, identity, source and
//! referenced sources. A batch decodes the closure of its own owners only; an
//! owner that names another owner reaches the symbols that owner names, so the
//! batch must read those too.
//!
//! Four selections of owners, each read both ways on the model and compared
//! element by element as JSON:
//!
//! - `doors_and_windows`: the doors and windows `rvt-ifc` draws from their saved
//!   graphics.
//! - `owners_and_their_symbols`: those, and the symbols their meshes draw from
//!   (the sources their primitives name) as owners too. Every door and window
//!   then names an owner, which is the shape the batches must follow.
//! - `owners_and_every_other_symbol`: the doors and windows, and every second
//!   one of those symbols as an owner, so that a symbol a batch reads can name
//!   an owner meshed, and freed, with an earlier batch (#1429); the symbols
//!   not selected are shared, not owners.
//! - `every_boxed_element`: every element with a Tag and a location, when there
//!   are no more than 8,000 (a file with more is skipped).
//!
//! `owners_naming_owners` counts the elements whose mesh draws from another
//! selected owner. The probe exits with an error when any selection differs.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1075_saved_batches -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rvt::RevitFile;
use rvt::control::WalkerControl;
use rvt::ifc::entities::IfcEntity;
use rvt::ifc::{ExportQualityMode, IfcModel, RvtDocExporter};
use rvt::research::native_document::Options;
use rvt::research::native_saved_scene;

const MOST_OWNERS: usize = 8_000;

/// A progress event as the CLIs print it: stage, done, total.
type Progress = (&'static str, u64, Option<u64>);

/// A control that records every progress event it is given, and the record.
fn recording() -> (WalkerControl, Arc<Mutex<Vec<Progress>>>) {
    let events = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&events);
    let control = WalkerControl::new().with_progress(move |event| {
        kept.lock().unwrap_or_else(|e| e.into_inner()).push((
            event.stage.as_str(),
            event.done,
            event.total,
        ));
    });
    (control, events)
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_1075_saved_batches MODEL.rvt"))?;
    let mut rf = RevitFile::open_with_limits(&path, rvt::cli::open_limits(None, None, false))?;
    let release = rf.basic_file_info().map(|b| b.version).unwrap_or(0);
    println!("release {release}");
    if !(2023..=2027).contains(&release) {
        println!("saved graphics are not read for this release");
        return Ok(());
    }
    let model = RvtDocExporter.export_with_mode_and_limits(
        &mut rf,
        ExportQualityMode::Scaffold,
        rvt::walker::WalkerLimits::default(),
    )?;

    let mut identical = true;
    let doors_and_windows = tagged(&model, true);
    let (same, sources) = compare(&mut rf, "doors_and_windows", &doors_and_windows)?;
    identical &= same;

    let mut with_symbols = doors_and_windows.clone();
    with_symbols.extend(sources.iter().copied());
    identical &= compare(&mut rf, "owners_and_their_symbols", &with_symbols)?.0;

    let mut with_half = doors_and_windows.clone();
    with_half.extend(sources.iter().copied().step_by(2));
    identical &= compare(&mut rf, "owners_and_every_other_symbol", &with_half)?.0;

    let every = tagged(&model, false);
    if every.len() > MOST_OWNERS {
        println!(
            "saved_batches every_boxed_element owners {} skipped: over {MOST_OWNERS}",
            every.len()
        );
    } else {
        identical &= compare(&mut rf, "every_boxed_element", &every)?.0;
    }

    println!("saved_batches identical {identical}");
    anyhow::ensure!(identical, "the batched read differs from the one-shot read");
    Ok(())
}

/// The Tags of the model's elements that have a location: the doors and
/// windows alone, or every one.
fn tagged(model: &IfcModel, doors_and_windows_only: bool) -> BTreeSet<u64> {
    model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                location_feet: Some(_),
                ..
            } if !doors_and_windows_only
                || matches!(ifc_type.as_str(), "IFCDOOR" | "IFCWINDOW") =>
            {
                tag.parse().ok()
            }
            _ => None,
        })
        .collect()
}

/// Read `ids` both ways and compare the elements. Returns whether they are the
/// same, and the sources the one-shot meshes draw from besides their own.
fn compare(
    rf: &mut RevitFile,
    scope: &str,
    ids: &BTreeSet<u64>,
) -> anyhow::Result<(bool, BTreeSet<u64>)> {
    let options = Options {
        selected_ids: ids.clone(),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        ..Default::default()
    };

    let (control, one_shot_progress) = recording();
    let started = Instant::now();
    let scene = native_saved_scene::extract_at_detail_with_control(rf, &options, 3, &control)?;
    let one_shot_ms = started.elapsed().as_millis();
    let mut sources = BTreeSet::new();
    let mut naming_owners = 0usize;
    let mut one_shot: BTreeMap<u64, serde_json::Value> = BTreeMap::new();
    for element in &scene.elements {
        let others: Vec<u64> = element
            .referenced_graph_sources
            .keys()
            .copied()
            .filter(|source| *source != element.id)
            .collect();
        naming_owners += usize::from(others.iter().any(|source| ids.contains(source)));
        sources.extend(others);
        one_shot.insert(element.id, serde_json::to_value(element)?);
    }
    drop(scene);

    let (control, batched_progress) = recording();
    let started = Instant::now();
    let mut batched: BTreeMap<u64, serde_json::Value> = BTreeMap::new();
    native_saved_scene::extract_in_batches_with_control(rf, &options, 3, &control, |element| {
        batched.insert(element.id, serde_json::to_value(&element)?);
        Ok(())
    })?;
    let batched_ms = started.elapsed().as_millis();

    let mut different = Vec::new();
    for (id, value) in &one_shot {
        if batched.get(id) != Some(value) {
            different.push(*id);
        }
    }
    let one_shot_progress = one_shot_progress.lock().unwrap_or_else(|e| e.into_inner());
    let batched_progress = batched_progress.lock().unwrap_or_else(|e| e.into_inner());
    let same_progress = *one_shot_progress == *batched_progress;
    let identical = one_shot.len() == batched.len() && different.is_empty() && same_progress;
    println!(
        "saved_batches {scope} owners {} elements one_shot {} batched {} owners_naming_owners {naming_owners} different {} progress_events one_shot {} batched {} same_progress {same_progress} one_shot_ms {one_shot_ms} batched_ms {batched_ms} (batched_ms includes each element's JSON) identical {identical}",
        ids.len(),
        one_shot.len(),
        batched.len(),
        different.len(),
        one_shot_progress.len(),
        batched_progress.len(),
    );
    if !identical {
        println!(
            "  first different owners: {:?}",
            different.iter().take(5).collect::<Vec<_>>()
        );
    }
    Ok((identical, sources))
}
