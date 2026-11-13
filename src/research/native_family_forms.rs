//! Whether a loaded family draws visible geometry with no material, from
//! the family's own document embedded in the project (RE-204, #355).
//!
//! A loaded family's document is a keyed block of the project's partitions,
//! named by the 16-byte key its `Family` record holds (RE-163). Its solid
//! forms (`ExtrusionElem`, `RevolutionElem`, `SweepElem`, `BlendElem`,
//! `SweptBlendElem`) hold their material in `m_materialId`, −1 where none
//! is fixed, whether they cut (`m_cutting`, a void) and their `Visible`
//! value (`IS_VISIBLE_PARAM`, −1006205) in their `ParamValueSetInt`.
//!
//! A type whose geometry-material map maps some geometry to −1 has
//! `<Unnamed>` in Revit's export only where its family has a visible solid
//! form with no material: on Snowdon Towers the "Washer_Dryer_Combo"'s four
//! such extrusions, and "TableWithUmbrella"'s two sweeps, are not visible,
//! and Revit writes no `<Unnamed>` for their 22 instances, while "Sink
//! Vanity-Round"'s one is visible and its 31 instances have it (RE-204).
//! A family with no such form whose −1 geometry may be a nested family's
//! says nothing: "Cash Register_Touch Screen_Generic"'s forms all have
//! materials, it nests an instance, and Revit writes `<Unnamed>`. One that
//! nests none shows none: RE1 Electrical's "284621-FirePanel", whose 93
//! free-form solids and model text all have a material.
//!
//! A material only hidden forms have is left out of the export too (RE-205),
//! including a nested family's (RE-206). A nested family's form whose
//! `Visible` is bound to one of its family parameters (the form's
//! `FamilyParametrizedElemParamsCell` names it) is shown by that parameter's
//! value on the nested instance, else on its type; a parameter is matched
//! across the two documents by its caption (`ParamDef*.m_caption`). On
//! Snowdon Towers "AC_Bandstand Canopy" nests a dome whose "Include Dome"
//! is 1 on its type and 0 on the one instance, and Revit leaves its
//! "Acrylic, Clear" out.

use crate::le_bytes;
use crate::native_parameters::ObjectGraph;
use crate::{RevitFile, native_document, native_parameters, native_segments};
use std::collections::{BTreeMap, BTreeSet};

/// The model geometry classes read: solid forms, free-form (imported or
/// direct) solids and model text.
const FORM_CLASSES: [&str; 7] = [
    "ExtrusionElem",
    "RevolutionElem",
    "SweepElem",
    "BlendElem",
    "SweptBlendElem",
    "FreeFormElement",
    "Text3dElem",
];

/// The other classes a family document is read for.
const OTHER_CLASSES: [&str; 4] = [
    "FamilyInstance",
    "FamilySymbol",
    "ParamElemFamily",
    "Family",
];

/// `IS_VISIBLE_PARAM`.
const VISIBLE_PARAMETER: i64 = -1_006_205;

/// The channels element records are on.
const CHANNELS: [u64; 3] = [101, 102, 103];

/// What a family's own document shows of its forms' materials.
#[derive(Clone, Debug, Default)]
pub struct FamilyForms {
    /// Whether it shows model geometry with no material: true where a
    /// visible solid form has none; false where every such form is hidden
    /// or void, or where it has none and nests no family instance and every
    /// form decodes; `None` otherwise.
    pub shows_unset: Option<bool>,
    /// The materials only its hidden forms, or its nested families' hidden
    /// forms, have: Revit's export leaves them out (RE-205: Snowdon Towers'
    /// "Refrig_Base_wDrawers" maps a hidden clearance extrusion to
    /// "Clearance").
    pub hidden_only: BTreeSet<u32>,
}

/// A form of a family document.
struct Form {
    material: Option<i64>,
    cutting: bool,
    /// Its stored `Visible`, where it holds one.
    visible: Option<i64>,
    /// The family parameter its `Visible` is bound to.
    bound: Option<u64>,
}

/// An instance of a nested family in a family document.
struct Nested {
    symbol: u64,
    visible: bool,
    /// Its parameter values: parameter ElementId and integer value.
    params: Vec<(u64, i64)>,
}

/// What one family document holds.
#[derive(Default)]
struct Document {
    forms: Vec<Form>,
    nested: Vec<Nested>,
    /// Each type's family and parameter values.
    symbols: BTreeMap<u64, (u64, Vec<(u64, i64)>)>,
    /// Each family parameter's caption.
    captions: BTreeMap<u64, String>,
    /// The `Family` records it holds, with their bodies.
    families: Vec<(u64, Vec<u8>)>,
    unread: bool,
    /// A group of its blocks did not parse, so records of it are missing
    /// (RE-312). Kept apart from `unread`, which the material readers use.
    partial: bool,
}

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: Option<&crate::graph_value::Fv>) -> Option<i64> {
    value
        .and_then(|v| v.pointer("/m_id/m_id64"))
        .and_then(crate::graph_value::Fv::as_i64)
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
            let id = u64::try_from(element_id(entry.get("m_paramId"))?).ok()?;
            Some((id, entry.get("m_int")?.as_i64()?))
        })
        .collect()
}

/// A graph's stored `Visible`.
fn stored_visible(graph: &ObjectGraph) -> Option<i64> {
    graph
        .objects
        .iter()
        .filter(|object| object.class_name == "ParamValueSetInt")
        .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
        .flatten()
        .find(|entry| element_id(entry.get("m_paramId")) == Some(VISIBLE_PARAMETER))
        .and_then(|entry| entry.get("m_value")?.as_i64())
}

/// Read every block of `blocks` into a [`Document`] of its family.
fn read_documents(
    rf: &mut RevitFile,
    registry: &crate::schema_registry::Registry,
    version: u32,
    streams: &[String],
    blocks: &BTreeMap<[u8; 16], u64>,
) -> anyhow::Result<BTreeMap<u64, Document>> {
    let tag = |name: &str| registry.named(name).map(|class| class.tag);
    let form_tags: BTreeSet<u16> = FORM_CLASSES.iter().filter_map(|name| tag(name)).collect();
    let other_tags: BTreeSet<u16> = OTHER_CLASSES.iter().filter_map(|name| tag(name)).collect();
    let instance_tag = tag("FamilyInstance");
    let mut documents: BTreeMap<u64, Document> = BTreeMap::new();
    let budget = native_document::group_budget(rf, native_document::DEFAULT_MAX_GROUP_BYTES);
    for name in streams {
        let prepared = rf.read_prepared_stream(name)?;
        let inflated = rf.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            registry,
            native_document::partition_budget(
                budget,
                native_document::DEFAULT_MAX_GROUP_BYTES,
                inflated.as_deref(),
            ),
            |source, bytes| {
                let Some(family) = source.content_key.and_then(|key| blocks.get(&key)) else {
                    return Ok(());
                };
                if !CHANNELS.contains(&source.channel) {
                    return Ok(());
                }
                let document = documents.entry(*family).or_default();
                let Ok(records) = native_document::group_records(source, bytes, version) else {
                    document.partial = true;
                    return Ok(());
                };
                for (id, _, start, end) in records {
                    let body = &bytes[start..end];
                    let Some(class) = le_bytes::u16_at(body, 0) else {
                        continue;
                    };
                    let is_form = form_tags.contains(&class);
                    if !is_form && !other_tags.contains(&class) {
                        continue;
                    }
                    let Ok(graph) = native_parameters::decode_graph(body, registry) else {
                        if is_form || Some(class) == instance_tag {
                            document.unread = true;
                        }
                        continue;
                    };
                    let Some(root) = graph.objects.first() else {
                        continue;
                    };
                    if is_form {
                        let bound = graph
                            .objects
                            .iter()
                            .filter(|object| {
                                object.class_name == "FamilyParametrizedElemParamsCell"
                            })
                            .filter_map(|object| object.fields.get("m_paramDrivenData")?.as_array())
                            .flatten()
                            .find(|entry| {
                                element_id(entry.get("m_elemPropId")) == Some(VISIBLE_PARAMETER)
                            })
                            .and_then(|entry| element_id(entry.get("m_famParamId")))
                            .and_then(|id| u64::try_from(id).ok());
                        document.forms.push(Form {
                            material: element_id(root.fields.get("m_materialId")),
                            cutting: root
                                .fields
                                .get("m_cutting")
                                .and_then(crate::graph_value::Fv::as_bool)
                                .unwrap_or(false),
                            visible: stored_visible(&graph),
                            bound,
                        });
                        continue;
                    }
                    match root.class_name.as_str() {
                        "FamilyInstance" => {
                            let Some(symbol) = element_id(root.fields.get("m_masterSymbolId"))
                                .and_then(|id| u64::try_from(id).ok())
                            else {
                                document.unread = true;
                                continue;
                            };
                            document.nested.push(Nested {
                                symbol,
                                visible: stored_visible(&graph) != Some(0),
                                params: family_params(&graph),
                            });
                        }
                        "FamilySymbol" => {
                            if let Some(family) = element_id(root.fields.get("m_familyId"))
                                .and_then(|id| u64::try_from(id).ok())
                            {
                                document.symbols.insert(id, (family, family_params(&graph)));
                            }
                        }
                        "ParamElemFamily" => {
                            if let Some(caption) = graph
                                .objects
                                .iter()
                                .find_map(|object| object.fields.get("m_caption")?.as_str())
                            {
                                document.captions.insert(id, caption.to_string());
                            }
                        }
                        "Family" => document.families.push((id, body.to_vec())),
                        _ => {}
                    }
                }
                Ok(())
            },
        )?;
    }
    Ok(documents)
}

/// The block each of `families` names, by key.
fn blocks_of(
    bodies: &[(u64, Vec<u8>)],
    families: &BTreeSet<u64>,
    keys: &BTreeSet<[u8; 16]>,
) -> BTreeMap<[u8; 16], u64> {
    let mut out = BTreeMap::new();
    for (family, body) in bodies {
        // Every key has its block: no later body changes `out`.
        if out.len() == keys.len() {
            break;
        }
        if !families.contains(family) {
            continue;
        }
        if keys.len() <= KEY_SEARCHES {
            for key in keys {
                if !out.contains_key(key) && memchr::memmem::find(body, key).is_some() {
                    out.insert(*key, *family);
                }
            }
            continue;
        }
        // A key is in a body exactly where one of its 16-byte windows is
        // the key, so one pass over the body answers every key: a search
        // for each key cost bodies × keys (#1833).
        for window in body.windows(16) {
            let Ok(key) = <[u8; 16]>::try_from(window) else {
                continue;
            };
            if keys.contains(&key) {
                out.entry(key).or_insert(*family);
            }
        }
    }
    out
}

/// Up to this many keys, a search of each body for each key, which `memchr`
/// vectorises, is cheaper than one pass with a set lookup at every byte.
const KEY_SEARCHES: usize = 16;

/// What [`load`] reads: the schema, the release, the partitions, the block
/// keys and the family documents.
struct Loaded {
    registry: crate::schema_registry::Registry,
    version: u32,
    streams: Vec<String>,
    keys: BTreeSet<[u8; 16]>,
    blocks: BTreeMap<[u8; 16], u64>,
    documents: BTreeMap<u64, Document>,
}

/// The documents of `families`, by ElementId, with what was read to find
/// them. `None` where there are no families or none has a document.
fn load(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
    releases: &std::ops::RangeInclusive<u32>,
) -> anyhow::Result<Option<Loaded>> {
    if families.is_empty() {
        return Ok(None);
    }
    let version = rf.layout_release()?;
    anyhow::ensure!(
        releases.contains(&version),
        "family forms are not measured on Revit {version}"
    );
    let registry =
        crate::schema_registry::parse(&native_document::read_single(rf, "Formats/Latest")?)?;
    // In partition-number order, the oldest copy of a record first: a reader
    // that keeps the last copy of a record gets the newest (#1356).
    let streams = rf.partition_stream_names();

    // Each wanted family's record outside every block, and every block key.
    let mut bodies: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut keys: BTreeSet<[u8; 16]> = BTreeSet::new();
    let budget = native_document::group_budget(rf, native_document::DEFAULT_MAX_GROUP_BYTES);
    for name in &streams {
        let prepared = rf.read_prepared_stream(name)?;
        let inflated = rf.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            &registry,
            native_document::partition_budget(
                budget,
                native_document::DEFAULT_MAX_GROUP_BYTES,
                inflated.as_deref(),
            ),
            |source, bytes| {
                if let Some(key) = source.content_key {
                    keys.insert(key);
                    return Ok(());
                }
                if !CHANNELS.contains(&source.channel) {
                    return Ok(());
                }
                let Ok(records) = native_document::group_records(source, bytes, version) else {
                    return Ok(());
                };
                for (id, _, start, end) in records {
                    if families.contains(&id) {
                        bodies.push((id, bytes[start..end].to_vec()));
                    }
                }
                Ok(())
            },
        )?;
    }
    let blocks = blocks_of(&bodies, families, &keys);
    if blocks.is_empty() {
        return Ok(None);
    }
    let documents = read_documents(rf, &registry, version, &streams, &blocks)?;
    Ok(Some(Loaded {
        registry,
        version,
        streams,
        keys,
        blocks,
        documents,
    }))
}

/// [`FamilyForms`] of each of `families`, by the family's ElementId. A family
/// whose document is not found, or holds no form or nested instance that
/// decodes, is left out. Errs where the file's partitions or schema are not
/// read.
pub fn family_forms(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, FamilyForms>> {
    let mut out = BTreeMap::new();
    let Some(Loaded {
        registry,
        version,
        streams,
        keys,
        documents,
        ..
    }) = load(rf, families, &native_document::Options::default().releases)?
    else {
        return Ok(out);
    };

    // The documents of the families nested in them.
    let mut nested_families: BTreeSet<u64> = BTreeSet::new();
    let mut nested_bodies: Vec<(u64, Vec<u8>)> = Vec::new();
    for document in documents.values() {
        for instance in &document.nested {
            if let Some((family, _)) = document.symbols.get(&instance.symbol) {
                nested_families.insert(*family);
            }
        }
        nested_bodies.extend(document.families.iter().cloned());
    }
    let nested_blocks = blocks_of(&nested_bodies, &nested_families, &keys);
    let nested_documents = if nested_blocks.is_empty() {
        BTreeMap::new()
    } else {
        read_documents(rf, &registry, version, &streams, &nested_blocks)?
    };

    for (family, document) in &documents {
        if document.forms.is_empty() && document.nested.is_empty() {
            continue;
        }
        let (mut visible, mut hidden) = (BTreeSet::new(), BTreeSet::new());
        let (mut unset, mut visible_unset) = (false, false);
        for form in document.forms.iter().filter(|form| !form.cutting) {
            let shown = form.visible != Some(0);
            if form.material == Some(-1) {
                unset = true;
                visible_unset |= shown;
            }
            if let Some(material) = form.material.and_then(|id| u32::try_from(id).ok()) {
                if shown {
                    visible.insert(material);
                } else {
                    hidden.insert(material);
                }
            }
        }
        for instance in &document.nested {
            let Some((nested_family, type_params)) = document.symbols.get(&instance.symbol) else {
                continue;
            };
            let Some(nested) = nested_documents.get(nested_family) else {
                continue;
            };
            // A parameter's integer value on the instance, else its type,
            // by the caption the nested document gives it.
            let value_of = |caption: &str| {
                let find = |params: &[(u64, i64)]| {
                    params.iter().find_map(|(id, value)| {
                        (document.captions.get(id).map(String::as_str) == Some(caption))
                            .then_some(*value)
                    })
                };
                find(&instance.params).or_else(|| find(type_params))
            };
            for form in nested.forms.iter().filter(|form| !form.cutting) {
                let value = form
                    .bound
                    .and_then(|parameter| nested.captions.get(&parameter))
                    .and_then(|caption| value_of(caption))
                    .or(form.visible);
                let shown = instance.visible && value != Some(0);
                if let Some(material) = form.material.and_then(|id| u32::try_from(id).ok()) {
                    if shown {
                        visible.insert(material);
                    } else {
                        hidden.insert(material);
                    }
                }
            }
        }
        let shows_unset = if visible_unset {
            Some(true)
        } else if unset || (document.nested.is_empty() && !document.unread) {
            Some(false)
        } else {
            None
        };
        out.insert(
            *family,
            FamilyForms {
                shows_unset,
                hidden_only: hidden.difference(&visible).copied().collect(),
            },
        );
    }
    Ok(out)
}

/// A model form of a loaded family's document, and what shows it (RE-312).
#[derive(Clone, Debug)]
pub struct ModelForm {
    /// Whether it is a void, which draws nothing.
    pub cutting: bool,
    /// Its stored `Visible`, where it holds one.
    pub visible: Option<i64>,
    /// Whether its `Visible` is bound to a family parameter.
    pub bound: bool,
    /// The caption of that parameter in the family's own document, where it
    /// was read.
    pub bound_caption: Option<String>,
}

/// The model forms of a loaded family's document (RE-312).
#[derive(Clone, Debug, Default)]
pub struct FamilySolids {
    pub forms: Vec<ModelForm>,
    /// True where the document nests a family instance, or a form or an
    /// instance in it did not decode, or a group of it did not parse: the
    /// forms may not be all it draws.
    pub incomplete: bool,
}

/// [`FamilySolids`] of each of `families`, by the family's ElementId. A
/// family whose document is not found is left out. Errs where the file's
/// partitions or schema are not read.
pub fn family_solids(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, FamilySolids>> {
    let mut out = BTreeMap::new();
    let Some(loaded) = load(rf, families, &native_document::Options::default().releases)? else {
        return Ok(out);
    };
    for (family, document) in &loaded.documents {
        out.insert(
            *family,
            FamilySolids {
                forms: document
                    .forms
                    .iter()
                    .map(|form| ModelForm {
                        cutting: form.cutting,
                        visible: form.visible,
                        bound: form.bound.is_some(),
                        bound_caption: form
                            .bound
                            .and_then(|id| document.captions.get(&id).cloned()),
                    })
                    .collect(),
                incomplete: !document.nested.is_empty() || document.unread || document.partial,
            },
        );
    }
    Ok(out)
}

/// The records of loaded families' own documents whose class is one of
/// `classes` (every record when it is empty), each as its decoded graph, by
/// the family's ElementId (RE-318). A family whose document is not found is
/// left out, as is a record that does not decode. Errs where the file's
/// partitions or schema are not read.
pub fn family_records(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
    classes: &BTreeSet<String>,
) -> anyhow::Result<BTreeMap<u64, Vec<(u64, ObjectGraph)>>> {
    family_records_in(
        rf,
        families,
        classes,
        &native_document::Options::default().releases,
        None,
    )
}

/// [`family_records`] on any of `releases` (the caller's reader is measured
/// on them: [`native_document::NATIVE_RELEASES`] is every release the native
/// framing holds on, RE-280), and only the records of `ids` where it is
/// given. A non-empty `classes` none of which the file's schema names reads
/// no record (RE-327).
pub fn family_records_in(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
    classes: &BTreeSet<String>,
    releases: &std::ops::RangeInclusive<u32>,
    ids: Option<&BTreeSet<u64>>,
) -> anyhow::Result<BTreeMap<u64, Vec<(u64, ObjectGraph)>>> {
    Ok(family_records_at(rf, families, classes, releases, ids, false)?.0)
}

/// The records of a family's document by the family's ElementId.
pub type FamilyRecords = BTreeMap<u64, Vec<(u64, ObjectGraph)>>;

/// Where each record of a family's document starts, by the family's ElementId
/// and the record's: the partition and the offset in it inflated (#1368). Of a
/// record held in several partitions, the place of the copy in the highest
/// partition, the copy a `FormDocument` keeps; none where that copy's bytes
/// are not the partition's own.
pub type RecordPlaces = BTreeMap<(u64, u64), (String, usize)>;

/// [`family_records_in`], also giving, when `want_places` asks, where each
/// record starts (#1368). Nothing is kept for the places when it does not.
pub fn family_records_at(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
    classes: &BTreeSet<String>,
    releases: &std::ops::RangeInclusive<u32>,
    ids: Option<&BTreeSet<u64>>,
    want_places: bool,
) -> anyhow::Result<(FamilyRecords, RecordPlaces)> {
    let mut out: FamilyRecords = BTreeMap::new();
    let mut places = RecordPlaces::new();
    let Some(loaded) = load(rf, families, releases)? else {
        return Ok((out, places));
    };
    let tags: BTreeSet<u16> = classes
        .iter()
        .filter_map(|name| loaded.registry.named(name).map(|class| class.tag))
        .collect();
    if !classes.is_empty() && tags.is_empty() {
        return Ok((out, places));
    }
    let budget = native_document::group_budget(rf, native_document::DEFAULT_MAX_GROUP_BYTES);
    for name in &loaded.streams {
        let prepared = rf.read_prepared_stream(name)?;
        let inflated = rf.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            &loaded.registry,
            native_document::partition_budget(
                budget,
                native_document::DEFAULT_MAX_GROUP_BYTES,
                inflated.as_deref(),
            ),
            |source, bytes| {
                let Some(family) = source.content_key.and_then(|key| loaded.blocks.get(&key))
                else {
                    return Ok(());
                };
                if !CHANNELS.contains(&source.channel) {
                    return Ok(());
                }
                let Ok(records) = native_document::group_records(source, bytes, loaded.version)
                else {
                    return Ok(());
                };
                for (id, pos, start, end) in records {
                    let body = &bytes[start..end];
                    let Some(class) = le_bytes::u16_at(body, 0) else {
                        continue;
                    };
                    if !tags.is_empty() && !tags.contains(&class) {
                        continue;
                    }
                    if ids.is_some_and(|wanted| !wanted.contains(&id)) {
                        continue;
                    }
                    if let Ok(graph) = native_parameters::decode_graph(body, &loaded.registry) {
                        if want_places {
                            // The last copy's place, as the last copy's graph
                            // is the one a `FormDocument` keeps: a copy whose
                            // bytes are not the partition's own leaves no place.
                            let at = source
                                .inflated_offset
                                .and_then(|base| base.checked_add(pos));
                            match at {
                                Some(at) => {
                                    places.insert((*family, id), (name.clone(), at));
                                }
                                None => {
                                    places.remove(&(*family, id));
                                }
                            }
                        }
                        out.entry(*family).or_default().push((id, graph));
                    }
                }
                Ok(())
            },
        )?;
    }
    Ok((out, places))
}

/// Each selected record's graph, by ElementId.
fn read_graphs(
    rf: &mut RevitFile,
    ids: &BTreeSet<u64>,
    mut each: impl FnMut(u64, &ObjectGraph),
) -> anyhow::Result<()> {
    if ids.is_empty() {
        return Ok(());
    }
    let options = native_document::Options {
        selected_ids: ids.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        if let Some(graph) = record.graph.as_ref() {
            each(record.identity.element_id, graph);
        }
        Ok(())
    })?;
    Ok(())
}

/// A graph's root element as the id in `field`.
fn root_id(graph: &ObjectGraph, class: &str, field: &str) -> Option<u64> {
    let root = graph.objects.first()?;
    if root.class_name != class {
        return None;
    }
    u64::try_from(element_id(root.fields.get(field))?).ok()
}

/// The family types of `types` whose saved graphics draw nothing visible
/// (RE-363, #1496): at the Fine detail level Revit's exporter asks for, they
/// decode cleanly to no surface, with a branch left out as not visible. Revit's
/// exporter writes a family instance that is not a door or a window only when
/// its 3D body has a solid or a mesh (`FamilyInstanceExporter` in revit-ifc,
/// CLEANROOM.md source 5). A type with no saved graphics is not one.
pub fn types_without_visible_body(
    rf: &mut RevitFile,
    types: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    const GRAPHICS_CHANNEL: u64 = 103;
    const FINE_DETAIL: i64 = 3;
    // A symbol's graphics name the symbols nested in it, so reading goes on
    // until no new one is named. The bound stops a file whose symbols name
    // each other for ever.
    const MAX_ROUNDS: usize = 128;
    let mut options = native_document::Options {
        channels: BTreeSet::from([GRAPHICS_CHANNEL]),
        max_graph_values: 1_000_000,
        max_graph_objects: 1_000_000,
        releases: native_document::NATIVE_RELEASES,
        ..native_document::Options::default()
    };
    let mut graphs: BTreeMap<u64, ObjectGraph> = BTreeMap::new();
    let mut attempted: BTreeSet<u64> = BTreeSet::new();
    let mut pending = types.clone();
    for _ in 0..MAX_ROUNDS {
        if pending.is_empty() {
            break;
        }
        attempted.extend(&pending);
        options.selected_ids = std::mem::take(&mut pending);
        let mut fresh: Vec<(u64, ObjectGraph)> = Vec::new();
        native_document::extract_graphs(rf, &options, |record| {
            if let Some(graph) = record.graph {
                fresh.push((record.identity.element_id, graph));
            }
            Ok(())
        })?;
        for (id, graph) in fresh {
            for object in &graph.objects {
                if object.class_name != "InstanceInfo" {
                    continue;
                }
                let symbol = element_id(object.fields.get("m_symbolId"))
                    .and_then(|symbol| u64::try_from(symbol).ok())
                    .filter(|symbol| *symbol > 0 && !attempted.contains(symbol));
                pending.extend(symbol);
            }
            graphs.insert(id, graph);
        }
    }
    let resolver = |id: u64| graphs.get(&id);
    Ok(types
        .iter()
        .copied()
        .filter(|id| {
            graphs.get(id).is_some_and(|graph| {
                let drawn = crate::native_saved_mesh::graphics_with_resolver_at_detail(
                    graph,
                    &resolver,
                    FINE_DETAIL,
                );
                drawn.primitives.is_empty()
                    && drawn.diagnostics.is_empty()
                    && drawn.excluded_visibility_branches > 0
            })
        })
        .collect())
}

/// The family instances of `instances` that have no 3D body (RE-312, #1262).
///
/// Revit's exporter writes a family instance that is not a door or a window
/// only when it has a 3D body (`FamilyInstanceExporter.
/// ExportFamilyInstanceElement` in revit-ifc, CLEANROOM.md source 5). An
/// instance has none when no model form of its family's document is drawn
/// for its parameter values: a form is drawn unless it is a void or its
/// `Visible` is 0, where `Visible` is the value of the family parameter it
/// is bound to (the instance's, else its type's, matched to the family's by
/// caption) or else its stored value. On the BIM4LCA terrain office the 92
/// bicycle bays of "Parking Space" carry "Stripe Y/N" as 0 and are not in
/// Revit's export, and its 2 car bays do not.
///
/// An instance is left out of the result, so is not called bodiless, where
/// its family's document is not found, holds no form, nests a family
/// instance, has a form or instance that does not decode or a group that
/// does not parse, or has a form bound to a parameter whose caption or value
/// was not read: the stored `Visible` of a bound form says nothing of what
/// the parameter shows.
pub fn instances_without_body(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    let mut symbol_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut instance_params: BTreeMap<u64, Vec<(u64, i64)>> = BTreeMap::new();
    read_graphs(rf, instances, |id, graph| {
        if let Some(symbol) = root_id(graph, "FamilyInstance", "m_masterSymbolId") {
            symbol_of.insert(id, symbol);
            instance_params.insert(id, family_params(graph));
        }
    })?;

    // The types, and the captions of the parameters the instances set.
    let symbols: BTreeSet<u64> = symbol_of.values().copied().collect();
    let mut selected = symbols.clone();
    selected.extend(instance_params.values().flatten().map(|(id, _)| *id));
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut symbol_params: BTreeMap<u64, Vec<(u64, i64)>> = BTreeMap::new();
    let mut captions: BTreeMap<u64, String> = BTreeMap::new();
    let caption_of = |graph: &ObjectGraph| {
        graph
            .objects
            .iter()
            .find_map(|object| object.fields.get("m_caption")?.as_str())
            .map(str::to_string)
    };
    read_graphs(rf, &selected, |id, graph| {
        if symbols.contains(&id) {
            if let Some(family) = root_id(graph, "FamilySymbol", "m_familyId") {
                family_of.insert(id, family);
                symbol_params.insert(id, family_params(graph));
            }
        } else if let Some(caption) = caption_of(graph) {
            captions.insert(id, caption);
        }
    })?;
    let unread: BTreeSet<u64> = symbol_params
        .values()
        .flatten()
        .map(|(id, _)| *id)
        .filter(|id| !selected.contains(id))
        .collect();
    read_graphs(rf, &unread, |id, graph| {
        if let Some(caption) = caption_of(graph) {
            captions.insert(id, caption);
        }
    })?;

    let families: BTreeSet<u64> = family_of.values().copied().collect();
    let solids = family_solids(rf, &families)?;
    let mut out = BTreeSet::new();
    for (instance, symbol) in &symbol_of {
        let Some(solid) = family_of.get(symbol).and_then(|family| solids.get(family)) else {
            continue;
        };
        if solid.incomplete || solid.forms.is_empty() {
            continue;
        }
        // The instance's value of a parameter, else its type's.
        let mut by_caption: BTreeMap<&str, i64> = BTreeMap::new();
        let own = instance_params.get(instance);
        let of_type = symbol_params.get(symbol);
        for (parameter, value) in of_type.into_iter().chain(own).flatten() {
            if let Some(caption) = captions.get(parameter) {
                by_caption.insert(caption.as_str(), *value);
            }
        }
        let (mut drawn, mut unresolved) = (false, false);
        for form in solid.forms.iter().filter(|form| !form.cutting) {
            let shown = if form.bound {
                form.bound_caption
                    .as_deref()
                    .and_then(|caption| by_caption.get(caption).copied())
                    .map(|value| value != 0)
            } else {
                Some(form.visible != Some(0))
            };
            match shown {
                Some(true) => drawn = true,
                Some(false) => {}
                None => unresolved = true,
            }
        }
        if !drawn && !unresolved {
            out.insert(*instance);
        }
    }
    Ok(out)
}
