//! #1603, RE-485: is `<Unnamed>` in a Revit 2015 to 2023 door or window's
//! material set constant within a drawn symbol (Revit's type) and within a
//! family, and what do the symbol's map tag counts say?
//!
//! FACT to check: RE-483 left 15 of the spent Revit 2018 project's 68 windows
//! with identical facts of the host family's forms, `<Unnamed>` written on 8
//! and not on 7. Whether the cause is the type's or the instance's is not in
//! those counts: this reads the grouping.
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_unnamed_by_type_vs_ifc.py`
//! reads:
//!   wtype <tag> <IFC class> grouped=<0|1> map_unset=<0|1|na> found=<0|1> \
//!        shown=<n> hidden=<n> voids=<n> nested=<n> zero=<n> unset_bound=<n> \
//!        symbol=<id> family=<id> tags=<n> unset_tags=<n>
//! where, for the family's forms, `shown` is the number of forms with no
//! material that are not voids and whose stored `Visible` is not 0, `hidden`
//! those whose is 0, `voids` the voids with no material and `nested` the
//! nested family instances, `zero` the forms of any material with a stored
//! `Visible` of 0 and `unset_bound` the forms with no material whose `Visible`
//! is bound to a family parameter (as RE-482); `symbol` and `family` are the ElementIds
//! of the drawn symbol and its family (0 where not read); `tags` is the number
//! of geometry tags the symbol's `m_geomTag2MaterialId` holds and `unset_tags`
//! how many of them map to no material (-1).
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_unnamed_by_type_before_2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_family_forms::family_records_in;

const CLASSES: [&str; 2] = ["IFCWINDOW", "IFCDOOR"];

/// The form classes `native_family_forms` reads, and the nested instances.
const FORM_CLASSES: [&str; 7] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
    "Text3dElem",
];

/// `IS_VISIBLE_PARAM`.
const VISIBLE_PARAMETER: i64 = -1_006_205;

/// The ElementId in `value`, in either shape: `{"m_id": {"m_id64": n}}` from
/// Revit 2024, `{"m_id": {"m_id": n}}` before it (RE-280).
fn id_in_either(value: Option<&Fv>) -> Option<i64> {
    value.and_then(|v| {
        v.pointer("/m_id/m_id64")
            .or_else(|| v.pointer("/m_id/m_id"))
            .and_then(Fv::as_i64)
    })
}

/// A drawn symbol's map: whether it holds a -1, its family, and its tag counts.
struct Map {
    unset: bool,
    family: Option<u64>,
    tags: usize,
    unset_tags: usize,
}

/// What one family's forms hold.
#[derive(Default)]
struct Facts {
    shown: usize,
    hidden: usize,
    voids: usize,
    nested: usize,
    zero: usize,
    unset_bound: usize,
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
                        symbol = id_in_either(object.fields.get("m_symbolId"))
                            .and_then(|id| u64::try_from(id).ok())
                            .filter(|id| *id != 0);
                    }
                    "ElementGroupMembership" => {
                        grouped |=
                            id_in_either(object.fields.get("m_groupId")).is_some_and(|id| id != 0);
                    }
                    _ => {}
                }
            }
            drawn.insert(record.identity.element_id, (symbol, grouped));
            Ok(())
        },
    )?;

    // Each drawn symbol's family, and whether its map holds a -1.
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
            let unset_tags = entries
                .iter()
                .filter(|entry| id_in_either(entry.pointer("/second")) == Some(-1))
                .count();
            let family = id_in_either(root.fields.get("m_familyId"))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            maps.insert(
                record.identity.element_id,
                Map {
                    unset: unset_tags > 0,
                    family,
                    tags: entries.len(),
                    unset_tags,
                },
            );
            Ok(())
        },
    )?;

    // The families' own documents: their forms' facts.
    let families: BTreeSet<u64> = maps.values().filter_map(|map| map.family).collect();
    let mut classes: BTreeSet<String> = FORM_CLASSES.iter().map(ToString::to_string).collect();
    classes.insert("FamilyInstance".into());
    let documents =
        family_records_in(&mut rf, &families, &classes, &NATIVE_RELEASES, None).unwrap_or_default();
    let mut facts: BTreeMap<u64, Facts> = BTreeMap::new();
    let (mut forms, mut zero, mut one, mut other, mut none) =
        (0usize, 0usize, 0usize, 0usize, 0usize);
    for (family, records) in &documents {
        let here = facts.entry(*family).or_default();
        for (_, graph) in records {
            let Some(root) = graph.objects.first() else {
                continue;
            };
            if root.class_name == "FamilyInstance" {
                here.nested += 1;
                continue;
            }
            forms += 1;
            let stored = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "ParamValueSetInt")
                .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
                .flatten()
                .find(|entry| id_in_either(entry.get("m_paramId")) == Some(VISIBLE_PARAMETER))
                .and_then(|entry| entry.get("m_value")?.as_i64());
            match stored {
                Some(0) => zero += 1,
                Some(1) => one += 1,
                Some(_) => other += 1,
                None => none += 1,
            }
            let material = id_in_either(root.fields.get("m_materialId"));
            let cutting = root
                .fields
                .get("m_cutting")
                .and_then(Fv::as_bool)
                .unwrap_or(false);
            let bound = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "FamilyParametrizedElemParamsCell")
                .filter_map(|object| object.fields.get("m_paramDrivenData")?.as_array())
                .flatten()
                .any(|entry| id_in_either(entry.get("m_elemPropId")) == Some(VISIBLE_PARAMETER));
            if stored == Some(0) {
                here.zero += 1;
            }
            if material == Some(-1) {
                match (cutting, stored == Some(0)) {
                    (true, _) => here.voids += 1,
                    (false, true) => here.hidden += 1,
                    (false, false) => here.shown += 1,
                }
                here.unset_bound += usize::from(bound);
            }
        }
    }

    for (tag, class) in &elements {
        let (symbol, grouped) = drawn.get(tag).copied().unwrap_or((None, false));
        let map = symbol.and_then(|id| maps.get(&id));
        let (map_unset, family) = match map {
            Some(map) => (if map.unset { "1" } else { "0" }, map.family),
            None => ("na", None),
        };
        let found = family.and_then(|id| facts.get(&id));
        let empty = Facts::default();
        let here = found.unwrap_or(&empty);
        println!(
            "wtype {tag} {class} grouped={} map_unset={map_unset} found={} shown={} hidden={} voids={} nested={} zero={} unset_bound={} symbol={} family={} tags={} unset_tags={}",
            u8::from(grouped),
            u8::from(found.is_some()),
            here.shown,
            here.hidden,
            here.voids,
            here.nested,
            here.zero,
            here.unset_bound,
            symbol.unwrap_or(0),
            family.unwrap_or(0),
            map.map_or(0, |map| map.tags),
            map.map_or(0, |map| map.unset_tags),
        );
    }
    println!(
        "family visible values: release {version} forms {forms} stored zero {zero} stored one {one} stored other {other} stored none {none}"
    );
    eprintln!(
        "{} doors and windows, {} symbol maps read, {} families with a document",
        elements.len(),
        maps.len(),
        documents.len()
    );
    Ok(())
}
