//! #323: what a Revit 2025 stair's and its parts' native records hold that
//! Revit's export names them by (RE-65: `Family:Stair:ElementId`, a part's
//! family from its type's construction, `<stair> Run 2`).
//!
//! FACT under test: on Revit 2025, where RE-65's byte anchors are absent,
//! the native records of a `Stair`, `StairsRun`, `StairsLanding` and
//! `StairsStringer` and of every element id they hold (their types) carry
//! the type names, construction and monolithic flags the names are made of.
//!
//! Output:
//!   elem <id> <class>
//!   native <id> <object index> <record class>/<object class> <member=value ...>
//!   held <id> <object index> <record class>/<object class> <member=value ...>
//!
//! `native` lines are the parts' own records; `held` lines the records of
//! the element ids those name. `tools/re/stair_names_list.py` prints Revit's
//! `Name` and `ObjectType` for each `elem` beside them.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_323_stair_names_2025 -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::walker::iter_elements;
use serde_json::Value;

const CLASSES: &[&str] = &["Stair", "StairsRun", "StairsLanding", "StairsStringer"];

fn members(fields: &Value, held: &mut BTreeSet<u64>) -> String {
    fields
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .filter_map(|(name, value)| match value {
            Value::Number(_) | Value::Bool(_) => Some(format!("{name}={value}")),
            Value::String(text) => Some(format!("{name}={}", text.replace(' ', "_"))),
            _ => {
                let id = value.get("m_id")?.get("m_id64")?.as_u64()?;
                held.insert(id);
                Some(format!("{name}=@{id}"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let mut wanted: BTreeSet<u64> = BTreeSet::new();
    for element in iter_elements(&mut rf)? {
        if let (Some(id), true) = (element.id, CLASSES.contains(&element.class.as_str())) {
            println!("elem {id} {}", element.class);
            wanted.insert(u64::from(id));
        }
    }
    if wanted.is_empty() {
        return Ok(());
    }
    let mut held: BTreeSet<u64> = BTreeSet::new();
    for (selected, label) in [(wanted.clone(), "native"), (BTreeSet::new(), "held")] {
        let selected = if label == "held" {
            held.difference(&wanted).copied().collect()
        } else {
            selected
        };
        if selected.is_empty() {
            continue;
        }
        let options = Options {
            selected_ids: selected,
            ..Options::default()
        };
        let mut found: BTreeSet<u64> = BTreeSet::new();
        extract_graphs(&mut rf, &options, |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let id = record.identity.element_id;
            for (index, object) in graph.objects.iter().enumerate() {
                let line = members(&object.fields, &mut found);
                println!("{label} {id} {index} {class}/{} {line}", object.class_name);
            }
            Ok(())
        })?;
        if label == "native" {
            held = found;
        }
    }
    Ok(())
}
