//! Why the BIM4LCA office's curtain walls take no type name (#35, W7).
//!
//! FACT under test: a curtain wall takes its type the way every other wall
//! does (#322, RE-44): the one wall type-definition record its own element
//! record's reference list names (RE-28), named by its serialised data.
//! The office's 220 curtain walls take none, where its 607 walls all do.
//! The probe tells which step fails for each.
//!
//! Output:
//!   summary <IFC class> records=<n> no_record=<n> type_refs=<0|1|2+ counts>
//!   cw <tag> stream=<s> offset=<o> types=[<id>...] refs=[<id>:<mark>...]
//!   type <id> definition=<bool> names=[...]
//! where a reference's mark is `D` for a wall type-definition record, `T`
//! for any other wall type record, `W` for another wall, `-` otherwise.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re230_curtain_wall_types -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::partition_element_records::{
    OST_WALLS, PartitionElementRecord, file_bbox_marker, find_category_records_assigned,
};
use rvt::research::partition_names::{element_data_header, find_element_data_names};
use rvt::research::partition_type_records::{scan_type_records, type_definition_ids};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut tags: BTreeMap<&'static str, BTreeSet<u32>> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let class = match ifc_type.to_ascii_uppercase().as_str() {
                "IFCCURTAINWALL" => "IfcCurtainWall",
                "IFCWALL" | "IFCWALLSTANDARDCASE" => "IfcWall",
                _ => continue,
            };
            if let Ok(id) = tag.parse() {
                tags.entry(class).or_default().insert(id);
            }
        }
    }
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let Some(marker) = file_bbox_marker(&mut rf, version) else {
        println!("release {version}: no bbox marker");
        return Ok(());
    };
    let assigned = rf.second_prologue_ids();
    let empty = BTreeMap::new();
    let mut records: BTreeMap<u32, Vec<PartitionElementRecord>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for record in find_category_records_assigned(
            &stream,
            inflated.bytes(),
            OST_WALLS,
            &declared,
            &marker,
            assigned.get(&stream).unwrap_or(&empty),
        ) {
            records.entry(record.element_id).or_default().push(record);
        }
    }
    let type_records = scan_type_records(&mut rf, version, OST_WALLS, &declared)?;
    let definitions = type_definition_ids(&type_records);
    let any_type: BTreeSet<u32> = type_records.iter().map(|r| r.element_id).collect();
    let walls: BTreeSet<u32> = records.keys().copied().collect();
    let mut wanted: BTreeSet<u32> = BTreeSet::new();
    for (class, ids) in &tags {
        let mut by_count: BTreeMap<usize, usize> = BTreeMap::new();
        let mut no_record = 0;
        for id in ids {
            let Some(found) = records.get(id) else {
                no_record += 1;
                continue;
            };
            let types: BTreeSet<u32> = found
                .iter()
                .flat_map(|r| r.references.iter())
                .filter_map(|&slot| u32::try_from(slot).ok())
                .filter(|slot| definitions.contains(slot))
                .collect();
            *by_count.entry(types.len().min(2)).or_default() += 1;
            if *class != "IfcCurtainWall" {
                continue;
            }
            for record in found {
                let mut types = Vec::new();
                let refs: Vec<String> = record
                    .references
                    .iter()
                    .map(|&slot| match u32::try_from(slot) {
                        Ok(v) if definitions.contains(&v) => {
                            types.push(v);
                            wanted.insert(v);
                            format!("{v}:D")
                        }
                        Ok(v) if any_type.contains(&v) => {
                            wanted.insert(v);
                            format!("{v}:T")
                        }
                        Ok(v) if walls.contains(&v) => format!("{v}:W"),
                        Ok(v) => format!("{v}:-"),
                        Err(_) => format!("{slot:#x}"),
                    })
                    .collect();
                println!(
                    "cw {id} stream={} offset={} second={} types={types:?} refs=[{}]",
                    record.stream,
                    record.offset,
                    record.id_from_enclosing_record,
                    refs.join(" ")
                );
            }
        }
        println!(
            "summary {class} records={} no_record={no_record} type_refs={by_count:?}",
            ids.len() - no_record
        );
    }
    let Some(header) = element_data_header(version) else {
        println!("release {version}: no element-data header");
        return Ok(());
    };
    let mut names: BTreeMap<u32, BTreeSet<String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (id, name) in find_element_data_names(inflated.bytes(), &header, &wanted) {
            names
                .entry(id)
                .or_default()
                .insert(format!("{stream}={name}"));
        }
    }
    for id in &wanted {
        println!(
            "type {id} definition={} names={:?}",
            definitions.contains(id),
            names.get(id).cloned().unwrap_or_default()
        );
    }
    Ok(())
}
