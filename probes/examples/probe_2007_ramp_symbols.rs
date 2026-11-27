//! #2007: does a ramp's own symbol record say whether Revit's export makes it a container
//! (an `IfcRamp` with an `IfcRampFlight` part) or a plain `IfcRamp`?
//!
//! Also prints every field of each `Ramp` record (`ramp <id> field <name> <value>`) and reads
//! every record a Ramp field names by ElementId (its type, its levels) with the symbols, so a
//! ramp type's limits can be set beside each ramp's slope (#2007).
//!
//! FACT under test: each `Ramp` record names its own symbol (`m_stairsSymbolId`, its id plus
//! one on every ramp of the BIM4LCA files), and RE-467 found the ramp records themselves hold
//! the same classes on container and plain ramps. `revit-ifc` (CLEANROOM.md source 5) makes a
//! container in `StairsExporter.ExportLegacyStairOrRampAsContainer` when the native
//! `ExporterIFCUtils.GetLegacyStairOrRampComponents` returns the ramp's components, a call
//! whose logic is not in its source; its geometry route (`RampExporter.ExportRamp`) cannot give
//! one flight with no landing, which is what the containers hold. So the symbol, the record
//! that holds a sketched ramp's runs and landings, is where a difference is looked for: on the
//! BIM4LCA housing the container is ramp 1347699, on the office 1440596, and the office's
//! plain ramps are 1439414, 1440087, 1465824 and 1465840.
//!
//! Output, one model per run:
//!   ramp <ElementId> symbol <ElementId>
//!   symbol <ramp ElementId> <symbol ElementId> record <class> objects <class>=<n> ...
//!   symbol <ramp ElementId> <symbol ElementId> field <object class> <name> <value, cut to 400>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_2007_ramp_symbols -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut symbols: BTreeMap<u64, u64> = BTreeMap::new();
    // Every record a Ramp's own fields name, its type among them, read beside the symbols.
    let mut referred: BTreeMap<u64, u64> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_classes: BTreeSet::from(["Ramp".to_string()]),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            for object in &graph.objects {
                if object.class_name != "Ramp" {
                    continue;
                }
                if let Some(fields) = object.fields.as_object() {
                    for (name, value) in fields {
                        let cut: String = value.to_string().chars().take(400).collect();
                        println!("ramp {} field {name} {cut}", record.identity.element_id);
                        if let Some(id) = value["m_id"]["m_id64"]
                            .as_i64()
                            .and_then(|i| u64::try_from(i).ok())
                        {
                            referred.entry(id).or_insert(record.identity.element_id);
                        }
                    }
                }
                let symbol = object.fields["m_stairsSymbolId"]["m_id"]["m_id64"].as_i64();
                if let Some(symbol) = symbol.and_then(|s| u64::try_from(s).ok()) {
                    symbols.insert(record.identity.element_id, symbol);
                }
            }
            Ok(())
        },
    )?;
    for (ramp, symbol) in &symbols {
        println!("ramp {ramp} symbol {symbol}");
    }
    let mut owner: BTreeMap<u64, u64> = symbols.iter().map(|(r, s)| (*s, *r)).collect();
    for (id, ramp) in referred {
        owner.entry(id).or_insert(ramp);
    }
    if owner.is_empty() {
        return Ok(());
    }
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: owner.keys().copied().collect(),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let Some(ramp) = owner.get(&id) else {
                return Ok(());
            };
            let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                *classes.entry(object.class_name.as_str()).or_default() += 1;
            }
            let census: Vec<String> = classes
                .iter()
                .map(|(name, count)| format!("{name}={count}"))
                .collect();
            let class = record.class_name.clone().unwrap_or_default();
            println!(
                "symbol {ramp} {id} record {class} objects {}",
                census.join(" ")
            );
            for object in &graph.objects {
                if let Some(fields) = object.fields.as_object() {
                    for (name, value) in fields {
                        let cut: String = value.to_string().chars().take(400).collect();
                        println!(
                            "symbol {ramp} {id} field {} {name} {cut}",
                            object.class_name
                        );
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}
