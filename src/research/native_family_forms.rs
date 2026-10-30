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
use crate::{RevitFile, compression, native_document, native_parameters, native_segments};
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
}

/// `{"m_id":{"m_id64":n}}` as `n`.
fn element_id(value: Option<&serde_json::Value>) -> Option<i64> {
    value
        .and_then(|v| v.pointer("/m_id/m_id64"))
        .and_then(serde_json::Value::as_i64)
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
    for name in streams {
        let stored = rf.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        let inflated = rf.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            registry,
            256 * 1024 * 1024,
            |source, bytes| {
                let Some(family) = source.content_key.and_then(|key| blocks.get(&key)) else {
                    return Ok(());
                };
                if !CHANNELS.contains(&source.channel) {
                    return Ok(());
                }
                let Ok(records) = native_document::group_records(source, bytes, version) else {
                    return Ok(());
                };
                let document = documents.entry(*family).or_default();
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
                                .and_then(serde_json::Value::as_bool)
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
        if !families.contains(family) {
            continue;
        }
        for key in keys {
            if memchr::memmem::find(body, key).is_some() {
                out.entry(*key).or_insert(*family);
            }
        }
    }
    out
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
    if families.is_empty() {
        return Ok(out);
    }
    let version = rf.layout_release()?;
    anyhow::ensure!(
        native_document::Options::default()
            .releases
            .contains(&version),
        "family forms are not measured on Revit {version}"
    );
    let registry =
        crate::schema_registry::parse(&native_document::read_single(rf, "Formats/Latest")?)?;
    let mut streams: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
        .cloned()
        .collect();
    streams.sort();

    // Each wanted family's record outside every block, and every block key.
    let mut bodies: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut keys: BTreeSet<[u8; 16]> = BTreeSet::new();
    for name in &streams {
        let stored = rf.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        let inflated = rf.inflated_partition(name).ok();
        native_segments::walk_inflated(
            &prepared,
            inflated.as_deref(),
            &registry,
            256 * 1024 * 1024,
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
        return Ok(out);
    }
    let documents = read_documents(rf, &registry, version, &streams, &blocks)?;

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
