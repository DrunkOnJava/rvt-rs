//! B46 (probe): the native element fields `m_createdPhaseId` and
//! `m_demolishedPhaseId` on Revit 2025 and 2026, with the native record
//! framing's release gate opened on this branch.
//!
//! For each model: whether `native_document::extract` completes (its framing,
//! count and body-size checks all hold) or the error it stops on; the records
//! by status; each element's created and demolished phase where its saved
//! metadata gives them; the elements with a demolished phase, and whether
//! Revit's export (`<model>.ifc` beside it) holds their `Tag`; and RE1
//! Plumbing's water closet tank 442378 (B38).
//!
//! Usage: probe_b46_native_phases <model.rvt>

use rvt::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

const TANK: u64 = 442378;

fn exported_tags(path: &std::path::Path) -> Option<BTreeSet<u64>> {
    let step = std::fs::read_to_string(path.with_extension("ifc")).ok()?;
    Some(
        step.lines()
            .filter(|line| line.starts_with('#'))
            .filter_map(|line| line.rsplit('\'').nth(1)?.parse().ok())
            .collect(),
    )
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let exported = exported_tags(&path);
    let options = native_document::Options {
        channels: BTreeSet::from([101]),
        ..native_document::Options::default()
    };
    let mut statuses: BTreeMap<String, usize> = BTreeMap::new();
    let mut phases: BTreeMap<(Option<i64>, Option<i64>), usize> = BTreeMap::new();
    let mut demolished: Vec<(u64, Option<String>, i64, Option<bool>)> = Vec::new();
    let mut tank = None;
    let mut with_metadata = 0usize;
    let result = native_document::extract(&mut rf, &options, |record| {
        *statuses.entry(record.status.clone()).or_default() += 1;
        let id = record.identity.element_id;
        if let Some(metadata) = &record.saved_metadata {
            with_metadata += 1;
            let created = metadata.root_references.get("m_createdPhaseId").copied();
            let gone = metadata.root_references.get("m_demolishedPhaseId").copied();
            *phases.entry((created, gone)).or_default() += 1;
            if let Some(gone) = gone.filter(|gone| *gone >= 0) {
                demolished.push((
                    id,
                    record.class_name.clone(),
                    gone,
                    exported.as_ref().map(|tags| tags.contains(&id)),
                ));
            }
            if id == TANK {
                tank = Some(format!(
                    "class {:?} status {} created {created:?} demolished {gone:?} roots {:?}",
                    record.class_name, record.status, metadata.root_references
                ));
            }
        } else if id == TANK {
            tank = Some(format!(
                "class {:?} status {} no metadata: {:?}",
                record.class_name, record.status, record.metadata_diagnostic
            ));
        }
        Ok(())
    });
    match result {
        Ok(summary) => println!(
            "Revit {version}: extract ok, {} indexed, {} graveyard rows, {} historical skipped, {with_metadata} with saved metadata",
            summary.indexed_elements, summary.graveyard_records, summary.skipped_historical_records
        ),
        Err(error) => {
            println!("Revit {version}: extract failed: {error:#}");
            return Ok(());
        }
    }
    println!("statuses {statuses:?}");
    let mut by_count: Vec<_> = phases.into_iter().collect();
    by_count.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    for ((created, gone), count) in by_count.iter().take(12) {
        println!("  created {created:?} demolished {gone:?}: {count}");
    }
    println!("{} elements with a demolished phase", demolished.len());
    for (id, class, gone, in_export) in demolished.iter().take(20) {
        println!("  {id} {class:?} demolished in {gone}, in Revit's export {in_export:?}");
    }
    if let Some(tank) = tank {
        println!("442378: {tank}");
    }
    Ok(())
}
