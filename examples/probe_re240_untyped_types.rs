//! RE-240 (#35, W7): which record names the type of an element rvt-rs
//! writes with no type.
//!
//! FACT under test: on the BIM4LCA office rvt-rs writes 12 pipe segments,
//! 9 building element proxies and 5 ramps with neither Revit's Name nor its
//! ObjectType, and so without their type's `Reference` (`names_vs_ifc`,
//! `type_objects_vs_ifc`). Revit's export names them for a type (`JSV-3`,
//! `JSV-4`, `JSV-5`, `AP-ulkoluiska`). Hypothesis: the element's own native
//! record names its type by ElementId, and the type's native record holds
//! its name in `SymbolInfo.m_name`, as a curtain wall type's does (RE-230).
//!
//! Output, for each pipe segment, proxy, ramp, ramp flight, slab and stair
//! the export writes whose name is not `<family>:<type>:<id>`:
//!   el <tag> <IFC class> <name> class=<native class|-> refs=<id>@<object>.<path> ...
//! every ElementId its native record names; then for each id named:
//!   ref <id> class=<native class|-> name=<SymbolInfo.m_name|-> family=<SymbolInfo.m_familyName|->
//! spaces as `_`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re240_untyped_types -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;

const CLASSES: [&str; 6] = [
    "IFCPIPESEGMENT",
    "IFCBUILDINGELEMENTPROXY",
    "IFCRAMP",
    "IFCRAMPFLIGHT",
    "IFCSLAB",
    "IFCSTAIR",
];

/// Every ElementId (`{"m_id": {"m_id64": n}}`) under `value`, with its path.
fn ids(value: &Value, path: &str, out: &mut Vec<(String, u64)>) {
    match value {
        Value::Object(map) => {
            if let Some(id) = map
                .get("m_id")
                .and_then(|v| v.get("m_id64"))
                .and_then(Value::as_u64)
            {
                out.push((path.to_string(), id));
                return;
            }
            for (key, child) in map {
                ids(child, &format!("{path}.{key}"), out);
            }
        }
        Value::Array(items) => {
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
            .and_then(Value::as_str)
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
