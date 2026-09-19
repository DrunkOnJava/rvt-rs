//! Experimental string parameters in independently measured partition profiles.
//!
//! Independently observed framing: `[ElementId:u32][unattributed:u32]
//! [body_len:u32][body][body_len:u32]`. Within the body, the Element base
//! carrier `c0 02 01 00 00 00` repeats the same ElementId. Both identities,
//! both lengths, declared-ID membership, and finite bounds are required.
//!
//! This exposes authored source values, not canonical facility identities.
//! It does not select current revisions, classify geometry, infer units, or
//! attach type parameters to instances. Repeated frames and values remain
//! separate. The implementation is experimental and profile-gated to 2023;
//! it is not wired into IFC export. See the 2026-09-11 reconnaissance addendum.
//! A separate, explicitly unowned 2027 occurrence decoder follows the controlled
//! Native record projection with bounded, fail-closed string decoding.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::ops::Range;

const ELEMENT_CARRIER: [u8; 6] = [0xc0, 0x02, 1, 0, 0, 0];
const MAX_HEADER_DISTANCE: usize = 256;
/// Hard bound for a single research record; no size-proportional allocation.
pub const MAX_RECORD_BODY_BYTES: usize = 16 * 1024 * 1024;
/// Bound on one recovered string's UTF-16 code units.
pub const MAX_STRING_UNITS: usize = 4096;
/// Authored instance Mark parameter identifier in the recorded profile.
pub const INSTANCE_MARK_PARAMETER_ID: i32 = -1_001_203;
/// Authored Type Mark identifier in the recorded profile.
pub const TYPE_MARK_PARAMETER_ID: i32 = -1_001_405;

/// An independently bounded record and its repeated owner identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedParameterRecord {
    /// Revision-local Revit ElementId; must be scoped by the source document.
    pub element_id: u32,
    /// Location of the outer ElementId in the supplied inflated buffer.
    pub offset: usize,
    /// Bytes containing the record body, excluding both length fields.
    pub body_range: Range<usize>,
    /// Offset of the repeated ElementId within the body.
    pub repeated_id_offset: usize,
    /// First body word, recorded without a semantic class-name claim.
    pub class_tag: u16,
}

/// A string value found inside one independently bounded owner record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnedStringParameter {
    pub element_id: u32,
    pub parameter_id: i32,
    /// Location of the parameter identifier in the supplied inflated buffer.
    pub offset: usize,
    pub value: String,
}

/// An unowned string-parameter occurrence in the experimentally measured 2027
/// profile. No element identity or revision selection is implied by a match.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringParameterOccurrence2027 {
    pub parameter_id: i64,
    /// Identifier offset within the caller-supplied bounded record body.
    pub offset: usize,
    pub value: String,
}

/// Why a candidate was not decoded. These are diagnostics, not silent drops.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StringParameterIssueKind2027 {
    UnsupportedProfile,
    UnsupportedParameter,
    InvalidLimit,
    TruncatedLength,
    TruncatedPayload,
    ResourceLimit,
    InvalidUtf16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringParameterIssue2027 {
    pub kind: StringParameterIssueKind2027,
    pub offset: Option<usize>,
    pub declared_units: Option<u32>,
}

/// Result for the supplied bounded body only, never whole-file completeness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StringParameterScan2027 {
    pub occurrences: Vec<StringParameterOccurrence2027>,
    pub issues: Vec<StringParameterIssue2027>,
    pub max_string_units: usize,
}

/// Scan independently established record boundaries or an exact field span.
/// Whole-member scans can mistake warning references for string values.
/// This narrow 2027 profile recognizes only measured Mark/Type Mark ids and
/// preserves all valid UTF-16, including controls. Resource limits and invalid
/// candidates are explicit in the returned report; callers must inspect issues.
/// This does not establish owner identity, current revision, or file completeness.
pub fn scan_bounded_string_parameter_occurrences_2027(
    record_body: &[u8],
    parameter_id: i64,
    revit_version: u32,
) -> StringParameterScan2027 {
    scan_bounded_string_parameter_occurrences_2027_with_limit(
        record_body,
        parameter_id,
        revit_version,
        MAX_STRING_UNITS,
    )
}

/// Configurable resource budget (UTF-16 code units), not a format limit.
/// The caller's body must already have independently validated bounds.
pub fn scan_bounded_string_parameter_occurrences_2027_with_limit(
    record_body: &[u8],
    parameter_id: i64,
    revit_version: u32,
    max_string_units: usize,
) -> StringParameterScan2027 {
    let mut result = StringParameterScan2027 {
        occurrences: Vec::new(),
        issues: Vec::new(),
        max_string_units,
    };
    let unsupported = if revit_version != 2027 {
        Some(StringParameterIssueKind2027::UnsupportedProfile)
    } else if ![
        i64::from(INSTANCE_MARK_PARAMETER_ID),
        i64::from(TYPE_MARK_PARAMETER_ID),
    ]
    .contains(&parameter_id)
    {
        Some(StringParameterIssueKind2027::UnsupportedParameter)
    } else if max_string_units > MAX_RECORD_BODY_BYTES / 2 {
        Some(StringParameterIssueKind2027::InvalidLimit)
    } else {
        None
    };
    if let Some(kind) = unsupported {
        result.issues.push(StringParameterIssue2027 {
            kind,
            offset: None,
            declared_units: None,
        });
        return result;
    }
    let needle = parameter_id.to_le_bytes();
    for (offset, window) in record_body.windows(needle.len()).enumerate() {
        if window != needle {
            continue;
        }
        let Some(count) = u32_at(record_body, offset + 8) else {
            result.issues.push(StringParameterIssue2027 {
                kind: StringParameterIssueKind2027::TruncatedLength,
                offset: Some(offset),
                declared_units: None,
            });
            continue;
        };
        let start = offset + 12;
        let bytes = usize::try_from(count)
            .ok()
            .and_then(|n| n.checked_mul(2))
            .and_then(|n| start.checked_add(n))
            .and_then(|end| record_body.get(start..end));
        let Some(bytes) = bytes else {
            result.issues.push(StringParameterIssue2027 {
                kind: StringParameterIssueKind2027::TruncatedPayload,
                offset: Some(offset),
                declared_units: Some(count),
            });
            continue;
        };
        if count as usize > max_string_units {
            result.issues.push(StringParameterIssue2027 {
                kind: StringParameterIssueKind2027::ResourceLimit,
                offset: Some(offset),
                declared_units: Some(count),
            });
            continue;
        }
        let units: Vec<_> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        match String::from_utf16(&units) {
            Ok(value) => result.occurrences.push(StringParameterOccurrence2027 {
                parameter_id,
                offset,
                value,
            }),
            Err(_) => result.issues.push(StringParameterIssue2027 {
                kind: StringParameterIssueKind2027::InvalidUtf16,
                offset: Some(offset),
                declared_units: Some(count),
            }),
        }
    }
    result
}

/// A source-authored label in the FamilyInstance trailer. The wire does not
/// supply a parameter identifier here, so this remains a distinct encoding
/// rather than being relabelled as a generic Mark parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstanceTailLabel {
    pub element_id: u32,
    /// Optional declared ElementId following the label; relationship semantics are unknown.
    pub trailing_reference: Option<u32>,
    /// Offset of the UTF-16 value in the supplied inflated buffer.
    pub offset: usize,
    pub value: String,
}

/// A property-store string occurrence inside a validated record. A record may
/// contain nested family/type stores, so `record_element_id` is a containing
/// context, not a claim about final parameter ownership or inheritance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyStringOccurrence {
    pub record_element_id: u32,
    /// Project-local parameter IDs must remain scoped by the source model.
    pub parameter_id: i32,
    pub offset: usize,
    pub storage_flags: u16,
    pub value: String,
}

/// Find an explicitly requested property-store string: FF×4, i32 parameter,
/// zero u32, u16 flags (observed 0/1), u32 UTF-16 length, value. This does not
/// resolve the parameter's name or flatten nested stores into instance fields.
pub fn find_property_string_occurrences(
    buf: &[u8],
    record: &OwnedParameterRecord,
    parameter_id: i32,
) -> Vec<PropertyStringOccurrence> {
    let Some(body) = buf.get(record.body_range.clone()) else {
        return Vec::new();
    };
    let needle = parameter_id.to_le_bytes();
    let mut out = Vec::new();
    for (at, window) in body.windows(4).enumerate() {
        if window != needle
            || at < 4
            || body[at - 4..at] != [0xff; 4]
            || u32_at(body, at + 4) != Some(0)
        {
            continue;
        }
        let Some(flags) = body.get(at + 8..at + 10) else {
            continue;
        };
        let flags = u16::from_le_bytes([flags[0], flags[1]]);
        if flags > 1 {
            continue;
        }
        let Some(count) = u32_at(body, at + 10) else {
            continue;
        };
        let count = count as usize;
        if count > MAX_STRING_UNITS {
            continue;
        }
        let start = at + 14;
        let Some(bytes) = body.get(start..start + count * 2) else {
            continue;
        };
        let units: Vec<_> = bytes
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        let Ok(value) = String::from_utf16(&units) else {
            continue;
        };
        if value.chars().any(char::is_control) {
            continue;
        }
        out.push(PropertyStringOccurrence {
            record_element_id: record.element_id,
            parameter_id,
            offset: record.body_range.start + at,
            storage_flags: flags,
            value,
        });
    }
    out
}

/// Recover the bounded 2023 FamilyInstance trailer label: ten zero bytes,
/// u32 marker 2, u32 UTF-16-unit count, value, then `01 00 00 00 00 00 00 00 00`
/// at the exact end of the already validated owner body. An optional declared
/// ElementId may precede that suffix; its relationship semantics are unknown.
/// The body's first tag
/// must be 0x0740. This is a label occurrence, not a category or parameter-role
/// inference; callers must preserve that distinction.
pub fn find_instance_tail_label(
    buf: &[u8],
    record: &OwnedParameterRecord,
    declared_ids: &BTreeSet<u32>,
) -> Option<InstanceTailLabel> {
    let body = buf.get(record.body_range.clone())?;
    const SUFFIX: [u8; 9] = [1, 0, 0, 0, 0, 0, 0, 0, 0];
    const PREFIX: [u8; 14] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0];
    if record.class_tag != 0x0740 || !body.ends_with(&SUFFIX) {
        return None;
    }
    let suffix_start = body.len().checked_sub(SUFFIX.len())?;
    let mut candidate = None;
    for reference_bytes in [0, 4] {
        let Some(end) = suffix_start.checked_sub(reference_bytes) else {
            continue;
        };
        let trailing_reference = if reference_bytes == 4 {
            let id = u32_at(body, end)?;
            if id == 0 || !declared_ids.contains(&id) {
                continue;
            }
            Some(id)
        } else {
            None
        };
        let first = end
            .saturating_sub(4 + MAX_STRING_UNITS * 2)
            .max(PREFIX.len());
        for at in first..end.saturating_sub(3) {
            if body.get(at - PREFIX.len()..at) != Some(PREFIX.as_slice()) {
                continue;
            }
            let count = u32_at(body, at)? as usize;
            if count == 0 || count > MAX_STRING_UNITS || at + 4 + count * 2 != end {
                continue;
            }
            let units: Vec<_> = body[at + 4..end]
                .chunks_exact(2)
                .map(|p| u16::from_le_bytes([p[0], p[1]]))
                .collect();
            let Ok(value) = String::from_utf16(&units) else {
                continue;
            };
            if value.chars().any(char::is_control) {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            candidate = Some(InstanceTailLabel {
                element_id: record.element_id,
                trailing_reference,
                offset: record.body_range.start + at + 4,
                value,
            });
        }
    }
    candidate
}

fn u32_at(buf: &[u8], at: usize) -> Option<u32> {
    let bytes: [u8; 4] = buf.get(at..at.checked_add(4)?)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// Recover bounded owner records from this profile. Unsupported releases
/// return no records rather than applying a plausible but unproven pattern.
///
/// Supply contiguous inflated bytes: a record crossing a member boundary
/// cannot be recovered from either member in isolation. Callers retain the
/// containing stream/member locator and may supply overlapping windows.
pub fn scan_owned_parameter_records(
    buf: &[u8],
    declared_ids: &BTreeSet<u32>,
    revit_version: u32,
) -> Vec<OwnedParameterRecord> {
    if revit_version != 2023 {
        return Vec::new();
    }
    let mut records = Vec::new();
    for (at, window) in buf.windows(ELEMENT_CARRIER.len()).enumerate() {
        if window != ELEMENT_CARRIER {
            continue;
        }
        let repeated_id_offset = at + ELEMENT_CARRIER.len();
        let Some(element_id) = u32_at(buf, repeated_id_offset) else {
            continue;
        };
        if element_id == 0 || !declared_ids.contains(&element_id) {
            continue;
        }
        let mut candidate = None;
        let mut ambiguous = false;
        for offset in at.saturating_sub(MAX_HEADER_DISTANCE)..at.saturating_sub(11) {
            if u32_at(buf, offset) != Some(element_id) {
                continue;
            }
            let Some(length) = u32_at(buf, offset + 8) else {
                continue;
            };
            let length = length as usize;
            if !(2..=MAX_RECORD_BODY_BYTES).contains(&length) {
                continue;
            }
            let start = offset + 12;
            let Some(end) = start.checked_add(length) else {
                continue;
            };
            if end < repeated_id_offset + 4 || u32_at(buf, end) != Some(length as u32) {
                continue;
            }
            if candidate.is_some() {
                ambiguous = true;
                break;
            }
            candidate = Some(OwnedParameterRecord {
                element_id,
                offset,
                body_range: start..end,
                repeated_id_offset,
                class_tag: u16::from_le_bytes([buf[start], buf[start + 1]]),
            });
        }
        if !ambiguous {
            if let Some(record) = candidate {
                records.push(record);
            }
        }
    }
    records
}

/// Recover occurrences of one explicitly requested string parameter from a
/// validated owner record. The wire value is an i32 parameter identifier,
/// u32 UTF-16-unit count, then that many UTF-16LE units.
///
/// Mark and Type Mark have been measured against reference exports in the
/// initial profile. Other identifiers are a research surface, not a claim
/// that their storage type is String. An occurrence is not proof of which
/// source revision is applicable. Never silently select the last duplicate.
pub fn find_string_parameter(
    buf: &[u8],
    record: &OwnedParameterRecord,
    parameter_id: i32,
) -> Vec<OwnedStringParameter> {
    let Some(body) = buf.get(record.body_range.clone()) else {
        return Vec::new();
    };
    let needle = parameter_id.to_le_bytes();
    let mut values = Vec::new();
    for (relative, window) in body.windows(4).enumerate() {
        if window != needle {
            continue;
        }
        let Some(count) = u32_at(body, relative + 4) else {
            continue;
        };
        let count = count as usize;
        if count > MAX_STRING_UNITS {
            continue;
        }
        let start = relative + 8;
        let Some(end) = start.checked_add(count * 2) else {
            continue;
        };
        let Some(bytes) = body.get(start..end) else {
            continue;
        };
        let units: Vec<_> = bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        let Ok(value) = String::from_utf16(&units) else {
            continue;
        };
        if value.chars().any(char::is_control) {
            continue;
        }
        values.push(OwnedStringParameter {
            element_id: record.element_id,
            parameter_id,
            offset: record.body_range.start + relative,
            value,
        });
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_2027_reports_resource_and_malformed_candidates() {
        let mut bytes = i64::from(INSTANCE_MARK_PARAMETER_ID).to_le_bytes().to_vec();
        bytes.extend_from_slice(&4097u32.to_le_bytes());
        bytes.extend_from_slice(&[65u8, 0].repeat(4097));
        let limited = scan_bounded_string_parameter_occurrences_2027(
            &bytes,
            i64::from(INSTANCE_MARK_PARAMETER_ID),
            2027,
        );
        assert!(limited.occurrences.is_empty());
        assert_eq!(
            limited.issues[0].kind,
            StringParameterIssueKind2027::ResourceLimit
        );
        assert_eq!(limited.issues[0].declared_units, Some(4097));
        let allowed = scan_bounded_string_parameter_occurrences_2027_with_limit(
            &bytes,
            i64::from(INSTANCE_MARK_PARAMETER_ID),
            2027,
            4097,
        );
        assert!(allowed.issues.is_empty());
        assert_eq!(allowed.occurrences[0].value.len(), 4097);
        bytes.truncate(12);
        let truncated = scan_bounded_string_parameter_occurrences_2027(
            &bytes,
            i64::from(INSTANCE_MARK_PARAMETER_ID),
            2027,
        );
        assert_eq!(
            truncated.issues[0].kind,
            StringParameterIssueKind2027::TruncatedPayload
        );
        let unsupported = scan_bounded_string_parameter_occurrences_2027(
            &[],
            i64::from(INSTANCE_MARK_PARAMETER_ID),
            2023,
        );
        assert_eq!(
            unsupported.issues[0].kind,
            StringParameterIssueKind2027::UnsupportedProfile
        );
        bytes[8..12].copy_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0xd800u16.to_le_bytes());
        let invalid = scan_bounded_string_parameter_occurrences_2027(
            &bytes,
            i64::from(INSTANCE_MARK_PARAMETER_ID),
            2027,
        );
        assert_eq!(
            invalid.issues[0].kind,
            StringParameterIssueKind2027::InvalidUtf16
        );
    }

    #[test]
    fn native_2027_string_width_and_unicode_unit_controls() {
        for value in [
            "Märk_墙_🚪".to_owned(),
            "line\nwith\tcontrols".to_owned(),
            "X".repeat(127),
            "X".repeat(128),
            "X".repeat(255),
            "X".repeat(256),
            String::new(),
        ] {
            let mut bytes = i64::from(INSTANCE_MARK_PARAMETER_ID).to_le_bytes().to_vec();
            let units: Vec<_> = value.encode_utf16().collect();
            bytes.extend_from_slice(&(units.len() as u32).to_le_bytes());
            for unit in units {
                bytes.extend_from_slice(&unit.to_le_bytes());
            }
            let values = scan_bounded_string_parameter_occurrences_2027(
                &bytes,
                i64::from(INSTANCE_MARK_PARAMETER_ID),
                2027,
            )
            .occurrences;
            assert_eq!(values.len(), 1);
            assert_eq!(values[0].value, value);
            assert!(
                scan_bounded_string_parameter_occurrences_2027(
                    &bytes,
                    i64::from(INSTANCE_MARK_PARAMETER_ID),
                    2023
                )
                .occurrences
                .is_empty()
            );
            bytes[4..8].fill(0); // zero extension is not the observed signed identifier
            assert!(
                scan_bounded_string_parameter_occurrences_2027(
                    &bytes,
                    i64::from(INSTANCE_MARK_PARAMETER_ID),
                    2027
                )
                .occurrences
                .is_empty()
            );
        }
    }

    #[test]
    fn native_2027_string_rejects_truncation_and_unknown_parameter_type() {
        let mut bytes = i64::from(TYPE_MARK_PARAMETER_ID).to_le_bytes().to_vec();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&0xd800u16.to_le_bytes()); // isolated surrogate
        assert!(
            scan_bounded_string_parameter_occurrences_2027(
                &bytes,
                i64::from(TYPE_MARK_PARAMETER_ID),
                2027
            )
            .occurrences
            .is_empty()
        );
        bytes[12..14].copy_from_slice(&65u16.to_le_bytes());
        assert_eq!(
            scan_bounded_string_parameter_occurrences_2027(
                &bytes,
                i64::from(TYPE_MARK_PARAMETER_ID),
                2027
            )
            .occurrences
            .len(),
            1
        );
        for end in 0..bytes.len() {
            assert!(
                scan_bounded_string_parameter_occurrences_2027(
                    &bytes[..end],
                    i64::from(TYPE_MARK_PARAMETER_ID),
                    2027
                )
                .occurrences
                .is_empty()
            );
        }
        assert!(
            scan_bounded_string_parameter_occurrences_2027(&bytes, -123, 2027)
                .occurrences
                .is_empty()
        );
    }

    fn fixture(id: u32, value: &str) -> Vec<u8> {
        let mut body = vec![0; 44];
        body[0..2].copy_from_slice(&0x0740u16.to_le_bytes());
        body[38..44].copy_from_slice(&ELEMENT_CARRIER);
        body.extend_from_slice(&id.to_le_bytes());
        body.extend_from_slice(&INSTANCE_MARK_PARAMETER_ID.to_le_bytes());
        let units: Vec<_> = value.encode_utf16().collect();
        body.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            body.extend_from_slice(&unit.to_le_bytes());
        }
        let mut data = Vec::new();
        data.extend_from_slice(&id.to_le_bytes());
        data.extend_from_slice(&0x1234u32.to_le_bytes());
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data.extend_from_slice(&body);
        data.extend_from_slice(&(body.len() as u32).to_le_bytes());
        data
    }

    fn fixture_with_tail(id: u32, value: &str) -> Vec<u8> {
        let mut data = fixture(id, "PARAMETER-MARK");
        data.truncate(data.len() - 4);
        data.extend_from_slice(&[0; 10]);
        data.extend_from_slice(&2u32.to_le_bytes());
        let units: Vec<_> = value.encode_utf16().collect();
        data.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        data.extend_from_slice(&[1, 0, 0, 0, 0, 0, 0, 0, 0]);
        let length = (data.len() - 12) as u32;
        data[8..12].copy_from_slice(&length.to_le_bytes());
        data.extend_from_slice(&length.to_le_bytes());
        data
    }

    #[test]
    fn trailer_label_is_preserved_separately_from_parameter_mark() {
        let data = fixture_with_tail(123, "D-α-01");
        let records = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023);
        let label = find_instance_tail_label(&data, &records[0], &BTreeSet::from([123])).unwrap();
        assert_eq!(label.value, "D-α-01");
        assert_eq!(label.element_id, 123);
        assert_eq!(
            find_string_parameter(&data, &records[0], INSTANCE_MARK_PARAMETER_ID)[0].value,
            "PARAMETER-MARK"
        );
    }

    #[test]
    fn trailer_requires_exact_class_prefix_count_and_suffix() {
        let original = fixture_with_tail(123, "D-01");
        let ids = BTreeSet::from([123]);
        let record = scan_owned_parameter_records(&original, &ids, 2023).remove(0);
        let label = find_instance_tail_label(&original, &record, &ids).unwrap();
        for at in [
            12,
            label.offset - 5,
            label.offset - 4,
            record.body_range.end - 9,
        ] {
            let mut changed = original.clone();
            changed[at] ^= 1;
            let record = scan_owned_parameter_records(&changed, &ids, 2023).remove(0);
            assert!(find_instance_tail_label(&changed, &record, &ids).is_none());
        }
    }

    #[test]
    fn trailer_rejects_invalid_utf16_and_never_reads_past_owner_body() {
        let mut data = fixture_with_tail(123, "D-01");
        let record = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023).remove(0);
        let at = find_instance_tail_label(&data, &record, &BTreeSet::from([123]))
            .unwrap()
            .offset;
        data[at..at + 2].copy_from_slice(&0xd800u16.to_le_bytes());
        assert!(find_instance_tail_label(&data, &record, &BTreeSet::from([123])).is_none());
        for length in 0..record.body_range.end {
            assert!(
                find_instance_tail_label(&data[..length], &record, &BTreeSet::from([123]))
                    .is_none()
            );
        }
    }

    #[test]
    fn trailer_optional_reference_must_be_declared_and_is_not_interpreted() {
        let mut data = fixture_with_tail(123, "D-01");
        let at = data.len() - 13;
        data.splice(at..at, 456u32.to_le_bytes());
        let length = (data.len() - 16) as u32;
        data[8..12].copy_from_slice(&length.to_le_bytes());
        let end = data.len();
        data[end - 4..].copy_from_slice(&length.to_le_bytes());
        let ids = BTreeSet::from([123, 456]);
        let record = scan_owned_parameter_records(&data, &ids, 2023).remove(0);
        let label = find_instance_tail_label(&data, &record, &ids).unwrap();
        assert_eq!(label.value, "D-01");
        assert_eq!(label.trailing_reference, Some(456));
        assert!(find_instance_tail_label(&data, &record, &BTreeSet::from([123])).is_none());
        data[at..at + 4].fill(0);
        assert!(find_instance_tail_label(&data, &record, &ids).is_none());
    }

    fn append_property(data: &mut Vec<u8>, parameter: i32, flags: u16, value: &str) {
        data.truncate(data.len() - 4);
        data.extend_from_slice(&[0xff; 4]);
        data.extend_from_slice(&parameter.to_le_bytes());
        data.extend_from_slice(&0u32.to_le_bytes());
        data.extend_from_slice(&flags.to_le_bytes());
        let units: Vec<_> = value.encode_utf16().collect();
        data.extend_from_slice(&(units.len() as u32).to_le_bytes());
        for unit in units {
            data.extend_from_slice(&unit.to_le_bytes());
        }
        let length = (data.len() - 12) as u32;
        data[8..12].copy_from_slice(&length.to_le_bytes());
        data.extend_from_slice(&length.to_le_bytes());
    }

    #[test]
    fn property_strings_preserve_empty_conflicts_flags_and_containing_context() {
        let mut data = fixture(123, "A");
        append_property(&mut data, 456, 0, "");
        append_property(&mut data, 456, 1, "Panel-α");
        append_property(&mut data, 456, 1, "Panel-B");
        append_property(&mut data, 789, 1, "Unrelated");
        let record = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023).remove(0);
        let values = find_property_string_occurrences(&data, &record, 456);
        assert_eq!(values.len(), 3);
        assert_eq!(
            values.iter().map(|v| v.value.as_str()).collect::<Vec<_>>(),
            ["", "Panel-α", "Panel-B"]
        );
        assert_eq!(values[0].storage_flags, 0);
        assert_eq!(values[1].storage_flags, 1);
        assert!(
            values
                .iter()
                .all(|v| v.record_element_id == 123 && v.parameter_id == 456)
        );
        assert!(
            values
                .windows(2)
                .all(|pair| pair[0].offset < pair[1].offset)
        );
    }

    #[test]
    fn property_strings_reject_bad_framing_utf16_and_out_of_body_reads() {
        let mut original = fixture(123, "A");
        append_property(&mut original, 456, 1, "Panel");
        let record =
            scan_owned_parameter_records(&original, &BTreeSet::from([123]), 2023).remove(0);
        let at = find_property_string_occurrences(&original, &record, 456)[0].offset;
        for (relative, replacement) in [(-4isize, 0), (4, 1), (8, 2), (10, 255)] {
            let mut changed = original.clone();
            changed[(at as isize + relative) as usize] = replacement;
            assert!(find_property_string_occurrences(&changed, &record, 456).is_empty());
        }
        let mut changed = original.clone();
        changed[at + 14..at + 16].copy_from_slice(&0xd800u16.to_le_bytes());
        assert!(find_property_string_occurrences(&changed, &record, 456).is_empty());
        changed[at + 14..at + 16].copy_from_slice(&10u16.to_le_bytes());
        assert!(find_property_string_occurrences(&changed, &record, 456).is_empty());
        for end in 0..record.body_range.end {
            assert!(find_property_string_occurrences(&original[..end], &record, 456).is_empty());
        }
    }

    #[test]
    fn frame_crossing_a_member_boundary_requires_contiguous_window() {
        let data = fixture_with_tail(123, "D-01");
        let ids = BTreeSet::from([123]);
        let split = 72;
        assert!(scan_owned_parameter_records(&data[..split], &ids, 2023).is_empty());
        assert!(scan_owned_parameter_records(&data[split..], &ids, 2023).is_empty());
        let mut window = data[..split].to_vec();
        window.extend_from_slice(&data[split..]);
        let records = scan_owned_parameter_records(&window, &ids, 2023);
        assert_eq!(records.len(), 1);
        assert_eq!(
            find_instance_tail_label(&window, &records[0], &ids)
                .unwrap()
                .value,
            "D-01"
        );
    }

    #[test]
    fn recovers_unicode_mark_with_both_owner_and_length_checks() {
        let data = fixture(123, "AHU-α-1");
        let records = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023);
        assert_eq!(records.len(), 1);
        let values = find_string_parameter(&data, &records[0], INSTANCE_MARK_PARAMETER_ID);
        assert_eq!(values.len(), 1);
        assert_eq!(values[0].element_id, 123);
        assert_eq!(values[0].value, "AHU-α-1");
    }

    #[test]
    fn requires_declared_owner_supported_profile_and_matching_trailer() {
        let mut data = fixture(123, "AHU-1");
        let ids = BTreeSet::from([123]);
        assert!(scan_owned_parameter_records(&data, &BTreeSet::new(), 2023).is_empty());
        assert!(scan_owned_parameter_records(&data, &ids, 2026).is_empty());
        let last = data.len() - 1;
        data[last] ^= 1;
        assert!(scan_owned_parameter_records(&data, &ids, 2023).is_empty());
    }

    #[test]
    fn truncated_records_and_mismatched_owner_do_not_emit() {
        let mut data = fixture(123, "AHU-1");
        let ids = BTreeSet::from([123]);
        for end in 0..data.len() {
            assert!(scan_owned_parameter_records(&data[..end], &ids, 2023).is_empty());
        }
        data[56..60].copy_from_slice(&124u32.to_le_bytes());
        assert!(scan_owned_parameter_records(&data, &ids, 2023).is_empty());
    }

    #[test]
    fn oversized_string_does_not_read_the_following_record() {
        let mut data = fixture(123, "A");
        data[64..68].copy_from_slice(&u32::MAX.to_le_bytes());
        let records = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023);
        assert_eq!(records.len(), 1);
        assert!(find_string_parameter(&data, &records[0], INSTANCE_MARK_PARAMETER_ID).is_empty());
    }

    #[test]
    fn repeated_revisions_are_kept_separately() {
        let mut data = fixture(123, "AHU-OLD");
        data.extend(fixture(123, "AHU-NEW"));
        let records = scan_owned_parameter_records(&data, &BTreeSet::from([123]), 2023);
        assert_eq!(records.len(), 2);
        let values: Vec<_> = records
            .iter()
            .flat_map(|r| find_string_parameter(&data, r, INSTANCE_MARK_PARAMETER_ID))
            .map(|v| v.value)
            .collect();
        assert_eq!(values, vec!["AHU-OLD", "AHU-NEW"]);
    }
}
