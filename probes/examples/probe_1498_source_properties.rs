//! #1498: which `*Source` properties the export writes, and which readers'
//! value sources the elements that carry each value have.
//!
//! FACT under test: every value of a `*Source` property that rvt-rs writes on a
//! building element (`BodySource`, `ProfileSource`, `BodyHeightSource`,
//! `BodyBaseSource`, `BodyEndSource`, `PlacementAxesSource`, ...) is either
//! listed by a reader of the provenance table or is a declared gap. This prints,
//! for each model, each such property value with how many elements carry it and
//! how many of those have a value source from each reader, so that the values
//! with no reader can be read off. Counts only: no ElementId, name or
//! coordinate.
//!
//! Output, one line per value:
//!   source <property>=<value> elements <n> readers <reader>:<n> ...
//!   source total: elements <n>, values <n>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1498_source_properties -- MODEL.rvt

use std::collections::BTreeMap;

use anyhow::Result;
use rvt::RevitFile;
use rvt::ifc::entities::{IfcEntity, PropertyValue};
use rvt::ifc::{ExportOptions, run_export};

/// Elements and, by reader, how many of them have a value source from it.
type Seen = BTreeMap<(String, String), (usize, BTreeMap<String, usize>)>;

fn inventory(path: &str) -> Result<()> {
    let mut rf = RevitFile::open(path)?;
    let run = run_export(
        &mut rf,
        &ExportOptions::default().with_element_provenance(true),
    )?;
    let table = run.diagnostics.decoded.element_provenance.as_ref();
    let mut seen = Seen::new();
    let mut elements = 0usize;
    for entity in &run.model.entities {
        let IfcEntity::BuildingElement {
            type_guid,
            property_set,
            ..
        } = entity
        else {
            continue;
        };
        elements += 1;
        let id = type_guid.as_deref().and_then(|tag| tag.parse::<u32>().ok());
        let readers: Vec<&str> = id
            .and_then(|id| table?.elements.get(&id))
            .map(|entry| {
                entry
                    .value_sources
                    .iter()
                    .map(|source| source.reader.as_str())
                    .collect()
            })
            .unwrap_or_default();
        for property in property_set.iter().flat_map(|set| &set.properties) {
            let PropertyValue::Text(value) = &property.value else {
                continue;
            };
            if !property.name.ends_with("Source") {
                continue;
            }
            let held = seen
                .entry((property.name.clone(), value.clone()))
                .or_default();
            held.0 += 1;
            for reader in &readers {
                *held.1.entry((*reader).to_string()).or_default() += 1;
            }
        }
    }
    for ((name, value), (count, by_reader)) in &seen {
        let listed: Vec<String> = by_reader
            .iter()
            .map(|(reader, n)| format!("{reader}:{n}"))
            .collect();
        println!(
            "source {name}={value} elements {count} readers {}",
            if listed.is_empty() {
                "none".to_string()
            } else {
                listed.join(" ")
            }
        );
    }
    println!("source total: elements {elements}, values {}", seen.len());
    Ok(())
}

fn main() {
    let paths: Vec<String> = std::env::args()
        .skip(1)
        .filter(|arg| !arg.starts_with("--"))
        .collect();
    for path in &paths {
        if let Err(error) = inventory(path) {
            println!("error {path}: {error:#}");
        }
    }
}
