//! #976: what a curtain mullion's native records hold that Revit's export
//! draws its body from (its type's profile, its length along its grid
//! segment), on a sample of mullions.
//!
//! FACT under test: a `CurtainWallMullion`'s native record and its type's
//! give the profile (width on each side, thickness, shape) and the segment
//! Revit sweeps it along.
//!
//! Output, for every `RVT_PROBE_STEP`th mullion (default 400) in ElementId
//! order, and for every element id its records hold:
//!   elem <id> box <location x y z> <width depth height>
//!   native <id> <object index> <record class>/<object class> <member=value ...>
//!   held <id> <object index> <record class>/<object class> <member=value ...>
//!
//! Arrays print as JSON, at most 200 characters. `tools/re/mullion_bodies_list.py`
//! prints Revit's body of each `elem` beside them.
//!
//! Usage:
//!   cargo run --profile ci --example probe_976_mullions -- MODEL.rvt

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
                Some(format!("{name}={}", &text[..text.len().min(200)]))
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
    let step: usize = std::env::var("RVT_PROBE_STEP")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(400)
        .max(1);
    let mut rf = RevitFile::open(&path)?;
    let mut mullions: Vec<(u32, Vec<f64>)> = iter_elements(&mut rf)?
        .filter(|element| element.class == "CurtainWallMullion")
        .filter_map(|element| {
            let id = element.id?;
            let float = |wanted: &str| {
                element.fields.iter().find_map(|(name, value)| match value {
                    InstanceField::Float { value, .. } if name == wanted => Some(*value),
                    _ => None,
                })
            };
            let bbox: Option<Vec<f64>> = [
                "m_locationX",
                "m_locationY",
                "m_locationZ",
                "m_bboxWidth",
                "m_bboxDepth",
                "m_bboxHeight",
            ]
            .iter()
            .map(|name| float(name))
            .collect();
            Some((id, bbox.unwrap_or_default()))
        })
        .collect();
    mullions.sort_by_key(|(id, _)| *id);
    let sample: Vec<(u32, Vec<f64>)> = mullions.into_iter().step_by(step).collect();
    let wanted: BTreeSet<u64> = sample.iter().map(|(id, _)| u64::from(*id)).collect();
    for (id, bbox) in &sample {
        let text: Vec<String> = bbox.iter().map(|v| format!("{v:.6}")).collect();
        println!("elem {id} box {}", text.join(" "));
    }
    if wanted.is_empty() {
        return Ok(());
    }
    let mut held: BTreeSet<u64> = BTreeSet::new();
    let options = Options {
        selected_ids: wanted.clone(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate().take(12) {
            let line = members(&object.fields, &mut held);
            println!("native {id} {index} {class}/{} {line}", object.class_name);
        }
        Ok(())
    })?;
    let types: BTreeSet<u64> = held.difference(&wanted).copied().collect();
    let options = Options {
        selected_ids: types,
        ..Options::default()
    };
    let mut ignored = BTreeSet::new();
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        if !class.contains("Mullion") && !class.contains("Symbol") && !class.contains("Type") {
            return Ok(());
        }
        let id = record.identity.element_id;
        for (index, object) in graph.objects.iter().enumerate().take(8) {
            let line = members(&object.fields, &mut ignored);
            println!("held {id} {index} {class}/{} {line}", object.class_name);
        }
        Ok(())
    })?;
    Ok(())
}
