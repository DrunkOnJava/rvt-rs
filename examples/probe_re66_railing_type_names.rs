//! RE-66: railing types without element data keep their name in their type
//! object.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, 101 of the
//! 131 railings Revit's export names have a type whose element data (RE-44)
//! carries its name. The other 30 name one of 6 railing types that have no
//! element data. Every railing type keeps its name in its type object
//! (`01 00 00 00 · u64 id`, tag `db 0f` 0x47 bytes past the id) as
//! `u32 n · n UTF-16 units` ending where `ff ff ff ff 0b 02` first starts
//! past +0x100. That is at +0x1ae on most types, and at +0x1cb on the two
//! that also carry a Uniformat code. On the 14 types that have both, the two
//! names agree. For all 30 railings, Revit's export names the railing
//! `Railing:<that name>:<ElementId>`.
//!
//! The probe prints each railing record: its id, its one railing type, and
//! that type's name from its element data and from its type object.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re66_railing_type_names -- \
//!     MODEL.rvt > rows.tsv

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_type_records as ptr;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let (Some(marker), Some(header)) = (
        per::bbox_marker(version),
        rvt::partition_names::element_data_header(version),
    ) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let types = ptr::type_definition_ids(&ptr::scan_type_records(
        &mut rf,
        version,
        per::OST_STAIRS_RAILING,
        &declared,
    )?);
    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut railings: BTreeMap<u32, Option<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        for record in per::find_category_records_assigned(
            &stream,
            inflated.bytes(),
            per::OST_STAIRS_RAILING,
            &declared,
            &marker,
            ids,
        ) {
            railings
                .entry(record.element_id)
                .or_insert_with(|| ptr::unique_type_reference(&record.references, &types));
        }
    }
    let wanted: BTreeSet<u32> = railings.values().flatten().copied().collect();
    let mut data_names = BTreeMap::new();
    let mut object_names = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        data_names.extend(rvt::partition_names::find_element_data_names(
            buf, &header, &wanted,
        ));
        object_names.extend(rvt::partition_names::find_railing_type_names(buf, &wanted));
    }
    eprintln!(
        "Revit {version}: {} railing records, {} railing types named by them, {} with element data, {} with a type-object name",
        railings.len(),
        wanted.len(),
        data_names.len(),
        object_names.len()
    );
    for (id, type_id) in &railings {
        let label = |names: &BTreeMap<u32, String>| {
            type_id
                .and_then(|t| names.get(&t))
                .cloned()
                .unwrap_or_else(|| "-".into())
        };
        println!(
            "{id}\t{}\t{}\t{}",
            type_id.map_or("-".into(), |t| t.to_string()),
            label(&data_names),
            label(&object_names)
        );
    }
    Ok(())
}
