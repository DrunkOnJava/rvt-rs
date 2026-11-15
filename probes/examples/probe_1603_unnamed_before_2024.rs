//! #1603, RE-482: what decides whether Revit writes `<Unnamed>` for a door or
//! window of a Revit 2015 to 2023 file, and does the family-forms reader find a
//! form hidden by a stored `Visible` of 0 before 2024?
//!
//! FACT to check: RE-481 found that the Revit 2024 rule (`<Unnamed>` only where
//! the family shows a form with no material) is wrong on the spent Revit 2018
//! project, where ` <Unnamed>` is written on 37 of the 38 windows whose family
//! shows no unset form and on 8 of the 21 whose family shows one, and that its
//! control had no form hidden by a `Visible` of 0. Which facts of a family's
//! forms separate the doors and windows Revit gives `<Unnamed>` from those it
//! does not, on each project?
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_unnamed_vs_ifc.py`
//! reads:
//!   wfam <tag> <IFC class> grouped=<0|1> map_unset=<0|1|na> found=<0|1> \
//!        shown=<n> hidden=<n> voids=<n> nested=<n> zero=<n> unset_bound=<n>
//! where `map_unset` says whether the drawn symbol's `m_geomTag2MaterialId`
//! maps some geometry to no material (-1; `na` where the map was not read),
//! `found` whether its family's document gave a decoded record, and, for the
//! family's forms: `shown` the forms with no material that are not voids and
//! whose stored `Visible` is not 0, `hidden` those whose is 0, `voids` the
//! voids with no material, `nested` the nested family instances, `zero` the
//! forms of any material with a stored `Visible` of 0, `unset_bound` the
//! forms with no material whose `Visible` is bound to a family parameter.
//! And one line per model, the counts the held-out scrub keeps:
//!   family visible values: release R forms F stored zero a stored one b \
//!       stored other c stored none d
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_unnamed_before_2024 -- MODEL.rvt

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
    let mut maps: BTreeMap<u64, (bool, Option<u64>)> = BTreeMap::new();
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
            let unset = entries
                .iter()
                .any(|entry| id_in_either(entry.pointer("/second")) == Some(-1));
            let family = id_in_either(root.fields.get("m_familyId"))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            maps.insert(record.identity.element_id, (unset, family));
            Ok(())
        },
    )?;

    // The families' own documents: their forms' facts.
    let families: BTreeSet<u64> = maps.values().filter_map(|(_, family)| *family).collect();
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
            Some((unset, family)) => (if *unset { "1" } else { "0" }, *family),
            None => ("na", None),
        };
        let found = family.and_then(|id| facts.get(&id));
        let empty = Facts::default();
        let here = found.unwrap_or(&empty);
        println!(
            "wfam {tag} {class} grouped={} map_unset={map_unset} found={} shown={} hidden={} voids={} nested={} zero={} unset_bound={}",
            u8::from(grouped),
            u8::from(found.is_some()),
            here.shown,
            here.hidden,
            here.voids,
            here.nested,
            here.zero,
            here.unset_bound,
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
