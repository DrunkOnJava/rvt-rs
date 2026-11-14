//! The sets of a multistory stair, read by member name from its native
//! records (#1977, RE-463).
//!
//! Revit's export writes a multistory stair once per storey: the stair on
//! its own Level and a copy, with its own parts, on each Level above that the
//! sets of the group leave uncovered. The file keeps the group in a record of
//! the class `MultistoryStairs`:
//!
//! - `m_standardStairsId` names the stair that repeats;
//! - its `ElementIdIndexPairSet` lists `(index, ElementId)` pairs that fall in
//!   sets of 14 indexes, one stair with its landing, runs and supports;
//! - the `StairsElement` of each set names its base and top Level
//!   (`m_baseLevelId`, `m_topLevelId`).
//!
//! Nothing in the records says how far the stair is repeated:
//! `m_multistoryUpToLevelId` is -1 on every stair and the group's
//! `MultistoryStairsDataCell` is not among its objects (RE-463), so the
//! span a copy fills is inferred from the Levels the sets span.

use std::collections::{BTreeMap, BTreeSet};

use crate::RevitFile;
use crate::graph_value::Fv;
use crate::native_document::{Options, extract_graphs};

/// Releases these records are measured on: Autodesk's Snowdon Towers
/// architectural sample, the one file with a Revit export of a multistory
/// stair (RE-463, RE-353).
pub const MULTISTORY_REVIT_VERSIONS: &[u32] = &[2024];

/// How many indexes one set of a group spans (RE-463).
const SET_INDEXES: i64 = 14;

/// One set of a group: a stair and the Levels it spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StairSet {
    /// The set's number in its group: its members' index divided by 14.
    pub number: i64,
    /// The `StairsElement` among the set's members.
    pub stair: u64,
    /// The Level the stair starts on.
    pub base_level: u64,
    /// The Level the stair ends on.
    pub top_level: u64,
}

/// A `MultistoryStairs` record and the sets that hold a stair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MultistoryGroup {
    /// The record's ElementId.
    pub id: u64,
    /// The stair the record names as the one that repeats.
    pub standard_stair: Option<u64>,
    /// The sets with a stair whose base and top Level are read, by number.
    pub sets: Vec<StairSet>,
}

/// The ElementId in `{"m_id": {"m_id": n}}` or `{"m_id": {"m_id64": n}}`.
fn id_of(value: &Fv) -> Option<u64> {
    let inner = value.get("m_id")?;
    inner
        .get("m_id")
        .or_else(|| inner.get("m_id64"))?
        .as_i64()
        .and_then(|id| u64::try_from(id).ok())
        .filter(|id| *id > 0)
}

/// The groups of every multistory stair in the file, with the sets that hold a
/// stair. A file with no such record gives none; a set whose stair names no
/// base or top Level is left out.
pub fn multistory_groups(rf: &mut RevitFile) -> anyhow::Result<Vec<MultistoryGroup>> {
    let options = Options {
        selected_classes: ["MultistoryStairs".to_string()].into_iter().collect(),
        graph_classes: Some(["MultistoryStairs".to_string()].into_iter().collect()),
        releases: crate::native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    // Each record's pairs, (index, ElementId) in index order, and its standard stair.
    let mut groups: BTreeMap<u64, (Option<u64>, Vec<(i64, u64)>)> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let id = record.identity.element_id;
        let mut standard = None;
        let mut pairs: Vec<(i64, u64)> = Vec::new();
        for object in &graph.objects {
            if object.class_name == "MultistoryStairs" {
                standard = object.fields.get("m_standardStairsId").and_then(id_of);
            }
            if object.class_name == "ElementIdIndexPairSet" {
                for entry in object
                    .fields
                    .get("m_data")
                    .and_then(Fv::as_array)
                    .into_iter()
                    .flatten()
                {
                    if let (Some(index), Some(member)) = (
                        entry.get("m_index").and_then(Fv::as_i64),
                        entry.get("m_elementId").and_then(id_of),
                    ) {
                        pairs.push((index, member));
                    }
                }
            }
        }
        pairs.sort_unstable();
        groups.insert(id, (standard, pairs));
        Ok(())
    })?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }

    // The Levels of every member that is a stair.
    let members: BTreeSet<u64> = groups
        .values()
        .flat_map(|(_, pairs)| pairs.iter().map(|(_, member)| *member))
        .collect();
    let options = Options {
        selected_ids: members,
        graph_classes: Some(["StairsElement".to_string()].into_iter().collect()),
        releases: crate::native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let mut levels: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        if record.class_name.as_deref() != Some("StairsElement") {
            return Ok(());
        }
        let Some(root) = record
            .graph
            .as_ref()
            .and_then(|graph| graph.objects.first())
        else {
            return Ok(());
        };
        if let (Some(base), Some(top)) = (
            root.fields.get("m_baseLevelId").and_then(id_of),
            root.fields.get("m_topLevelId").and_then(id_of),
        ) {
            levels.insert(record.identity.element_id, (base, top));
        }
        Ok(())
    })?;

    let mut out = Vec::new();
    for (id, (standard_stair, pairs)) in groups {
        let mut sets: BTreeMap<i64, Vec<u64>> = BTreeMap::new();
        for (index, member) in pairs {
            sets.entry(index.div_euclid(SET_INDEXES))
                .or_default()
                .push(member);
        }
        let sets = sets
            .into_iter()
            .filter_map(|(number, members)| {
                // A set holds one stair; one that holds none, or two, is left out.
                let mut stairs = members.iter().filter(|member| levels.contains_key(*member));
                let (Some(&stair), None) = (stairs.next(), stairs.next()) else {
                    return None;
                };
                let &(base_level, top_level) = levels.get(&stair)?;
                Some(StairSet {
                    number,
                    stair,
                    base_level,
                    top_level,
                })
            })
            .collect();
        out.push(MultistoryGroup {
            id,
            standard_stair,
            sets,
        });
    }
    Ok(out)
}
