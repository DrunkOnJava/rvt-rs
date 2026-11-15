//! #1603, RE-481: do the rules RE-204 to RE-206 bring the geometry-material
//! map of the symbol a Revit 2015 to 2023 door or window draws to Revit's
//! material set?
//!
//! FACT to check: RE-478 found that the drawn symbol's map equals Revit's set
//! on 558 of 992 doors and windows (603 once Revit 2018's spelling of the
//! unset material is dropped), and that the other 389 need the rules the
//! family's own document gives on Revit 2024 and later: `<Unnamed>` is written
//! only where the family shows a visible form with no material (RE-204), and a
//! material only hidden forms hold is left out (RE-205, RE-206). RE-479 found
//! the family documents' forms on every release, with their ids in the older
//! shape before 2024. `native_family_forms::family_forms_in` reads them in
//! either shape. Applied as `refine_type_materials` applies them, do the rules
//! make the map Revit's set?
//!
//! The rules: the map's materials minus the family's hidden-only materials,
//! and `<Unnamed>` where the map holds geometry mapped to no material (−1) and
//! the family shows unset (`shows_unset` is not false; a family whose forms do
//! not decode keeps the map as it is, `<Unnamed>` included).
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_family_rules_vs_ifc.py`
//! reads:
//!   wrule <tag> <IFC class> grouped=<0|1> map=<read|absent> <names> \
//!         rule=<read|absent> <names> forms=<found|none> \
//!         shows_unset=<true|false|unknown> hidden=<n>
//! with a set's names `a|b` (each percent-encoded), `-` for none, and `+unset`
//! where the set holds geometry mapped to no material. Ids and names: the
//! held-out scrub drops these lines and keeps only the scorer's counts.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_family_rules_pre2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_family_forms::family_forms_in;

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

/// A symbol's `m_geomTag2MaterialId`: the materials of the file it names, and
/// whether any geometry maps to none (−1).
#[derive(Default)]
struct Map {
    materials: BTreeSet<u32>,
    unset: bool,
    family: Option<u64>,
}

/// A set of materials as the scorer reads it.
fn show(ids: &BTreeSet<u32>, unset: bool, names: &BTreeMap<u32, String>) -> String {
    let mut list: Vec<String> = ids
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
    if unset {
        text.push_str("+unset");
    }
    text
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

    // Each instance's drawn symbol (Revit's type, RE-477) and whether it is a
    // member of a model group.
    let mut drawn: BTreeMap<u64, (Option<u64>, bool)> = BTreeMap::new();
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
            let (mut symbol, mut grouped) = (None, false);
            for object in &graph.objects {
                match object.class_name.as_str() {
                    "InstanceInfo" => {
                        symbol = object.fields.get("m_symbolId").and_then(element_id);
                    }
                    "ElementGroupMembership" => {
                        grouped |= object
                            .fields
                            .get("m_groupId")
                            .and_then(element_id)
                            .is_some();
                    }
                    _ => {}
                }
            }
            drawn.insert(record.identity.element_id, (symbol, grouped));
            Ok(())
        },
    )?;

    let declared = rvt::elem_table::declared_ids(&rvt::elem_table::parse_records(&mut rf)?);
    let names: BTreeMap<u32, String> =
        if rvt::research::partition_element_records_2023::is_32bit_release(version) {
            rvt::research::partition_materials::scan_materials_32(&mut rf, version, &declared).1
        } else {
            rvt::research::partition_materials::scan_material_names(&mut rf, version, &declared)?
        };
    let symbols: BTreeSet<u64> = drawn.values().filter_map(|(symbol, _)| *symbol).collect();
    let mut maps: BTreeMap<u64, Map> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: symbols,
            releases: NATIVE_RELEASES,
            ..Options::default()
        },
        |record| {
            let Some(root) = record
                .graph
                .as_ref()
                .and_then(|graph| graph.objects.first())
            else {
                return Ok(());
            };
            let Some(entries) = root
                .fields
                .get("m_geomTag2MaterialId")
                .and_then(Fv::as_array)
            else {
                return Ok(());
            };
            let mut here = Map {
                family: root.fields.get("m_familyId").and_then(element_id),
                ..Map::default()
            };
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

    let families: BTreeSet<u64> = maps.values().filter_map(|map| map.family).collect();
    let forms = match family_forms_in(&mut rf, &families, &NATIVE_RELEASES) {
        Ok(forms) => forms,
        Err(error) => {
            eprintln!("family forms not read: {error}");
            BTreeMap::new()
        }
    };

    for (tag, class) in &elements {
        let (symbol, grouped) = drawn.get(tag).copied().unwrap_or((None, false));
        let map = symbol.and_then(|id| maps.get(&id));
        let Some(map) = map else {
            println!(
                "wrule {tag} {class} grouped={} map=absent - rule=absent - forms=none shows_unset=unknown hidden=0",
                u8::from(grouped)
            );
            continue;
        };
        let family = map.family.and_then(|id| forms.get(&id));
        // As `refine_type_materials` applies them: the materials only hidden
        // forms hold are left out, and `<Unnamed>` is written where the map
        // has geometry with none and the family shows it. A family whose
        // forms do not decode says nothing.
        let hidden = family.map_or(0, |family| {
            map.materials
                .iter()
                .filter(|id| family.hidden_only.contains(id))
                .count()
        });
        let kept: BTreeSet<u32> = map
            .materials
            .iter()
            .copied()
            .filter(|id| !family.is_some_and(|family| family.hidden_only.contains(id)))
            .collect();
        let shown = family.and_then(|family| family.shows_unset).unwrap_or(true);
        let shows = match family.map(|family| family.shows_unset) {
            Some(Some(true)) => "true",
            Some(Some(false)) => "false",
            _ => "unknown",
        };
        println!(
            "wrule {tag} {class} grouped={} map=read {} rule=read {} forms={} shows_unset={shows} hidden={hidden}",
            u8::from(grouped),
            show(&map.materials, map.unset, &names),
            show(&kept, map.unset && shown, &names),
            if family.is_some() { "found" } else { "none" },
        );
    }
    eprintln!(
        "{} doors and windows, {} symbol maps read, {} families with forms",
        elements.len(),
        maps.len(),
        forms.len()
    );
    Ok(())
}
