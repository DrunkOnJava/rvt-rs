//! #1603, RE-475: which id the native record of a Revit 2015 to 2023 family
//! instance that the decode gives no type names as its type.
//!
//! FACT to check: the 68 windows of the spent Revit 2018 project hold Revit's
//! own type id in their reference list as an id with neither a record nor a
//! name entry (`probe_1603_unjoined_lists_pre2024`, run 43685039364), so the
//! scan cannot name it. Each window's native record, read by member name, names
//! ElementIds too, and the type's own native record holds its name in
//! `SymbolInfo.m_name` (RE-230, RE-240). The id shape before Revit 2024 is
//! `{"m_id": {"m_id": n}}` (RE-280), so both shapes are read.
//!
//! Output, for each window, door, railing, stair and furniture the export
//! writes whose name is not `<family>:<type>:<id>`, in the form of
//! `probe_re240_untyped_types` so that `tools/re/untyped_types_vs_ifc.py`
//! scores it against Revit's export:
//!   el <tag> <IFC class> <name> class=<native class|-> refs=<id>@<object>.<path> ...
//! every ElementId its native record names; then for each id named:
//!   ref <id> class=<native class|-> name=<SymbolInfo.m_name|-> family=<SymbolInfo.m_familyName|->
//! spaces as `_`. Ids and names: the held-out scrub drops these lines.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1603_native_instance_types_pre2024 -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};

const CLASSES: [&str; 5] = [
    "IFCWINDOW",
    "IFCDOOR",
    "IFCRAILING",
    "IFCSTAIR",
    "IFCFURNITURE",
];

/// Every ElementId under `value`, with its path: `{"m_id": {"m_id64": n}}` from
/// Revit 2024, `{"m_id": {"m_id": n}}` before it (RE-280).
fn ids(value: &Fv, path: &str, out: &mut Vec<(String, u64)>) {
    match value {
        Fv::Object(map) => {
            if let Some(id) = map
                .get("m_id")
                .and_then(|v| v.get("m_id64").or_else(|| v.get("m_id")))
                .and_then(Fv::as_u64)
            {
                out.push((path.to_string(), id));
                return;
            }
            for (key, child) in map.iter() {
                ids(child, &format!("{path}.{key}"), out);
            }
        }
        Fv::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                ids(child, &format!("{path}[{index}]"), out);
            }
        }
        _ => {}
    }
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut elements: BTreeMap<u64, (String, String)> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            name,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) && name.matches(':').count() < 2 {
                if let Ok(id) = tag.parse() {
                    elements
                        .entry(id)
                        .or_insert((upper, name.replace(' ', "_")));
                }
            }
        }
    }
    let mut found: BTreeMap<u64, (String, Vec<(String, u64)>)> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: elements.keys().copied().collect(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let mut refs = Vec::new();
            for object in &graph.objects {
                ids(&object.fields, &object.class_name, &mut refs);
            }
            found.insert(
                record.identity.element_id,
                (
                    record.class_name.clone().unwrap_or_else(|| "-".into()),
                    refs,
                ),
            );
            Ok(())
        },
    )?;
    let mut wanted: BTreeSet<u64> = BTreeSet::new();
    for (id, (class, name)) in &elements {
        let (native, refs) = found
            .get(id)
            .map(|(c, r)| (c.as_str(), r.as_slice()))
            .unwrap_or(("-", &[][..]));
        let shown: Vec<String> = refs
            .iter()
            .filter(|(_, r)| r != id)
            .map(|(p, r)| {
                wanted.insert(*r);
                format!("{r}@{p}")
            })
            .collect();
        println!(
            "el {id} {class} {name} class={native} refs={}",
            shown.join(" ")
        );
    }
    let text = |graph: &rvt::research::native_parameters::ObjectGraph, field: &str| {
        graph
            .objects
            .iter()
            .find(|o| o.class_name == "SymbolInfo")
            .and_then(|o| o.fields.get(field))
            .and_then(Fv::as_str)
            .map(|s| s.replace(' ', "_"))
            .unwrap_or_else(|| "-".into())
    };
    let mut named: BTreeMap<u64, String> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: wanted.clone(),
            ..Options::default()
        },
        |record| {
            let (name, family) = match record.graph.as_ref() {
                Some(graph) => (text(graph, "m_name"), text(graph, "m_familyName")),
                None => ("-".into(), "-".into()),
            };
            named.insert(
                record.identity.element_id,
                format!(
                    "class={} name={name} family={family}",
                    record.class_name.as_deref().unwrap_or("-")
                ),
            );
            Ok(())
        },
    )?;
    for id in &wanted {
        println!(
            "ref {id} {}",
            named
                .get(id)
                .map(String::as_str)
                .unwrap_or("class=- name=- family=-")
        );
    }
    eprintln!(
        "{} elements, {} native records, {} ids named",
        elements.len(),
        found.len(),
        wanted.len()
    );
    Ok(())
}
