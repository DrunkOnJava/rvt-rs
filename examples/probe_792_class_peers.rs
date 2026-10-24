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
//! Output:
//!   element <ifc class> <id> <native class> <root scalar fields as JSON> <InstanceInfo scalar fields as JSON | null>
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
        Ok(())
    })?;
    eprintln!("{} elements of {CLASSES:?}", class_of.len());
    Ok(())
}
