//! Per-element IFC export-type overrides carried in the Revit 2024
//! partition parameter blocks (#212, RE-22).
//!
//! Revit's IFC exporter maps a Revit category to an IFC entity type,
//! but a single element may carry an instance-level override — the
//! `IFC Export As` parameter — that redirects it to a different type.
//! On `2024_Core_Interior.rvt` twenty `OST_Floors` elements are
//! exported as `IfcShadingDevice` instead of `IfcSlab` while sharing
//! their `FloorType` with elements that stay `IfcSlab`: the reference
//! export carries `IFCSLABTYPE` **and** `IFCSHADINGDEVICETYPE` rows
//! with the same `Tag` (`4166`, `71848`), which is only possible if
//! the choice is made per instance.
//!
//! The override is a UTF-16LE string in the element's parameter
//! block, framed as
//!
//! ```text
//! -0x11e  u64  owning ElementId (confirmation slot)
//! -0x0dc  u64  owning ElementId
//! -0x004  u32  value length in UTF-16 code units
//! +0x000  2*n  the value, UTF-16LE ("IfcShadingDevice")
//! +0x0..  u64  parameter-definition ElementId (17368 / 17493 here)
//! ```
//!
//! Both owner slots must carry the same `ElementId` and it must be
//! declared in `Global/ElemTable`; anything else is discarded. That
//! test is what separates a real override from the parameter
//! *definition* block, which repeats the same value string with no
//! owner (one such block exists on this file) and from the second
//! string of each pair, whose owner slots hold sentinels.
//!
//! # Honesty
//!
//! - The two owner offsets are **measured**, on one file, over thirty
//!   accepted entries. They are not derived from a parsed parameter
//!   block header; the block framing itself is not decoded.
//! - The value string is returned verbatim. Nothing here decides what
//!   an override *means* — [`crate::ifc::category_map`] decides which
//!   values it is willing to act on, and an unrecognised value leaves
//!   the element on its category's default mapping.
//! - `IfcShadingDevice` is the only value corpus-proven today. The
//!   scan is general because the framing is; the claim is not.
//!
//! Measured on `2024_Core_Interior.rvt`: 31 accepted entries naming
//! 21 distinct ElementIds, every one of them `IfcShadingDevice`. The
//! 20 that are also standalone placed instances
//! ([`crate::partition_element_records::PartitionElementRecord::is_exported_instance`])
//! are exactly the 20 `IFCSHADINGDEVICE` `Tag` values in Revit's own
//! export — no misses, no extras. The 21st (`16925`) is a container
//! member, which the instance rule already excludes.

use crate::{Result, RevitFile, compression};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Releases where this framing is corpus-proven.
pub const IFC_EXPORT_OVERRIDE_SUPPORTED_REVIT_VERSIONS: &[u32] = &[2024];

/// Bytes from the value string back to the owning `ElementId`.
pub const OWNER_OFFSET_BEFORE_VALUE: usize = 220;

/// Bytes from the value string back to the confirmation copy of the
/// owning `ElementId`. Both slots must agree.
pub const OWNER_CONFIRM_OFFSET_BEFORE_VALUE: usize = 286;

/// Bytes from the value string back to its `u32` length prefix.
pub const LENGTH_PREFIX_OFFSET_BEFORE_VALUE: usize = 4;

/// Shortest override value the scan will accept, in UTF-16 units.
pub const MIN_VALUE_CHARS: usize = 4;

/// Longest override value the scan will accept, in UTF-16 units.
pub const MAX_VALUE_CHARS: usize = 64;

/// Prefix every IFC entity name carries; the scan anchor.
pub const VALUE_PREFIX: &str = "Ifc";

/// One accepted override entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IfcExportOverride {
    /// Stream the entry was found in, e.g. `"Partitions/46"`.
    pub stream: String,
    /// Byte offset of the value string in the inflated stream.
    pub offset: usize,
    /// ElementId the override applies to.
    pub element_id: u32,
    /// The value string, verbatim (e.g. `"IfcShadingDevice"`).
    pub value: String,
}

/// Whether this release's override framing is proven.
pub fn supports_revit_version(revit_version: u32) -> bool {
    IFC_EXPORT_OVERRIDE_SUPPORTED_REVIT_VERSIONS.contains(&revit_version)
}

fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    buf.get(off..off + 4)
        .map(|s| u32::from_le_bytes(s.try_into().expect("4 bytes")))
}

fn read_u64(buf: &[u8], off: usize) -> Option<u64> {
    buf.get(off..off + 8)
        .map(|s| u64::from_le_bytes(s.try_into().expect("8 bytes")))
}

/// Decode one override entry whose value string starts at `offset`.
///
/// Fail-closed at every step: a bad length, a non-alphanumeric value,
/// disagreeing owner slots, or an owner that `Global/ElemTable` does
/// not declare all reject the entry.
pub fn decode_at(
    stream: &str,
    buf: &[u8],
    offset: usize,
    declared: &BTreeSet<u32>,
) -> Option<IfcExportOverride> {
    if offset < OWNER_CONFIRM_OFFSET_BEFORE_VALUE {
        return None;
    }
    let chars = read_u32(buf, offset - LENGTH_PREFIX_OFFSET_BEFORE_VALUE)? as usize;
    if !(MIN_VALUE_CHARS..=MAX_VALUE_CHARS).contains(&chars) {
        return None;
    }
    let end = offset.checked_add(chars.checked_mul(2)?)?;
    if end > buf.len() {
        return None;
    }
    let mut value = String::with_capacity(chars);
    for index in 0..chars {
        let at = offset + index * 2;
        let unit = u16::from_le_bytes([buf[at], buf[at + 1]]);
        // An IFC entity name is ASCII alphanumeric; anything else here
        // means the anchor landed inside unrelated text.
        let byte = u8::try_from(unit).ok()?;
        if !byte.is_ascii_alphanumeric() {
            return None;
        }
        value.push(char::from(byte));
    }
    if !value.starts_with(VALUE_PREFIX) {
        return None;
    }
    let owner = read_u64(buf, offset - OWNER_OFFSET_BEFORE_VALUE)?;
    let confirm = read_u64(buf, offset - OWNER_CONFIRM_OFFSET_BEFORE_VALUE)?;
    if owner != confirm || owner == 0 || owner > u64::from(u32::MAX) {
        return None;
    }
    let element_id = owner as u32;
    if !declared.contains(&element_id) {
        return None;
    }
    Some(IfcExportOverride {
        stream: stream.to_string(),
        offset,
        element_id,
        value,
    })
}

/// Find every accepted override entry in one inflated stream.
pub fn find_overrides(
    stream: &str,
    buf: &[u8],
    declared: &BTreeSet<u32>,
) -> Vec<IfcExportOverride> {
    let needle: Vec<u8> = VALUE_PREFIX
        .encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect();
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor + needle.len() <= buf.len() {
        let Some(found) = find_subslice(&buf[cursor..], &needle) else {
            break;
        };
        let hit = cursor + found;
        if let Some(entry) = decode_at(stream, buf, hit, declared) {
            out.push(entry);
        }
        cursor = hit + 1;
    }
    out
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    let first = needle[0];
    let last = haystack.len() - needle.len();
    let mut index = 0usize;
    while index <= last {
        let delta = haystack[index..=last].iter().position(|b| *b == first)?;
        let start = index + delta;
        if &haystack[start..start + needle.len()] == needle {
            return Some(start);
        }
        index = start + 1;
    }
    None
}

/// Collapse accepted entries to one value per ElementId.
///
/// An ElementId that names two different values is dropped: an
/// override that cannot be read unambiguously is not an override.
pub fn overrides_by_element_id(
    entries: impl IntoIterator<Item = IfcExportOverride>,
) -> BTreeMap<u32, String> {
    let mut out: BTreeMap<u32, String> = BTreeMap::new();
    let mut conflicting: BTreeSet<u32> = BTreeSet::new();
    for entry in entries {
        match out.get(&entry.element_id) {
            Some(existing) if *existing != entry.value => {
                conflicting.insert(entry.element_id);
            }
            Some(_) => {}
            None => {
                out.insert(entry.element_id, entry.value);
            }
        }
    }
    for id in conflicting {
        out.remove(&id);
    }
    out
}

/// Scan every `Partitions/*` stream for IFC export-type overrides.
///
/// Returns an empty map for unsupported releases (fail closed).
pub fn scan_ifc_export_overrides(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, String>> {
    if !supports_revit_version(revit_version) || declared.is_empty() {
        return Ok(BTreeMap::new());
    }
    let streams: Vec<String> = rf
        .stream_names()
        .into_iter()
        .filter(|s| s.starts_with("Partitions/"))
        .collect();
    let mut entries = Vec::new();
    for stream in streams {
        let Ok(raw) = rf.read_stream(&stream) else {
            continue;
        };
        let chunks = compression::inflate_all_chunks_for_stream(&stream, &raw);
        let concat: Vec<u8> = chunks.into_iter().flatten().collect();
        entries.extend(find_overrides(&stream, &concat, declared));
    }
    Ok(overrides_by_element_id(entries))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(ids: &[u32]) -> BTreeSet<u32> {
        ids.iter().copied().collect()
    }

    /// Build a buffer whose value string sits at
    /// [`OWNER_CONFIRM_OFFSET_BEFORE_VALUE`], with both owner slots set.
    fn synth(value: &str, owner: u64, confirm: u64) -> (Vec<u8>, usize) {
        let start = OWNER_CONFIRM_OFFSET_BEFORE_VALUE;
        let units: Vec<u16> = value.encode_utf16().collect();
        let mut buf = vec![0xffu8; start + units.len() * 2 + 32];
        buf[start - OWNER_OFFSET_BEFORE_VALUE..start - OWNER_OFFSET_BEFORE_VALUE + 8]
            .copy_from_slice(&owner.to_le_bytes());
        buf[start - OWNER_CONFIRM_OFFSET_BEFORE_VALUE
            ..start - OWNER_CONFIRM_OFFSET_BEFORE_VALUE + 8]
            .copy_from_slice(&confirm.to_le_bytes());
        buf[start - LENGTH_PREFIX_OFFSET_BEFORE_VALUE..start]
            .copy_from_slice(&(units.len() as u32).to_le_bytes());
        for (index, unit) in units.iter().enumerate() {
            let at = start + index * 2;
            buf[at..at + 2].copy_from_slice(&unit.to_le_bytes());
        }
        (buf, start)
    }

    #[test]
    fn decodes_a_well_formed_override() {
        let (buf, at) = synth("IfcShadingDevice", 20953, 20953);
        let entry = decode_at("Partitions/46", &buf, at, &declared(&[20953])).expect("decodes");
        assert_eq!(entry.element_id, 20953);
        assert_eq!(entry.value, "IfcShadingDevice");
        assert_eq!(entry.offset, at);
    }

    #[test]
    fn rejects_disagreeing_owner_slots() {
        let (buf, at) = synth("IfcShadingDevice", 20953, 64160);
        assert!(decode_at("Partitions/46", &buf, at, &declared(&[20953, 64160])).is_none());
    }

    #[test]
    fn rejects_owner_absent_from_elem_table() {
        let (buf, at) = synth("IfcShadingDevice", 20953, 20953);
        assert!(decode_at("Partitions/46", &buf, at, &declared(&[9999])).is_none());
    }

    #[test]
    fn rejects_sentinel_owner_slots() {
        let (buf, at) = synth("IfcShadingDevice", u64::MAX, u64::MAX);
        assert!(decode_at("Partitions/46", &buf, at, &declared(&[20953])).is_none());
    }

    #[test]
    fn rejects_a_value_that_is_not_an_entity_name() {
        let (buf, at) = synth("Ifc Shading", 20953, 20953);
        assert!(decode_at("Partitions/46", &buf, at, &declared(&[20953])).is_none());
    }

    #[test]
    fn rejects_a_length_prefix_that_does_not_match() {
        let (mut buf, at) = synth("IfcShadingDevice", 20953, 20953);
        buf[at - LENGTH_PREFIX_OFFSET_BEFORE_VALUE..at]
            .copy_from_slice(&(MAX_VALUE_CHARS as u32 + 1).to_le_bytes());
        assert!(decode_at("Partitions/46", &buf, at, &declared(&[20953])).is_none());
    }

    #[test]
    fn scan_finds_the_entry_at_its_anchor() {
        let (buf, at) = synth("IfcShadingDevice", 20953, 20953);
        let found = find_overrides("Partitions/46", &buf, &declared(&[20953]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].offset, at);
        assert_eq!(found[0].element_id, 20953);
    }

    #[test]
    fn conflicting_values_for_one_id_are_dropped() {
        let entry = |value: &str| IfcExportOverride {
            stream: "Partitions/46".into(),
            offset: 0,
            element_id: 7,
            value: value.into(),
        };
        let map = overrides_by_element_id([entry("IfcShadingDevice"), entry("IfcSlab")]);
        assert!(map.is_empty());
        let map = overrides_by_element_id([entry("IfcShadingDevice"), entry("IfcShadingDevice")]);
        assert_eq!(map.get(&7).map(String::as_str), Some("IfcShadingDevice"));
    }

    #[test]
    fn unsupported_release_yields_nothing() {
        assert!(!supports_revit_version(2023));
        assert!(supports_revit_version(2024));
    }
}
