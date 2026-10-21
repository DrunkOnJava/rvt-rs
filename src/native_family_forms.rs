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

/// `IS_VISIBLE_PARAM`.
const VISIBLE_PARAMETER: i64 = -1_006_205;

/// The channels element records are on.
const CHANNELS: [u64; 3] = [101, 102, 103];

/// Whether each of `families` shows model geometry with no material, by
/// the family's ElementId: true where a visible solid form has none; false
/// where every such form is hidden or void, or where it has none and nests
/// no family instance and every form decodes. A family whose document is
/// not found, or that has no such form but nests an instance or holds a
/// form that does not decode, is left out. Errs where the file's partitions or
/// schema are not read.
pub fn visible_unset_forms(
    rf: &mut RevitFile,
    families: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, bool>> {
    let mut out = BTreeMap::new();
    if families.is_empty() {
        return Ok(out);
    }
    let version = rf.basic_file_info()?.version;
    let registry =
        crate::schema_registry::parse(&native_document::read_single(rf, "Formats/Latest")?)?;
    let mut names: Vec<String> = rf
        .stream_names()
        .iter()
        .filter(|name| name.starts_with("Partitions/"))
        .cloned()
        .collect();
    names.sort();

    // Each wanted family's record outside every block, and every block key.
    let mut bodies: Vec<(u64, Vec<u8>)> = Vec::new();
    let mut keys: BTreeSet<[u8; 16]> = BTreeSet::new();
    for name in &names {
        let stored = rf.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
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
        })?;
    }
    let mut family_of: BTreeMap<[u8; 16], u64> = BTreeMap::new();
    for (family, body) in &bodies {
        for key in &keys {
            if memchr::memmem::find(body, key).is_some() {
                family_of.entry(*key).or_insert(*family);
            }
        }
    }
    if family_of.is_empty() {
        return Ok(out);
    }

    // The forms of those blocks, and whether a block nests another family's
    // instance or holds a record that does not decode: either can hold
    // geometry with no material that is not read here.
    let tags: BTreeSet<u16> = FORM_CLASSES
        .iter()
        .filter_map(|name| registry.named(name).map(|class| class.tag))
        .collect();
    let instance_tag = registry.named("FamilyInstance").map(|class| class.tag);
    #[derive(Default)]
    struct Seen {
        visible_unset: bool,
        unset: bool,
        unread: bool,
    }
    let mut seen: BTreeMap<u64, Seen> = BTreeMap::new();
    for name in &names {
        let stored = rf.read_stream(name)?;
        let prepared = compression::prepare_stream_for_inflate(name, &stored);
        native_segments::walk(&prepared, &registry, 256 * 1024 * 1024, |source, bytes| {
            let Some(family) = source.content_key.and_then(|key| family_of.get(&key)) else {
                return Ok(());
            };
            if !CHANNELS.contains(&source.channel) {
                return Ok(());
            }
            let Ok(records) = native_document::group_records(source, bytes, version) else {
                return Ok(());
            };
            let state = seen.entry(*family).or_default();
            for (_, _, start, end) in records {
                let body = &bytes[start..end];
                let Some(tag) = body.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]])) else {
                    continue;
                };
                if Some(tag) == instance_tag {
                    state.unread = true;
                }
                if !tags.contains(&tag) {
                    continue;
                }
                let Ok(graph) = native_parameters::decode_graph(body, &registry) else {
                    state.unread = true;
                    continue;
                };
                let Some(form) = graph.objects.first() else {
                    continue;
                };
                let unset = form
                    .fields
                    .get("m_materialId")
                    .and_then(|id| id.pointer("/m_id/m_id64"))
                    .and_then(serde_json::Value::as_i64)
                    == Some(-1);
                let cutting = form
                    .fields
                    .get("m_cutting")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false);
                let hidden = graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "ParamValueSetInt")
                    .filter_map(|object| object.fields.get("m_paramSet")?.as_array())
                    .flatten()
                    .any(|entry| {
                        entry
                            .pointer("/m_paramId/m_id/m_id64")
                            .and_then(serde_json::Value::as_i64)
                            == Some(VISIBLE_PARAMETER)
                            && entry.get("m_value").and_then(serde_json::Value::as_i64) == Some(0)
                    });
                if unset && !cutting {
                    state.unset = true;
                    state.visible_unset |= !hidden;
                }
            }
            Ok(())
        })?;
    }
    for (family, state) in seen {
        if state.visible_unset {
            out.insert(family, true);
        } else if state.unset || !state.unread {
            out.insert(family, false);
        }
    }
    Ok(out)
}
