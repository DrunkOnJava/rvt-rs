//! RE-129 (probe): where a pipe's or duct's type is named.
//!
//! Revit's export names a pipe `Pipe Types:<type>:<id>` and a duct
//! `Rectangular Duct:<type>:<id>`, but rvt-rs finds no type for them: the
//! type rule (`partition_names::resolve_type`) needs exactly one id in the
//! record's reference list with a name entry of the element's own category.
//!
//! This probe prints every name entry of the pipe and duct categories, and
//! for each pipe and duct record its reference list, each id with its name
//! entry (name and category) where it has one.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re129_mep_curve_types -- MODEL.rvt

use rvt::RevitFile;
use rvt::partition_element_records as per;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names = rf.element_names();
    let categories = [per::OST_PIPE_CURVES, per::OST_DUCT_CURVES];
    println!("Revit {version}: {} name entries", names.entries.len());
    for entry in names.entries.values() {
        if categories.contains(&entry.builtin_category) {
            println!(
                "entry {} {:?} category {}",
                entry.element_id, entry.name, entry.builtin_category
            );
        }
    }
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    for record in records.iter().take(12) {
        let Ok(inflated) = rf.inflated_partition(&record.stream) else {
            continue;
        };
        let references = per::decode_reference_list(
            inflated.bytes(),
            record.offset + per::REFERENCE_LIST_OFFSET,
        )
        .unwrap_or_default();
        println!(
            "record {} category {} class {} references {}",
            record.element_id,
            record.builtin_category,
            record.class_tag,
            references.len()
        );
        for id in references {
            let named = u32::try_from(id)
                .ok()
                .and_then(|id| names.entries.get(&id))
                .map(|e| format!("{:?} category {}", e.name, e.builtin_category))
                .unwrap_or_default();
            println!("    {id} {named}");
        }
    }
    Ok(())
}
