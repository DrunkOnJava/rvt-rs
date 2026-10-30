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
//!   object <owner> <class name> <class tag> <start> <token> <fields>   every object
//!     of a family's graph but its cells, and of an instance's graph but its
//!     root, that is not a geometry object; pipes and ducts (`RbsPipeCurve`,
//!     `RbsDuctCurve` and their flex kinds) are listed this way too
//!   place <instance> <m_instOrigin, m_RefDir, m_zAxis, m_flippedX,
//!     m_flippedY, m_insertOrientation, m_mirrored as a JSON object>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re212_connector_data -- MODEL.rvt

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
        selected_classes: [
            "Family",
            "FamilySymbol",
            "FamilyInstance",
            "RbsPipeCurve",
            "RbsDuctCurve",
            "RbsFlexPipeCurve",
            "RbsFlexDuctCurve",
        ]
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
        if class != "FamilySymbol" {
            for object in graph.objects.iter().skip(1) {
                if object.class_name == "ConnectorDataCell" {
                    continue;
                }
                if object.class_name.starts_with("G") && object.class_name != "GInstance" {
                    continue;
                }
                println!(
                    "object {id} {} {} {} {} {}",
                    object.class_name,
                    object.class_tag,
                    object.start,
                    object.token,
                    serde_json::to_string(&object.fields)?
                );
            }
        }
        if class == "FamilyInstance" {
            let place: serde_json::Map<String, Value> = [
                "m_instOrigin",
                "m_RefDir",
                "m_zAxis",
                "m_flippedX",
                "m_flippedY",
                "m_insertOrientation",
                "m_mirrored",
            ]
            .iter()
            .filter_map(|name| Some((name.to_string(), root.fields.get(*name)?.clone())))
            .collect();
            println!("place {id} {}", serde_json::to_string(&place)?);
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
