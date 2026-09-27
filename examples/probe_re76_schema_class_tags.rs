//! FACT (RE-76, #154, #223): a partition element record names its class at
//! `+0x4a`, as the definition ordinal of the file's own `Formats/Latest`
//! (the first class is 12; each further definition, top-level or inline,
//! takes the next number). `formats::schema_classes` reads the whole
//! page-stripped schema by its grammar and resolves every tag.
//!
//! Verify: the per-category table must show one class per category, the
//! class a BIM user would expect (walls `SWall`, floors `Floor`, rooms
//! `RoomElem`), and families split into `FamilyInstance` for placed
//! instances and `FamilySymbol` for type symbols:
//!
//!   cargo run --profile ci --example probe_re76_schema_class_tags -- FILE.rvt ...
use rvt::RevitFile;
use rvt::elem_table;
use rvt::partition_element_records::scan_category_records_multi;
use std::collections::{BTreeMap, BTreeSet};

const CATEGORIES: &[(i64, &str)] = &[
    (-2000011, "OST_Walls"),
    (-2000032, "OST_Floors"),
    (-2000160, "OST_Rooms"),
    (-2000023, "OST_Doors"),
    (-2000014, "OST_Windows"),
    (-2000100, "OST_Columns"),
    (-2000035, "OST_Roofs"),
    (-2000038, "OST_Ceilings"),
    (-2000080, "OST_Furniture"),
    (-2000120, "OST_Stairs"),
];

fn main() -> rvt::Result<()> {
    for path in std::env::args().skip(1) {
        let mut rf = RevitFile::open(&path)?;
        let version = rf.basic_file_info()?.version;
        let classes = rf.schema_classes()?;
        println!(
            "== {path}\n   Revit {version}: {} classes, tags {}..{}, {} bytes read, stopped: {:?}",
            classes.classes.len(),
            classes.classes.first().map_or(0, |c| c.tag),
            classes.classes.last().map_or(0, |c| c.tag),
            classes.parsed_bytes,
            classes.stopped,
        );
        let declared: BTreeSet<u32> = elem_table::parse_records(&mut rf)?
            .into_iter()
            .map(|r| r.id_primary)
            .collect();
        let ids: Vec<i64> = CATEGORIES.iter().map(|(id, _)| *id).collect();
        let records = scan_category_records_multi(&mut rf, version, &ids, &declared)?;
        let mut table: BTreeMap<(&str, bool, String), usize> = BTreeMap::new();
        for record in &records {
            let category = CATEGORIES
                .iter()
                .find(|(id, _)| *id == record.builtin_category)
                .map_or("?", |(_, name)| name);
            let class = classes.by_tag(record.class_tag).map_or_else(
                || format!("unresolved {:#06x}", record.class_tag),
                |c| c.name.clone(),
            );
            *table
                .entry((category, record.is_exported_instance(), class))
                .or_default() += 1;
        }
        for ((category, instance, class), count) in table {
            let kind = if instance { "export" } else { "other" };
            println!("   {category:14} {kind:6} {class:28} {count}");
        }
    }
    Ok(())
}
