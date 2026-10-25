//! W6: what decides where a BIM4LCA office wall ends at a join.
//!
//! FACT under test: on the office 195 of 607 walls end up to 1.65 ft from
//! Revit's body (`tools/re/wall_ends_list.py`): exterior walls of type US-1
//! cut back 1.3287 ft at an end Revit runs through, and partitions (VS-*)
//! drawn past an end Revit stops at. Revit's export of the office writes no
//! `IfcRelConnectsPathElements`, so each end's join is to be read from the
//! file. This prints, for a few such walls, every member of every object of
//! their native records, and for each wall their element record names (its
//! join partners, RE-29 §2) the walker's line and box fields and its native
//! root object, so the join data can be found by comparing a wall that is
//! cut with one that is not.
//!
//! Output:
//!   wall <id> <field>=<value> ...        the walker's numeric fields
//!   ref <id> <element id> <class>
//!   native <id> <object index> <class>/<object class> <member>=<json> ...
//!
//! Usage:
//!   cargo run --profile ci --example probe_office_wall_joins -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_element_records::{REFERENCE_LIST_OFFSET, decode_reference_list};
use rvt::walker::{InstanceField, iter_elements};
use serde_json::Value;

/// 1348465 and 1348538: US-1, cut 1.3287 ft short at both ends; 1348462:
/// short at its end only; 1436343: VS-2, 1.6538 ft long at its start;
/// 3039425: US-2, 1.4928 ft short at its start.
const WALLS: &[u32] = &[1348465, 1348538, 1348462, 1436343, 3039425];

const MEMBER_CHARS: usize = 700;

fn members(fields: &Value) -> String {
    fields
        .as_object()
        .into_iter()
        .flat_map(|members| members.iter())
        .map(|(name, value)| {
            let text = value.to_string();
            let text: String = text.chars().take(MEMBER_CHARS).collect();
            format!("{name}={text}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let wanted: BTreeSet<u32> = WALLS.iter().copied().collect();
    let elements: Vec<_> = iter_elements(&mut rf)?.collect();
    let classes: BTreeMap<u64, String> = elements
        .iter()
        .filter_map(|e| Some((u64::from(e.id?), e.class.clone())))
        .collect();
    let mut partners: BTreeSet<u64> = BTreeSet::new();
    let mut walls: BTreeSet<u64> = wanted.iter().map(|&id| u64::from(id)).collect();
    for element in elements
        .iter()
        .filter(|e| e.id.is_some_and(|id| wanted.contains(&id)))
    {
        let id = element.id.unwrap_or_default();
        let mut stream = None;
        let mut offset = None;
        for (name, value) in &element.fields {
            match (name.as_str(), value) {
                ("m_source_stream", InstanceField::String(v)) => stream = Some(v.clone()),
                ("m_source_offset", InstanceField::Integer { value, .. }) => {
                    offset = usize::try_from(*value).ok();
                }
                _ => {}
            }
        }
        let (Some(stream), Some(offset)) = (stream, offset) else {
            continue;
        };
        let references = rf
            .inflated_partition(&stream)
            .ok()
            .and_then(|inflated| {
                decode_reference_list(inflated.bytes(), offset + REFERENCE_LIST_OFFSET)
            })
            .unwrap_or_default();
        for r in references {
            let class = classes.get(&r).map(String::as_str).unwrap_or("-");
            println!("ref {id} {r} {class}");
            if class == "Wall" {
                partners.insert(r);
            }
        }
    }
    walls.extend(partners.iter().copied());
    for element in elements
        .iter()
        .filter(|e| e.id.is_some_and(|id| walls.contains(&u64::from(id))))
    {
        let numeric: Vec<String> = element
            .fields
            .iter()
            .filter_map(|(name, value)| match value {
                InstanceField::Float { value, .. } => Some(format!("{name}={value:.6}")),
                InstanceField::Integer { value, .. } if !name.starts_with("m_source") => {
                    Some(format!("{name}={value}"))
                }
                InstanceField::ElementId { id, .. } => Some(format!("{name}=@{id}")),
                _ => None,
            })
            .collect();
        println!(
            "wall {} {}",
            element.id.unwrap_or_default(),
            numeric.join(" ")
        );
    }
    let options = Options {
        selected_ids: walls.clone(),
        ..Options::default()
    };
    extract_graphs(&mut rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let full = u32::try_from(id).is_ok_and(|id| wanted.contains(&id));
        for (index, object) in graph.objects.iter().enumerate() {
            if index > 0 && !full {
                break;
            }
            println!(
                "native {id} {index} {class}/{} {}",
                object.class_name,
                members(&object.fields)
            );
        }
        Ok(())
    })?;
    Ok(())
}
