//! #1603, RE-478: which symbol of a Revit 2015 to 2023 door or window holds the
//! geometry-material map that names Revit's materials for it.
//!
//! FACT to check: the type of such an element is `InstanceInfo.m_symbolId`
//! (Revit's Tag of the type) and its name is on the master symbol its record
//! names, `m_masterSymbolId` (RE-475, RE-477). RE-473 found the family type's
//! `m_geomTag2MaterialId` on every release from 2015, in the older id shape
//! `{"m_id": {"m_id": n}}`, and found that the scanned map of the symbol that
//! draws a window is not Revit's (53 of 55 windows apart in the first run of
//! RE-477). Which of the two symbols the instance names holds the map that
//! equals Revit's material set for the window, and does the other?
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_type_maps_vs_ifc.py`
//! reads:
//!   wmap <tag> <IFC class> grouped=<0|1> drawn=<id|-> master=<id|-> \
//!        drawn_map=<read|absent> <names> master_map=<read|absent> <names>
//! with a map's names `a|b` (spaces as `_`), `-` for none, and `+unset` when
//! some geometry maps to no material (-1). Ids and names: the held-out scrub
//! drops these lines, and keeps only the scorer's counts.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_window_door_type_maps_pre2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};

const CLASSES: [&str; 2] = ["IFCWINDOW", "IFCDOOR"];

/// The ElementId in `value`, in either shape: `{"m_id": {"m_id64": n}}` from
/// Revit 2024, `{"m_id": {"m_id": n}}` before it (RE-280).
fn element_id(value: &Fv) -> Option<u64> {
    value
        .get("m_id")
        .and_then(|v| v.get("m_id64").or_else(|| v.get("m_id")))
        .and_then(Fv::as_u64)
        .filter(|id| *id != 0)
}

/// What one instance's record names.
#[derive(Default)]
struct Instance {
    drawn: Option<u64>,
    master: Option<u64>,
    grouped: bool,
}

/// A symbol's `m_geomTag2MaterialId`: the material ids it names, and whether
/// any geometry maps to none (-1).
#[derive(Default)]
struct Map {
    materials: BTreeSet<u32>,
    unset: bool,
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut elements: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) {
                if let Ok(id) = tag.parse() {
                    elements.entry(id).or_insert(upper);
                }
            }
        }
    }

    let mut instances: BTreeMap<u64, Instance> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: elements.keys().copied().collect(),
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let mut here = Instance::default();
            for object in &graph.objects {
                match object.class_name.as_str() {
                    "InstanceInfo" => {
                        here.drawn = object.fields.get("m_symbolId").and_then(element_id);
                    }
                    "ElementGroupMembership" => {
                        here.grouped |= object
                            .fields
                            .get("m_groupId")
                            .and_then(element_id)
                            .is_some();
                    }
                    _ => {}
                }
            }
            here.master = graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_masterSymbolId"))
                .and_then(element_id);
            instances.insert(record.identity.element_id, here);
            Ok(())
        },
    )?;

    let symbols: BTreeSet<u64> = instances
        .values()
        .flat_map(|i| [i.drawn, i.master])
        .flatten()
        .collect();
    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names: BTreeMap<u32, String> =
        if rvt::research::partition_element_records_2023::is_32bit_release(version) {
            rvt::research::partition_materials::scan_materials_32(&mut rf, version, &declared).1
        } else {
            rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?
        };
    let mut maps: BTreeMap<u64, Map> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: symbols,
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            let Some(entries) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
                .and_then(|root| root.fields.get("m_geomTag2MaterialId"))
                .and_then(Fv::as_array)
            else {
                return Ok(());
            };
            let mut here = Map::default();
            for entry in entries {
                let id = entry
                    .pointer("/second/m_id/m_id64")
                    .or_else(|| entry.pointer("/second/m_id/m_id"))
                    .and_then(Fv::as_i64);
                match id {
                    Some(-1) => here.unset = true,
                    Some(id) => {
                        if let Some(id) = u32::try_from(id).ok().filter(|id| names.contains_key(id))
                        {
                            here.materials.insert(id);
                        }
                    }
                    None => {}
                }
            }
            maps.insert(record.identity.element_id, here);
            Ok(())
        },
    )?;

    let show = |symbol: Option<u64>| -> String {
        let Some(map) = symbol.and_then(|id| maps.get(&id)) else {
            return "absent -".into();
        };
        let mut list: Vec<String> = map
            .materials
            .iter()
            .filter_map(|id| names.get(id))
            .map(|name| name.replace(' ', "_"))
            .collect();
        list.sort();
        let mut text = if list.is_empty() {
            "-".to_string()
        } else {
            list.join("|")
        };
        if map.unset {
            text.push_str("+unset");
        }
        format!("read {text}")
    };
    for (tag, class) in &elements {
        let instance = instances.get(tag);
        let drawn = instance.and_then(|i| i.drawn);
        let master = instance.and_then(|i| i.master);
        println!(
            "wmap {tag} {class} grouped={} drawn={} master={} drawn_map={} master_map={}",
            u8::from(instance.is_some_and(|i| i.grouped)),
            drawn.map_or("-".into(), |id| id.to_string()),
            master.map_or("-".into(), |id| id.to_string()),
            show(drawn),
            show(master),
        );
    }
    eprintln!(
        "{} doors and windows, {} native records, {} symbol maps read",
        elements.len(),
        instances.len(),
        maps.len()
    );
    Ok(())
}
