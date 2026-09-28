//! RE-124 probe: Revit 2026 partition element records, read with the marker
//! the file's own schema gives (RE-80, #421).
//!
//! FACT (to be measured): on a Revit 2026 project, records carrying the
//! schema-derived marker keep the 2024/2025 frame, and the RE-21 instance
//! rule (declared ElementId, no container, placed) selects the elements
//! Revit's own IFC export holds.
//!
//! Prints the marker, the ElementHeader tag, and per recovered category the
//! placed-instance ElementIds, one `category name count ids` line each, for
//! `tools/re/instances_vs_ifc_tags.py` to score against an export.
//!
//! ```bash
//! cargo run --profile ci --example probe_re124_revit_2026_records -- house.rvt > ids.txt
//! ```

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: probe_re124_revit_2026_records <file.rvt>")?;
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let marker = per::schema_bbox_marker(&mut rf).ok_or("schema gives no record marker")?;
    let header_tag = rf
        .schema_classes()?
        .classes
        .iter()
        .find(|c| c.name == "ElementHeader")
        .map(|c| c.tag);
    println!("release {version} marker {marker:02x?} ElementHeader tag {header_tag:?}");
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    println!("declared ids {}", declared.len());

    let mut frames = 0usize;
    let mut by_category: BTreeMap<i64, BTreeSet<u32>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let inflated = rf.inflated_partition(&stream)?;
        let buf = inflated.bytes();
        for (category, _) in per::RECOVERED_CATEGORIES {
            for record in
                per::find_category_records_with_marker(&stream, buf, category, &declared, &marker)
            {
                frames += 1;
                if record.container == per::CONTAINER_NONE
                    && record.placement_kind == per::PLACEMENT_KIND_INSTANCE
                {
                    by_category
                        .entry(category)
                        .or_default()
                        .insert(record.element_id);
                }
            }
        }
    }
    println!("frames with a declared ElementId {frames}");
    for (category, ids) in &by_category {
        let name = per::RECOVERED_CATEGORIES
            .iter()
            .find(|(c, _)| c == category)
            .map(|(_, n)| *n)
            .unwrap_or("?");
        let list: Vec<String> = ids.iter().map(u32::to_string).collect();
        println!(
            "category {category} {name} {} {}",
            ids.len(),
            list.join(",")
        );
    }
    Ok(())
}
