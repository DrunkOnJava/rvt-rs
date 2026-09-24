//! RE-64: a curtain panel that is a wall names a wall type.
//!
//! FACT: Revit lets a curtain grid cell hold a basic wall instead of a
//! panel. The element's record keeps the curtain-panel category
//! (`OST_CurtainWallPanels`), but its reference list names no panel type:
//! it names one wall type instead. On Autodesk's Snowdon Towers sample:
//! - 18 panels name wall type 1506825, which has 4 compound layers, and
//!   Revit's own IFC export names all 18 `Basic Wall:<that type>:<ElementId>`;
//! - 23 panels name wall type 1005464 ("Solar Wall Window Frame"), which has
//!   none, and Revit's export contains none of them;
//! - the other 486 panel records name no wall type (family panels).
//!
//! The probe prints one tab-separated row per curtain-panel record:
//! - its id;
//! - how many panel types its list names;
//! - the wall types it names, each as `id(layers) name`;
//! - the one layered wall type's name, or `-`.
//!
//! Join the rows to Revit's export by `Tag`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re64_panel_wall_types -- \
//!     MODEL.rvt > rows.tsv

use rvt::RevitFile;
use rvt::partition_element_records as per;
use rvt::partition_type_records as ptr;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let Some(marker) = per::bbox_marker(version) else {
        eprintln!("Revit {version}: element records are not read on this release");
        return Ok(());
    };
    let type_ids = |rf: &mut RevitFile, category: i64| -> rvt::Result<BTreeSet<u32>> {
        Ok(ptr::type_definition_ids(&ptr::scan_type_records(
            rf, version, category, &declared,
        )?))
    };
    let wall_types = type_ids(&mut rf, per::OST_WALLS)?;
    let panel_types = type_ids(&mut rf, per::OST_CURTAIN_WALL_PANELS)?;
    let materials: BTreeSet<u32> =
        ptr::scan_type_records(&mut rf, version, ptr::OST_MATERIALS, &declared)?
            .iter()
            .map(|record| record.element_id)
            .collect();

    let assigned = rf.second_prologue_ids();
    let none = BTreeMap::new();
    let mut panels = Vec::new();
    let mut seen = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let ids = assigned.get(&stream).unwrap_or(&none);
        for record in per::find_category_records_assigned(
            &stream,
            inflated.bytes(),
            per::OST_CURTAIN_WALL_PANELS,
            &declared,
            &marker,
            ids,
        ) {
            if seen.insert(record.element_id) {
                panels.push(record);
            }
        }
    }
    let named = |references: &[u64], types: &BTreeSet<u32>| -> Vec<u32> {
        let mut out: Vec<u32> = references
            .iter()
            .filter_map(|slot| u32::try_from(*slot).ok())
            .filter(|id| types.contains(id))
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    };
    let wanted: BTreeSet<u32> = panels
        .iter()
        .flat_map(|record| named(&record.references, &wall_types))
        .collect();
    let layers = rvt::partition_compound_structure::scan_type_layers(
        &mut rf, version, &wanted, &materials, &declared,
    )?;
    let layered: BTreeSet<u32> = layers
        .iter()
        .filter(|(_, layers)| !layers.is_empty())
        .map(|(id, _)| *id)
        .collect();
    let mut names: BTreeMap<u32, String> = BTreeMap::new();
    if let Some(header) = rvt::partition_names::element_data_header(version) {
        for stream in rf.partition_stream_names() {
            let Ok(inflated) = rf.inflated_partition(&stream) else {
                continue;
            };
            for (id, name) in
                rvt::partition_names::find_element_data_names(inflated.bytes(), &header, &wanted)
            {
                names.entry(id).or_insert(name);
            }
        }
    }
    eprintln!(
        "Revit {version}: {} curtain-panel records, {} wall types, {} named by a panel, {} of them layered",
        panels.len(),
        wall_types.len(),
        wanted.len(),
        layered.len()
    );
    for record in &panels {
        let walls = named(&record.references, &wall_types);
        let with_layers: Vec<u32> = walls
            .iter()
            .copied()
            .filter(|id| layered.contains(id))
            .collect();
        println!(
            "{}\t{}\t{}\t{}",
            record.element_id,
            named(&record.references, &panel_types).len(),
            walls
                .iter()
                .map(|id| format!(
                    "{id}({}) {}",
                    layers.get(id).map_or(0, Vec::len),
                    names.get(id).map_or("-", String::as_str)
                ))
                .collect::<Vec<_>>()
                .join(" "),
            match with_layers.as_slice() {
                [id] => names.get(id).cloned().unwrap_or_else(|| "-".into()),
                _ => "-".into(),
            },
        );
    }
    Ok(())
}
