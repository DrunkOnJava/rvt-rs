//! Round-trip verification harness.
//!
//! Proves that the reader and writer are byte-exact inverses for any
//! instance the reader understands cleanly. Use this module to gate
//! changes to [`crate::walker::read_field_by_type`] /
//! [`crate::walker::decode_instance`] against their
//! [`crate::walker::write_field_by_type`] /
//! [`crate::walker::encode_instance`] counterparts — a reader change that
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
//!   typed [`crate::walker::InstanceField`] variant (not `Bytes` fallback).
//!
//! Byte-exact implies typed-clean when the reader is faithful. A
//! typed-clean result that isn't byte-exact is a bug in either the
//! reader or the writer — the harness reports which field diverged
//! so the fix lives in the right place.
// Tracker: WRT-04.

use crate::formats::ClassEntry;
use crate::walker::{
    Completeness, DecodedElement, InstanceField, decode_instance, encode_instance,
};

/// Result of a single round-trip verification. Populated by
/// [`verify_instance_round_trip`] and inspected by downstream
/// tooling to decide whether a change to the reader / writer
/// preserves the invariant.
#[derive(Debug, Clone)]
#[non_exhaustive]
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
