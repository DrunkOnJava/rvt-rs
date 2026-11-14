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

/// A record's standard stair and its `(index, ElementId)` pairs, before the
/// sets are cut out of them.
type RawGroup = (Option<u64>, Vec<(i64, u64)>);

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
///
/// One read of the file's records, which keeps no record index on the handle
/// ([`Options::index_records`]): the export calls it once, while the handle
/// still holds the partitions' inflated copies, and a read made after the
/// saved-graphics pass has let them go inflates the whole file again (+408 MiB
/// of peak and +1 s on Snowdon Towers architectural, #1977).
pub fn multistory_groups(rf: &mut RevitFile) -> anyhow::Result<Vec<MultistoryGroup>> {
    let classes: BTreeSet<String> = ["MultistoryStairs", "StairsElement"]
        .into_iter()
        .map(str::to_string)
        .collect();
    let options = Options {
        selected_classes: classes.clone(),
        graph_classes: Some(classes),
        releases: crate::native_document::NATIVE_RELEASES,
        index_records: false,
        ..Options::default()
    };
    // Each group's pairs, (index, ElementId) in index order, and its standard
    // stair; each stair's base and top Level.
    let mut groups: BTreeMap<u64, RawGroup> = BTreeMap::new();
    let mut levels: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    extract_graphs(rf, &options, |record| {
        let Some(graph) = &record.graph else {
            return Ok(());
        };
        let id = record.identity.element_id;
        match record.class_name.as_deref() {
            Some("MultistoryStairs") => {
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
            }
            Some("StairsElement") => {
                let ends = graph.objects.first().map(|root| {
                    (
                        root.fields.get("m_baseLevelId").and_then(id_of),
                        root.fields.get("m_topLevelId").and_then(id_of),
                    )
                });
                if let Some((Some(base), Some(top))) = ends {
                    levels.insert(id, (base, top));
                }
            }
            _ => {}
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
