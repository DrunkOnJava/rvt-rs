//! Text parameters of element types, stored under their BuiltInParameter
//! (#35, RE-77).
//!
//! An element's parameter values sit in value blocks that open with the
//! element's ElementId followed by 56 bytes of `0xff` and 3 zero bytes
//! (Discussion #112). Within a type's block, some text parameters are
//! stored as `i64 BuiltInParameter · u32 n · UTF-16 × n`. An entry belongs to
//! the block it follows, up to the next block, and the block's ElementId
//! must be declared in `Global/ElemTable`. The partition record chain
//! (RE-35) cannot attribute these: it covers only each partition's leading
//! records, while most type records sit in the loaded families' documents
//! after it (4% of Snowdon Towers' inflated partitions are in the chain).
//!
//! Measured on Autodesk's Snowdon Towers (Revit 2024) against the VIM export
//! of its 2027 edition, per type that both hold (see
//! `reports/element-framing/RE-77-type-text-parameters.md`). Instance
//! parameters such as Mark and Comments are not stored this way and are not
//! read. A type whose blocks carry two different values for one parameter
//! gets neither (fail closed).

use crate::RevitFile;
use crate::partition_element_records::bbox_marker;
use std::collections::{BTreeMap, BTreeSet};

/// The type parameters read, by BuiltInParameter id, with the name Revit
/// shows for each.
pub const TYPE_TEXT_PARAMETERS: &[(i64, &str)] = &[
    (-1001405, "Type Mark"),
    (-1010103, "Description"),
    (-1001206, "Fire Rating"),
];

/// Longest value read, in UTF-16 units.
const MAX_VALUE_UNITS: usize = 400;

/// The bytes that follow an ElementId to open its value block.
const VALUE_BLOCK_MARK: [u8; 59] = {
    let mut mark = [0xffu8; 59];
    mark[56] = 0;
    mark[57] = 0;
    mark[58] = 0;
    mark
};

/// Every type's text parameters in one inflated partition, by the declared
/// ElementId of the value block each entry follows.
pub fn scan_partition(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
) -> BTreeMap<u32, BTreeMap<&'static str, String>> {
    // Block starts: the mark, not preceded by a further 0xff (a longer run
    // of 0xff is padding, not a block), with a declared ElementId before it.
    let blocks: Vec<(usize, u32)> = memchr::memmem::find_iter(buf, &VALUE_BLOCK_MARK)
        .filter(|&at| at >= 8 && buf[at - 1] != 0xff)
        .filter_map(|at| {
            let id = u64::from_le_bytes(buf[at - 8..at].try_into().ok()?);
            let id = u32::try_from(id).ok()?;
            declared_ids.contains(&id).then_some((at, id))
        })
        .collect();
    let mut values: BTreeMap<(u32, &'static str), Option<String>> = BTreeMap::new();
    for (parameter, name) in TYPE_TEXT_PARAMETERS {
        for at in memchr::memmem::find_iter(buf, &parameter.to_le_bytes()) {
            let Some(text) = text_at(buf, at + 8) else {
                continue;
            };
            let index = blocks.partition_point(|(start, _)| *start < at);
            let Some(&(_, owner)) = index.checked_sub(1).and_then(|i| blocks.get(i)) else {
                continue;
            };
            values
                .entry((owner, name))
                .and_modify(|seen| {
                    if seen.as_deref() != Some(text.as_str()) {
                        *seen = None;
                    }
                })
                .or_insert(Some(text));
        }
    }
    let mut out: BTreeMap<u32, BTreeMap<&'static str, String>> = BTreeMap::new();
    for ((owner, name), value) in values {
        if let Some(value) = value {
            out.entry(owner).or_default().insert(name, value);
        }
    }
    out
}

/// Every type's text parameters in the file, by type ElementId. Empty for a
/// release whose element records are not decoded.
pub fn type_text_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
) -> BTreeMap<u32, BTreeMap<&'static str, String>> {
    if bbox_marker(revit_version).is_none() {
        return BTreeMap::new();
    }
    let Ok(records) = crate::elem_table::parse_records(rf) else {
        return BTreeMap::new();
    };
    let declared: BTreeSet<u32> = records.iter().map(|r| r.id_primary).collect();
    let mut out: BTreeMap<u32, BTreeMap<&'static str, String>> = BTreeMap::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, parameters) in scan_partition(inflated.bytes(), &declared) {
            out.entry(owner).or_default().extend(parameters);
        }
    }
    out
}

/// A `u32`-counted UTF-16 string at `at` of printable characters.
fn text_at(buf: &[u8], at: usize) -> Option<String> {
    let len = u32::from_le_bytes(buf.get(at..at + 4)?.try_into().ok()?) as usize;
    if !(1..=MAX_VALUE_UNITS).contains(&len) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + len * 2)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let text = String::from_utf16(&units).ok()?;
    text.chars().all(|c| !c.is_control()).then_some(text)
}
