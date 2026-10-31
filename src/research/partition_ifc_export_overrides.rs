//! Per-element IFC export-type overrides carried in the Revit 2024
//! partition parameter blocks (#212, RE-22).
//!
//! **Superseded for the export by RE-45** ([`find_export_parameters`]):
//! the value is one entry `i64 BuiltInParameter · u32 n · UTF-16` of the
//! element's serialised data, keyed by [`IFC_EXPORT_ELEMENT_AS`], and a type
//! carries [`IFC_EXPORT_ELEMENT_TYPE_AS`] for its elements, each with a
//! predefined type beside it. That read works on Revit 2024 and 2025 and
//! gives the same 21 owners on `2024_Core_Interior.rvt` as the owner-offset
//! read below. Nothing calls that read now; it records RE-22's framing.
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
//! - `IfcShadingDevice` was the first value proven, on floors. The scan
//!   is general because the framing is; which values the writer acts on,
//!   and where Revit's exporter ignores one (curtain walls in 2024 files,
//!   RE-188), is decided in [`crate::ifc::category_map`].
//!
//! Measured on `2024_Core_Interior.rvt`: 31 accepted entries naming
//! 21 distinct ElementIds, every one of them `IfcShadingDevice`. The
//! 20 that are also standalone placed instances
//! ([`crate::partition_element_records::PartitionElementRecord::is_exported_instance`])
//! are exactly the 20 `IFCSHADINGDEVICE` `Tag` values in Revit's own
//! export — no misses, no extras. The 21st (`16925`) is a container
//! member, which the instance rule already excludes.

use crate::le_bytes;
use crate::partition_element_records::find_subslice;
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
    let chars = le_bytes::u32_at(buf, offset - LENGTH_PREFIX_OFFSET_BEFORE_VALUE)? as usize;
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
        let unit = le_bytes::u16_at(buf, at)?;
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
    let owner = le_bytes::u64_at(buf, offset - OWNER_OFFSET_BEFORE_VALUE)?;
    let confirm = le_bytes::u64_at(buf, offset - OWNER_CONFIRM_OFFSET_BEFORE_VALUE)?;
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
    let units = le_bytes::u32_at(buf, at)? as usize;
    if !(1..=MAX_VALUE_CHARS).contains(&units) {
        return None;
    }
    let bytes = buf.get(at + 4..at + 4 + units * 2)?;
    let mut value = String::with_capacity(units);
    for unit in le_bytes::u16_units(bytes) {
        let byte = u8::try_from(unit).ok()?;
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
/// different values is dropped. Revit 2024 and later read the owner from
/// the data object instead ([`find_export_parameters_in_objects`]).
pub fn find_export_parameters(
    buf: &[u8],
    header: &[u8; 10],
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, ExportParameters> {
    collect_export_parameters(buf, declared, |hit| {
        let from = hit.saturating_sub(EXPORT_PARAMETER_WINDOW);
        let start = memchr::memmem::rfind(&buf[from..hit], header)?;
        le_bytes::u64_at(buf, from + start + header.len()).and_then(|id| u32::try_from(id).ok())
    })
}

/// [`find_export_parameters`] on Revit 2024 and later (#792): an entry
/// belongs to the data object (RE-153) whose payload holds it, whose
/// Adler-32 verifies, and that object's ElementId must be declared. An entry
/// in no data object is not read. `objects` is
/// [`crate::partition_room_parameters::data_objects`] of `buf`.
///
/// On the BIM4LCA office's building permit file the header rule gave
/// `IfcSpace` entries to doors (37), curtain walls (13), sketch lines (11)
/// and plumbing fixtures (10). Five of those fixtures are drains (four
/// floor traps and a roof drain), and their own `IfcSpace` hid their
/// types' `IfcWasteTerminalType`, while Revit exports all 78 of the file's
/// drains as `IfcWasteTerminal`.
pub fn find_export_parameters_in_objects(
    buf: &[u8],
    objects: &[(usize, crate::partition_room_parameters::DataObject)],
    declared: &BTreeSet<u32>,
) -> BTreeMap<u32, ExportParameters> {
    use crate::partition_room_parameters::DATA_OBJECT_HEADER;
    collect_export_parameters(buf, declared, |hit| {
        let index = objects.partition_point(|(p, _)| *p <= hit).checked_sub(1)?;
        let (p, object) = objects[index];
        (hit >= p + DATA_OBJECT_HEADER && hit + 8 <= object.end).then_some(object.element_id)
    })
}

/// Every export parameter entry in `buf` whose owner `owner_of` finds and
/// `declared` holds, by owner. A key an owner sets to two different values
/// is dropped.
fn collect_export_parameters(
    buf: &[u8],
    declared: &BTreeSet<u32>,
    owner_of: impl Fn(usize) -> Option<u32>,
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
            let Some(owner) = owner_of(hit).filter(|owner| declared.contains(owner)) else {
                continue;
            };
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

/// [`find_export_parameters`] over every `Partitions/*` stream, or on Revit
/// 2024 and later [`find_export_parameters_in_objects`]. An element whose
/// streams disagree about a key loses that key. Empty for a release with no
/// measured element-data header.
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
        let found = if revit_version >= 2024 {
            let Ok(objects) = rf.memoised_partition_data_objects(&stream) else {
                continue;
            };
            find_export_parameters_in_objects(inflated.bytes(), &objects, declared)
        } else {
            find_export_parameters(inflated.bytes(), &header, declared)
        };
        for (owner, parameters) in found {
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
