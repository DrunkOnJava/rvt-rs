//! RE-269 (#35, W7): which curtain mullions and panels Revit's export gives
//! `IsExternal` true, replayed from the exporter's cache.
//!
//! FACT under test: revit-ifc's `CategoryUtil.IsElementExternal` (CLEANROOM.md
//! source 5) caches every value under an ElementId in one dictionary, and at
//! IFC_v24.1.0 and IFC_v24.2.0 it also accepts an element type, which it
//! caches as not external where the type has no Function. A curtain wall's
//! property sets are created when its wrapper is disposed, instances first
//! (`ProductWrapper.Dispose`: `CreateInternalRevitPropertySets`), then the
//! types (`CreateElementTypeProperties`). So the members of the first curtain
//! wall that uses a member type get the wall's value (a family instance takes
//! its host's), and the type then caches false for the members of every later
//! curtain wall. On the BIM4LCA office this reproduces Revit's export on
//! every member (RE-241's counts: 498 of 10,003 mullions and 30 of 260 panels
//! true). Verify with `tools/re/member_is_external_cache_vs_ifc.py <probe
//! output> <Revit's export>`.
//!
//! Output: a first line with the file's Revit build and release, then one JSON
//! line per `IfcMember` and `IfcPlate` the export writes:
//!   {"member": <Tag>, "ifc": <IFC class>, "type": <m_masterSymbolId>,
//!    "host": <m_hostId>, "host_class": <native class of the host>,
//!    "host_function": <native Function of the host's type, RE-185>,
//!    "type_function": <the member type's FUNCTION_PARAM entry, RE-158>}
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_is_external_members -- MODEL.rvt

use std::collections::{BTreeMap, BTreeSet};

use rvt::RevitFile;
use rvt::ifc::RvtDocExporter;
use rvt::ifc::entities::IfcEntity;
use rvt::research::native_document::{Options, extract_graphs};
use rvt::research::native_parameters::ObjectGraph;
use rvt::research::native_wall_types;
use rvt::research::partition_room_parameters::{FUNCTION_PARAMETER, scan_integer_parameter};
use serde_json::json;

fn linked(graph: &ObjectGraph, field: &str) -> Option<u64> {
    graph.objects.iter().find_map(|object| {
        object
            .fields
            .get(field)?
            .pointer("/m_id/m_id64")?
            .as_i64()
            .and_then(|id| u64::try_from(id).ok())
    })
}

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: MODEL.rvt");
    let mut rf = RevitFile::open(&path)?;
    let info = rf.basic_file_info()?;
    println!(
        "{}",
        json!({ "build": info.build.as_deref(), "release": info.version })
    );
    let model = RvtDocExporter.export_with_diagnostics(&mut rf)?.model;
    let classes: BTreeMap<u64, String> = model
        .entities
        .iter()
        .filter_map(|entity| match entity {
            IfcEntity::BuildingElement {
                ifc_type,
                type_guid: Some(tag),
                ..
            } => {
                let upper = ifc_type.to_ascii_uppercase();
                if upper == "IFCMEMBER" || upper == "IFCPLATE" {
                    Some((tag.parse::<u64>().ok()?, upper))
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect();
    let ids: BTreeSet<u64> = classes.keys().copied().collect();
    let mut links: BTreeMap<u64, [Option<u64>; 2]> = BTreeMap::new();
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
            links.insert(
                record.identity.element_id,
                [linked(graph, "m_masterSymbolId"), linked(graph, "m_hostId")],
            );
            Ok(())
        },
    )?;
    let hosts: BTreeSet<u64> = links.values().filter_map(|[_, host]| *host).collect();
    let mut host_classes: BTreeMap<u64, String> = BTreeMap::new();
    if !hosts.is_empty() {
        extract_graphs(
            &mut rf,
            &Options {
                selected_ids: hosts.clone(),
                ..Options::default()
            },
            |record| {
                if let Some(class) = record.class_name.as_deref() {
                    host_classes.insert(record.identity.element_id, class.to_string());
                }
                Ok(())
            },
        )?;
    }
    let functions = native_wall_types::wall_type_functions(&mut rf, &hosts).unwrap_or_default();
    let owners: BTreeSet<u32> = links
        .values()
        .filter_map(|[kind, _]| *kind)
        .filter_map(|kind| u32::try_from(kind).ok())
        .collect();
    let type_functions = scan_integer_parameter(&mut rf, info.version, &owners, FUNCTION_PARAMETER);
    for (tag, ifc) in &classes {
        let [kind, host] = links.get(tag).copied().unwrap_or([None, None]);
        println!(
            "{}",
            json!({
                "member": tag,
                "ifc": ifc,
                "type": kind,
                "host": host,
                "host_class": host.and_then(|h| host_classes.get(&h)),
                "host_function": host.and_then(|h| functions.get(&h)),
                "type_function": kind
                    .and_then(|k| u32::try_from(k).ok())
                    .and_then(|k| type_functions.get(&k)),
            })
        );
    }
    eprintln!(
        "{} members and plates, {} links",
        classes.len(),
        links.len()
    );
    Ok(())
}
