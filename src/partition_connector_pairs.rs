//! Which element each end of a duct or pipe is joined to (RE-138, #528).
//!
//! A duct's or pipe's object holds its connectors' joins after its
//! `RbsCurveConnectorManager` anchor (RE-134), as counted lists of
//! references:
//!
//! ```text
//! +0   u32 2
//! +4   u64 the duct or pipe itself   u32 connector index   u32 1
//! +20  u64 the element joined to it  u32 connector index   u32 1
//! ```
//!
//! A join is written in two blocks, the lists of the earlier at 1,021 to 1,130
//! bytes from the anchor and of the later at 1,217 to 1,362 on the RE1 models.
//! The earlier copy stores the duct's own connector index the other way round
//! from Revit's (0 for the connector Revit numbers 1, and 1 for 0); the later
//! copy stores it as Revit numbers it. A join has a later copy for most ends,
//! and on some the earlier block holds one-reference lists instead, so a list
//! in the later block is read as stored, and one in the earlier block is read
//! inverted when no later list gives the same join. The index of the element
//! joined at the other end is stored as Revit numbers it in both.
//!
//! The pairs are Revit's `IfcRelConnectsPorts` of the ports named
//! `<In or Out>Port_<ElementId>_<index>`, scored by
//! `tools/re/connector_pairs_vs_ifc.py`.
//!
//! A fitting's joins are written in the same list form, with the fitting as
//! the first reference, but not after an anchor of a fitting's own: they are
//! found by the list alone, anywhere in a partition (RE-141). A fitting's list
//! is written once and carries its own connector index as Revit numbers it
//! (a tee's are 1, 2 and 3), where a duct's or pipe's earlier copy is
//! inverted. A fitting's own record holds only the sorted ids of the elements
//! it refers to.
//!
//! A terminal's or a piece of equipment's joins are lists of the same form
//! with it first, both indices as Revit numbers them (RE-170): RE1 Plumbing's
//! fixtures to their pipes, in the later block after the pipe's anchor, and
//! RE1 Mechanical's air terminals to their duct fittings and its air handling
//! unit to two of its ducts and pipes, anywhere. A duct's or pipe's own lists
//! do not hold those joins.

use crate::{Result, RevitFile};
use std::collections::BTreeSet;

/// Releases where this layout is measured: the RE1 models.
pub const CONNECTOR_PAIR_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2025];

/// Whether `revit_version` is a release this layout is measured on.
pub fn supports_revit_version(revit_version: u32) -> bool {
    CONNECTOR_PAIR_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

/// The schema class the anchor is made of.
const CONNECTOR_MANAGER: &str = "RbsCurveConnectorManager";

/// First and last byte after an anchor a list may start at. The lists measured
/// start at 1,021 to 1,362.
const SCAN_FROM: usize = 900;
const SCAN_TO: usize = 1600;

/// A list at or after this many bytes from the anchor is in the later block:
/// the earlier block's lists end at 1,130 and the later block's start at 1,217
/// on RE1 Mechanical and Plumbing.
const LATER_BLOCK_FROM: usize = 1170;

/// A list holds two references: the curve's own connector and the one joined.
const LIST_ENTRIES: u32 = 2;
const ENTRY_BYTES: usize = 16;
const LIST_BYTES: usize = 4 + LIST_ENTRIES as usize * ENTRY_BYTES;

/// The `u32` after a reference's index.
const REFERENCE_FLAG: u32 = 1;

/// The value of a reference's element that names none.
const UNSET: u64 = u64::MAX;

/// A duct or pipe has two connectors, numbered 0 and 1.
const CURVE_CONNECTORS: u32 = 2;

/// A connector index of a fitting, terminal or piece of equipment, or of the
/// element it is joined to, is below this: RE1 Mechanical's air handling unit
/// numbers its connectors to 11.
const LISTED_CONNECTORS: u32 = 32;

/// One connector of a duct, pipe or fitting and the connector it is joined to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConnectorPair {
    /// The duct, pipe or fitting.
    pub element: u64,
    /// Its connector as Revit numbers it: 0 or 1 for a duct or pipe, from 1
    /// for a fitting.
    pub index: u32,
    /// The element joined to that connector.
    pub other: u64,
    /// That element's connector, as Revit numbers it.
    pub other_index: u32,
}

/// One list as stored.
struct RawList {
    at: usize,
    own: u64,
    own_index: u32,
    other: u64,
    other_index: u32,
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        buf.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn u64_at(buf: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        buf.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The element, connector index and flag of the reference at `at`.
fn reference_at(buf: &[u8], at: usize) -> Option<(u64, u32, u32)> {
    Some((
        u64_at(buf, at)?,
        u32_at(buf, at + 8)?,
        u32_at(buf, at + 12)?,
    ))
}

/// The list at `at` in `tail`, when it is a join of one of `curves`.
fn list_at(tail: &[u8], at: usize, curves: &BTreeSet<u64>) -> Option<RawList> {
    if u32_at(tail, at)? != LIST_ENTRIES {
        return None;
    }
    let (own, own_index, own_flag) = reference_at(tail, at + 4)?;
    let (other, other_index, other_flag) = reference_at(tail, at + 4 + ENTRY_BYTES)?;
    (curves.contains(&own)
        && own_index < CURVE_CONNECTORS
        && own_flag == REFERENCE_FLAG
        && other_flag == REFERENCE_FLAG
        && other != UNSET
        && other != own)
        .then_some(RawList {
            at,
            own,
            own_index,
            other,
            other_index,
        })
}

/// Every join of one of `curves` in the bytes from an anchor on.
fn lists_in(tail: &[u8], curves: &BTreeSet<u64>) -> Vec<RawList> {
    let mut out = Vec::new();
    let mut at = SCAN_FROM;
    while at < SCAN_TO {
        match list_at(tail, at, curves) {
            Some(list) => {
                out.push(list);
                at += LIST_BYTES;
            }
            None => at += 1,
        }
    }
    out
}

/// The connector pairs the lists of one anchor give. A list in the later block
/// carries the curve's own connector index as Revit numbers it; one in the
/// earlier block carries it inverted, and is used only where no list of the
/// later block gives the same join.
fn resolve(lists: &[RawList]) -> Vec<ConnectorPair> {
    let later: BTreeSet<(u64, u64, u32)> = lists
        .iter()
        .filter(|list| list.at >= LATER_BLOCK_FROM)
        .map(|list| (list.own, list.other, list.other_index))
        .collect();
    lists
        .iter()
        .filter_map(|list| {
            let in_later_block = list.at >= LATER_BLOCK_FROM;
            if !in_later_block && later.contains(&(list.own, list.other, list.other_index)) {
                return None;
            }
            let index = if in_later_block {
                list.own_index
            } else {
                CURVE_CONNECTORS - 1 - list.own_index
            };
            Some(ConnectorPair {
                element: list.own,
                index,
                other: list.other,
                other_index: list.other_index,
            })
        })
        .collect()
}

/// The connector pairs of `curves` (the ElementIds of ducts and pipes), sorted
/// and without repeats. Empty for a release this layout is not measured on.
pub fn scan_connector_pairs(
    rf: &mut RevitFile,
    curves: &BTreeSet<u64>,
) -> Result<Vec<ConnectorPair>> {
    if curves.is_empty() || !supports_revit_version(rf.basic_file_info()?.version) {
        return Ok(Vec::new());
    }
    let classes = rf.schema_classes()?;
    let Some(tag) = classes
        .classes
        .iter()
        .find(|class| class.name == CONNECTOR_MANAGER)
        .map(|class| class.tag)
    else {
        return Ok(Vec::new());
    };
    let mut anchor = tag.to_le_bytes().to_vec();
    anchor.extend_from_slice(&[0xff; 4]);
    anchor.extend_from_slice(&tag.wrapping_add(1).to_le_bytes());
    let mut pairs: BTreeSet<ConnectorPair> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &anchor) {
            let tail = &buf[at..(at + SCAN_TO + LIST_BYTES).min(buf.len())];
            pairs.extend(resolve(&lists_in(tail, curves)));
        }
    }
    Ok(pairs.into_iter().collect())
}

/// The list at `at` in `buf` when it joins one of `fittings` (or terminals
/// and equipment, RE-170) to one of `others` (RE-141).
fn fitting_list_at(
    buf: &[u8],
    at: usize,
    fittings: &BTreeSet<u64>,
    others: &BTreeSet<u64>,
) -> Option<ConnectorPair> {
    if u32_at(buf, at)? != LIST_ENTRIES {
        return None;
    }
    let (element, index, own_flag) = reference_at(buf, at + 4)?;
    let (other, other_index, other_flag) = reference_at(buf, at + 4 + ENTRY_BYTES)?;
    (fittings.contains(&element)
        && others.contains(&other)
        && other != element
        && index < LISTED_CONNECTORS
        && other_index < LISTED_CONNECTORS
        && own_flag == REFERENCE_FLAG
        && other_flag == REFERENCE_FLAG)
        .then_some(ConnectorPair {
            element,
            index,
            other,
            other_index,
        })
}

/// The connector pairs of `fittings` (the ElementIds of duct and pipe
/// fittings, terminals and equipment) joined to one of `others`, sorted and
/// without repeats (RE-141, RE-170). Empty for a release this layout is not
/// measured on.
pub fn scan_fitting_pairs(
    rf: &mut RevitFile,
    fittings: &BTreeSet<u64>,
    others: &BTreeSet<u64>,
) -> Result<Vec<ConnectorPair>> {
    if fittings.is_empty() || !supports_revit_version(rf.basic_file_info()?.version) {
        return Ok(Vec::new());
    }
    let count = LIST_ENTRIES.to_le_bytes();
    let mut pairs: BTreeSet<ConnectorPair> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        let buf = inflated.bytes();
        for at in memchr::memmem::find_iter(buf, &count) {
            pairs.extend(fitting_list_at(buf, at, fittings, others));
        }
    }
    Ok(pairs.into_iter().collect())
}
