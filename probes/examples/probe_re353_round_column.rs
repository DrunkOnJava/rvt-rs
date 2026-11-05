//! RE-353 (#1203, W7): what rvt-rs's decode holds for the BIM4LCA office's one
//! round column (3053209), for finding why its `OuterSurfaceArea`, the
//! circumference times the height, is not written.
//!
//! FACT under test: the column's type (3053174) holds one named length
//! parameter, `Diameter` (RE-291), so the decode carries
//! `m_column_round_diameter`, its pair steps are read and name nothing, and
//! its body is the record box. The decode of run 42120085285 carried the pair
//! fields and no diameter, so the type's named lengths are listed, with every
//! copy of every length entry of its value blocks at full precision.
//!
//! Output:
//!   element <id> class <class>
//!   field <name> <value>        fields naming the column, its type or its sections
//!   entry <stream> <parameter> <value>   every `f64 · ff × 8 · i64` entry of the type's blocks
//!   named <type> <name>=<value> ...      `type_named_lengths` of the type
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re353_round_column -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::partition_schema_mvp::recover_partition_schema_mvp;
use rvt::research::partition_type_parameters::{partition_length_entries, type_named_lengths};
use rvt::walker::WalkerLimits;

const COLUMN: u32 = 3_053_209;
const TYPE: u32 = 3_053_174;

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
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in partition_length_entries(inflated.bytes(), &declared) {
            if owner != TYPE {
                continue;
            }
            for (parameter, value) in entries {
                println!("entry {stream} {parameter} {value:?}");
            }
        }
    }
    let named = type_named_lengths(&mut rf, version, &BTreeSet::from([TYPE]));
    println!("named {TYPE} {named:?}");
    Ok(())
}
