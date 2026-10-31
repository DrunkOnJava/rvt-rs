//! A family symbol's connectors, from its own native record and its family's
//! (#528).
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
//!
//! A `FamilyInstance` record holds a `Connector` object per connector, with
//! its index in `m_nIndex` and, in `m_arrRefs`, the elements joined to it,
//! each with its own connector index (`m_nIndex`) and a connection type
//! (`m_connType`). The type-1 references are the joins: on RE1 Mechanical
//! they are Revit's 73 port-to-port connections and on RE1 Plumbing its 126,
//! with nothing else but the two of a tank Revit's export leaves out (RE-171,
//! Measure run 37303312731). A duct or pipe joins a family instance at every
//! end, so the instances' connectors hold every join.
// Tracker: B54.

use crate::native_json::{element_id, element_id_i64, finite_triple};
use crate::partition_connector_pairs::ConnectorPair;
use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// The domains whose connectors Revit's export writes ports for: HVAC,
/// electrical and piping.
const PORT_DOMAINS: std::ops::RangeInclusive<i64> = 1..=3;

/// The connection type of a connector reference that is a join (RE-171);
/// type 4 names the connector's system.
const JOIN: u64 = 1;

/// The IFC4 `PredefinedType` of a port on a connector of Revit's `domain`
/// (#828): HVAC `DUCT`, electrical `CABLE`, piping `PIPE`, as Revit's IFC4
/// export types the ports of the BIM4LCA office, and `NOTDEFINED` for an
/// undefined domain, as Revit's exporter types it (revit-ifc
/// `ConnectorExporter`; a cable tray or conduit connector, which it types
/// `CABLECARRIER`, has no port, RE-169).
pub fn port_type_of_domain(domain: i64) -> &'static str {
    match domain {
        1 => "DUCT",
        2 => "CABLE",
        3 => "PIPE",
        _ => "NOTDEFINED",
    }
}

/// A connector's Flow Direction in its family (RE-212): the `m_int` of this
/// built-in parameter in the connector's own `FamilyParams`, 1 In, 2 Out and
/// 0 bidirectional. Piping connectors carry the first, HVAC ones the second.
const PIPE_FLOW_DIRECTION: i64 = -1_140_248;
const DUCT_FLOW_DIRECTION: i64 = -1_140_219;

/// The IFC4 `FlowDirection` of a port (RE-212), from its connector's
/// `domain`, its family's Flow Direction (`family`) and the direction its
/// instance's connector calculation holds (`calculated`, its `m_eDirection`,
/// numbered as the family's): what revit-ifc's `ConnectorExporter` writes
/// from the API's `Connector.Direction`. Only HVAC and piping connectors have
/// a direction there, so every other domain is `SOURCEANDSINK`, as on every
/// electrical port of the RE1 models and the BIM4LCA office. An HVAC or
/// piping connector takes its family's direction where the family sets one,
/// and its calculated one where the family leaves it bidirectional:
/// measured on RE1 Plumbing and Mechanical, 224 of 224 family-instance
/// ports, among them an air handler's piping connector whose calculation
/// says Out and whose family and Revit's export say In. `None` where either
/// needed value is not read. An electrical connector whose description sets
/// `Flow=Source` or `Flow=Sink` is exported with that direction by Revit;
/// rvt-rs does not read the description (#834), so such a port is written
/// `SOURCEANDSINK`.
pub fn port_flow_direction(
    domain: i64,
    family: Option<i64>,
    calculated: Option<i64>,
) -> Option<&'static str> {
    if !matches!(domain, 1 | 3) {
        return Some("SOURCEANDSINK");
    }
    match family? {
        0 => flow_direction_of(calculated?),
        direction => flow_direction_of(direction),
    }
}

/// The IFC4 `FlowDirection` a connector direction numbered as a family's
/// Flow Direction names: 1 In `SINK`, 2 Out `SOURCE`, 0 bidirectional
/// `SOURCEANDSINK` (RE-212).
pub fn flow_direction_of(direction: i64) -> Option<&'static str> {
    match direction {
        0 => Some("SOURCEANDSINK"),
        1 => Some("SINK"),
        2 => Some("SOURCE"),
        _ => None,
    }
}

/// A port Revit's export writes on a family instance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InstancePort {
    /// The connector's index in its symbol.
    pub index: u32,
    /// Revit's `Domain` of the connector.
    pub domain: i64,
    /// The port's IFC4 `FlowDirection` ([`port_flow_direction`]), `None`
    /// where it is not read.
    pub flow_direction: Option<&'static str>,
    /// Where the connector is in the symbol that draws the instance
    /// (`InstanceInfo.m_symbolId`), `None` where it is not read.
    pub frame: Option<ConnectorFrame>,
}

/// A connector's point and Z axis in the coordinates of the symbol that
/// draws its instance (#869, RE-215): the `m_trf` of that symbol's
/// `m_arrConnectorData` entry, whose `m_or` is the point and the third
/// column of whose `m_3x3` is the axis. Through the instance's transform
/// (RE-87) they give the port's point and Z axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConnectorFrame {
    /// Feet.
    pub origin: [f64; 3],
    pub z_axis: [f64; 3],
}

/// A `m_trf` value as a [`ConnectorFrame`]; `None` unless its point is
/// finite and its axis a unit vector.
fn connector_frame(trf: &serde_json::Value) -> Option<ConnectorFrame> {
    let origin = finite_triple(trf.get("m_or")?)?;
    let [r0, r1, r2] = trf.get("m_3x3")?.as_array()?.as_slice() else {
        return None;
    };
    let rows = [finite_triple(r0)?, finite_triple(r1)?, finite_triple(r2)?];
    let z_axis = rows.map(|row| row[2]);
    let length = z_axis.iter().map(|v| v * v).sum::<f64>().sqrt();
    ((length - 1.0).abs() <= 1e-6).then_some(ConnectorFrame { origin, z_axis })
}

/// Each connector's Flow Direction in a `Family` record's graph (RE-212), by
/// connector index. A `ConnectorDataCell` names its own parameters with
/// `m_oRelevantParams`, an object the record holds after every cell; the
/// cells' and those `FamilyParams` objects' orders are the same, so the n-th
/// cell by that reference's offset owns the n-th `FamilyParams` after it.
/// Empty where the counts differ.
fn family_flow_directions(graph: &crate::native_parameters::ObjectGraph) -> BTreeMap<i64, i64> {
    let mut cells: Vec<(u64, i64, i64)> = graph
        .objects
        .iter()
        .filter(|object| object.class_name == "ConnectorDataCell")
        .filter_map(|cell| {
            let offset = cell
                .fields
                .get("m_oRelevantParams")?
                .get("offset")?
                .as_u64()?;
            let index = cell.fields.get("m_index")?.as_i64()?;
            let domain = cell.fields.get("m_domain")?.as_i64()?;
            Some((offset, index, domain))
        })
        .collect();
    cells.sort_unstable();
    let Some(&(first, _, _)) = cells.first() else {
        return BTreeMap::new();
    };
    let mut params: Vec<&crate::native_parameters::GraphObject> = graph
        .objects
        .iter()
        .filter(|object| object.class_name == "FamilyParams")
        .filter(|object| u64::try_from(object.start).is_ok_and(|start| start > first))
        .collect();
    params.sort_by_key(|object| object.start);
    if params.len() != cells.len() {
        return BTreeMap::new();
    }
    cells
        .iter()
        .zip(params)
        .filter_map(|(&(_, index, domain), params)| {
            let wanted = match domain {
                3 => PIPE_FLOW_DIRECTION,
                1 => DUCT_FLOW_DIRECTION,
                _ => return None,
            };
            let value = params
                .fields
                .get("m_params")?
                .as_array()?
                .iter()
                .find(|param| param.get("m_paramId").and_then(element_id_i64) == Some(wanted))?
                .get("m_int")?
                .as_i64()?;
            Some((index, value))
        })
        .collect()
}

/// Each connector's calculated direction in a family instance's or a duct's
/// or pipe's record graph (RE-212), by connector index: the `m_eDirection`
/// of the `MEPFamilyConnectorCalculation*` or `SegmentConnectorCalculation`
/// object whose `m_pConnector` names the connector's object, numbered as a
/// family's Flow Direction.
fn calculated_directions(graph: &crate::native_parameters::ObjectGraph) -> BTreeMap<u32, i64> {
    let index_of_token: BTreeMap<u64, u32> = graph
        .objects
        .iter()
        .filter(|object| object.class_name == "Connector")
        .filter_map(|connector| {
            let index = connector.fields.get("m_nIndex")?.as_u64()?;
            Some((u64::from(connector.token), u32::try_from(index).ok()?))
        })
        .collect();
    graph
        .objects
        .iter()
        .filter(|object| {
            object
                .class_name
                .starts_with("MEPFamilyConnectorCalculation")
                || object.class_name == "SegmentConnectorCalculation"
        })
        .filter_map(|calculation| {
            let token = calculation
                .fields
                .get("m_pConnector")?
                .get("pointer_token")?
                .as_u64()?;
            let direction = calculation.fields.get("m_eDirection")?.as_i64()?;
            Some((*index_of_token.get(&token)?, direction))
        })
        .collect()
}

/// What [`connector_ports`] reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ConnectorPorts {
    /// For each family instance whose symbol has connectors, the ports
    /// Revit's export writes for it, in the symbol's order: every connector
    /// outside the cable tray and conduit domain, with its domain and flow
    /// direction.
    pub instances: BTreeMap<u64, Vec<InstancePort>>,
    /// Each duct's and pipe's connector's calculated direction, by ElementId
    /// and connector index, numbered as a family's Flow Direction: 1 In,
    /// 2 Out, 0 bidirectional (RE-212).
    pub segments: BTreeMap<(u64, u32), i64>,
}

/// The ports of `instances` (a family instance's ElementId and its
/// symbol's) and the calculated directions of the connectors of `segments`
/// (ducts and pipes), in one pass over the partitions. Errs where the native
/// record path does not read the file's release.
pub fn connector_ports(
    rf: &mut RevitFile,
    instances: &BTreeMap<u64, u64>,
    segments: &BTreeSet<u64>,
) -> anyhow::Result<ConnectorPorts> {
    let mut connectors: BTreeMap<u64, (u64, Vec<i64>)> = BTreeMap::new();
    let mut frames: BTreeMap<u64, BTreeMap<i64, ConnectorFrame>> = BTreeMap::new();
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    let mut cells: BTreeMap<u64, BTreeMap<i64, i64>> = BTreeMap::new();
    let mut family_directions: BTreeMap<u64, BTreeMap<i64, i64>> = BTreeMap::new();
    let mut calculated: BTreeMap<u64, BTreeMap<u32, i64>> = BTreeMap::new();
    let mut segment_directions: BTreeMap<(u64, u32), i64> = BTreeMap::new();
    // Tracker: B79.
    // One pass reads the instances, their symbols, the segments and every
    // family:
    // which families the symbols name is known only once they are read, and
    // a second pass over the partitions costs more than decoding the
    // families none names.
    let mut selected: BTreeSet<u64> = instances.values().copied().collect();
    selected.extend(instances.keys().copied());
    selected.extend(segments.iter().copied());
    let options = native_document::Options {
        selected_ids: selected,
        selected_classes: BTreeSet::from(["Family".to_string(), "FamilySymbol".to_string()]),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match record.class_name.as_deref() {
            Some("FamilySymbol") => {
                let Some(root) = graph.objects.first() else {
                    return Ok(());
                };
                let entries: Vec<&serde_json::Value> = root
                    .fields
                    .get("m_arrConnectorData")
                    .and_then(|entries| entries.as_array())
                    .into_iter()
                    .flatten()
                    .collect();
                let indices: Vec<i64> = entries
                    .iter()
                    .filter_map(|entry| entry.get("m_index")?.as_i64())
                    .collect();
                let symbol_frames: BTreeMap<i64, ConnectorFrame> = entries
                    .iter()
                    .filter_map(|entry| {
                        let index = entry.get("m_index")?.as_i64()?;
                        Some((index, connector_frame(entry.get("m_trf")?)?))
                    })
                    .collect();
                if !symbol_frames.is_empty() {
                    frames.insert(id, symbol_frames);
                }
                if let (false, Some(family)) = (
                    indices.is_empty(),
                    root.fields.get("m_familyId").and_then(element_id),
                ) {
                    connectors.insert(id, (family, indices));
                }
            }
            Some("Family") => {
                let domains = cells.entry(id).or_default();
                for object in graph
                    .objects
                    .iter()
                    .filter(|object| object.class_name == "ConnectorDataCell")
                {
                    let value = |name: &str| object.fields.get(name).and_then(|v| v.as_i64());
                    if let (Some(index), Some(domain)) = (value("m_index"), value("m_domain")) {
                        domains.insert(index, domain);
                    }
                }
                family_directions.insert(id, family_flow_directions(graph));
            }
            Some("FamilyInstance") if instances.contains_key(&id) => {
                calculated.insert(id, calculated_directions(graph));
                // The symbol that draws the instance (RE-215), whose
                // connectors are where the instance's are.
                if let Some(symbol) = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "InstanceInfo")
                    .and_then(|object| object.fields.get("m_symbolId"))
                    .and_then(element_id)
                {
                    drawn_by.insert(id, symbol);
                }
            }
            _ if segments.contains(&id) => {
                for (index, direction) in calculated_directions(graph) {
                    segment_directions.insert((id, index), direction);
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let instance_ports = instances
        .iter()
        .filter_map(|(&instance, symbol)| {
            let (family, indices) = connectors.get(symbol)?;
            let domains = cells.get(family)?;
            let directions = family_directions.get(family);
            let calculations = calculated.get(&instance);
            let drawn_frames = drawn_by
                .get(&instance)
                .and_then(|symbol| frames.get(symbol));
            let ports: Vec<InstancePort> = indices
                .iter()
                .filter_map(|&index| {
                    let domain = *domains.get(&index)?;
                    PORT_DOMAINS.contains(&domain).then_some(())?;
                    let index_u32 = u32::try_from(index).ok()?;
                    Some(InstancePort {
                        index: index_u32,
                        domain,
                        flow_direction: port_flow_direction(
                            domain,
                            directions.and_then(|d| d.get(&index).copied()),
                            calculations.and_then(|c| c.get(&index_u32).copied()),
                        ),
                        frame: drawn_frames.and_then(|f| f.get(&index)).copied(),
                    })
                })
                .collect();
            (!ports.is_empty()).then_some((instance, ports))
        })
        .collect();
    Ok(ConnectorPorts {
        instances: instance_ports,
        segments: segment_directions,
    })
}

/// Every join the connectors of `instances` (family instances' ElementIds)
/// list (RE-171), one per connector and element joined to it, sorted. Errs
/// where the native record path does not read the file's release.
pub fn family_instance_joins(
    rf: &mut RevitFile,
    instances: &BTreeSet<u64>,
) -> anyhow::Result<Vec<ConnectorPair>> {
    let mut joins = BTreeSet::new();
    if instances.is_empty() {
        return Ok(Vec::new());
    }
    let options = native_document::Options {
        selected_ids: instances.clone(),
        ..native_document::Options::default()
    };
    native_document::extract_graphs(rf, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let index_of = |value: Option<&serde_json::Value>| {
            value
                .and_then(|index| index.as_u64())
                .and_then(|index| u32::try_from(index).ok())
        };
        for connector in graph
            .objects
            .iter()
            .filter(|object| object.class_name == "Connector")
        {
            let Some(index) = index_of(connector.fields.get("m_nIndex")) else {
                continue;
            };
            for reference in connector
                .fields
                .get("m_arrRefs")
                .and_then(|references| references.as_array())
                .into_iter()
                .flatten()
            {
                if reference.get("m_connType").and_then(|kind| kind.as_u64()) != Some(JOIN) {
                    continue;
                }
                if let (Some(other), Some(other_index)) = (
                    reference.get("m_id").and_then(element_id),
                    index_of(reference.get("m_nIndex")),
                ) {
                    joins.insert(ConnectorPair {
                        element: record.identity.element_id,
                        index,
                        other,
                        other_index,
                    });
                }
            }
        }
        Ok(())
    })?;
    Ok(joins.into_iter().collect())
}
