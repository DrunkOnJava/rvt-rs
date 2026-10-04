//! B54 (probe): are a family instance's ports its symbol's connectors?
//!
//! A FamilySymbol's native record carries `m_arrConnectorData`, each entry an
//! `m_index`; its Family carries a `ConnectorDataCell` per connector with
//! that index, its domain and its connector type. For every element Revit's
//! export writes ports for (`Port_<ElementId>_<index>`), this compares the
//! port indices with the indices of the element's symbol's connectors, and
//! tallies agreement by the element's IFC entity in Revit's export and by the
//! connectors' domains.
//!
//! Usage: probe_b54_symbol_connectors <RE1-*.rvt>

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

fn main() -> anyhow::Result<()> {
    let path = PathBuf::from(std::env::args().nth(1).expect("model path"));
    let name = path.to_string_lossy().to_string();
    if !(name.contains("Electrical") || name.contains("Plumbing") || name.contains("Mechanical")) {
        println!("not an RE1 MEP model");
        return Ok(());
    }
    let step = std::fs::read_to_string(path.with_extension("ifc"))?;
    let mut ports: BTreeMap<u64, BTreeSet<i64>> = BTreeMap::new();
    let mut entity_of: BTreeMap<u64, String> = BTreeMap::new();
    for line in step.lines() {
        if line.contains("IFCDISTRIBUTIONPORT(") {
            let Some(port) = line.split('\'').nth(3) else {
                continue;
            };
            let rest = port
                .trim_start_matches("InPort_")
                .trim_start_matches("OutPort_")
                .trim_start_matches("Port_");
            if let Some((id, index)) = rest.split_once('_') {
                if let (Ok(id), Ok(index)) = (id.parse(), index.parse()) {
                    ports.entry(id).or_default().insert(index);
                }
            }
        } else if let Some((_, body)) = line.split_once('=') {
            if let (Some((entity, _)), Some(tag)) = (
                body.split_once('('),
                line.rsplit('\'').nth(1).and_then(|t| t.parse::<u64>().ok()),
            ) {
                entity_of.entry(tag).or_insert_with(|| entity.to_string());
            }
        }
    }
    let mut rf = RevitFile::open(&path)?;
    let mut symbol_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut symbol_connectors: BTreeMap<u64, Vec<i64>> = BTreeMap::new();
    let mut family_of: BTreeMap<u64, u64> = BTreeMap::new();
    let mut family_domains: BTreeMap<u64, BTreeMap<i64, (i64, i64)>> = BTreeMap::new();
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
                let indices = root
                    .fields
                    .get("m_arrConnectorData")
                    .and_then(|v| v.as_array())
                    .map(|entries| {
                        entries
                            .iter()
                            .filter_map(|e| e.get("m_index").and_then(|i| i.as_i64()))
                            .collect()
                    })
                    .unwrap_or_default();
                symbol_connectors.insert(id, indices);
                if let Some(family) = root.fields.get("m_familyId").and_then(id_of) {
                    family_of.insert(id, family);
                }
            }
            Some("Family") => {
                let cells = family_domains.entry(id).or_default();
                for object in &graph.objects {
                    if object.class_name == "ConnectorDataCell" {
                        let get = |k: &str| object.fields.get(k).and_then(|v| v.as_i64());
                        if let (Some(index), Some(domain), Some(kind)) =
                            (get("m_index"), get("m_domain"), get("m_connectorType"))
                        {
                            cells.insert(index, (domain, kind));
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let mut tally: BTreeMap<(String, &'static str), usize> = BTreeMap::new();
    let mut shown = 0;
    for (element, wanted) in &ports {
        let entity = entity_of.get(element).cloned().unwrap_or_default();
        let symbol = symbol_of.get(element);
        let indices: BTreeSet<i64> = symbol
            .and_then(|s| symbol_connectors.get(s))
            .map(|v| v.iter().copied().collect())
            .unwrap_or_default();
        let verdict = if symbol.is_none() {
            "not a family instance"
        } else if &indices == wanted {
            "symbol's connectors are Revit's ports"
        } else if indices.is_superset(wanted) {
            "symbol has more connectors"
        } else {
            "differ"
        };
        *tally.entry((entity.clone(), verdict)).or_default() += 1;
        if verdict != "symbol's connectors are Revit's ports"
            && verdict != "not a family instance"
            && shown < 25
        {
            let domains = symbol
                .and_then(|s| family_of.get(s))
                .and_then(|f| family_domains.get(f));
            println!(
                "{element} {entity}: Revit ports {wanted:?}, symbol {symbol:?} connectors {indices:?}, family cells (index: domain, type) {domains:?}"
            );
            shown += 1;
        }
    }
    for ((entity, verdict), count) in tally {
        println!("{count:4}  {entity}  {verdict}");
    }
    Ok(())
}
