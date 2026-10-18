//! Every declared element's GlobalId, with the category its element record
//! carries (#763).
//!
//! Revit's exporter gives an element the GlobalId rvt-rs rebuilds from the
//! file (RE-48), so a GlobalId in Revit's own export names the ElementId it
//! came from even where the export carries no `Tag`, as on `IfcSpace`. The
//! probe prints one line per element `Global/ElemTable` declares: the
//! ElementId, its GlobalId, and for each partition element record of that
//! id (RE-21), its `BuiltInCategory` (the names of
//! `BUILTIN_CATEGORY_NAMES`, plus rooms, areas and MEP spaces), whether it is
//! a placed instance, a type symbol or neither, and whether it is a
//! container member. `tools/re/global_id_categories_vs_ifc.py` pairs it with
//! the `IfcSpace`s of Revit's export, which tells the rooms in Revit's export
//! that rvt-rs leaves out from its areas and the family instances it exports
//! as spaces.
//!
//! Usage:
//!   cargo run --profile ci --example probe_global_id_categories -- FILE.rvt

use rvt::RevitFile;
use rvt::partition_element_records::{BUILTIN_CATEGORY_NAMES, scan_category_records_multi};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Autodesk `BuiltInCategory` `OST_Rooms`, `OST_Areas` and `OST_MEPSpaces`,
/// which [`BUILTIN_CATEGORY_NAMES`] does not name.
const SPACE_CATEGORIES: [(i64, &str); 3] = [
    (-2_000_160, "Rooms"),
    (-2_003_200, "Areas"),
    (-2_003_600, "MEP Spaces"),
];

fn main() -> rvt::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("usage: FILE.rvt"));
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let global_ids = rvt::revit_global_ids::revit_global_ids(&mut rf)?;
    let categories: Vec<(i64, &str)> = BUILTIN_CATEGORY_NAMES
        .iter()
        .copied()
        .chain(SPACE_CATEGORIES)
        .collect();
    let ids: Vec<i64> = categories.iter().map(|(id, _)| *id).collect();
    let mut records: BTreeMap<u32, Vec<String>> = BTreeMap::new();
    for record in scan_category_records_multi(&mut rf, version, &ids, &declared)? {
        let name = categories
            .iter()
            .find(|(id, _)| *id == record.builtin_category)
            .map_or("-", |(_, name)| *name);
        let kind = if record.is_placed_instance() {
            "placed"
        } else if record.is_type_symbol() {
            "symbol"
        } else {
            "other"
        };
        let member = if record.is_container_member() {
            "member"
        } else {
            "free"
        };
        records
            .entry(record.element_id)
            .or_default()
            .push(format!("{name}|{kind}|{member}"));
    }
    println!(
        "release {version}: {} declared elements, {} with a GlobalId, {} with a record",
        declared.len(),
        global_ids.len(),
        records.len()
    );
    for id in &declared {
        let global_id = global_ids.get(id).map_or("-", String::as_str);
        let record = records
            .get(id)
            .map_or_else(|| "-".to_string(), |list| list.join(";"));
        println!("  {id} {global_id} {record}");
    }
    Ok(())
}
