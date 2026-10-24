//! #792: what sets the element Revit's export writes in another class, or
//! leaves out, apart from the others of its class.
//!
//! FACT under test: on the BIM4LCA office, Revit writes plumbing fixture
//! 3095244 as a plain `IfcFlowTerminal` where rvt-rs, like Revit for every
//! other fixture, writes `IfcSanitaryTerminal`, and the building-permit file
//! has a column, 1927742, that Revit's export leaves out. Neither carries an
//! IFC export value. For every element rvt-rs writes as one of the classes
//! below, this prints its native record's class and the scalar fields of its
//! graph's root object, and of an `InstanceInfo` object where it has one, for
//! `tools/re/class_peers_vs_ifc.py` to split by what Revit's export does with
//! each element.
//!
//! A second pass prints the same for each element's type (`m_masterSymbolId`)
//! and that type's family (`m_familyId`), and every element's whole graph is
//! printed too, for the fields nested below the root.
//!
//! Output:
//!   element <ifc class> <id> <native class> <root scalar fields as JSON> <InstanceInfo scalar fields as JSON | null>
//!   owner <ifc class> <element id> <type|family> <id> <native class> <root scalar fields as JSON>
//!   full <id> <every object's class and fields as JSON, at most 20000 characters>
//!
//! Usage:
//!   cargo run --profile ci --example probe_792_class_peers -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::{Map, Value};

const CLASSES: &[&str] = &["IFCSANITARYTERMINAL", "IFCCOLUMN"];

/// The fields of `object` that are numbers, booleans or strings, and the
/// element ids of those that are one.
fn scalars(fields: &Value) -> Value {
    let mut out = Map::new();
    if let Some(object) = fields.as_object() {
        for (name, value) in object {
            match value {
                Value::Number(_) | Value::Bool(_) | Value::String(_) => {
                    out.insert(name.clone(), value.clone());
                }
                Value::Object(inner) => {
                    if let Some(id) = inner
                        .get("m_id")
                        .and_then(|v| v.get("m_id64"))
                        .filter(|v| v.is_number())
                    {
                        out.insert(name.clone(), id.clone());
                    }
                }
                _ => {}
            }
        }
    }
    Value::Object(out)
}

/// The element id a field holds, as `{"m_id": {"m_id64": n}}`.
fn id_of(fields: &Value, name: &str) -> Option<u64> {
    fields.get(name)?.get("m_id")?.get("m_id64")?.as_u64()
}

fn print_full(
    id: u64,
    graph: &rvt::research::native_parameters::ObjectGraph,
) -> anyhow::Result<()> {
    let objects: Vec<Value> = graph
        .objects
        .iter()
        .map(|o| serde_json::json!({"class": o.class_name, "fields": o.fields}))
        .collect();
    let mut text = serde_json::to_string(&objects)?;
    if text.len() > 20000 {
        let mut cut = 20000;
        while !text.is_char_boundary(cut) {
            cut -= 1;
        }
        text.truncate(cut);
    }
    println!("full {id} {text}");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut class_of: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if CLASSES.contains(&upper.as_str()) {
                if let Ok(id) = tag.parse::<u64>() {
                    class_of.insert(id, upper);
                }
            }
        }
    }
    let options = Options {
        selected_ids: class_of.keys().copied().collect::<BTreeSet<_>>(),
        selected_classes: BTreeSet::new(),
        ..Options::default()
    };
    if options.selected_ids.is_empty() {
        eprintln!("no element of {CLASSES:?}");
        return Ok(());
    }
    let mut type_of: BTreeMap<u64, u64> = BTreeMap::new();
    extract_graphs(&mut rf, &options, |record| {
        let Some(class) = class_of.get(&record.identity.element_id) else {
            return Ok(());
        };
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let info = graph
            .objects
            .iter()
            .find(|o| o.class_name == "InstanceInfo")
            .map(|o| scalars(&o.fields))
            .unwrap_or(Value::Null);
        println!(
            "element {class} {} {} {} {}",
            record.identity.element_id,
            record.class_name.as_deref().unwrap_or("-"),
            serde_json::to_string(&scalars(&root.fields))?,
            serde_json::to_string(&info)?
        );
        if let Some(symbol) = id_of(&root.fields, "m_masterSymbolId") {
            type_of.insert(record.identity.element_id, symbol);
        }
        print_full(record.identity.element_id, graph)?;
        Ok(())
    })?;
    // The elements' types, then the types' families.
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let types: BTreeSet<u64> = type_of.values().copied().collect();
    let options = Options {
        selected_ids: types.clone(),
        selected_classes: BTreeSet::new(),
        ..Options::default()
    };
    let mut type_fields: BTreeMap<u64, (String, Value)> = BTreeMap::new();
    if !types.is_empty() {
        extract_graphs(&mut rf, &options, |record| {
            let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
                return Ok(());
            };
            if let Some(family) = id_of(&root.fields, "m_familyId") {
                family_of.insert(record.identity.element_id, family);
            }
            type_fields.insert(
                record.identity.element_id,
                (
                    record.class_name.clone().unwrap_or_default(),
                    scalars(&root.fields),
                ),
            );
            Ok(())
        })?;
    }
    let families: BTreeSet<u64> = family_of.values().copied().collect();
    let options = Options {
        selected_ids: families.clone(),
        selected_classes: BTreeSet::new(),
        ..Options::default()
    };
    let mut family_fields: BTreeMap<u64, (String, Value)> = BTreeMap::new();
    if !families.is_empty() {
        extract_graphs(&mut rf, &options, |record| {
            let Some(root) = record.graph.as_ref().and_then(|g| g.objects.first()) else {
                return Ok(());
            };
            family_fields.insert(
                record.identity.element_id,
                (
                    record.class_name.clone().unwrap_or_default(),
                    scalars(&root.fields),
                ),
            );
            Ok(())
        })?;
    }
    for (element, symbol) in &type_of {
        let Some(class) = class_of.get(element) else {
            continue;
        };
        if let Some((native, fields)) = type_fields.get(symbol) {
            println!(
                "owner {class} {element} type {symbol} {native} {}",
                serde_json::to_string(fields)?
            );
        }
        if let Some((family, (native, fields))) = family_of
            .get(symbol)
            .and_then(|f| family_fields.get(f).map(|v| (f, v)))
        {
            println!(
                "owner {class} {element} family {family} {native} {}",
                serde_json::to_string(fields)?
            );
        }
    }
    eprintln!("{} elements of {CLASSES:?}", class_of.len());
    Ok(())
}
