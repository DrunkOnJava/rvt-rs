//! RE-299 (#1016, W7): which sketch line starts a floor's, roof's or
//! ceiling's profile in Revit's export, and so which way its `Width` and
//! `Length` run.
//!
//! FACT under test: revit-ifc's `WidthCalculator` and `LengthCalculator`
//! (CLEANROOM.md source 5) take a slab's `Width` and `Length` from the
//! extrusion's creation data, the extents of its rectangular profile along
//! the two axes of the profile's own plane. On the BIM4LCA office the extent
//! along the first edge of Revit's profile is `Length` and the other is
//! `Width`, on slabs that are all unturned in the model, so the axes are the
//! model's X and Y or the first edge's direction. Hypothesis: the first edge
//! is the sketch line of the lowest ElementId.
//!
//! Output, one block per element that owns sketch lines of a floor, roof or
//! ceiling record: `owner <ElementId> <category> lines <n>` and, one per
//! line in ElementId order, `line <ElementId> <min x> <min y> <max x> <max y>`
//! (feet, the line's record box, in the model's internal frame).
//!
//! `tools/re/slab_first_edge_vs_ifc.py` reads it.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re299_slab_first_edge -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::partition_element_records as per;
use std::collections::BTreeMap;

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let records = per::scan_category_records_multi(
        &mut rf,
        version,
        &[
            per::OST_FLOORS,
            per::OST_ROOFS,
            per::OST_CEILINGS,
            per::OST_SKETCH_LINES,
        ],
        &declared,
    )?;
    let mut plates: BTreeMap<u32, i64> = BTreeMap::new();
    for record in &records {
        if record.builtin_category != per::OST_SKETCH_LINES {
            plates
                .entry(record.element_id)
                .or_insert(record.builtin_category);
        }
    }
    let mut owners: BTreeMap<u32, BTreeMap<u32, [f64; 6]>> = BTreeMap::new();
    for record in &records {
        if record.builtin_category != per::OST_SKETCH_LINES {
            continue;
        }
        if let Some(owner) = record
            .owner_reference
            .filter(|owner| plates.contains_key(owner))
        {
            owners
                .entry(owner)
                .or_default()
                .entry(record.element_id)
                .or_insert(record.bbox_feet);
        }
    }
    for (owner, lines) in &owners {
        let category = match plates[owner] {
            per::OST_FLOORS => "floor",
            per::OST_ROOFS => "roof",
            _ => "ceiling",
        };
        println!("owner {owner} {category} lines {}", lines.len());
        for (id, b) in lines {
            println!("line {id} {} {} {} {}", b[0], b[1], b[3], b[4]);
        }
    }
    eprintln!("{} plates own sketch lines", owners.len());
    Ok(())
}
