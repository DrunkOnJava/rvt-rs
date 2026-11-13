//! #1899: what a covering's native record is, and what the saved-parameter
//! reader finds in it, on the saves of one project.
//!
//! FACT under test: on Autodesk's `rac_advanced` the 2023 save's export gives
//! two of its 65 `IfcCovering`, the furred ceilings 247617 and 247677, no
//! `Pset_CoveringCommon` and no `Pset_EnvironmentalImpactIndicators`, where
//! every other save from 2015 to 2022 gives them both (six values fewer). This
//! probe prints, for every covering the export writes, its native record (the
//! class the record opens with, the decode status and diagnostic, the stream it
//! is in and its size) and the saved parameters the reader finds for it, so
//! that the 2022 and 2023 saves can be compared line by line: a record that
//! does not decode, or decodes with no parameters, is where the values are lost.
//!
//! Output:
//!   file <path> release <year>
//!   covering <ElementId> <predefined type> <name> record <class> status
//!     <status> diagnostic <text|-> stream <stream> bytes <n> graph <objects|->
//!     saved <count> [<parameter name>,...]
//!   covering <ElementId> <predefined type> <name> record - saved <count> [...]
//!     for a covering the native read returned no record for.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1899_covering_parameters -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract};
use rvt::research::native_element_parameters;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    println!("file {path} release {release}");
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let coverings: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                name,
                type_guid: Some(tag),
                predefined_type,
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCCOVERING") => Some((
                tag.parse().ok()?,
                format!(
                    "{} {}",
                    predefined_type.as_deref().unwrap_or("-"),
                    name.replace(' ', "_")
                ),
            )),
            _ => None,
        })
        .collect();
    let ids: BTreeSet<u64> = coverings.keys().copied().collect();

    let mut records: BTreeMap<u64, String> = BTreeMap::new();
    extract(
        &mut rf,
        &Options {
            selected_ids: ids.clone(),
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            records.insert(
                record.identity.element_id,
                format!(
                    "{} status {} diagnostic {} stream {} bytes {} graph {}",
                    record.class_name.as_deref().unwrap_or("-"),
                    record.status.replace(' ', "_"),
                    record
                        .diagnostic
                        .as_deref()
                        .map_or("-".to_string(), |text| text.replace(' ', "_")),
                    record.source.stream,
                    record.source.body_bytes,
                    record
                        .graph
                        .as_ref()
                        .map_or("-".to_string(), |graph| graph.objects.len().to_string()),
                ),
            );
            Ok(())
        },
    )?;

    let read = native_element_parameters::read(&mut rf, &ids)?;
    for (id, label) in &coverings {
        let entries = read.json_entries(u32::try_from(*id).ok());
        let names: Vec<String> = entries
            .iter()
            .filter_map(|entry| entry.get("name").and_then(|name| name.as_str()))
            .map(|name| name.replace(' ', "_"))
            .collect();
        println!(
            "covering {id} {label} record {} saved {} [{}]",
            records.get(id).map_or("-", String::as_str),
            entries.len(),
            names.join(",")
        );
    }
    Ok(())
}
