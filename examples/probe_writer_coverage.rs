//! Probe (C9): which writer paths a file's export reaches.
//!
//! Exports the file and counts the IFC entities whose writer paths the
//! committed synthetic fixtures exercise but no running check does: mapped
//! items and representation maps (IFC-21), faceted breps (IFC-20), I-shape
//! and circle profiles (IFC-24), and material profile set usages (IFC-30).
//!
//! Usage:
//!   cargo run --profile ci --example probe_writer_coverage -- MODEL.rvt

use rvt::RevitFile;
use rvt::ifc::{RvtDocExporter, write_step};

const ENTITIES: [&str; 7] = [
    "IFCMAPPEDITEM",
    "IFCREPRESENTATIONMAP",
    "IFCFACETEDBREP",
    "IFCISHAPEPROFILEDEF",
    "IFCCIRCLEPROFILEDEF",
    "IFCMATERIALPROFILESETUSAGE",
    "IFCMATERIALPROFILESET",
];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
    let step = write_step(&result.model);
    let counts: Vec<String> = ENTITIES
        .iter()
        .map(|entity| {
            let needle = format!("={entity}(");
            format!("{entity}={}", step.matches(&needle).count())
        })
        .collect();
    println!("{}", counts.join(" "));
    Ok(())
}
