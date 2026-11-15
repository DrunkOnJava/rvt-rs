//! #1603, RE-485: is `<Unnamed>` in a Revit 2015 to 2023 door or window's
//! material set constant within a drawn symbol (Revit's type) and within a
//! family, and do the nested family instances' own `Visible` values split the
//! windows that the host family's forms leave together?
//!
//! FACT to check: RE-483 left 15 of the spent Revit 2018 project's 68 windows
//! with identical facts of the host family's forms, `<Unnamed>` written on 8
//! and not on 7. Whether the cause is the type's or the instance's, or a
//! nested instance a parameter shows or hides, is not in those counts.
//!
//! Output, for each door and window the export writes (an IFC class and an
//! ElementId Tag), in the wording `tools/re/window_door_unnamed_by_type_vs_ifc.py`
//! reads:
//!   wtype <tag> <IFC class> grouped=<0|1> map_unset=<0|1|na> found=<0|1> \
//!        eff_shown=<n> eff_hidden=<n> unresolved=<n> bound=<n> resolved=<n> \
//!        ambiguous=<n> voids=<n> nested=<n> nested_hidden=<n> nested_bound=<n> \
//!        symbol=<id> family=<id> tags=<n> unset_tags=<n>
//! where the first twelve counts are RE-483's (the family's forms with no
//! material by their effective visibility, a `Visible` bound to a family
//! parameter being read from the parameter's value on the instance, else on its
//! type; `voids`; `nested` instances); `nested_hidden` counts the nested family
//! instances whose stored `Visible` is 0 and `nested_bound` those whose
//! `Visible` is bound to a family parameter; `symbol` and `family` are the
//! ElementIds of the drawn symbol and its family (0 where not read); `tags` is
//! the number of geometry tags the symbol's `m_geomTag2MaterialId` holds and
//! `unset_tags` how many of them map to no material (-1).
//!
//! Each record of a family's document is read once, from its last copy; two
//! more lines per model count what the probe saw, in the wording the held-out
//! scrub keeps:
//!   bound visible values: release R unset forms U bound B resolved C unresolved D ambiguous E
//!   family record copies: release R records N held more than once C copies differing D
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
use rvt::research::native_parameters::ObjectGraph;

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
fn family_params(graph: &ObjectGraph) -> Vec<(u64, i64)> {
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
    nested_hidden: usize,
    nested_bound: usize,
    captions: BTreeMap<u64, String>,
}

impl Facts {
    /// Whether two family parameters share `caption`.
    fn ambiguous(&self, caption: &str) -> bool {
        self.captions
            .values()
            .filter(|c| c.as_str() == caption)
            .count()
            > 1
    }
}

/// An element's own parameters: its drawn symbol, whether it is grouped, and
/// the parameter values its instance sets.
struct Instance {
    symbol: Option<u64>,
    grouped: bool,
    params: Vec<(u64, i64)>,
}

/// A type's map: whether it holds a -1, its family, its parameter values and
/// the number of geometry tags it maps and how many map to no material.
struct Type {
    unset: bool,
    family: Option<u64>,
    params: Vec<(u64, i64)>,
    tags: usize,
    unset_tags: usize,
}

/// A graph's stored `Visible`.
fn stored_visible(graph: &ObjectGraph) -> Option<i64> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "ParamValueSetInt")
        .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
        .flatten()
        .find(|entry| id_in_either(entry.get("m_paramId")) == Some(VISIBLE_PARAMETER))
        .and_then(|entry| entry.get("m_value")?.as_i64())
}

/// The family parameter a graph's `Visible` is bound to: `None` where it is
/// not bound, `Some(None)` where it is bound and the parameter was not read.
fn bound_visible(graph: &ObjectGraph) -> Option<Option<u64>> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "FamilyParametrizedElemParamsCell")
        .filter_map(|object| object.fields.get("m_paramDrivenData")?.as_array())
        .flatten()
        .find(|entry| id_in_either(entry.get("m_elemPropId")) == Some(VISIBLE_PARAMETER))
        .map(|entry| id_in_either(entry.get("m_famParamId")).and_then(|id| u64::try_from(id).ok()))
}

/// What the probe reads of one record of a family's document; two copies of a
/// record are equal where they agree on all of it.
#[derive(PartialEq)]
enum Record {
    Nested {
        stored: Option<i64>,
        bound: bool,
    },
    Param(Option<String>),
    Form {
        material: Option<i64>,
        cutting: bool,
        stored: Option<i64>,
        bound: Option<Option<u64>>,
    },
}

fn read_record(graph: &ObjectGraph) -> Option<Record> {
    let root = graph.objects.first()?;
    if root.class_name == "FamilyInstance" {
        let stored = stored_visible(graph);
        let bound = bound_visible(graph).is_some();
        return Some(Record::Nested { stored, bound });
    }
    if root.class_name == "ParamElemFamily" {
        let caption = graph
            .objects
            .iter()
            .find_map(|object| object.fields.get("m_caption")?.as_str());
        return Some(Record::Param(caption.map(str::to_string)));
    }
    let (stored, bound) = (stored_visible(graph), bound_visible(graph));
    Some(Record::Form {
        material: id_in_either(root.fields.get("m_materialId")),
        cutting: root
            .fields
            .get("m_cutting")
            .and_then(Fv::as_bool)
            .unwrap_or(false),
        stored,
        bound,
    })
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
            let unset_tags = entries
                .iter()
                .filter(|entry| id_in_either(entry.pointer("/second")) == Some(-1))
                .count();
            let family = id_in_either(root.fields.get("m_familyId"))
                .and_then(|id| u64::try_from(id).ok())
                .filter(|id| *id != 0);
            types.insert(
                record.identity.element_id,
                Type {
                    unset: unset_tags > 0,
                    family,
                    params: family_params(graph),
                    tags: entries.len(),
                    unset_tags,
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
    // A record held in several partitions is read once, from its last copy (the
    // newest, as `FormDocument` keeps it); how many records are held more than
    // once, and in how many the copies differ in what is read, is counted.
    let (mut records_read, mut copied, mut differing) = (0usize, 0usize, 0usize);
    for (family, records) in &documents {
        let here = facts.entry(*family).or_default();
        let mut latest: BTreeMap<u64, Record> = BTreeMap::new();
        let mut copies: BTreeMap<u64, bool> = BTreeMap::new();
        for (id, graph) in records {
            let Some(record) = read_record(graph) else {
                continue;
            };
            if let Some(previous) = latest.get(id) {
                *copies.entry(*id).or_insert(false) |= *previous != record;
            }
            latest.insert(*id, record);
        }
        records_read += latest.len();
        copied += copies.len();
        differing += copies.values().filter(|differs| **differs).count();
        for (id, record) in latest {
            match record {
                Record::Nested { stored, bound } => {
                    here.nested += 1;
                    here.nested_hidden += usize::from(stored == Some(0));
                    here.nested_bound += usize::from(bound);
                }
                Record::Param(Some(caption)) => {
                    here.captions.insert(id, caption);
                }
                Record::Param(None) => {}
                Record::Form {
                    material,
                    cutting,
                    stored,
                    bound,
                } => {
                    if material != Some(-1) {
                        continue;
                    }
                    if cutting {
                        here.voids += 1;
                        continue;
                    }
                    here.unset.push(UnsetForm {
                        stored_shown: stored != Some(0),
                        bound,
                    });
                }
            }
        }
    }

    let (mut unset_forms, mut bound_forms, mut resolved_forms) = (0usize, 0usize, 0usize);
    let (mut unresolved_forms, mut ambiguous_forms) = (0usize, 0usize);
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
        // Two project parameters with the same caption are ambiguous: neither
        // value is taken.
        let mut by_caption: BTreeMap<&str, (u64, i64)> = BTreeMap::new();
        let mut shared: BTreeSet<&str> = BTreeSet::new();
        for (parameter, value) in kind
            .map(|t| t.params.as_slice())
            .unwrap_or_default()
            .iter()
            .chain(instance.map(|i| i.params.as_slice()).unwrap_or_default())
        {
            if let Some(caption) = project_captions.get(parameter) {
                match by_caption.insert(caption.as_str(), (*parameter, *value)) {
                    Some((other, _)) if other != *parameter => {
                        shared.insert(caption.as_str());
                    }
                    _ => {}
                }
            }
        }
        let (mut shown, mut hidden, mut unresolved, mut bound, mut resolved, mut ambiguous) =
            (0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
        if let Some(here) = found {
            for form in &here.unset {
                let effective = match form.bound {
                    None => Some(form.stored_shown),
                    Some(parameter) => {
                        bound += 1;
                        let caption = parameter.and_then(|id| here.captions.get(&id));
                        if caption.is_some_and(|c| here.ambiguous(c) || shared.contains(c.as_str()))
                        {
                            ambiguous += 1;
                            None
                        } else {
                            caption
                                .and_then(|caption| by_caption.get(caption.as_str()))
                                .map(|(_, value)| *value != 0)
                        }
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
            ambiguous_forms += ambiguous;
        }
        println!(
            "wtype {tag} {class} grouped={} map_unset={map_unset} found={} eff_shown={shown} eff_hidden={hidden} unresolved={unresolved} bound={bound} resolved={resolved} ambiguous={ambiguous} voids={} nested={} nested_hidden={} nested_bound={} symbol={} family={} tags={} unset_tags={}",
            u8::from(instance.is_some_and(|i| i.grouped)),
            u8::from(found.is_some()),
            found.map_or(0, |here| here.voids),
            found.map_or(0, |here| here.nested),
            found.map_or(0, |here| here.nested_hidden),
            found.map_or(0, |here| here.nested_bound),
            symbol.unwrap_or(0),
            family.unwrap_or(0),
            kind.map_or(0, |t| t.tags),
            kind.map_or(0, |t| t.unset_tags),
        );
    }
    println!(
        "bound visible values: release {version} unset forms {unset_forms} bound {bound_forms} resolved {resolved_forms} unresolved {unresolved_forms} ambiguous {ambiguous_forms}"
    );
    println!(
        "family record copies: release {version} records {records_read} held more than once {copied} copies differing {differing}"
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
