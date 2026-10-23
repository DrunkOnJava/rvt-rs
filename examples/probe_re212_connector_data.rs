//! RE-212 (#834, #869): what a family's connector cells, its symbols'
//! connector data and its instances' connectors hold, beyond the index and
//! domain rvt-rs reads.
//!
//! FACT under test: Revit's export names each port of a family instance
//! `InPort_`, `OutPort_` or `Port_<ElementId>_<index>` and places it at its
//! connector. rvt-rs reads only each connector's index and domain
//! (`native_connectors`, RE-169), so it writes no piping or HVAC
//! `FlowDirection` and places only the ports a duct or pipe joins (#922).
//! This probe prints every field of those objects;
//! `tools/re/connector_data_vs_ifc.py` lines them up with Revit's ports.
//!
//! Output, one JSON value per line:
//!   cell <family> <ConnectorDataCell fields>
//!   symbol <symbol> <family> <m_arrConnectorData>
//!   instance <instance> <symbol> <Connector objects' fields>
//!   fields <class> <root field names>   once per class
//!
//! Usage:
//!   cargo run --profile ci --example probe_re212_connector_data -- MODEL.rvt

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use serde_json::Value;
use std::collections::BTreeSet;

fn element_id(value: &Value) -> Option<i64> {
    value.get("m_id")?.get("m_id64")?.as_i64()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let options = Options {
        selected_classes: ["Family", "FamilySymbol", "FamilyInstance"]
            .map(String::from)
            .into(),
        ..Options::default()
    };
    let mut shown = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        if shown.insert(class.to_string()) {
            let names: Vec<&String> = root
                .fields
                .as_object()
                .map(|o| o.keys().collect())
                .unwrap_or_default();
            println!("fields {class} {}", serde_json::to_string(&names)?);
        }
        match class {
            "Family" => {
                for cell in graph
                    .objects
                    .iter()
                    .filter(|o| o.class_name == "ConnectorDataCell")
                {
                    println!("cell {id} {}", serde_json::to_string(&cell.fields)?);
                }
            }
            "FamilySymbol" => {
                let data = root
                    .fields
                    .get("m_arrConnectorData")
                    .filter(|data| data.as_array().is_some_and(|a| !a.is_empty()));
                if let Some(data) = data {
                    let family = root.fields.get("m_familyId").and_then(element_id);
                    println!(
                        "symbol {id} {} {}",
                        family.map_or("-".to_string(), |f| f.to_string()),
                        serde_json::to_string(data)?
                    );
                }
            }
            "FamilyInstance" => {
                let connectors: Vec<&Value> = graph
                    .objects
                    .iter()
                    .filter(|o| o.class_name == "Connector")
                    .map(|o| &o.fields)
                    .collect();
                if !connectors.is_empty() {
                    let symbol = ["m_masterSymbolId", "m_symbolId", "m_typeId"]
                        .iter()
                        .find_map(|name| root.fields.get(*name).and_then(element_id));
                    println!(
                        "instance {id} {} {}",
                        symbol.map_or("-".to_string(), |s| s.to_string()),
                        serde_json::to_string(&connectors)?
                    );
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(())
}
