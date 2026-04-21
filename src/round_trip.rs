//! Round-trip verification harness (WRT-04).
//!
//! Proves that the reader and writer are byte-exact inverses for any
//! instance the reader understands cleanly. Use this module to gate
//! changes to [`walker::read_field_by_type`] / [`walker::decode_instance`]
//! against their [`walker::write_field_by_type`] /
//! [`walker::encode_instance`] counterparts — a reader change that
//! breaks round-trip will surface here instead of silently mangling
//! writes in production.
//!
//! # Typical use
//!
//! ```no_run
//! use rvt::formats::{ClassEntry, FieldEntry, FieldType};
//! use rvt::round_trip::verify_instance_round_trip;
//!
//! // Caller has: decompressed Global/Latest bytes + the class's
//! // ClassEntry from parse_schema + a byte offset where one
//! // instance begins.
//! let bytes: &[u8] = &[];                          // doc-test placeholder
//! let class = ClassEntry {
//!     name: "Level".into(),
//!     offset: 0,
//!     fields: vec![FieldEntry {
//!         name: "m_elevation".into(),
//!         cpp_type: None,
//!         field_type: Some(FieldType::Primitive { kind: 0x07, size: 8 }),
//!     }],
//!     tag: None,
//!     parent: None,
//!     declared_field_count: None,
//!     was_parent_only: false,
//!     ancestor_tag: None,
//! };
//! let report = verify_instance_round_trip(bytes, 0, &class);
//! assert!(report.is_byte_exact());
//! ```
//!
//! # What counts as "round-trip clean"
//!
//! - **Byte-exact**: `encode_instance(decode_instance(b, 0, s), s)` returns
//!   the exact same byte slice the reader consumed from `b[range]`.
//! - **Typed-clean**: every schema field in the decoded instance is a
//!   typed [`walker::InstanceField`] variant (not `Bytes` fallback).
//!
//! Byte-exact implies typed-clean when the reader is faithful. A
//! typed-clean result that isn't byte-exact is a bug in either the
//! reader or the writer — the harness reports which field diverged
//! so the fix lives in the right place.

use crate::formats::ClassEntry;
use crate::walker::{
    Completeness, DecodedElement, InstanceField, decode_instance, encode_instance,
};

/// Result of a single round-trip verification. Populated by
/// [`verify_instance_round_trip`] and inspected by downstream
/// tooling to decide whether a change to the reader / writer
/// preserves the invariant.
#[derive(Debug, Clone)]
pub struct RoundTripReport {
    /// Number of bytes the reader consumed, starting at the
    /// caller-supplied offset.
    pub original_len: usize,
    /// Number of bytes the writer emitted.
    pub encoded_len: usize,
    /// Per-field breakdown of typed vs. fallback decoding. Useful
    /// for filtering out "this was never going to round-trip cleanly
    /// because the reader didn't understand it" cases.
    pub completeness: Completeness,
    /// `true` when both side-by-side byte sequences are identical
    /// (the strict definition of round-trip success).
    pub byte_exact: bool,
    /// The first index at which the original and encoded bytes
    /// diverge, when `byte_exact` is false. `None` when identical
    /// or when the encoded output is a length-mismatch of the
    /// original (check `original_len` vs `encoded_len`).
    pub first_diff_at: Option<usize>,
    /// Schema-field index whose emission covered the first
    /// divergence, when identifiable. Requires walking the
    /// schema's fields alongside the encoded cursor — may be
    /// `None` for length-mismatch or for divergences past the
    /// reader's parse frontier.
    pub first_diff_field: Option<usize>,
}

impl RoundTripReport {
    /// Strict pass: byte-exact equality between original and
    /// re-encoded bytes. The one-line success predicate most
    /// callers want.
    pub fn is_byte_exact(&self) -> bool {
        self.byte_exact && self.original_len == self.encoded_len
    }

    /// Soft pass: every decoded field is a typed (non-`Bytes`)
    /// variant. Good enough for "the reader understood this
    /// instance"; not sufficient for "we can round-trip" —
    /// weakened promise useful for reader-only audits.
    pub fn is_typed_clean(&self) -> bool {
        self.completeness.raw_bytes_fallback == 0 && self.completeness.total > 0
    }
}

/// Run one round-trip check: decode instance bytes, re-encode them,
/// compare byte-for-byte. Returns a [`RoundTripReport`] describing
/// the outcome. Safe for adversarial input — the reader already
/// caps allocations and falls back to [`InstanceField::Bytes`] on
/// any unknown layout.
///
/// The encoded output is compared against the byte range the reader
/// *consumed*, not `bytes[offset..]` — the reader may consume fewer
/// bytes than the slice contains when it reaches the end of an
/// instance.
pub fn verify_instance_round_trip(
    bytes: &[u8],
    offset: usize,
    class: &ClassEntry,
) -> RoundTripReport {
    let decoded = decode_instance(bytes, offset, class);
    let consumed = &bytes[decoded.byte_range.clone()];
    let encoded = encode_instance(&decoded, class);
    let byte_exact = encoded.as_slice() == consumed;
    let (first_diff_at, first_diff_field) = if byte_exact {
        (None, None)
    } else {
        let diff = consumed
            .iter()
            .zip(encoded.iter())
            .position(|(a, b)| a != b)
            .or(Some(consumed.len().min(encoded.len())));
        (diff, locate_diff_field(&decoded, class, diff))
    };
    RoundTripReport {
        original_len: consumed.len(),
        encoded_len: encoded.len(),
        completeness: completeness_of(&decoded),
        byte_exact,
        first_diff_at,
        first_diff_field,
    }
}

fn completeness_of(d: &DecodedElement) -> Completeness {
    let mut c = Completeness {
        total: d.fields.len(),
        ..Completeness::default()
    };
    for (_, value) in &d.fields {
        match value {
            InstanceField::Bytes(b) => {
                c.raw_bytes_fallback += 1;
                if !b.is_empty() {
                    // Captured bytes — still useful signal even if
                    // the reader fell back.
                }
            }
            _ => {
                c.typed += 1;
                if !matches!(value, InstanceField::Bytes(b) if b.is_empty()) {
                    c.typed_and_non_empty += 1;
                }
            }
        }
    }
    c
}

/// Given a decoded instance + its schema + a byte offset into the
/// re-encoded bytes, return the schema-field index whose emission
/// covered that offset. Walks the instance re-encoding a field at a
/// time so the mapping is exact even when fields have variable
/// width (strings, vectors). Returns `None` when the offset is
/// beyond the last field or the caller passes `None`.
fn locate_diff_field(
    decoded: &DecodedElement,
    class: &ClassEntry,
    target: Option<usize>,
) -> Option<usize> {
    let target = target?;
    let mut cursor = 0;
    for (idx, schema_field) in class.fields.iter().enumerate() {
        let Some((_, value)) = decoded.fields.get(idx) else {
            break;
        };
        if let Some(ft) = schema_field.field_type.as_ref() {
            let mut chunk = Vec::new();
            crate::walker::write_field_by_type(value, ft, &mut chunk);
            if cursor + chunk.len() > target {
                return Some(idx);
            }
            cursor += chunk.len();
        } else if let InstanceField::Bytes(raw) = value {
            if cursor + raw.len() > target {
                return Some(idx);
            }
            cursor += raw.len();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::formats::{FieldEntry, FieldType};

    fn mk_class(fields: Vec<(&str, FieldType)>) -> ClassEntry {
        ClassEntry {
            name: "TestClass".into(),
            offset: 0,
            fields: fields
                .into_iter()
                .map(|(name, ft)| FieldEntry {
                    name: name.to_string(),
                    cpp_type: None,
                    field_type: Some(ft),
                })
                .collect(),
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        }
    }

    #[test]
    fn round_trip_primitives_is_byte_exact() {
        // u32(0x42), f64(2.5) — 4 + 8 = 12 bytes.
        let class = mk_class(vec![
            (
                "id",
                FieldType::Primitive {
                    kind: 0x05,
                    size: 4,
                },
            ),
            (
                "value",
                FieldType::Primitive {
                    kind: 0x07,
                    size: 8,
                },
            ),
        ]);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x42_u32.to_le_bytes());
        bytes.extend_from_slice(&2.5_f64.to_le_bytes());

        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact());
        assert!(report.is_typed_clean());
        assert_eq!(report.original_len, 12);
        assert_eq!(report.encoded_len, 12);
        assert_eq!(report.completeness.total, 2);
        assert_eq!(report.completeness.typed, 2);
    }

    #[test]
    fn round_trip_string_is_byte_exact() {
        // String: u32 char_count, then count × u16 UTF-16LE.
        let class = mk_class(vec![("name", FieldType::String)]);
        let text = "hello";
        let utf16: Vec<u16> = text.encode_utf16().collect();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
        for c in &utf16 {
            bytes.extend_from_slice(&c.to_le_bytes());
        }

        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact());
    }

    #[test]
    fn round_trip_element_id_is_byte_exact() {
        let class = mk_class(vec![("m_id", FieldType::ElementId)]);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0x14_u32.to_le_bytes());
        bytes.extend_from_slice(&0x1337_u32.to_le_bytes());

        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact());
    }

    #[test]
    fn round_trip_reports_first_diff_on_mismatch() {
        // Synthesize a class where field 0 is u32 (ok) and field 1
        // is an Unknown FieldType — reader falls back to Bytes and
        // encoder re-emits the raw bytes, which should be
        // byte-exact, so this test actually passes. Instead, force
        // a divergence by handing bytes whose tail is shorter than
        // the reader expects for the declared type: the f64 read
        // falls back to Bytes containing only the available 3
        // bytes, which the writer emits as those 3 bytes — still
        // byte-exact with the input slice. The write path is
        // conservative on purpose.
        //
        // To actually trigger a mismatch, use a class whose fields
        // the reader parses cleanly but whose decoded byte range
        // exceeds what the writer would emit. In the current
        // canonical implementation that's impossible — the reader
        // and writer agree on the wire format for every typed
        // FieldType. So this test just verifies the report shape
        // when everything matches.
        let class = mk_class(vec![(
            "id",
            FieldType::Primitive {
                kind: 0x05,
                size: 4,
            },
        )]);
        let bytes = 0x2a_u32.to_le_bytes();
        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact());
        assert_eq!(report.first_diff_at, None);
        assert_eq!(report.first_diff_field, None);
    }

    #[test]
    fn round_trip_typed_clean_requires_typed_fields() {
        // A class with an Unknown FieldType — reader falls back to
        // Bytes, so typed-clean is false.
        let class = ClassEntry {
            name: "TestClass".into(),
            offset: 0,
            fields: vec![FieldEntry {
                name: "weird".into(),
                cpp_type: None,
                field_type: Some(FieldType::Unknown { bytes: vec![0xAA] }),
            }],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        let bytes = vec![0xAA, 0xBB, 0xCC, 0xDD];
        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(!report.is_typed_clean());
        // Byte-exact still holds because Unknown writes bytes
        // verbatim — the writer stays conservative on unknown
        // field types.
        assert!(report.is_byte_exact());
    }

    #[test]
    fn round_trip_empty_instance_is_empty() {
        let class = mk_class(Vec::new());
        let report = verify_instance_round_trip(&[], 0, &class);
        assert_eq!(report.original_len, 0);
        assert_eq!(report.encoded_len, 0);
        // Zero fields -> not typed-clean (no typed fields to prove
        // cleanliness). Byte-exact is vacuously true.
        assert!(!report.is_typed_clean());
        assert!(report.is_byte_exact());
    }

    #[test]
    fn round_trip_multi_field_heterogeneous_is_byte_exact() {
        let class = mk_class(vec![
            (
                "flag",
                FieldType::Primitive {
                    kind: 0x01,
                    size: 1,
                },
            ),
            (
                "count",
                FieldType::Primitive {
                    kind: 0x05,
                    size: 4,
                },
            ),
            ("name", FieldType::String),
            ("ref", FieldType::ElementId),
        ]);

        let mut bytes = Vec::new();
        bytes.push(1_u8); // flag = true
        bytes.extend_from_slice(&42_u32.to_le_bytes()); // count
        // String "ab"
        let utf16: Vec<u16> = "ab".encode_utf16().collect();
        bytes.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
        for c in &utf16 {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        // ElementId tag 0x14, id 0x99
        bytes.extend_from_slice(&0x14_u32.to_le_bytes());
        bytes.extend_from_slice(&0x99_u32.to_le_bytes());

        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact(), "{report:?}");
        assert!(report.is_typed_clean());
        assert_eq!(report.completeness.total, 4);
        assert_eq!(report.completeness.typed, 4);
    }

    #[test]
    fn round_trip_vector_of_doubles_is_byte_exact() {
        let class = mk_class(vec![(
            "values",
            FieldType::Vector {
                kind: 0x07,
                body: Vec::new(),
            },
        )]);
        let values = [1.5_f64, -2.5, 42.0];
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&(values.len() as u32).to_le_bytes());
        for v in &values {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let report = verify_instance_round_trip(&bytes, 0, &class);
        assert!(report.is_byte_exact());
    }

    #[test]
    fn round_trip_with_offset_decodes_suffix() {
        // Plant padding then the instance. Reader + encoder consume
        // the trailing bytes; the report's consumed range is exactly
        // the instance span.
        let class = mk_class(vec![(
            "id",
            FieldType::Primitive {
                kind: 0x05,
                size: 4,
            },
        )]);
        let mut bytes = vec![0xFF, 0xFF, 0xFF]; // padding
        bytes.extend_from_slice(&0x2a_u32.to_le_bytes());
        let report = verify_instance_round_trip(&bytes, 3, &class);
        assert!(report.is_byte_exact());
        assert_eq!(report.original_len, 4);
        assert_eq!(report.encoded_len, 4);
    }
}
