//! #1603, RE-483: is `<Unnamed>` in a Revit 2015 to 2023 door or window's
//! material set decided by whether its family's form with no material is
//! effectively shown, once a `Visible` bound to a family parameter is read
//! from that parameter's value on the instance or its type?
//!
//! FACT to check: RE-482 found that no category of the host family's forms by
//! their stored `Visible` separates the doors and windows whose set holds
//! `<Unnamed>` from those whose set does not. A stored `Visible` says nothing
//! of what a form bound to a family parameter shows: on the spent Revit 2018
//! project 44 of the 45 windows whose only unset form is stored hidden have
//! that form bound, and the 8 whose bound unset form is stored shown have no
//! `<Unnamed>` written. What does the bound parameter's value say?
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_unnamed_bound_vs_ifc.py`
//! reads:
//!   wbound <tag> <IFC class> grouped=<0|1> map_unset=<0|1|na> found=<0|1> \
//!        eff_shown=<n> eff_hidden=<n> unresolved=<n> bound=<n> resolved=<n> \
//!        voids=<n> nested=<n>
//! where, for the family's forms with no material that are not voids,
//! `eff_shown` and `eff_hidden` count those a stored `Visible` that is not
//! bound (or the bound parameter's value on the instance, else on its type, by
//! the parameter's caption, as `instances_without_body` reads it) shows or
//! hides, `unresolved` those bound to a parameter whose caption or value was
//! not read, `bound` those bound at all and `resolved` those of them whose
//! value was read; `voids` counts the forms with no material that are voids
//! and `nested` the nested family instances. And one line per model, the
//! counts the held-out scrub keeps:
//!   bound visible values: release R unset forms U bound B resolved C unresolved D
//! summed over the doors and windows whose type map holds a -1.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_unnamed_bound_before_2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::native_family_forms::family_records_in;

const CLASSES: [&str; 2] = ["IFCWINDOW", "IFCDOOR"];

/// The form classes `native_family_forms` reads, the nested instances and the
/// family parameters whose captions name the parameters a form is bound to.
const RECORD_CLASSES: [&str; 9] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
    "Text3dElem",
    "FamilyInstance",
    "ParamElemFamily",
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

/// A graph's `FamilyParams` values: parameter ElementId and `m_int`.
fn family_params(graph: &rvt::research::graph_value::ObjectGraph) -> Vec<(u64, i64)> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "FamilyParams")
        .filter_map(|object| object.fields.get("m_params")?.as_array())
        .flatten()
        .filter_map(|entry| {
            let id = u64::try_from(id_in_either(entry.get("m_paramId"))?).ok()?;
            Some((id, entry.get("m_int")?.as_i64()?))
        })
        .collect()
}

/// A form of a family with no material that is not a void: whether its stored
/// `Visible` shows it, and the family parameter its `Visible` is bound to
/// (`Some(None)` where it is bound and the parameter was not read).
struct UnsetForm {
    stored_shown: bool,
    bound: Option<Option<u64>>,
}

/// What one family's document holds.
#[derive(Default)]
struct Facts {
    unset: Vec<UnsetForm>,
    voids: usize,
    nested: usize,
    captions: BTreeMap<u64, String>,
}

/// An element's own parameters: its drawn symbol, whether it is grouped, and
/// the parameter values its instance sets.
struct Instance {
    symbol: Option<u64>,
    grouped: bool,
    params: Vec<(u64, i64)>,
}

/// A type's map: whether it holds a -1, its family and its parameter values.
struct Type {
    unset: bool,
    family: Option<u64>,
    params: Vec<(u64, i64)>,
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

    // Each instance's drawn symbol (Revit's type, RE-477), whether it is a
    // member of a model group, and the parameter values it sets.
    let mut drawn: BTreeMap<u64, Instance> = BTreeMap::new();
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
            drawn.insert(
                record.identity.element_id,
                Instance {
                    symbol,
                    grouped,
                    params: family_params(graph),
                },
            );
            Ok(())
        },
    )?;

    // Each drawn symbol's family, parameter values, and whether its map holds
    // a -1.
    let symbols: BTreeSet<u64> = drawn.values().filter_map(|i| i.symbol).collect();
    let mut types: BTreeMap<u64, Type> = BTreeMap::new();
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
            let Some(root) = graph.objects.first() else {
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
            types.insert(
                record.identity.element_id,
                Type {
                    unset,
                    family,
                    params: family_params(graph),
                },
            );
            Ok(())
        },
    )?;

    // The captions of the parameters instances and types set.
    let parameters: BTreeSet<u64> = drawn
        .values()
        .flat_map(|i| i.params.iter().map(|(id, _)| *id))
        .chain(
            types
                .values()
                .flat_map(|t| t.params.iter().map(|(id, _)| *id)),
        )
        .collect();
    let mut project_captions: BTreeMap<u64, String> = BTreeMap::new();
    if !parameters.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: parameters,
                releases: NATIVE_RELEASES,
                ..Options::default()
            },
            |record| {
                let caption = record.graph.as_ref().and_then(|graph| {
                    graph
                        .objects
                        .iter()
                        .find_map(|object| object.fields.get("m_caption")?.as_str())
                        .map(str::to_string)
                });
                if let Some(caption) = caption {
                    project_captions.insert(record.identity.element_id, caption);
                }
                Ok(())
            },
        )?;
    }

    // The families' own documents: their forms' facts and parameter captions.
    let families: BTreeSet<u64> = types.values().filter_map(|t| t.family).collect();
    let classes: BTreeSet<String> = RECORD_CLASSES.iter().map(ToString::to_string).collect();
    let documents =
        family_records_in(&mut rf, &families, &classes, &NATIVE_RELEASES, None).unwrap_or_default();
    let mut facts: BTreeMap<u64, Facts> = BTreeMap::new();
    for (family, records) in &documents {
        let here = facts.entry(*family).or_default();
        for (id, graph) in records {
            let Some(root) = graph.objects.first() else {
                continue;
            };
            match root.class_name.as_str() {
                "FamilyInstance" => {
                    here.nested += 1;
                    continue;
                }
                "ParamElemFamily" => {
                    let caption = graph
                        .objects
                        .iter()
                        .find_map(|object| object.fields.get("m_caption")?.as_str());
                    if let Some(caption) = caption {
                        here.captions.insert(*id, caption.to_string());
                    }
                    continue;
                }
                _ => {}
            }
            if id_in_either(root.fields.get("m_materialId")) != Some(-1) {
                continue;
            }
            let cutting = root
                .fields
                .get("m_cutting")
                .and_then(Fv::as_bool)
                .unwrap_or(false);
            if cutting {
                here.voids += 1;
                continue;
            }
            let stored = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "ParamValueSetInt")
                .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
                .flatten()
                .find(|entry| id_in_either(entry.get("m_paramId")) == Some(VISIBLE_PARAMETER))
                .and_then(|entry| entry.get("m_value")?.as_i64());
            let bound = graph
                .objects
                .iter()
                .filter(|object| object.class_name == "FamilyParametrizedElemParamsCell")
                .filter_map(|object| object.fields.get("m_paramDrivenData")?.as_array())
                .flatten()
                .find(|entry| id_in_either(entry.get("m_elemPropId")) == Some(VISIBLE_PARAMETER))
                .map(|entry| {
                    id_in_either(entry.get("m_famParamId")).and_then(|id| u64::try_from(id).ok())
                });
            here.unset.push(UnsetForm {
                stored_shown: stored != Some(0),
                bound,
            });
        }
    }

    let (mut unset_forms, mut bound_forms, mut resolved_forms, mut unresolved_forms) =
        (0usize, 0usize, 0usize, 0usize);
    for (tag, class) in &elements {
        let instance = drawn.get(tag);
        let symbol = instance.and_then(|i| i.symbol);
        let kind = symbol.and_then(|id| types.get(&id));
        let map_unset = match kind {
            Some(t) if t.unset => "1",
            Some(_) => "0",
            None => "na",
        };
        let family = kind.and_then(|t| t.family);
        let found = family.and_then(|id| facts.get(&id));
        // The instance's value of a parameter, else its type's, by caption.
        let mut by_caption: BTreeMap<&str, i64> = BTreeMap::new();
        for (parameter, value) in kind
            .map(|t| t.params.as_slice())
            .unwrap_or_default()
            .iter()
            .chain(instance.map(|i| i.params.as_slice()).unwrap_or_default())
        {
            if let Some(caption) = project_captions.get(parameter) {
                by_caption.insert(caption.as_str(), *value);
            }
        }
        let (mut shown, mut hidden, mut unresolved, mut bound, mut resolved) =
            (0usize, 0usize, 0usize, 0usize, 0usize);
        if let Some(here) = found {
            for form in &here.unset {
                let effective = match form.bound {
                    None => Some(form.stored_shown),
                    Some(parameter) => {
                        bound += 1;
                        parameter
                            .and_then(|id| here.captions.get(&id))
                            .and_then(|caption| by_caption.get(caption.as_str()))
                            .map(|value| *value != 0)
                    }
                };
                match effective {
                    Some(true) => shown += 1,
                    Some(false) => hidden += 1,
                    None => unresolved += 1,
                }
                if form.bound.is_some() && effective.is_some() {
                    resolved += 1;
                }
            }
        }
        if map_unset == "1" {
            unset_forms += shown + hidden + unresolved;
            bound_forms += bound;
            resolved_forms += resolved;
            unresolved_forms += unresolved;
        }
        println!(
            "wbound {tag} {class} grouped={} map_unset={map_unset} found={} eff_shown={shown} eff_hidden={hidden} unresolved={unresolved} bound={bound} resolved={resolved} voids={} nested={}",
            u8::from(instance.is_some_and(|i| i.grouped)),
            u8::from(found.is_some()),
            found.map_or(0, |here| here.voids),
            found.map_or(0, |here| here.nested),
        );
    }
    println!(
        "bound visible values: release {version} unset forms {unset_forms} bound {bound_forms} resolved {resolved_forms} unresolved {unresolved_forms}"
    );
    eprintln!(
        "{} doors and windows, {} symbol maps read, {} families with a document, {} parameter captions",
        elements.len(),
        types.len(),
        documents.len(),
        project_captions.len()
    );
    Ok(())
}
