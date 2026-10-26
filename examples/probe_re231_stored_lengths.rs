//! RE-231 (#35, W7): which stored length is a length property Revit's export
//! writes and rvt-rs does not.
//!
//! FACT under test: on the BIM4LCA office Revit's export writes
//! `Pset_RailingCommon.Height` (4 ft on 28 railings, 0.9 m on 13),
//! `Pset_BeamCommon.Span` and the `Pset_StairFlightCommon` lengths, which
//! rvt-rs does not. RE-221 and RE-229 found sizes among a type's stored
//! lengths (`f64 · ff×8 · i64` entries). Hypothesis: each of these
//! properties is one stored length of the element or of its type, under one
//! parameter.
//!
//! Output, one line per element the export writes (curtain mullions and
//! panels left out):
//!   len <tag> <IFC class> <type|-> <owner>:<parameter id>:<name>=<feet> ...
//! where `owner` is `i` for a length stored under the element's own
//! ElementId and `t` for one under its type's, `name` a family parameter's
//! `ParamDefValue.m_caption` and `-` for a BuiltInParameter (negative id);
//! a value every partition does not agree on is left out.
//!
//! Usage:
//!   cargo run --profile ci --example probe_re231_stored_lengths -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::elem_table::{declared_ids, parse_records};
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::partition_schema_mvp::{TYPE_ID_FIELD, recover_partition_schema_mvp};
use rvt::research::partition_type_parameters::partition_length_entries;
use rvt::walker::{InstanceField, WalkerLimits};
use serde_json::Value;

const LEFT_OUT: [&str; 2] = ["IFCMEMBER", "IFCPLATE"];

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let version = rf.basic_file_info()?.version;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let mut classes: BTreeMap<u32, String> = BTreeMap::new();
    for entity in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            type_guid: Some(tag),
            ..
        } = entity
        {
            let upper = ifc_type.to_ascii_uppercase();
            if LEFT_OUT.contains(&upper.as_str()) {
                continue;
            }
            if let Ok(id) = tag.parse() {
                classes.entry(id).or_insert(upper);
            }
        }
    }
    let mvp = recover_partition_schema_mvp(&mut rf, version, WalkerLimits::default())?;
    let mut types: BTreeMap<u32, u32> = BTreeMap::new();
    for element in mvp
        .columns
        .iter()
        .chain(&mvp.walls)
        .chain(&mvp.doors)
        .chain(&mvp.windows)
        .chain(&mvp.slabs)
        .chain(&mvp.products)
    {
        let Some(id) = element.id.filter(|id| classes.contains_key(id)) else {
            continue;
        };
        if let Some(type_id) = element.fields.iter().find_map(|(name, value)| match value {
            InstanceField::ElementId { id, .. } if name == TYPE_ID_FIELD => Some(*id),
            _ => None,
        }) {
            types.entry(id).or_insert(type_id);
        }
    }
    let owners: BTreeSet<u32> = classes
        .keys()
        .copied()
        .chain(types.values().copied())
        .collect();
    let declared = declared_ids(&parse_records(&mut rf)?);
    let mut values: BTreeMap<(u32, i64), Option<f64>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, entries) in partition_length_entries(inflated.bytes(), &declared) {
            if !owners.contains(&owner) {
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
    let lengths = |owner: u32, prefix: &str| -> Vec<String> {
        values
            .range((owner, i64::MIN)..=(owner, i64::MAX))
            .filter_map(|((_, parameter), value)| {
                let value = (*value)?;
                let name = names.get(parameter).map_or("-", String::as_str);
                Some(format!("{prefix}:{parameter}:{name}={value:.6}"))
            })
            .collect()
    };
    for (id, class) in &classes {
        let mut entries = lengths(*id, "i");
        let type_id = types.get(id);
        if let Some(type_id) = type_id {
            entries.extend(lengths(*type_id, "t"));
        }
        let type_id = type_id.map_or_else(|| "-".to_string(), u32::to_string);
        println!("len {id} {class} {type_id} {}", entries.join(" "));
    }
    eprintln!(
        "{} elements, {} with a type, {} lengths, {} named parameters",
        classes.len(),
        types.len(),
        values.len(),
        names.len()
    );
    Ok(())
}
