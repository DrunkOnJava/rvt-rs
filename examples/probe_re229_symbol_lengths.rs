//! RE-229 (#810): every length the symbol that draws a door or window
//! stores, named, for finding where Revit's `OverallWidth` comes from where
//! neither the cut loop (RE-215) nor the record box (RE-218) gives it.
//!
//! FACT under test: on the BIM4LCA office, doors 1756089 and 1765267 are
//! 5.8629 ft wide in Revit's export; their record box is 5.9766 ft and their
//! symbol's cut solid 3.1168 ft (RE-225). RE-221 found a hollow section's
//! sizes among its type's family parameters. Hypothesis: the width revit-ifc's
//! `GetMinSymbolWidth` measures is one of the symbol's stored lengths.
//!
//! Output, one line per door or window the export writes:
//!   len <instance> <IFC class> <symbol> <parameter id>:<name>=<feet> ...
//! the name a family parameter's `ParamDefValue.m_caption`, `-` for a
//! BuiltInParameter (negative id) or a parameter not resolved; a value every
//! partition does not agree on is left out.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re229_symbol_lengths -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_type_parameters::partition_length_entries;
use serde_json::Value;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u64, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if upper == "IFCDOOR" || upper == "IFCWINDOW" {
                if let Ok(id) = tag.parse() {
                    classes.insert(id, upper);
                }
            }
        }
    }
    let mut drawn_by: BTreeMap<u64, u64> = BTreeMap::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: classes.keys().copied().collect(),
            ..Options::default()
        },
        |record| {
            if let Some(symbol) = record.graph.as_ref().and_then(|graph| {
                graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "InstanceInfo")?
                    .fields
                    .get("m_symbolId")?
                    .pointer("/m_id/m_id64")?
                    .as_i64()
            }) {
                if let Ok(symbol) = u64::try_from(symbol) {
                    drawn_by.insert(record.identity.element_id, symbol);
                }
            }
            Ok(())
        },
    )?;
    let symbols: BTreeSet<u32> = drawn_by
        .values()
        .filter_map(|s| u32::try_from(*s).ok())
        .collect();
    let declared = declared_ids(&parse_records(&mut rf)?);
    let mut values: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in partition_length_entries(inflated.bytes(), &declared) {
            if !symbols.contains(&owner) {
                continue;
            }
            for (parameter, value) in entries {
                values
                    .entry((owner, parameter))
                    .and_modify(|seen| {
                        if *seen != Some(value) {
                            *seen = None;
                        }
                    })
                    .or_insert(Some(value));
            }
        }
    }
    let parameters: BTreeSet<u64> = values
        .keys()
        .filter_map(|(_, p)| u64::try_from(*p).ok())
        .filter(|p| *p > 0)
        .collect();
    let mut names: BTreeMap<i64, String> = BTreeMap::new();
    if !parameters.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: parameters,
                ..Options::default()
            },
            |record| {
                let caption = record.graph.as_ref().and_then(|graph| {
                    graph
                        .objects
                        .iter()
                        .find(|o| o.class_name == "ParamDefValue")?
                        .fields
                        .get("m_caption")
                        .and_then(Value::as_str)
                        .map(str::to_string)
                });
                if let (Some(caption), Ok(id)) =
                    (caption, i64::try_from(record.identity.element_id))
                {
                    names.insert(id, caption.replace(' ', "_"));
                }
                Ok(())
            },
        )?;
    }
    for (instance, class) in &classes {
        let Some(symbol) = drawn_by.get(instance) else {
            println!("len {instance} {class} -");
            continue;
        };
        let owner = u32::try_from(*symbol).unwrap_or(0);
        let entries: Vec<String> = values
            .range((owner, i64::MIN)..=(owner, i64::MAX))
            .filter_map(|((_, parameter), value)| {
                let value = (*value)?;
                let name = names.get(parameter).map_or("-", String::as_str);
                Some(format!("{parameter}:{name}={value:.6}"))
            })
            .collect();
        println!("len {instance} {class} {symbol} {}", entries.join(" "));
    }
    eprintln!(
        "{} doors and windows, {} symbols, {} lengths, {} named",
        classes.len(),
        symbols.len(),
        values.len(),
        names.len()
    );
    Ok(())
}
