//! RE-377 (#1593): which record of a Revit 2015 to 2023 save holds the material
//! Revit's export gives a curtain panel.
//!
//! RE-367 reads a panel's material as its type's `m_renderStyleId`, the material
//! RE-166 and RE-201 measured as Revit's on Snowdon Towers, and finds it is not
//! Revit's on any of the 67 plates of Vitoria's `ALD26 Before`. This lists, for
//! every curtain panel (an element record of `OST_CurtainWallPanels`), each
//! field of the panel's own native record, of its type's record (the one id in
//! its reference list that has a name entry and no record of its own, RE-367)
//! and of the records its reference list names, that holds the ElementId of a
//! material. `tools/re/panel_material_sources_vs_ifc.py` then says, source by
//! source, how many plates have Revit's material from it.
//!
//! One JSON object per line:
//!
//! - `{"kind":"panel","id":P,"types":[T],"refs":[R...]}` for each panel;
//! - `{"kind":"record","id":N,"class":"...","roles":["instance","type","ref"],
//!   "sources":[...],"byte_hits":[...]}` for each native record read that holds
//!   a material. `sources` lists the fields of its graph that hold one:
//!   `path` is the object's class and the field path to the ElementId (`[]`
//!   for an array element) and `context` the array element's other fields where
//!   there is one, which says which parameter an entry is. `byte_hits` lists
//!   where its body holds a `u32` that is a material's ElementId whatever field
//!   it is in: the offset and the bytes around it;
//! - `{"kind":"materials","names":[...]}` and, for each material that some
//!   record of the file names, `{"kind":"material_refs","id","name","by_class"}`:
//!   how many records of each class hold its ElementId, its own record
//!   excepted, so that a scorer that knows Revit's material can say which
//!   classes of record name it;
//! - `{"kind":"gstyle","id","byte_hits","objects"}` for each graphics style (a
//!   category's or subcategory's Object Styles row) whose body holds a material,
//!   with every object of its decoded graph;
//! - `{"kind":"summary",...}`.
//!
//! The ElementIds and material names are the model's own, so a scorer prints
//! counts and a digest of `context` and nothing else.
//!
//! ```bash
//! cargo run --profile ci -p rvt-probes --example probe_re377_panel_material_sources -- \
//!     samples/2023-racadvancedsampleproject.rvt
//! ```

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::research::native_document::{self, Options};
use rvt::research::partition_element_records::OST_CURTAIN_WALL_PANELS;
use rvt::research::partition_element_records_2023::{is_32bit_release, scan_records};
use rvt::research::partition_materials::{scan_material_names, scan_materials_32};
use rvt::research::partition_names::name_entries_2023;
use serde_json::{Value, json};

/// How much of an array element's other fields a source keeps.
const CONTEXT_LIMIT: usize = 400;

/// How many byte hits a record's line lists.
const BYTE_HITS_LIMIT: usize = 24;

/// How much of a record's decoded root a line keeps.
const ROOT_LIMIT: usize = 4000;

struct Source {
    path: String,
    material: u32,
    name: String,
    context: Option<String>,
}

/// An ElementId field as the schema saves it: `{"m_id":{"m_id64":n}}` from
/// Revit 2024, `{"m_id":{"m_id":n}}` on 2015 to 2023.
fn element_id(value: &Value) -> Option<i64> {
    let identifier = value.get("m_id")?;
    identifier
        .get("m_id64")
        .or_else(|| identifier.get("m_id"))
        .and_then(Value::as_i64)
}

fn hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Where a record's body holds a little-endian `u32` that is the ElementId of
/// a material, whatever field it is in: the offset, the eight bytes before it
/// and the four after. A coincidence is possible and a decoded field is not
/// needed, so this finds a material the graph does not show.
fn byte_hits(body: &[u8], materials: &BTreeMap<u32, String>) -> Vec<Value> {
    let mut hits = Vec::new();
    for (offset, window) in body.windows(4).enumerate() {
        let word = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
        let Some(name) = materials.get(&word) else {
            continue;
        };
        let before = &body[offset.saturating_sub(8)..offset];
        let after = &body[offset + 4..(offset + 8).min(body.len())];
        hits.push(json!({
            "offset": offset,
            "material": word,
            "name": name,
            "before": hex(before),
            "after": hex(after),
        }));
    }
    hits
}

/// Every ElementId below `value` that is one of `materials`, with the path of
/// fields that leads to it. `parent` and `key` are the object holding `value`
/// and the field it is in.
fn walk(
    value: &Value,
    path: &str,
    parent: Option<(&Value, &str)>,
    materials: &BTreeMap<u32, String>,
    out: &mut Vec<Source>,
) {
    if let Some(id) = element_id(value) {
        let Some((id, name)) = u32::try_from(id)
            .ok()
            .and_then(|id| materials.get(&id).map(|name| (id, name)))
        else {
            return;
        };
        let context = parent.filter(|_| path.contains("[]")).map(|(object, key)| {
            let mut rest = object.clone();
            if let Some(map) = rest.as_object_mut() {
                map.remove(key);
            }
            rest.to_string()
                .chars()
                .take(CONTEXT_LIMIT)
                .collect::<String>()
        });
        out.push(Source {
            path: path.to_string(),
            material: id,
            name: name.clone(),
            context,
        });
        return;
    }
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                walk(
                    child,
                    &format!("{path}.{key}"),
                    Some((value, key.as_str())),
                    materials,
                    out,
                );
            }
        }
        Value::Array(items) => {
            for child in items {
                walk(child, &format!("{path}[]"), parent, materials, out);
            }
        }
        _ => {}
    }
}

/// The curtain panels of a 32-bit save with their type candidates and the
/// records their lists name, keyed by the panel's ElementId.
fn find_panels(
    rf: &mut RevitFile,
    release: u32,
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, (Vec<u32>, Vec<u32>)> {
    let records = scan_records(rf, release);
    let type_names = name_entries_2023(rf, declared);
    let record_ids: BTreeSet<u32> = records.iter().map(|r| r.element_id).collect();
    let mut panels: BTreeMap<u32, (Vec<u32>, Vec<u32>)> = BTreeMap::new();
    for record in records
        .iter()
        .filter(|r| r.builtin_category == OST_CURTAIN_WALL_PANELS)
    {
        let mut types = Vec::new();
        let mut refs = Vec::new();
        for &reference in &record.references {
            let Ok(id) = u32::try_from(reference) else {
                continue;
            };
            if id == record.element_id {
                continue;
            }
            if record_ids.contains(&id) {
                refs.push(id);
            } else if type_names.contains_key(&id) && !types.contains(&id) {
                types.push(id);
            }
        }
        panels.entry(record.element_id).or_insert((types, refs));
    }
    panels
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .ok_or_else(|| anyhow::anyhow!("usage: probe_re377_panel_material_sources <file.rvt>"))?;
    let mut rf = RevitFile::open(&path)?;
    let release = rf.layout_release()?;
    let table = parse_records(&mut rf)?;
    let declared = declared_ids(&table);
    let narrow = is_32bit_release(release);
    let materials: BTreeMap<u32, String> = if narrow {
        scan_materials_32(&mut rf, release, &declared).1
    } else {
        scan_material_names(&mut rf, release, &declared).unwrap_or_default()
    };

    // The panels of a 32-bit save: each one's type candidates and the records
    // its list names. Other releases get the whole-file pass alone.
    let panels = if narrow {
        find_panels(&mut rf, release, &declared)
    } else {
        BTreeMap::new()
    };
    let mut roles: BTreeMap<u64, BTreeSet<&'static str>> = BTreeMap::new();
    for (&panel, (types, refs)) in &panels {
        println!(
            "{}",
            json!({"kind": "panel", "id": panel, "types": types, "refs": refs})
        );
        roles
            .entry(u64::from(panel))
            .or_default()
            .insert("instance");
        for &id in types {
            roles.entry(u64::from(id)).or_default().insert("type");
        }
        for &id in refs {
            if !panels.contains_key(&id) {
                roles.entry(u64::from(id)).or_default().insert("ref");
            }
        }
    }

    let (mut read, mut with_material) = (0usize, 0usize);
    if !roles.is_empty() {
        let options = Options {
            selected_ids: roles.keys().copied().collect(),
            releases: native_document::NATIVE_RELEASES,
            keep_body: true,
            ..Options::default()
        };
        native_document::extract_graphs(&mut rf, &options, |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            read += 1;
            let mut sources = Vec::new();
            for object in &graph.objects {
                walk(
                    &object.fields.to_json(),
                    &object.class_name,
                    None,
                    &materials,
                    &mut sources,
                );
            }
            let hits = record
                .body
                .as_deref()
                .map(|body| byte_hits(body, &materials))
                .unwrap_or_default();
            if sources.is_empty() && hits.is_empty() {
                return Ok(());
            }
            with_material += 1;
            let id = record.identity.element_id;
            let class = graph.objects.first().map(|o| o.class_name.clone());
            // A type's own fields, to read what else it holds beside a material.
            let root = roles
                .get(&id)
                .filter(|roles| roles.contains("type"))
                .and_then(|_| graph.objects.first())
                .map(|object| {
                    object
                        .fields
                        .to_string()
                        .chars()
                        .take(ROOT_LIMIT)
                        .collect::<String>()
                });
            println!(
                "{}",
                json!({
                    "kind": "record",
                    "id": id,
                    "class": class,
                    "roles": roles.get(&id),
                    "sources": sources
                        .iter()
                        .map(|s| json!({
                            "path": s.path,
                            "material": s.material,
                            "name": s.name,
                            "context": s.context,
                        }))
                        .collect::<Vec<_>>(),
                    "root": root,
                    "byte_hits": hits.iter().take(BYTE_HITS_LIMIT).collect::<Vec<_>>(),
                    "byte_hits_total": hits.len(),
                })
            );
            Ok(())
        })?;
    }

    // Which classes of record name each material anywhere in the file, so that
    // where a material no panel record names is held can be read from a scorer
    // that knows the material.
    let mut named_by: BTreeMap<u32, BTreeMap<String, usize>> = BTreeMap::new();
    let everything = Options {
        releases: native_document::NATIVE_RELEASES,
        keep_body: true,
        graph_classes: Some(BTreeSet::from(["GStyleElem".to_string()])),
        ..Options::default()
    };
    native_document::extract_graphs(&mut rf, &everything, |record| {
        let Some(body) = record.body.as_deref() else {
            return Ok(());
        };
        let own = record.identity.element_id;
        let class = record.class_name.clone().unwrap_or_else(|| "?".into());
        let mut seen = BTreeSet::new();
        for window in body.windows(4) {
            let word = u32::from_le_bytes([window[0], window[1], window[2], window[3]]);
            if u64::from(word) != own && materials.contains_key(&word) {
                seen.insert(word);
            }
        }
        // A graphics style (a category's or subcategory's Object Styles row)
        // that holds a material: where, and every object of its graph (the
        // style data a pointer reaches holds the material field).
        if class == "GStyleElem" && !seen.is_empty() {
            let hits = byte_hits(body, &materials);
            let objects: Vec<Value> = record
                .graph
                .as_ref()
                .map(|graph| {
                    graph
                        .objects
                        .iter()
                        .map(|object| {
                            json!({
                                "class": object.class_name,
                                "fields": object
                                    .fields
                                    .to_string()
                                    .chars()
                                    .take(ROOT_LIMIT)
                                    .collect::<String>(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            println!(
                "{}",
                json!({
                    "kind": "gstyle",
                    "id": own,
                    "byte_hits": hits.iter().take(BYTE_HITS_LIMIT).collect::<Vec<_>>(),
                    "objects": objects,
                })
            );
        }
        for material in seen {
            *named_by
                .entry(material)
                .or_default()
                .entry(class.clone())
                .or_default() += 1;
        }
        Ok(())
    })?;
    println!(
        "{}",
        json!({"kind": "materials", "names": materials.values().collect::<Vec<_>>()})
    );
    for (material, classes) in &named_by {
        println!(
            "{}",
            json!({
                "kind": "material_refs",
                "id": material,
                "name": materials.get(material),
                "by_class": classes,
            })
        );
    }
    println!(
        "{}",
        json!({
            "kind": "summary",
            "release": release,
            "panels": panels.len(),
            "records_selected": roles.len(),
            "records_read": read,
            "records_with_a_material": with_material,
            "materials_named": materials.len(),
        })
    );
    Ok(())
}
