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
//!        drawn_map=<read|absent> <names> master_map=<read|absent> <names> \
//!        drawn_other=<names> master_other=<names> family_other=<names> \
//!        instance_other=<names>
//! with a map's names `a|b` (each percent-encoded, so a space is `%20` and a
//! `|` or `+` in a name cannot be taken for a separator), `-` for none, and
//! `+unset` when some geometry maps to no material (-1); `*_other` is the names of the
//! file's materials that the drawn symbol's record, the master symbol's, the
//! family's (the master's `m_familyId`) and the instance's name anywhere but in
//! that map. Ids and names: the held-out scrub drops these lines, and keeps only
//! the scorer's counts.
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

/// The ids among `ids` that are materials of the file.
fn materials_among(ids: &BTreeSet<u64>, names: &BTreeMap<u32, String>) -> BTreeSet<u32> {
    ids.iter()
        .filter_map(|id| u32::try_from(*id).ok())
        .filter(|id| names.contains_key(id))
        .collect()
}

/// The names of `ids`, encoded and sorted, joined with `|`; `-` for none.
fn list_names(ids: &BTreeSet<u32>, names: &BTreeMap<u32, String>) -> String {
    let mut list: Vec<String> = ids
        .iter()
        .filter_map(|id| names.get(id))
        .map(|name| encode(name))
        .collect();
    list.sort();
    if list.is_empty() {
        "-".into()
    } else {
        list.join("|")
    }
}

/// `name` with every byte but a letter, digit, `_`, `-`, `.` or `~` as `%XX`.
fn encode(name: &str) -> String {
    name.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"_-.~".contains(&byte) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

/// What one instance's record names.
#[derive(Default)]
struct Instance {
    drawn: Option<u64>,
    master: Option<u64>,
    grouped: bool,
    /// Every ElementId the record names.
    named: BTreeSet<u64>,
}

/// A symbol's `m_geomTag2MaterialId`: the material ids it names, and whether
/// any geometry maps to none (-1).
#[derive(Default)]
struct Map {
    materials: BTreeSet<u32>,
    unset: bool,
    /// Materials of the file named in the record outside the map.
    other: BTreeSet<u32>,
    family: Option<u64>,
}

/// Every ElementId under `value`.
fn all_ids(value: &Fv, out: &mut BTreeSet<u64>) {
    match value {
        Fv::Object(map) => {
            if let Some(id) = element_id(value) {
                out.insert(id);
                return;
            }
            for child in map.values() {
                all_ids(child, out);
            }
        }
        Fv::Array(items) => {
            for child in items.iter() {
                all_ids(child, out);
            }
        }
        _ => {}
    }
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
                all_ids(&object.fields, &mut here.named);
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
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let Some(entries) = graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_geomTag2MaterialId"))
                .and_then(Fv::as_array)
            else {
                return Ok(());
            };
            let mut here = Map::default();
            let mut elsewhere = BTreeSet::new();
            for object in &graph.objects {
                if let Fv::Object(fields) = &object.fields {
                    for (key, child) in fields.iter() {
                        if key != "m_geomTag2MaterialId" {
                            all_ids(child, &mut elsewhere);
                        }
                    }
                }
            }
            here.other = materials_among(&elsewhere, &names);
            here.family = graph
                .objects
                .first()
                .and_then(|root| root.fields.get("m_familyId"))
                .and_then(element_id);
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
            .map(|name| encode(name))
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
    // The families the two symbols name, and the materials their records name.
    let families: BTreeSet<u64> = instances
        .values()
        .flat_map(|i| [i.drawn, i.master])
        .flatten()
        .filter_map(|id| maps.get(&id).and_then(|map| map.family))
        .collect();
    let mut family_materials: BTreeMap<u64, BTreeSet<u32>> = BTreeMap::new();
    if !families.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: families,
                releases: NATIVE_RELEASES,
                ..Options::default()
            },
            |record| {
                let Some(graph) = record.graph.as_ref() else {
                    return Ok(());
                };
                let mut named = BTreeSet::new();
                for object in &graph.objects {
                    all_ids(&object.fields, &mut named);
                }
                family_materials
                    .insert(record.identity.element_id, materials_among(&named, &names));
                Ok(())
            },
        )?;
    }
    let other = |symbol: Option<u64>| -> String {
        symbol
            .and_then(|id| maps.get(&id))
            .map_or("-".into(), |map| list_names(&map.other, &names))
    };
    for (tag, class) in &elements {
        let instance = instances.get(tag);
        let drawn = instance.and_then(|i| i.drawn);
        let master = instance.and_then(|i| i.master);
        let family = drawn
            .and_then(|id| maps.get(&id))
            .or_else(|| master.and_then(|id| maps.get(&id)))
            .and_then(|map| map.family)
            .and_then(|id| family_materials.get(&id))
            .map_or("-".into(), |ids| list_names(ids, &names));
        let instance_other = instance.map_or("-".into(), |i| {
            list_names(&materials_among(&i.named, &names), &names)
        });
        println!(
            "wmap {tag} {class} grouped={} drawn={} master={} drawn_map={} master_map={} drawn_other={} master_other={} family_other={family} instance_other={instance_other}",
            u8::from(instance.is_some_and(|i| i.grouped)),
            drawn.map_or("-".into(), |id| id.to_string()),
            master.map_or("-".into(), |id| id.to_string()),
            show(drawn),
            show(master),
            other(drawn),
            other(master),
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
