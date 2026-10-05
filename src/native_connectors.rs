//! A family symbol's connectors, from its own native record and its family's
//! (B54, #528).
//!
//! A `FamilySymbol` record lists its connectors in `m_arrConnectorData`, each
//! entry with an `m_index` and a transform; its `Family` record holds a
//! `ConnectorDataCell` per connector with the same `m_index`, its `m_domain`
//! (Revit's `Domain`: 1 HVAC, 2 electrical, 3 piping, 4 cable tray and
//! conduit) and its connector type. Revit's export writes a port for each
//! connector of a family instance's symbol outside the cable tray and conduit
//! domain, joined or not: on RE1 Electrical, Plumbing and Mechanical every
//! family instance's ports are those indices (Measure run 37230166352), the
//! one panel with five conduit connectors among them included.

use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// The domains whose connectors Revit's export writes ports for: HVAC,
/// electrical and piping.
const PORT_DOMAINS: std::ops::RangeInclusive<i64> = 1..=3;

/// The ElementId an `ElementId` field of a native record holds.
fn element_id(value: &serde_json::Value) -> Option<u64> {
    value
        .get("m_id")?
        .get("m_id64")?
        .as_i64()
        .and_then(|id| u64::try_from(id).ok())
}

/// For each of `symbols` that is a `FamilySymbol` with connectors, the
/// connector indices Revit's export writes ports for, in the symbol's order.
/// Errs where the native record path does not read the file's release.
pub fn symbol_port_indices(
    rf: &mut RevitFile,
    symbols: &BTreeSet<u64>,
) -> anyhow::Result<BTreeMap<u64, Vec<u32>>> {
    let mut connectors: BTreeMap<u64, (u64, Vec<i64>)> = BTreeMap::new();
    let mut domains: BTreeMap<u64, BTreeMap<i64, i64>> = BTreeMap::new();
    // One pass reads the symbols and every family (B79): which families the
    // symbols name is known only once they are read, and a second pass over
    // the partitions costs more than decoding the families none names.
    let options = native_document::Options {
        selected_ids: symbols.clone(),
        selected_classes: BTreeSet::from(["Family".to_string()]),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        match record.class_name.as_deref() {
            Some("FamilySymbol") => {
                let Some(root) = graph.objects.first() else {
                    return Ok(());
                };
                let indices: Vec<i64> = root
                    .fields
                    .get("m_arrConnectorData")
                    .and_then(|entries| entries.as_array())
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry.get("m_index")?.as_i64())
                    .collect();
                if let (false, Some(family)) = (
                    indices.is_empty(),
                    root.fields.get("m_familyId").and_then(element_id),
                ) {
                    connectors.insert(record.identity.element_id, (family, indices));
                }
            }
            Some("Family") => {
                let cells = domains.entry(record.identity.element_id).or_default();
                for object in graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "ConnectorDataCell")
                {
                    let value = |name: &str| object.fields.get(name).and_then(|v| v.as_i64());
                    if let (Some(index), Some(domain)) = (value("m_index"), value("m_domain")) {
                        cells.insert(index, domain);
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    Ok(connectors
        .into_iter()
        .filter_map(|(symbol, (family, indices))| {
            let cells = domains.get(&family)?;
            let ports: Vec<u32> = indices
                .into_iter()
                .filter(|index| {
                    cells
                        .get(index)
                        .is_some_and(|domain| PORT_DOMAINS.contains(domain))
                })
                .filter_map(|index| u32::try_from(index).ok())
                .collect();
            (!ports.is_empty()).then_some((symbol, ports))
        })
        .collect())
}
