//! RE-72: the curtain grid a curtain panel lies on names its curtain wall.
//!
//! FACT: on Autodesk's Snowdon Towers 2024 architectural sample, a curtain
//! panel's reference list names, among others, an id with no element record
//! whose own element data carries a wall's ElementId at +85 and again at
//! +478: the curtain grid the panel lies on. That wall is the curtain wall
//! Revit's own IFC4 export aggregates the panel under, on all 18 panels that
//! name such a grid and that Revit aggregates, including panels that name
//! several walls and the nested curtain wall RE-46 gave a wrong parent.
//!
//! The probe prints one tab-separated row per curtain panel and mullion
//! element record, first copy of each id:
//! - its ElementId and `panel` or `mullion`;
//! - every id its counted reference list names, each as `id:kind:name`:
//!   - `kind` is `entry` for an id with a name entry (RE-38, with the
//!     entry's name), `grid` for a curtain grid (with the wall it names),
//!     `object` for an id with a type object (tag `db 0f`, with its
//!     element-data name), `wall` / `panel` / `mullion` for an element
//!     record of those categories, and `-` otherwise.
//!
//! Join the rows to Revit's export by `Tag`
//! (`tools/re/aggregates_vs_ifc.py` scores the result end to end).
//!
//! Usage:
//!   cargo run --profile ci --example probe_re72_curtain_panels -- MODEL.rvt > panels.tsv

use rvt::RevitFile;
use rvt::partition_element_records as per;
use std::collections::{BTreeMap, BTreeSet};

fn main() -> rvt::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let Some(header) = rvt::partition_names::element_data_header(version) else {
        eprintln!("Revit {version}: element data is not read on this release");
        return Ok(());
    };
    let categories = [
        per::OST_CURTAIN_WALL_PANELS,
        per::OST_WALLS,
        per::OST_CURTAIN_WALL_MULLIONS,
    ];
    let records = per::scan_category_records_multi(&mut rf, version, &categories, &declared)?;
    let mut kind_of: BTreeMap<u32, &str> = BTreeMap::new();
    for record in &records {
        let kind = match record.builtin_category {
            per::OST_CURTAIN_WALL_PANELS => "panel",
            per::OST_WALLS => "wall",
            _ => "mullion",
        };
        kind_of.entry(record.element_id).or_insert(kind);
    }
    let entries = rf.element_names();
    let named: BTreeSet<u32> = records
        .iter()
        .filter(|record| record.builtin_category != per::OST_WALLS)
        .flat_map(|record| record.references.iter())
        .filter_map(|slot| u32::try_from(*slot).ok())
        .collect();
    let mut objects = BTreeSet::new();
    let mut data_names = BTreeMap::new();
    // An id whose element data names a recorded wall at +85 and again at
    // +478: the curtain grid the part lies on.
    let mut grids: BTreeMap<u32, u32> = BTreeMap::new();
    let u64_at = |buf: &[u8], at: usize| {
        buf.get(at..at + 8)
            .map(|b| u64::from_le_bytes(b.try_into().expect("8 bytes")))
    };
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for hit in memchr::memmem::find_iter(inflated.bytes(), &header) {
            let data = hit + header.len() + 8;
            let Some(id) = u64_at(inflated.bytes(), hit + header.len())
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| named.contains(id) && !kind_of.contains_key(id))
            else {
                continue;
            };
            let (Some(first), Some(second)) = (
                u64_at(inflated.bytes(), data + 85),
                u64_at(inflated.bytes(), data + 478),
            ) else {
                continue;
            };
            let wall = u32::try_from(first)
                .ok()
                .filter(|wall| first == second && kind_of.get(wall) == Some(&"wall"));
            if let Some(wall) = wall {
                grids.entry(id).or_insert(wall);
            }
        }
        objects.extend(rvt::partition_names::find_type_object_ids(
            inflated.bytes(),
            &named,
        ));
        data_names.extend(rvt::partition_names::find_element_data_names(
            inflated.bytes(),
            &header,
            &named,
        ));
    }
    let mut seen = BTreeSet::new();
    let mut rows = 0;
    for record in &records {
        if record.builtin_category == per::OST_WALLS || !seen.insert(record.element_id) {
            continue;
        }
        let cells: Vec<String> = record
            .references
            .iter()
            .filter_map(|slot| u32::try_from(*slot).ok())
            .filter(|id| *id != record.element_id)
            .map(|id| {
                if let Some(entry) = entries.entries.get(&id) {
                    format!("{id}:entry:{}", entry.name)
                } else if let Some(wall) = grids.get(&id) {
                    format!("{id}:grid:{wall}")
                } else if objects.contains(&id) {
                    format!(
                        "{id}:object:{}",
                        data_names.get(&id).map_or("", String::as_str)
                    )
                } else {
                    format!("{id}:{}:", kind_of.get(&id).copied().unwrap_or("-"))
                }
            })
            .collect();
        let kind = if record.builtin_category == per::OST_CURTAIN_WALL_PANELS {
            "panel"
        } else {
            "mullion"
        };
        println!("{}\t{kind}\t{}", record.element_id, cells.join("\t"));
        rows += 1;
    }
    eprintln!(
        "Revit {version}: {rows} curtain panel and mullion records, {} grids naming a wall",
        grids.len()
    );
    Ok(())
}
