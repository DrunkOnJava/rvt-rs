//! #824: what the BIM4LCA office's roof hatches are hosted by, for the four
//! openings Revit's export cuts in roof 2991742 at them.
//!
//! FACT under test: each roof hatch's `FamilyInstance.m_hostId` names the
//! element whose face hosts it, and that element leads to roof 2991742.
//!
//! Output, for every id in `RVT_PROBE_IDS` (default the four hatches, their
//! two named hosts and the roof) and every element id those records hold:
//!   class <id> <element class> box <location x y z> <width depth height>
//!   native <id> <object index> <record class>/<object class> <member=value ...>
//!   held <id> <object index> <record class>/<object class> <member=value ...>
//!
//! Usage:
//!   cargo run --profile ci --example probe_824_hatch_hosts -- MODEL.rvt

use std::collections::BTreeSet;

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;

fn members(fields: &Value, held: &mut BTreeSet<u64>) -> String {
    fields
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .filter_map(|(name, value)| match value {
            Value::Number(_) | Value::Bool(_) => Some(format!("{name}={value}")),
            Value::String(text) => Some(format!("{name}={}", text.replace(' ', "_"))),
            Value::Array(_) => {
                let text = value.to_string();
                Some(format!("{name}={}", &text[..text.len().min(160)]))
            }
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
    let wanted: BTreeSet<u64> = std::env::var("RVT_PROBE_IDS")
        .unwrap_or_else(|_| "2053330,2053983,2896990,2896992,3079516,3073092,2991742".into())
        .split(',')
        .filter_map(|id| id.trim().parse().ok())
        .collect();
    let mut rf = RevitFile::open(&path)?;
    let mut held: BTreeSet<u64> = BTreeSet::new();
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for (round, label) in ["native", "held"].iter().enumerate() {
        let selected: BTreeSet<u64> = if round == 0 {
            wanted.clone()
        } else {
            held.difference(&seen).copied().collect()
        };
        if selected.is_empty() {
            continue;
        }
        seen.extend(selected.iter().copied());
        let options = Options {
            selected_ids: selected,
            ..Options::default()
        };
        let mut found = BTreeSet::new();
        extract_graphs(&mut rf, &options, |record| {
            let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
            else {
                return Ok(());
            };
            let id = record.identity.element_id;
            for (index, object) in graph.objects.iter().enumerate().take(10) {
                let line = members(&object.fields, &mut found);
                println!("{label} {id} {index} {class}/{} {line}", object.class_name);
            }
            Ok(())
        })?;
        held.extend(found);
    }
    for element in iter_elements(&mut rf)? {
        let Some(id) = element.id.map(u64::from).filter(|id| seen.contains(id)) else {
            continue;
        };
        let float = |wanted: &str| {
            element.fields.iter().find_map(|(name, value)| match value {
                InstanceField::Float { value, .. } if name == wanted => Some(*value),
                _ => None,
            })
        };
        let values: Vec<String> = [
            "m_locationX",
            "m_locationY",
            "m_locationZ",
            "m_bboxWidth",
            "m_bboxDepth",
            "m_bboxHeight",
        ]
        .iter()
        .map(|name| float(name).map_or("-".into(), |v| format!("{v:.6}")))
        .collect();
        println!("class {id} {} box {}", element.class, values.join(" "));
    }
    Ok(())
}
