//! RE-390 (#1624): what a railing's own record, and its stair's, hold that
//! tells whether Revit's export aggregates the railing under the stair.
//!
//! FACT under test: a railing whose record names a stair
//! (`BaseRailing.m_stairsId`, RE-254) is aggregated under that stair in Revit's
//! export on every railing of the BIM4LCA office but one: as built, Tag 2598405
//! (a handrail on Level_01) is a plain element of its storey while its stair,
//! 2598347, is in the Basement and aggregates the two other railings that name
//! it. Hypothesis: a field of that railing's record, or of its stair's, tells
//! it from the others.
//!
//! Output, for every railing the export writes and every stair one names:
//!   railing <tag> record <class> stair <stair id or -> objects <class=count ...>
//!   railing <tag> field <object class> <name> <value as JSON, cut at 300 characters>
//!   stair <id> record <class> objects <class=count ...>
//!   stair <id> field StairsElement <name> <value as JSON, cut at 300 characters>
//! The fields of a railing are those of the first 40 objects of its graph; a
//! stair's are those of its `StairsElement`.
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_re390_railing_stairs -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::graph_value::Fv;
use rvt::research::native_document::{Options, extract_graphs};

/// The stair a `BaseRailing`'s fields name.
fn stair_of(fields: &Fv) -> Option<u64> {
    let id = fields
        .get("m_stairsId")?
        .pointer("/m_id/m_id64")?
        .as_i64()?;
    u64::try_from(id).ok()
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let ids: BTreeSet<u64> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } if ifc_type.eq_ignore_ascii_case("IFCRAILING") => tag.parse().ok(),
            _ => None,
        })
        .collect();
    let mut stairs: BTreeSet<u64> = BTreeSet::new();
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: ids,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                *classes.entry(object.class_name.as_str()).or_default() += 1;
            }
            let stair = graph
                .objects
                .iter()
                .find(|object| object.class_name == "BaseRailing")
                .and_then(|object| stair_of(&object.fields));
            if let Some(stair) = stair {
                stairs.insert(stair);
            }
            let census: Vec<String> = classes
                .iter()
                .map(|(name, count)| format!("{name}={count}"))
                .collect();
            println!(
                "railing {id} record {} stair {} objects {}",
                record.class_name.clone().unwrap_or_default(),
                stair.map_or("-".to_string(), |stair| stair.to_string()),
                census.join(" ")
            );
            for object in graph.objects.iter().take(40) {
                if let Some(fields) = object.fields.as_object() {
                    for (name, value) in fields {
                        let cut: String = value.to_string().chars().take(300).collect();
                        println!("railing {id} field {} {name} {cut}", object.class_name);
                    }
                }
            }
            Ok(())
        },
    )?;
    extract_graphs(
        &mut rf,
        &Options {
            selected_ids: stairs,
            ..Options::default()
        },
        |record| {
            let Some(graph) = record.graph.as_ref() else {
                return Ok(());
            };
            let id = record.identity.element_id;
            let mut classes: BTreeMap<&str, usize> = BTreeMap::new();
            for object in &graph.objects {
                *classes.entry(object.class_name.as_str()).or_default() += 1;
            }
            let census: Vec<String> = classes
                .iter()
                .map(|(name, count)| format!("{name}={count}"))
                .collect();
            println!(
                "stair {id} record {} objects {}",
                record.class_name.clone().unwrap_or_default(),
                census.join(" ")
            );
            for object in graph
                .objects
                .iter()
                .filter(|object| object.class_name == "StairsElement")
            {
                if let Some(fields) = object.fields.as_object() {
                    for (name, value) in fields {
                        let cut: String = value.to_string().chars().take(300).collect();
                        println!("stair {id} field StairsElement {name} {cut}");
                    }
                }
            }
            Ok(())
        },
    )?;
    Ok(())
}
