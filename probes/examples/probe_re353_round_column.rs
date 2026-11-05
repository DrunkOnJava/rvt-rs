//! RE-353 (#1203, W7): what rvt-rs's decode holds for the BIM4LCA office's one
//! round column (3053209), for finding why its `OuterSurfaceArea`, the
//! circumference times the height, is not written.
//!
//! FACT under test: the column's type holds one named length parameter,
//! `Diameter` (RE-291), so the decode carries `m_column_round_diameter`, its
//! pair steps are read and name nothing, and its body is the record box.
//!
//! Output, one line per field of the column's decoded element whose name
//! mentions the column, its type or its sections:
//!   field <name> <value>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re353_round_column -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::walker::WalkerLimits;

const COLUMN: u32 = 3_053_209;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    for element in mvp.columns.iter().chain(mvp.products.iter()) {
        if element.id != Some(COLUMN) {
            continue;
        }
        println!("element {} class {}", COLUMN, element.class);
        for (name, value) in &element.fields {
            let shown = format!("{value:?}");
            let lower = name.to_ascii_lowercase();
            if lower.contains("column") || lower.contains("type") || lower.contains("section") {
                println!(
                    "field {name} {}",
                    shown.chars().take(160).collect::<String>()
                );
            }
        }
    }
    Ok(())
}
