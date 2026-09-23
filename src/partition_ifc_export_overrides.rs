//! Per-element IFC export-type overrides carried in the Revit 2024
//! partition parameter blocks (#212, RE-22).
//!
//! **Superseded for the export by RE-45** ([`find_export_parameters`]):
//! the value is one entry `i64 BuiltInParameter · u32 n · UTF-16` of the
//! element's serialised data, keyed by [`IFC_EXPORT_ELEMENT_AS`], and a type
//! carries [`IFC_EXPORT_ELEMENT_TYPE_AS`] for its elements, each with a
//! predefined type beside it. That read works on Revit 2024 and 2025 and
//! gives the same 21 owners on `2024_Core_Interior.rvt` as the owner-offset
//! read below, which is kept for API compatibility.
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

use crate::{Result, RevitFile};
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
    let streams = rf.partition_stream_names();
    let mut entries = Vec::new();
    for stream in streams {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        entries.extend(find_overrides(&stream, inflated.bytes(), declared));
    }
    Ok(overrides_by_element_id(entries))
}

/// Autodesk `BuiltInParameter.IFC_EXPORT_ELEMENT_AS`, an instance's
/// "Export to IFC As" (RE-45).
pub const IFC_EXPORT_ELEMENT_AS: i64 = -1_019_014;
/// Autodesk `BuiltInParameter.IFC_EXPORT_ELEMENT_TYPE_AS`, a type's
/// "Export Type to IFC As".
pub const IFC_EXPORT_ELEMENT_TYPE_AS: i64 = -1_019_015;
/// Autodesk `BuiltInParameter.IFC_EXPORT_PREDEFINEDTYPE`, an instance's
/// IFC predefined type.
pub const IFC_EXPORT_PREDEFINEDTYPE: i64 = -1_019_016;
/// Autodesk `BuiltInParameter.IFC_EXPORT_PREDEFINEDTYPE_TYPE`, a type's
/// IFC predefined type.
pub const IFC_EXPORT_PREDEFINEDTYPE_TYPE: i64 = -1_019_017;

/// Furthest an export parameter entry has to sit past the element-data
/// header of the element it belongs to. The furthest measured is `0x1ac`,
/// on `2024_Core_Interior.rvt`.
pub const EXPORT_PARAMETER_WINDOW: usize = 0x800;

/// The IFC export parameters one element's serialised data sets (RE-45).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExportParameters {
    /// [`IFC_EXPORT_ELEMENT_AS`], verbatim (e.g. `"ifcSlab"`).
    pub export_as: Option<String>,
    /// [`IFC_EXPORT_PREDEFINEDTYPE`], verbatim (e.g. `"ROOF"`).
    pub predefined_type: Option<String>,
    /// [`IFC_EXPORT_ELEMENT_TYPE_AS`], verbatim (e.g. `"IfcCoveringType"`).
    pub type_export_as: Option<String>,
    /// [`IFC_EXPORT_PREDEFINEDTYPE_TYPE`], verbatim (e.g. `"CLADDING"`).
    pub type_predefined_type: Option<String>,
}

impl ExportParameters {
    fn slot(&mut self, key: i64) -> Option<&mut Option<String>> {
        match key {
            IFC_EXPORT_ELEMENT_AS => Some(&mut self.export_as),
            IFC_EXPORT_PREDEFINEDTYPE => Some(&mut self.predefined_type),
            IFC_EXPORT_ELEMENT_TYPE_AS => Some(&mut self.type_export_as),
            IFC_EXPORT_PREDEFINEDTYPE_TYPE => Some(&mut self.type_predefined_type),
            _ => None,
        }
    }

    /// The entity an element of this type or instance exports as: the
    /// instance's own value, else its type's with the trailing `Type` of
    /// an IFC type entity dropped (`IfcCoveringType` is `IfcCovering`).
    pub fn effective_export_as(
        &self,
        type_parameters: Option<&ExportParameters>,
    ) -> Option<String> {
        if let Some(value) = &self.export_as {
            return Some(value.clone());
        }
        let value = type_parameters?.type_export_as.as_deref()?;
        let entity = match value.len().checked_sub(4) {
            Some(cut) if value[cut..].eq_ignore_ascii_case("type") => &value[..cut],
            _ => value,
        };
        Some(entity.to_string())
    }

    /// The predefined type: the instance's own, else its type's.
    pub fn effective_predefined_type(
        &self,
        type_parameters: Option<&ExportParameters>,
    ) -> Option<String> {
        self.predefined_type
            .clone()
            .or_else(|| type_parameters?.type_predefined_type.clone())
    }
}

/// One export parameter value: an IFC entity name for the two "export as"
/// keys, an enumerator-shaped identifier for the two predefined types.
fn export_parameter_value(buf: &[u8], at: usize, key: i64) -> Option<String> {
    let units = read_u32(buf, at)? as usize;
    if !(1..=MAX_VALUE_CHARS).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    let mut value = String::with_capacity(units);
    for pair in bytes.chunks_exact(2) {
        let byte = u8::try_from(u16::from_le_bytes([pair[0], pair[1]])).ok()?;
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return None;
        }
        value.push(char::from(byte));
    }
    let entity = matches!(key, IFC_EXPORT_ELEMENT_AS | IFC_EXPORT_ELEMENT_TYPE_AS);
    if entity {
        let prefixed = value
            .get(..VALUE_PREFIX.len())
            .is_some_and(|prefix| prefix.eq_ignore_ascii_case(VALUE_PREFIX));
        if !prefixed || value.len() == VALUE_PREFIX.len() || value.contains('_') {
            return None;
        }
    }
    Some(value)
}

/// Every IFC export parameter in one inflated partition, by the ElementId
/// whose serialised data holds it (RE-45).
///
/// An entry is `i64 BuiltInParameter · u32 n · n UTF-16 units`, one of
/// [`IFC_EXPORT_ELEMENT_AS`], [`IFC_EXPORT_ELEMENT_TYPE_AS`],
/// [`IFC_EXPORT_PREDEFINEDTYPE`] and [`IFC_EXPORT_PREDEFINEDTYPE_TYPE`]. It
/// belongs to the element whose data header (`header`, see
/// [`crate::partition_names::element_data_header`]) is the last one before
/// it, no more than [`EXPORT_PARAMETER_WINDOW`] back, and that element must
/// be declared in `Global/ElemTable`. A key an element sets to two
/// different values is dropped.
pub fn find_export_parameters(
    buf: &[u8],
    header: &[u8; 10],
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, ExportParameters> {
    let mut found: BTreeMap<(u32, i64), Option<String>> = BTreeMap::new();
    for key in [
        IFC_EXPORT_ELEMENT_AS,
        IFC_EXPORT_ELEMENT_TYPE_AS,
        IFC_EXPORT_PREDEFINEDTYPE,
        IFC_EXPORT_PREDEFINEDTYPE_TYPE,
    ] {
        for hit in memchr::memmem::find_iter(buf, &key.to_le_bytes()) {
            let Some(value) = export_parameter_value(buf, hit + 8, key) else {
                continue;
            };
            let from = hit.saturating_sub(EXPORT_PARAMETER_WINDOW);
            let Some(start) = memchr::memmem::rfind(&buf[from..hit], header) else {
                continue;
            };
            let Some(owner) =
                read_u64(buf, from + start + header.len()).and_then(|id| u32::try_from(id).ok())
            else {
                continue;
            };
            if !declared.contains(&owner) {
                continue;
            }
            match found.get_mut(&(owner, key)) {
                None => {
                    found.insert((owner, key), Some(value));
                }
                Some(held) => {
                    if held.as_deref() != Some(value.as_str()) {
                        *held = None;
                    }
                }
            }
        }
    }
    let mut out: BTreeMap<u32, ExportParameters> = BTreeMap::new();
    for ((owner, key), value) in found {
        let Some(value) = value else {
            continue;
        };
        if let Some(slot) = out.entry(owner).or_default().slot(key) {
            *slot = Some(value);
        }
    }
    out
}

/// [`find_export_parameters`] over every `Partitions/*` stream. An element
/// whose streams disagree about a key loses that key. Empty for a release
/// with no measured element-data header.
pub fn scan_export_parameters(
    rf: &mut RevitFile,
    revit_version: u32,
    declared: &BTreeSet<u32>,
) -> Result<BTreeMap<u32, ExportParameters>> {
    let Some(header) = crate::partition_names::element_data_header(revit_version) else {
        return Ok(BTreeMap::new());
    };
    let mut merged: BTreeMap<u32, ExportParameters> = BTreeMap::new();
    let mut conflicting: BTreeSet<(u32, i64)> = BTreeSet::new();
    for stream in rf.partition_stream_names() {
        let Ok(inflated) = rf.inflated_partition(&stream) else {
            continue;
        };
        for (owner, parameters) in find_export_parameters(inflated.bytes(), &header, declared) {
            let held = merged.entry(owner).or_default();
            for (key, value) in [
                (IFC_EXPORT_ELEMENT_AS, parameters.export_as),
                (IFC_EXPORT_PREDEFINEDTYPE, parameters.predefined_type),
                (IFC_EXPORT_ELEMENT_TYPE_AS, parameters.type_export_as),
                (
                    IFC_EXPORT_PREDEFINEDTYPE_TYPE,
                    parameters.type_predefined_type,
                ),
            ] {
                let Some(value) = value else {
                    continue;
                };
                let slot = held.slot(key).expect("an export parameter key");
                match slot {
                    None => *slot = Some(value),
                    Some(existing) if *existing != value => {
                        conflicting.insert((owner, key));
                    }
                    Some(_) => {}
                }
            }
        }
    }
    for (owner, key) in conflicting {
        if let Some(slot) = merged.get_mut(&owner).and_then(|p| p.slot(key)) {
            *slot = None;
        }
    }
    Ok(merged)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn element_data(header: &[u8; 10], id: u32, entries: &[(i64, &str)]) -> Vec<u8> {
        let mut out = header.to_vec();
        out.extend_from_slice(&u64::from(id).to_le_bytes());
        out.extend_from_slice(&[0xff; 40]);
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for (key, value) in entries {
            out.extend_from_slice(&key.to_le_bytes());
            let units: Vec<u16> = value.encode_utf16().collect();
            out.extend_from_slice(&(units.len() as u32).to_le_bytes());
            for unit in units {
                out.extend_from_slice(&unit.to_le_bytes());
            }
        }
        out
    }

    #[test]
    fn export_parameters_belong_to_the_element_whose_data_holds_them() {
        use crate::partition_names::{ELEMENT_DATA_HEADER, ELEMENT_DATA_HEADER_2025};
        // Core Interior floor 20912: exported as IfcSlab .ROOF.
        let mut buf = element_data(
            &ELEMENT_DATA_HEADER,
            20912,
            &[
                (IFC_EXPORT_PREDEFINEDTYPE, "ROOF"),
                (IFC_EXPORT_ELEMENT_AS, "ifcSlab"),
            ],
        );
        buf.extend(element_data(
            &ELEMENT_DATA_HEADER,
            20953,
            &[(IFC_EXPORT_ELEMENT_AS, "IfcShadingDevice")],
        ));
        let found =
            find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &BTreeSet::from([20912, 20953]));
        assert_eq!(found[&20912].export_as.as_deref(), Some("ifcSlab"));
        assert_eq!(found[&20912].predefined_type.as_deref(), Some("ROOF"));
        assert_eq!(found[&20953].export_as.as_deref(), Some("IfcShadingDevice"));
        assert_eq!(found[&20953].predefined_type, None);
        // The other release's header finds nothing.
        assert!(
            find_export_parameters(&buf, &ELEMENT_DATA_HEADER_2025, &BTreeSet::from([20912]))
                .is_empty()
        );
        // An undeclared owner is not an owner.
        assert!(
            find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &BTreeSet::from([1])).is_empty()
        );
    }

    #[test]
    fn a_type_export_parameter_applies_as_the_entity_it_names() {
        use crate::partition_names::ELEMENT_DATA_HEADER_2025;
        // teste_export_2025 wall type 402: its walls export as IfcCovering .CLADDING.
        let buf = element_data(
            &ELEMENT_DATA_HEADER_2025,
            402,
            &[
                (IFC_EXPORT_PREDEFINEDTYPE_TYPE, "CLADDING"),
                (IFC_EXPORT_ELEMENT_TYPE_AS, "IfcCoveringType"),
            ],
        );
        let found = find_export_parameters(&buf, &ELEMENT_DATA_HEADER_2025, &BTreeSet::from([402]));
        let wall_type = &found[&402];
        let instance = ExportParameters::default();
        assert_eq!(
            instance.effective_export_as(Some(wall_type)).as_deref(),
            Some("IfcCovering")
        );
        assert_eq!(
            instance
                .effective_predefined_type(Some(wall_type))
                .as_deref(),
            Some("CLADDING")
        );
        // The instance's own value wins.
        let own = ExportParameters {
            export_as: Some("IfcWall".into()),
            predefined_type: Some("SOLIDWALL".into()),
            ..ExportParameters::default()
        };
        assert_eq!(
            own.effective_export_as(Some(wall_type)).as_deref(),
            Some("IfcWall")
        );
        assert_eq!(
            own.effective_predefined_type(Some(wall_type)).as_deref(),
            Some("SOLIDWALL")
        );
        assert_eq!(instance.effective_export_as(None), None);
    }

    #[test]
    fn export_parameters_reject_what_is_not_an_override() {
        use crate::partition_names::ELEMENT_DATA_HEADER;
        let declared = BTreeSet::from([7]);
        // A parameter definition's group string under the same key.
        let buf = element_data(
            &ELEMENT_DATA_HEADER,
            7,
            &[(
                IFC_EXPORT_ELEMENT_TYPE_AS,
                "autodesk.parameter.group:materials-1.0.0",
            )],
        );
        assert!(find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &declared).is_empty());
        // An "export as" value that is not an IFC entity name.
        let buf = element_data(&ELEMENT_DATA_HEADER, 7, &[(IFC_EXPORT_ELEMENT_AS, "Slab")]);
        assert!(find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &declared).is_empty());
        // Too far past the element's data header.
        let mut buf = element_data(&ELEMENT_DATA_HEADER, 7, &[]);
        buf.extend(vec![0u8; EXPORT_PARAMETER_WINDOW]);
        buf.extend(
            element_data(&[0u8; 10], 0, &[(IFC_EXPORT_ELEMENT_AS, "IfcSlab")])[18..].to_vec(),
        );
        assert!(find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &declared).is_empty());
        // Two different values for one key.
        let mut buf = element_data(
            &ELEMENT_DATA_HEADER,
            7,
            &[(IFC_EXPORT_ELEMENT_AS, "IfcSlab")],
        );
        buf.extend(element_data(
            &ELEMENT_DATA_HEADER,
            7,
            &[(IFC_EXPORT_ELEMENT_AS, "IfcRoof")],
        ));
        assert!(
            find_export_parameters(&buf, &ELEMENT_DATA_HEADER, &declared)
                .get(&7)
                .is_none_or(|p| p.export_as.is_none())
        );
    }

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
