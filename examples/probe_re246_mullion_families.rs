//! RE-246 (#1016): what in a curtain mullion type's family says its kind
//! (rectangular, circular, corner), whatever language its name is in.
//!
//! FACT under test: RE-243 writes a mullion's `NetVolume` only for the
//! Rectangular Mullion and Quad Corner Mullion families, matched by their
//! English names, since no field of the mullion type (`SysMullionFamSym`)
//! sets them apart. Hypothesis: the family its `m_familyId` names holds the
//! kind in a field, the same on every file.
//!
//! Output, one line per mullion type and one per family its types name:
//!   type <type id> family=<family id> name=<SymbolInfo.m_name> classes=<class,...>
//!   typefields <type id> <object>.<field>=<value> ...
//!   family <family id> <record class> <object>.<field>=<value> ...
//! every number, boolean and short string of the family record's first 40
//! objects, spaces as `_`.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re246_mullion_families -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use serde_json::Value;

const OBJECTS: usize = 40;

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::String(s) if s.len() <= 60 => Some(s.replace(' ', "_")),
        Value::Object(_) => value
            .pointer("/m_id/m_id64")
            .and_then(Value::as_i64)
            .map(|id| format!("@{id}")),
        _ => None,
    }
}

fn scalars(graph: &ObjectGraph) -> Vec<String> {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    let mut parts = Vec::new();
    for object in graph.objects.iter().take(OBJECTS) {
        let n = seen.entry(object.class_name.as_str()).or_default();
        *n += 1;
        let label = if *n == 1 {
            object.class_name.clone()
        } else {
            format!("{}#{n}", object.class_name)
        };
        let Some(fields) = object.fields.as_object() else {
            continue;
        };
        for (name, value) in fields {
            if let Some(text) = scalar(value) {
                parts.push(format!("{label}.{name}={text}"));
            }
        }
    }
    parts
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut families: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_classes: BTreeSet::from(["SysMullionFamSym".to_string()]),
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let family = graph
                .objects
                .iter()
                .find(|object| object.class_name == "SysMullionFamSym")
                .and_then(|object| object.fields.get("m_familyId"))
                .and_then(|value| value.pointer("/m_id/m_id64"))
                .and_then(Value::as_i64)
                .and_then(|id| u64::try_from(id).ok());
            let name = graph
                .objects
                .iter()
                .find(|object| object.class_name == "SymbolInfo")
                .and_then(|object| object.fields.get("m_name"))
                .and_then(Value::as_str)
                .unwrap_or("-")
                .replace(' ', "_");
            let classes: BTreeSet<&str> = graph
                .objects
                .iter()
                .map(|object| object.class_name.as_str())
                .collect();
            println!(
                "type {} family={} name={name} classes={}",
                record.identity.element_id,
                family.map_or("-".into(), |f| f.to_string()),
                classes.into_iter().collect::<Vec<_>>().join(",")
            );
            if name != "-" {
                println!(
                    "typefields {} {}",
                    record.identity.element_id,
                    scalars(graph).join(" ")
                );
            }
            families.extend(family);
            Ok(())
        },
    )?;
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: families,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            println!(
                "family {} {} {}",
                record.identity.element_id,
                record.class_name.as_deref().unwrap_or("-"),
                scalars(graph).join(" ")
            );
            Ok(())
        },
    )?;
    Ok(())
}
