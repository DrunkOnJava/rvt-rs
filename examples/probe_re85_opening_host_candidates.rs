//! RE-85: a door's or window's host is the last wall its reference list
//! names before its own ElementId (#439).
//!
//! FACT: the first counted reference list of a door or window element record
//! (`+0x88`) names its host wall before the record's own ElementId, but not
//! always in the slot immediately before it (RE-23): families such as
//! Door-Opening and Schematic Opening Cut list their type there. Taking the
//! last reference before the record's own id that is an exported wall, and
//! skipping curtain walls, reproduces the host Revit's IFC export fills.
//!
//! Prints each 2024/2025 door and window record with the slot before its own
//! id and its whole reference list:
//!
//! ```text
//! cargo run --profile ci --example probe_re85_opening_host_candidates -- model.rvt
//! ```
//!
//! and `tools/re/opening_hosts_vs_ifc.py` compares the hosts an rvt-rs export
//! writes with Revit's own export of the same model.

use rvt::partition_element_records as records;

fn main() -> anyhow::Result<()> {
    let Some(path) = std::env::args().nth(1) else {
        anyhow::bail!("usage: probe_re85_opening_host_candidates FILE.rvt");
    };
    let mut rf = rvt::RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let Some(marker) = records::bbox_marker(version) else {
        anyhow::bail!("Revit {version}: element records are not decoded");
    };
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let assigned = rf.second_prologue_ids();
    let empty = Default::default();
    for stream in rf.partition_stream_names() {
        let buf = rf.inflated_partition(&stream)?;
        for list in records::find_categories_records_assigned(
            &stream,
            buf.bytes(),
            &[records::OST_DOORS, records::OST_WINDOWS],
            &declared,
            &marker,
            assigned.get(&stream).unwrap_or(&empty),
        ) {
            for record in list {
                println!(
                    "{}\t{}\t{:?}",
                    record.element_id,
                    record
                        .preceding_reference
                        .map_or(String::new(), |id| id.to_string()),
                    record.references
                );
            }
        }
    }
    Ok(())
}
