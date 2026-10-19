//! RE-197 (probe, #828): which field of a family's connector gives the flow
//! direction Revit's export writes on its port?
//!
//! Revit's IFC2X3 export of the RE1 MEP models names a family instance's
//! port `InPort_<ElementId>_<index>` or `OutPort_<ElementId>_<index>`, and
//! its IFC4 export of the BIM4LCA office writes `FlowDirection` on each
//! (`.SOURCEANDSINK.` on all 67). The family's `ConnectorDataCell` with the
//! connector's `m_index` holds its domain (RE-169) and other fields. For every
//! port of a family instance in Revit's export, this tallies each of the
//! cell's other fields, value by value, against the port's direction (`In`,
//! `Out`, or the IFC4 `FlowDirection`), so a field that decides it shows as
//! one value per direction.
//!
//! Usage: probe_re197_connector_directions <model.rvt>, with Revit's export
//! beside it as `<model>.ifc` or `<model>_Revit.ifc`.

use rvt::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

fn id_of(value: &serde_json::Value) -> Option<u64> {
    value
        .get("m_id")
        .and_then(|id| id.get("m_id64"))
        .and_then(|v| v.as_i64())
        .and_then(|v| u64::try_from(v).ok())
}

/// Each port's direction in Revit's export, by (ElementId, connector index):
/// `In` or `Out` from its name, else its `FlowDirection`.
fn port_directions(step: &str) -> BTreeMap<(u64, i64), String> {
    let mut out = BTreeMap::new();
    for line in step.lines() {
        let Some(at) = line.find("IFCDISTRIBUTIONPORT(") else {
            continue;
        };
        let Some(port) = line.split('\'').nth(3) else {
            continue;
        };
        let (named, rest) = if let Some(rest) = port.strip_prefix("InPort_") {
            (Some("In"), rest)
        } else if let Some(rest) = port.strip_prefix("OutPort_") {
            (Some("Out"), rest)
        } else if let Some(rest) = port.strip_prefix("Port_") {
            (None, rest)
        } else {
            continue;
        };
        let Some((id, index)) = rest.split_once('_') else {
            continue;
        };
        let (Ok(id), Ok(index)) = (id.parse(), index.parse()) else {
            continue;
        };
        let direction = named.map(str::to_string).unwrap_or_else(|| {
            line[at..]
                .split(',')
                .find(|field| field.contains("SOURCE") || field.contains("SINK"))
                .unwrap_or("none")
                .trim()
                .to_string()
        });
        out.insert((id, index), direction);
    }
    out
}

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let stem = path.with_extension("");
    let export = [
        path.with_extension("ifc"),
        PathBuf::from(format!("{}_Revit.ifc", stem.display())),
    ]
    .into_iter()
    .find(|candidate| candidate.exists());
    let Some(export) = export else {
        println!("no Revit export beside the model");
        return Ok(());
    };
    let directions = port_directions(&std::fs::read_to_string(&export)?);
    if directions.is_empty() {
        println!("no ports in Revit's export");
        return Ok(());
    }
    let mut rf = RevitFile::open(&path)?;
    let mut symbol_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut cells: BTreeMap<u64, BTreeMap<i64, BTreeMap<String, String>>> = BTreeMap::new();
    native_document::extract(&mut rf, &native_document::Options::default(), |record| {
        let id = record.identity.element_id;
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        match record.class_name.as_deref() {
            Some("FamilyInstance") => {
                if let Some(symbol) = root.fields.get("m_masterSymbolId").and_then(id_of) {
                    symbol_of.insert(id, symbol);
                }
            }
            Some("FamilySymbol") => {
                if let Some(family) = root.fields.get("m_familyId").and_then(id_of) {
                    family_of.insert(id, family);
                }
            }
            Some("Family") => {
                let family = cells.entry(id).or_default();
                for object in graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "ConnectorDataCell")
                {
                    let Some(index) = object.fields.get("m_index").and_then(|v| v.as_i64()) else {
                        continue;
                    };
                    let fields = object
                        .fields
                        .as_object()
                        .into_iter()
                        .flatten()
                        .filter(|(name, _)| name.as_str() != "m_index")
                        .map(|(name, value)| {
                            let text = value.to_string();
                            (name.clone(), text.chars().take(80).collect())
                        })
                        .collect();
                    family.insert(index, fields);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    // field -> value -> direction -> ports
    let mut tally: BTreeMap<String, BTreeMap<String, BTreeMap<String, usize>>> = BTreeMap::new();
    let mut by_direction: BTreeMap<String, usize> = BTreeMap::new();
    let mut unmatched = 0usize;
    for ((element, index), direction) in &directions {
        let cell = symbol_of
            .get(element)
            .and_then(|symbol| family_of.get(symbol))
            .and_then(|family| cells.get(family))
            .and_then(|family| family.get(index));
        let Some(cell) = cell else {
            unmatched += 1;
            continue;
        };
        *by_direction.entry(direction.clone()).or_default() += 1;
        for (field, value) in cell {
            *tally
                .entry(field.clone())
                .or_default()
                .entry(value.clone())
                .or_default()
                .entry(direction.clone())
                .or_default() += 1;
        }
    }
    println!(
        "ports in Revit's export: {}; on family instances with a connector cell: {} {:?}; others: {unmatched}",
        directions.len(),
        by_direction.values().sum::<usize>(),
        by_direction
    );
    let deciding: BTreeSet<&String> = tally
        .iter()
        .filter(|(_, values)| {
            values.len() > 1 && values.values().all(|directions| directions.len() == 1)
        })
        .map(|(field, _)| field)
        .collect();
    println!("fields whose every value has one direction: {deciding:?}");
    for (field, values) in &tally {
        println!("{field}:");
        for (value, directions) in values.iter().take(12) {
            println!("  {value}: {directions:?}");
        }
    }
    Ok(())
}
