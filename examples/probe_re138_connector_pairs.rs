//! RE-138 (probe): which element each end of a duct or pipe is joined to
//! (#528).
//!
//! Prints the ducts' and pipes' ElementIds and the connector pairs
//! `partition_connector_pairs` reads, one JSON object per line, for
//! `tools/re/connector_pairs_vs_ifc.py` to score against the
//! `IfcRelConnectsPorts` of Revit's own export.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re138_connector_pairs -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_connector_pairs as pairs;
use rvt::partition_element_records as per;
use std::collections::BTreeSet;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    println!(
        "{{\"revit\":{version},\"supported\":{}}}",
        pairs::supports_revit_version(version)
    );
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = per::scan_category_records_multi(
        &mut rf,
        version,
        &[per::OST_DUCT_CURVES, per::OST_PIPE_CURVES],
        &declared,
    )?;
    let curves: BTreeSet<u64> = records
        .iter()
        .map(|record| u64::from(record.element_id))
        .collect();
    for &id in &curves {
        println!("{{\"curve\":{id}}}");
    }
    let found = pairs::scan_connector_pairs(&mut rf, &curves)?;
    for pair in &found {
        println!(
            "{{\"pair\":[{},{},{},{}]}}",
            pair.element, pair.index, pair.other, pair.other_index
        );
    }
    eprintln!(
        "Revit {version}: {} ducts and pipes, {} connector pairs",
        curves.len(),
        found.len()
    );
    Ok(())
}
