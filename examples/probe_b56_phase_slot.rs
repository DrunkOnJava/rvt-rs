//! B56 (probe, #228): whether an element record's first reference slot is its
//! Phase Created. ElementId 3 is the ProjectPhase "New Construction" on Core
//! Interior and RE1 Mechanical, and the first slot is 3 on most of their
//! records (RE-155, RE-161). For each model the probe prints its phases (the
//! ProjectPhase data objects, RE-153, with their names), the first slot's
//! values with how many records carry each and how many of those elements
//! Revit's export beside the model holds (by `Tag`), and the first three
//! slots of RE1 Plumbing's 442378, the element Revit's export lacks (B38).
//!
//! Usage: probe_b56_phase_slot <model.rvt>

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_room_parameters::data_objects;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn first_string(bytes: &[u8]) -> Option<String> {
    (0..bytes.len().saturating_sub(4)).find_map(|q| {
        let n = u32::from_le_bytes(bytes[q..q + 4].try_into().ok()?) as usize;
        if !(2..=80).contains(&n) || q + 4 + 2 * n > bytes.len() {
            return None;
        }
        let units: Vec<u16> = bytes[q + 4..q + 4 + 2 * n]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        units
            .iter()
            .all(|u| (0x20..0x7f).contains(u))
            .then(|| String::from_utf16_lossy(&units))
    })
}

/// Every `Tag` of Revit's export: the last quoted string of each entity line
/// whose last quoted string is a number.
fn tags(step: &str) -> BTreeSet<u64> {
    step.lines()
        .filter_map(|line| line.rsplit('\'').nth(1)?.parse().ok())
        .collect()
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let slim = path
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(format!("../IFC Exports/{stem}_slim.ifc"));
    let reference = if slim.exists() {
        slim
    } else {
        path.with_extension("ifc")
    };
    let exported = std::fs::read_to_string(&reference).ok().map(|s| tags(&s));
    let mut rf = RevitFile::open(&path)?;
    let revit = rf.basic_file_info()?.version;
    let classes = rf.schema_classes()?;
    let phase_tag = classes
        .classes
        .iter()
        .find(|c| c.name == "ProjectPhase")
        .map(|c| u32::from(c.tag));
    let mut phases: BTreeMap<u32, Option<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for (p, object) in data_objects(buf) {
            if Some(object.class & 0xffff) == phase_tag {
                phases.insert(object.element_id, first_string(&buf[p..object.end]));
            }
        }
    }
    println!("phases: {phases:?}");
    let Some(marker) = per::file_bbox_marker(&mut rf, revit) else {
        println!("no bbox marker");
        return Ok(());
    };
    // first slot -> (records, of them in Revit's export)
    let mut first: BTreeMap<u64, (usize, usize)> = BTreeMap::new();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for span in per::partition_record_chain(buf, &marker) {
            if !seen.insert(span.element_id) {
                continue;
            }
            let Ok(own) = u32::try_from(span.element_id) else {
                continue;
            };
            let Some(frame) = per::decode_frame_as(&stream, buf, span.start, own, &marker) else {
                continue;
            };
            let Some(&slot) = frame.references.first() else {
                continue;
            };
            let entry = first.entry(slot).or_default();
            entry.0 += 1;
            if exported
                .as_ref()
                .is_some_and(|tags| tags.contains(&span.element_id))
            {
                entry.1 += 1;
            }
            if own == 442378 {
                println!(
                    "442378: first slots {:?}",
                    frame.references.iter().take(3).collect::<Vec<_>>()
                );
            }
        }
    }
    let mut ranked: Vec<(u64, (usize, usize))> = first.into_iter().collect();
    ranked.sort_by(|a, b| b.1.0.cmp(&a.1.0));
    for (slot, (records, in_export)) in ranked.into_iter().take(10) {
        let phase = u32::try_from(slot)
            .ok()
            .and_then(|s| phases.get(&s).cloned());
        println!(
            "first slot {slot}: {records} records, {in_export} in Revit's export; phase {phase:?}"
        );
    }
    Ok(())
}
