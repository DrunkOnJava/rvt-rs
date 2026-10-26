//! Which curtain mullions Revit's export draws as an extrusion of their
//! section along their length (#1016, RE-243).
//!
//! A curtain mullion's `Qto_MemberBaseQuantities.NetVolume` is its solid's
//! volume. Revit's export draws a mullion as an extrusion of its section,
//! whose volume is that section's area times its length, except where the
//! mullion is curved or cut: on the BIM4LCA office as built every mullion it
//! writes as a mesh stands on a curved curtain wall (an `ArcWall` record) or
//! on one whose record holds an edited profile (an `ExtrusionGStep`). A
//! mullion's host is the `m_hostId` and its type the `m_masterSymbolId` its
//! native record names; a type whose `SysMullionFamSym.m_profileId` is set
//! draws a profile family's section rather than its own.

use crate::{RevitFile, native_document};
use std::collections::{BTreeMap, BTreeSet};

/// `{"m_id":{"m_id64":n}}` as `n`, signed.
fn element_id(value: &serde_json::Value) -> Option<i64> {
    value
        .pointer("/m_id/m_id64")
        .and_then(serde_json::Value::as_i64)
}

/// The first id `field` of any of `graph`'s objects names, where positive.
fn linked(graph: &crate::native_parameters::ObjectGraph, field: &str) -> Option<u64> {
    graph
        .objects
        .iter()
        .find_map(|object| object.fields.get(field).and_then(element_id))
        .and_then(|id| u64::try_from(id).ok())
}

/// The mullions of `mullions` whose host, the curtain wall their native
/// record names, is a straight wall (`SWall`) whose record holds neither an
/// `ArcWall` nor an `ExtrusionGStep`, and whose type's record
/// (`SysMullionFamSym`) sets no profile, read in one pass. A mullion, host
/// or type whose copies disagree is left out.
pub fn mullions_on_plain_hosts(
    rf: &mut RevitFile,
    mullions: &BTreeSet<u64>,
) -> anyhow::Result<BTreeSet<u64>> {
    if mullions.is_empty() {
        return Ok(BTreeSet::new());
    }
    let options = native_document::Options {
        selected_ids: mullions.clone(),
        selected_classes: ["SWall", "ArcWall", "SysMullionFamSym"]
            .map(String::from)
            .into_iter()
            .collect(),
        ..native_document::Options::default()
    };
    let mut links: BTreeMap<u64, Option<(u64, u64)>> = BTreeMap::new();
    let mut plain: BTreeMap<u64, Option<bool>> = BTreeMap::new();
    native_document::extract_graphs(rf, &options, |record| {
        let (Some(class), Some(graph)) = (record.class_name.as_deref(), record.graph.as_ref())
        else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match class {
            "SWall" | "ArcWall" => {
                let straight = class == "SWall"
                    && !graph.objects.iter().any(|object| {
                        matches!(object.class_name.as_str(), "ArcWall" | "ExtrusionGStep")
                    });
                agree(&mut plain, id, Some(straight));
            }
            "SysMullionFamSym" => {
                let own = graph
                    .objects
                    .iter()
                    .find(|object| object.class_name == "SysMullionFamSym")
                    .and_then(|object| object.fields.get("m_profileId").and_then(element_id))
                    .map(|profile| profile < 0);
                agree(&mut plain, id, own);
            }
            _ => {}
        }
        if mullions.contains(&id) {
            let link = linked(graph, "m_hostId").zip(linked(graph, "m_masterSymbolId"));
            agree(&mut links, id, link);
        }
        Ok(())
    })?;
    let is_plain = |id: &u64| plain.get(id).copied().flatten() == Some(true);
    Ok(links
        .into_iter()
        .filter_map(|(mullion, link)| {
            let (host, mullion_type) = link?;
            (is_plain(&host) && is_plain(&mullion_type)).then_some(mullion)
        })
        .collect())
}

/// Record `value` for `id`, or `None` once two copies disagree or one has
/// none.
fn agree<T: PartialEq>(map: &mut BTreeMap<u64, Option<T>>, id: u64, value: Option<T>) {
    match map.get_mut(&id) {
        None => {
            map.insert(id, value);
        }
        Some(held) => {
            if *held != value {
                *held = None;
            }
        }
    }
}
