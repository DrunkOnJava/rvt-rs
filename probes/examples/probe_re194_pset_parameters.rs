//! RE-194: the values of a project's parameters named `<property set>.<property>`.
//!
//! FACT: on the BIM4LCA office (Revit 2024), the value of each parameter
//! named for a property set's property sits in the element's own data object
//! beside the parameter's ElementId: a Yes/No or integer as the `u32` after
//! it, a text as `u32 n · UTF-16 × n` after it, a number as the `f64`
//! immediately before it (a length in feet, a thermal transmittance in
//! W/(m²·K)). Wherever Revit's export writes such a property on an element
//! carrying the parameter, its value is that one, and it is on the element
//! itself, not on its type. Verify with the Measure workflow, `-f nordic=true
//! -f probe=probe_re194_pset_parameters -f probe_scorer=pset_parameters_vs_ifc`.
//!
//! Revit's IFC exporter reads a parameter named for a property set's property
//! (`Pset_WallCommon.FireRating`) on an element before anything else, as RE-193
//! measured for `IsExternal` (revit-ifc's pset definitions, CLEANROOM.md
//! source 5). The BIM4LCA office defines such parameters for fifteen property
//! sets. The probe finds every project and shared parameter whose name starts
//! with `Pset_` (RE-156's definitions), and prints one JSON line per element
//! of the partition decode with each such parameter's entry in the element's
//! own data object (RE-153), read every way a value has been seen stored:
//!
//! - `int`: the `u32` after the key, as a Yes/No or integer entry (RE-158);
//! - `text`: `u32 n · UTF-16 × n` after the key (RE-156);
//! - `after`: the `f64` after the key;
//! - `before`: the `f64` 16 bytes before the key where `ff` × 8 sits between,
//!   as a type's lengths are stored (RE-77);
//! - `just_before`: the `f64` immediately before the key.
//!
//! Where an element's copies in several partitions hold an entry, the latest
//! partition's is printed (#800), and none where that partition's entries
//! disagree, as the export takes them. A first line names the parameters, a
//! second counts, per parameter, the elements left out that way.
//! `tools/re/pset_parameters_vs_ifc.py` pairs the lines with Revit's export
//! and counts, per parameter, which reading equals the value Revit writes.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re194_pset_parameters -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_room_parameters::{DATA_OBJECT_HEADER, find_parameter_definitions};
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// The number of partition stream `stream` (`Partitions/398` is 398).
fn partition_number(stream: &str) -> u32 {
    stream
        .rsplit('/')
        .next()
        .and_then(|number| number.parse().ok())
        .unwrap_or(0)
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    buf.get(at..at.checked_add(4)?)
        .map(|b| u32::from_le_bytes(b.try_into().expect("4 bytes")))
}

fn f64_at(buf: &[u8], at: usize) -> Option<f64> {
    buf.get(at..at.checked_add(8)?)
        .map(|b| f64::from_le_bytes(b.try_into().expect("8 bytes")))
        .filter(|v| v.is_finite())
}

/// `u32 n · UTF-16 × n` at `at`, 1 to 256 units with no control character.
fn text_at(buf: &[u8], at: usize) -> Option<String> {
    let n = usize::try_from(u32_at(buf, at)?).ok()?;
    if !(1..=256).contains(&n) {
        return None;
    }
    let units: Vec<u16> = buf
        .get(at + 4..at + 4 + 2 * n)?
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    (!text.chars().any(char::is_control)).then_some(text)
}

/// Every reading of the entry keyed at `hit` in `payload`.
fn readings(payload: &[u8], hit: usize) -> Value {
    let before = hit
        .checked_sub(16)
        .filter(|&at| payload.get(at + 8..hit) == Some(&[0xff; 8][..]))
        .and_then(|at| f64_at(payload, at));
    json!({
        "int": u32_at(payload, hit + 8),
        "text": text_at(payload, hit + 8),
        "after": f64_at(payload, hit + 8),
        "before": before,
        "just_before": hit.checked_sub(8).and_then(|at| f64_at(payload, at)),
    })
}

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let elements =
        recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?.into_elements();
    let global_ids = rvt::research::revit_global_ids::revit_global_ids(&mut rf).unwrap_or_default();
    let ids: BTreeSet<u32> = elements.iter().filter_map(|element| element.id).collect();
    let names: BTreeMap<i64, String> = find_parameter_definitions(&mut rf, version)
        .into_iter()
        .filter(|(name, _)| name.starts_with("Pset_"))
        .map(|(name, id)| (i64::from(id), name))
        .collect();
    println!(
        "{}",
        json!({ "parameters": names.values().collect::<Vec<_>>() })
    );

    // (element, parameter name) -> (partition, readings), the latest
    // partition's, or None where that partition's entries disagree, as
    // `scan_parameter_entries` takes them.
    let mut found: BTreeMap<(u32, String), (u32, Option<Value>)> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let (Ok(inflated), Ok(objects)) = (
            rf.inflated_partition(&stream),
            rf.partition_data_objects(&stream),
        ) else {
            continue;
        };
        let partition = partition_number(&stream);
        let buf = inflated.bytes();
        for (start, object) in objects.iter() {
            if !ids.contains(&object.element_id) {
                continue;
            }
            let Some(payload) = buf.get(start + DATA_OBJECT_HEADER..object.end) else {
                continue;
            };
            for (parameter, name) in &names {
                let tag = parameter.to_le_bytes();
                for hit in memchr::memmem::find_iter(payload, &tag) {
                    let key = (object.element_id, name.clone());
                    let value = readings(payload, hit);
                    match found.get_mut(&key) {
                        Some((held, _)) if *held > partition => {}
                        Some((held, reading)) if *held == partition => {
                            if reading.as_ref() != Some(&value) {
                                *reading = None;
                            }
                        }
                        _ => {
                            found.insert(key, (partition, Some(value)));
                        }
                    }
                }
            }
        }
    }

    let mut by_element: BTreeMap<u32, BTreeMap<String, Value>> = BTreeMap::new();
    let mut conflicting: BTreeMap<String, usize> = BTreeMap::new();
    for ((element, name), (_, value)) in found {
        match value {
            Some(value) => {
                by_element.entry(element).or_default().insert(name, value);
            }
            None => *conflicting.entry(name).or_default() += 1,
        }
    }
    println!("{}", json!({ "conflicting": conflicting }));
    for element in &elements {
        let Some(id) = element.id else {
            continue;
        };
        let Some(values) = by_element.get(&id) else {
            continue;
        };
        println!(
            "{}",
            json!({
                "id": id,
                "global_id": global_ids.get(&id),
                "class": element.class,
                "values": values,
            })
        );
    }
    Ok(())
}
