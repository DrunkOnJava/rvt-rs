//! RE-41: where an ElemTable record's two ids differ, the second is the
//! element's ElementId.
//!
//! FACT: a 40-byte `Global/ElemTable` record carries two `u32` ids, at
//! `+16` (`id_primary`) and `+36` (`id_secondary`). They are equal on nearly
//! every record. On Autodesk's Snowdon Towers 2024 samples they differ on
//! 84 records (Architectural) and 27 (Structural). On every one of those,
//! `id_secondary` is the ElementId of a record in the partition record chain
//! (RE-35) and `id_primary` is not, and in Revit's own IFC4 export of the
//! architectural model 39 of the `id_secondary` values are element `Tag`s
//! while no `id_primary` is.
//!
//! The probe prints how many chain records each id set covers and, for each
//! record whose ids differ, both ids and which one is a chain record. Grep
//! the reference export for the printed ids to check the `Tag`s.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re41_elem_table_secondary_ids -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{bbox_marker, partition_record_chain};
use std::collections::BTreeSet;
use std::path::PathBuf;

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let records = rvt::elem_table::parse_records(&mut rf)?;
    let primary: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let secondary: BTreeSet<u32> = records.iter().map(|r| r.id_secondary).collect();
    let declared = rvt::elem_table::declared_ids(&records);
    let Some(marker) = bbox_marker(version) else {
        println!("release {version}: no known record marker");
        return Ok(());
    };
    let mut chain: BTreeSet<u32> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        chain.extend(
            partition_record_chain(inflated.bytes(), &marker)
                .into_iter()
                .filter_map(|span| u32::try_from(span.element_id).ok()),
        );
    }
    let missing = |set: &BTreeSet<u32>| chain.difference(set).count();
    println!(
        "release {version}: {} records, {} chain records; chain ids not in id_primary {}, not in id_secondary {}, not declared {}",
        records.len(),
        chain.len(),
        missing(&primary),
        missing(&secondary),
        missing(&declared),
    );
    let differing: Vec<_> = records
        .iter()
        .filter(|r| r.id_primary != r.id_secondary)
        .collect();
    println!("{} records with differing ids", differing.len());
    for record in differing {
        println!(
            "  +{:#x}  id_primary {:>8} {:<5}  id_secondary {:>8} {}",
            record.offset,
            record.id_primary,
            if chain.contains(&record.id_primary) {
                "chain"
            } else {
                "-"
            },
            record.id_secondary,
            if chain.contains(&record.id_secondary) {
                "chain"
            } else {
                "-"
            },
        );
    }
    Ok(())
}
