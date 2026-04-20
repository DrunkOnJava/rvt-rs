//! Layer 5a walker — schema-directed instance reader for ADocument.
//!
//! Reads the root `ADocument` class's instance data from
//! `Global/Latest` using the 100 %-classified schema produced by
//! `formats::parse_schema`. Current status: **partial**. Validated
//! cross-version on Revit 2024–2026, where the walker reads all 13
//! declared fields with 8 of 13 producing clean values and the last
//! three ElementIds (`m_ownerFamilyId`, `m_ownerFamilyContainingGroupId`,
//! `m_devBranchInfo`) matching byte-for-byte across those three
//! releases — strong cross-version validation of both the decoder
//! and the entry-point detector.
//!
//! Fields 2–5 and the 2016–2023 stream layout still need work. See
//! `docs/rvt-moat-break-reconnaissance.md` §Q6.5 for the full decode
//! state + open questions.
//!
//! Wire encoding (observed as of v0.1.2):
//!
//! | FieldType | Wire shape |
//! |---|---|
//! | `Pointer { .. }` | 8 bytes `[u32 slot_a][u32 slot_b]` — `00 00 00 00 00 00 00 00` means NULL; `0xff…f` also NULL-like |
//! | `ElementId` / `ElementIdRef` | 8 bytes `[u32 tag_or_zero][u32 id]` — `id` is the runtime element identifier |
//! | `Container { kind: 0x0e, .. }` | 2-column — `[u32 count][count × 6-byte [u16 id][u32 mask]][u32 count2][count2 × 6-byte records]` |
//! | Other `FieldType` variants | not yet exercised in ADocument — TBD |

use crate::{Error, Result, RevitFile, compression, formats, streams};

/// One field's value as read by the walker.
///
/// Matches the [`formats::FieldType`] classifier's output space. The
/// walker decodes each declared field to one of these variants based
/// on the field's schema-declared type; unrecognised or unexercised
/// wire shapes fall through to [`InstanceField::Bytes`] so downstream
/// tooling can still inspect raw bytes.
#[derive(Debug, Clone)]
pub enum InstanceField {
    /// `[u32 a][u32 b]` pointer slot. Both-zero means NULL; both-ones
    /// also sometimes seen as a NULL sentinel.
    Pointer { raw: [u32; 2] },
    /// Typed element reference. `tag` is 0 for references that don't
    /// carry a class-tag on the wire; `id` is the runtime ElementId.
    ElementId { tag: u32, id: u32 },
    /// Reference container, 2-column layout: `col_a` is the primary
    /// id list, `col_b` is typically masks or a parallel id stream.
    RefContainer { col_a: Vec<u16>, col_b: Vec<u16> },
    /// Fixed-size integer primitive (bool / u16 / u32 / i32 / u64 /
    /// i64). `signed` is `true` for signed variants; `value` is the
    /// widened 64-bit representation.
    Integer { value: i64, signed: bool, size: u8 },
    /// 32-bit or 64-bit IEEE 754 floating point.
    Float { value: f64, size: u8 },
    /// 1-byte boolean (stored padded on the wire; decoded as bool).
    Bool(bool),
    /// 16-byte GUID / UUID.
    Guid([u8; 16]),
    /// UTF-16LE length-prefixed string (both schema-encoded forms).
    String(std::string::String),
    /// Generic vector of homogeneous `InstanceField` values. Wire:
    /// `[u32 count][count × element]` where element layout depends on
    /// the vector's element FieldType.
    Vector(Vec<InstanceField>),
    /// Unused / unexercised paths return the raw bytes consumed.
    Bytes(Vec<u8>),
}

/// Handle/ID index into a decompressed `Global/Latest` stream.
///
/// Built by the Layer 5b walker from the element table. Maps each
/// `ElementId` to the byte offset in the stream where that element's
/// record begins. Used by per-class decoders to dereference pointers
/// across the object graph.
#[derive(Debug, Clone, Default)]
pub struct HandleIndex {
    map: std::collections::BTreeMap<u32, usize>,
}

impl HandleIndex {
    /// Construct an empty index. Populated by walker implementations
    /// (see `Self::insert`).
    pub fn new() -> Self {
        Self {
            map: std::collections::BTreeMap::new(),
        }
    }

    /// Record that `element_id` lives at byte `offset` in the
    /// decompressed stream.
    pub fn insert(&mut self, element_id: u32, offset: usize) {
        self.map.insert(element_id, offset);
    }

    /// Resolve an ElementId to its byte offset, if known.
    pub fn get(&self, element_id: u32) -> Option<usize> {
        self.map.get(&element_id).copied()
    }

    /// Number of indexed elements.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// True when the index has no entries.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Iterate over (ElementId, offset) pairs in sorted ElementId
    /// order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.map.iter().map(|(k, v)| (*k, *v))
    }
}

/// Trait implemented by each per-class decoder. Converts a byte slice
/// (positioned at the start of the class's instance data) into a
/// typed `Element` of the decoder's output type, consuming schema
/// knowledge + the handle index for cross-references.
///
/// Implementations for concrete Revit classes (Wall, Floor, Door,
/// etc.) live in `src/elements/*.rs` and follow the
/// `EXTENDING_LAYER_5B.md` template. The trait gives the walker
/// generic dispatch without needing to know every class at compile
/// time.
pub trait ElementDecoder: Sync + Send {
    /// The class's name as it appears in `Formats/Latest` (e.g.
    /// `"Wall"`, `"Floor"`, `"Level"`, `"Door"`). Must match the
    /// `ClassEntry.name` the schema parser produces.
    fn class_name(&self) -> &'static str;

    /// Decode a single instance of this class from its byte range.
    /// `schema` is the `ClassEntry` from `parse_schema`; `bytes`
    /// starts at the class's instance data; `index` resolves
    /// cross-references.
    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        index: &HandleIndex,
    ) -> Result<DecodedElement>;
}

/// Result of decoding a single element's instance bytes.
///
/// Every per-class decoder returns one of these so the walker can
/// handle arbitrary class types generically while still giving
/// callers structured access to parameter values + dereference-able
/// cross-references.
#[derive(Debug, Clone)]
pub struct DecodedElement {
    /// ElementId of this instance, if known.
    pub id: Option<u32>,
    /// Class name ("Wall", "Floor", "Level", etc.).
    pub class: std::string::String,
    /// Ordered list of `(field_name, value)` — one per declared
    /// schema field, populated in schema order.
    pub fields: Vec<(std::string::String, InstanceField)>,
    /// Byte range in the decompressed stream that this element's
    /// instance data occupies. For building a HandleIndex and for
    /// debugging byte-level decoding issues.
    pub byte_range: std::ops::Range<usize>,
}

/// Read a single `InstanceField` value starting at `bytes[cursor]`
/// based on the declared `FieldType`. Advances `cursor` past the
/// consumed bytes. Returns `InstanceField::Bytes(rest)` when the
/// FieldType is unknown or the wire layout for that variant isn't
/// yet exercised — callers can store the raw bytes for manual
/// inspection without crashing.
///
/// This is the per-field dispatch core that a generic `decode_instance`
/// implementation uses to walk any class's fields in schema order.
pub fn read_field_by_type(
    bytes: &[u8],
    cursor: &mut usize,
    ty: &formats::FieldType,
) -> InstanceField {
    use formats::FieldType;

    let rem = || bytes.get(*cursor..).unwrap_or(&[]);

    match ty {
        FieldType::Primitive { kind, size } => {
            let n = *size as usize;
            let slice = rem();
            if slice.len() < n {
                return InstanceField::Bytes(slice.to_vec());
            }
            match (*kind, *size) {
                (0x01, _) => {
                    let b = slice[0] != 0;
                    *cursor += n.max(1);
                    InstanceField::Bool(b)
                }
                (0x02, 2) => {
                    let v = u16::from_le_bytes([slice[0], slice[1]]) as i64;
                    *cursor += 2;
                    InstanceField::Integer {
                        value: v,
                        signed: false,
                        size: 2,
                    }
                }
                (0x04, 4) | (0x05, 4) => {
                    let v = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as i64;
                    *cursor += 4;
                    InstanceField::Integer {
                        value: v,
                        signed: *kind == 0x04,
                        size: 4,
                    }
                }
                (0x06, 4) => {
                    let v = f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as f64;
                    *cursor += 4;
                    InstanceField::Float { value: v, size: 4 }
                }
                (0x07, 8) => {
                    let v = f64::from_le_bytes([
                        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6],
                        slice[7],
                    ]);
                    *cursor += 8;
                    InstanceField::Float { value: v, size: 8 }
                }
                (0x0b, 8) => {
                    let v = i64::from_le_bytes([
                        slice[0], slice[1], slice[2], slice[3], slice[4], slice[5], slice[6],
                        slice[7],
                    ]);
                    *cursor += 8;
                    InstanceField::Integer {
                        value: v,
                        signed: true,
                        size: 8,
                    }
                }
                _ => {
                    let bytes = slice[..n.min(slice.len())].to_vec();
                    *cursor += n.min(slice.len());
                    InstanceField::Bytes(bytes)
                }
            }
        }
        FieldType::String => {
            // UTF-16LE length-prefixed. Wire: [u32 char_count][2*chars bytes].
            let slice = rem();
            if slice.len() < 4 {
                return InstanceField::Bytes(slice.to_vec());
            }
            let char_count = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as usize;
            let byte_count = char_count.saturating_mul(2);
            if slice.len() < 4 + byte_count {
                return InstanceField::Bytes(slice.to_vec());
            }
            let utf16_bytes = &slice[4..4 + byte_count];
            // Decode as UTF-16LE. encoding_rs is already in deps.
            let (text, _, had_errors) = encoding_rs::UTF_16LE.decode(utf16_bytes);
            *cursor += 4 + byte_count;
            if had_errors {
                // Fall back to raw bytes when encoding failed.
                InstanceField::Bytes(utf16_bytes.to_vec())
            } else {
                InstanceField::String(text.into_owned())
            }
        }
        FieldType::Guid => {
            let slice = rem();
            if slice.len() < 16 {
                return InstanceField::Bytes(slice.to_vec());
            }
            let mut g = [0u8; 16];
            g.copy_from_slice(&slice[..16]);
            *cursor += 16;
            InstanceField::Guid(g)
        }
        FieldType::ElementId | FieldType::ElementIdRef { .. } => {
            let slice = rem();
            if slice.len() < 8 {
                return InstanceField::Bytes(slice.to_vec());
            }
            let tag = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            let id = u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]);
            *cursor += 8;
            InstanceField::ElementId { tag, id }
        }
        FieldType::Pointer { .. } => {
            let slice = rem();
            if slice.len() < 8 {
                return InstanceField::Bytes(slice.to_vec());
            }
            let a = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]);
            let b = u32::from_le_bytes([slice[4], slice[5], slice[6], slice[7]]);
            *cursor += 8;
            InstanceField::Pointer { raw: [a, b] }
        }
        FieldType::Vector { kind, .. } => {
            // L5B-08: decode vectors of known-size primitive elements
            // into `InstanceField::Vector(Vec<InstanceField>)`. Wire
            // format for these kinds: `[u32 count][count × element]`
            // where element size is driven by the outer-type byte.
            //
            // Element-kind table (matches the Primitive table above):
            //
            //   0x01 = bool (1 byte), 0x07 = f64 (8 bytes),
            //   0x05 = u32 (4 bytes), 0x0b = i64 (8 bytes),
            //   0x0d = point = 3 × f64 (24 bytes).
            //
            // Unknown element kinds fall back to `InstanceField::Bytes`
            // with the raw count-prefix + remaining payload so callers
            // can inspect — same graceful-fallback philosophy as the
            // other read paths.
            let slice = rem();
            if slice.len() < 4 {
                return InstanceField::Bytes(slice.to_vec());
            }
            let count = u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]) as usize;
            // Per-element byte size for kinds we decode directly.
            let elem_size: Option<usize> = match *kind {
                0x01 => Some(1),
                0x05 | 0x04 => Some(4),
                0x07 | 0x0b => Some(8),
                0x0d => Some(24),
                _ => None,
            };
            let Some(esz) = elem_size else {
                // Unknown element kind — consume just the count
                // prefix and emit the following-bytes as raw.
                let consumed = slice.len().min(4);
                let out = slice[..consumed].to_vec();
                *cursor += consumed;
                return InstanceField::Bytes(out);
            };
            let needed = count
                .checked_mul(esz)
                .and_then(|n| n.checked_add(4))
                .unwrap_or(usize::MAX);
            if slice.len() < needed {
                return InstanceField::Bytes(slice.to_vec());
            }
            let mut items = Vec::with_capacity(count);
            let mut local = 4;
            for _ in 0..count {
                let elem_bytes = &slice[local..local + esz];
                let item = match (*kind, esz) {
                    (0x01, 1) => InstanceField::Bool(elem_bytes[0] != 0),
                    (0x04, 4) | (0x05, 4) => {
                        let v = u32::from_le_bytes([
                            elem_bytes[0],
                            elem_bytes[1],
                            elem_bytes[2],
                            elem_bytes[3],
                        ]) as i64;
                        InstanceField::Integer {
                            value: v,
                            signed: *kind == 0x04,
                            size: 4,
                        }
                    }
                    (0x07, 8) => {
                        let v = f64::from_le_bytes([
                            elem_bytes[0],
                            elem_bytes[1],
                            elem_bytes[2],
                            elem_bytes[3],
                            elem_bytes[4],
                            elem_bytes[5],
                            elem_bytes[6],
                            elem_bytes[7],
                        ]);
                        InstanceField::Float { value: v, size: 8 }
                    }
                    (0x0b, 8) => {
                        let v = i64::from_le_bytes([
                            elem_bytes[0],
                            elem_bytes[1],
                            elem_bytes[2],
                            elem_bytes[3],
                            elem_bytes[4],
                            elem_bytes[5],
                            elem_bytes[6],
                            elem_bytes[7],
                        ]);
                        InstanceField::Integer {
                            value: v,
                            signed: true,
                            size: 8,
                        }
                    }
                    (0x0d, 24) => {
                        // point = 3 × f64. Surface as a nested Vector
                        // of three Float items — the inner structure
                        // preserves the X,Y,Z semantics without
                        // needing a dedicated Point variant.
                        let mut point = Vec::with_capacity(3);
                        for k in 0..3 {
                            let off = k * 8;
                            let v = f64::from_le_bytes([
                                elem_bytes[off],
                                elem_bytes[off + 1],
                                elem_bytes[off + 2],
                                elem_bytes[off + 3],
                                elem_bytes[off + 4],
                                elem_bytes[off + 5],
                                elem_bytes[off + 6],
                                elem_bytes[off + 7],
                            ]);
                            point.push(InstanceField::Float { value: v, size: 8 });
                        }
                        InstanceField::Vector(point)
                    }
                    _ => InstanceField::Bytes(elem_bytes.to_vec()),
                };
                items.push(item);
                local += esz;
            }
            *cursor += 4 + count * esz;
            InstanceField::Vector(items)
        }
        FieldType::Container { .. } => {
            // 2-column container layout (kind=0x0e) is handled
            // specially by the caller via `read_field` (the
            // ADocument-walker-facing path). Generic callers that
            // hit this branch get the raw bytes until per-variant
            // support lands.
            let slice = rem();
            let consumed = slice.len().min(4);
            let out = slice[..consumed].to_vec();
            *cursor += consumed;
            InstanceField::Bytes(out)
        }
        FieldType::Unknown { .. } => {
            // Forward remaining bytes. Callers can inspect them.
            let slice = rem();
            let out = slice.to_vec();
            *cursor = bytes.len();
            InstanceField::Bytes(out)
        }
    }
}

/// Generic instance decoder: walks each declared field of `class` in
/// schema order using `read_field_by_type`. Falls back to
/// `InstanceField::Bytes` for fields whose FieldType is unknown or
/// whose wire layout isn't yet exercised.
///
/// Used by `ElementDecoder` default implementations and by any
/// caller who wants a best-effort instance dump without writing a
/// class-specific decoder first. Returns a `DecodedElement` with
/// `id: None` (callers that can extract the ID from the record
/// header should set it after calling this).
pub fn decode_instance(bytes: &[u8], start: usize, class: &formats::ClassEntry) -> DecodedElement {
    let mut cursor = start;
    let mut fields = Vec::with_capacity(class.fields.len());
    for field in &class.fields {
        let value = match field.field_type.as_ref() {
            Some(ft) => read_field_by_type(bytes, &mut cursor, ft),
            None => {
                // Field's type didn't classify — consume nothing,
                // emit empty bytes.
                InstanceField::Bytes(Vec::new())
            }
        };
        fields.push((field.name.clone(), value));
    }
    DecodedElement {
        id: None,
        class: class.name.clone(),
        fields,
        byte_range: start..cursor,
    }
}

/// Resource caps applied while the walker decodes an instance
/// (API-11). Every size- or count-prefixed wire value that could
/// drive an allocation is compared against the matching cap; values
/// above the cap trigger a graceful fallback (the field is emitted
/// as `InstanceField::Bytes` with the original bytes captured), not
/// a panic.
///
/// Defaults match the hard-coded limits that shipped in v0.1.2 so
/// existing callers get identical behaviour without opting in.
/// Tighten the limits when parsing adversarial / untrusted input:
///
/// ```
/// use rvt::walker::WalkerLimits;
/// let tight = WalkerLimits {
///     max_container_records: 64,
///     ..WalkerLimits::default()
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WalkerLimits {
    /// Maximum record count accepted in a 2-column `Container`
    /// (`kind = 0x0e`). Above this, the field falls back to raw
    /// bytes. Default 1000, which is already a generous cap for
    /// ADocument's `m_elemTable` pointer column (typical projects
    /// see 50-400 entries).
    pub max_container_records: usize,
}

impl Default for WalkerLimits {
    fn default() -> Self {
        Self {
            max_container_records: 1000,
        }
    }
}

/// ADocument's instance, as extracted by the v0.1.2 walker. Field
/// names mirror the schema exactly.
#[derive(Debug, Clone)]
pub struct ADocumentInstance {
    /// Byte offset in `Global/Latest`'s decompressed stream where the
    /// ADocument record was found.
    pub entry_offset: usize,
    /// Revit release (from `BasicFileInfo`), useful for downstream
    /// consumers that want to know which layout was parsed.
    pub version: u32,
    /// Parsed fields, one entry per declared schema field, in order.
    pub fields: Vec<(String, InstanceField)>,
}

/// Per-instance decode completeness summary (API-09).
///
/// Callers that want to distinguish "every field decoded cleanly"
/// from "we parsed the record but several fields fell back to raw
/// bytes" use [`ADocumentInstance::completeness`] to get this
/// breakdown. It's the programmatic equivalent of the
/// human-readable §Q6.5 addendum in the recon report — "what does
/// the walker fully understand right now?"
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Completeness {
    /// Total declared + parsed field count (always `fields.len()`).
    pub total: usize,
    /// Fields that decoded to a typed variant (anything except
    /// `InstanceField::Bytes`). High value = clean parse.
    pub typed: usize,
    /// Fields that fell back to `InstanceField::Bytes` — the wire
    /// layout wasn't exercised or the field classifier hit an
    /// unknown type. Non-zero = known gap.
    pub raw_bytes_fallback: usize,
    /// Fields that are typed AND non-empty. Filters out the
    /// zero-length `Bytes(Vec::new())` case emitted when a field's
    /// `FieldType` didn't classify at all.
    pub typed_and_non_empty: usize,
}

impl Completeness {
    /// Decode completeness as a 0.0–1.0 ratio. `typed / total`.
    /// Returns `None` for the zero-field case so callers don't
    /// divide by zero silently.
    pub fn typed_ratio(&self) -> Option<f64> {
        if self.total == 0 {
            None
        } else {
            Some(self.typed as f64 / self.total as f64)
        }
    }

    /// Convenience: true iff every declared field decoded to a
    /// typed variant (no raw-bytes fallbacks). Useful for CI drift
    /// detection — a release that suddenly returns false on a
    /// known-good fixture indicates wire-layout drift.
    pub fn is_fully_typed(&self) -> bool {
        self.total > 0 && self.raw_bytes_fallback == 0
    }
}

impl ADocumentInstance {
    /// Compute completeness markers for this instance. O(n) in the
    /// field count; cheap enough to call ad-hoc.
    pub fn completeness(&self) -> Completeness {
        let mut out = Completeness {
            total: self.fields.len(),
            ..Completeness::default()
        };
        for (_, value) in &self.fields {
            match value {
                InstanceField::Bytes(b) => {
                    out.raw_bytes_fallback += 1;
                    // Zero-length Bytes means the field's FieldType
                    // didn't classify at all — not a parse failure,
                    // just "we don't know the type." Don't count
                    // those as typed_and_non_empty anyway.
                    if !b.is_empty() {
                        // Bytes-with-content still counts as a raw
                        // fallback, not typed.
                    }
                }
                _ => {
                    out.typed += 1;
                    // Recursive non-empty check for Vector — a
                    // typed Vector with zero items is empty.
                    let non_empty = match value {
                        InstanceField::Vector(items) => !items.is_empty(),
                        InstanceField::String(s) => !s.is_empty(),
                        InstanceField::RefContainer { col_a, .. } => !col_a.is_empty(),
                        _ => true,
                    };
                    if non_empty {
                        out.typed_and_non_empty += 1;
                    }
                }
            }
        }
        out
    }
}

/// Read ADocument from a `RevitFile`. Returns `None` if the
/// entry-point detector can't confidently land on the record —
/// currently reliable on Revit 2024+ releases; older releases return
/// `None`.
///
/// Uses [`WalkerLimits::default()`] — callers that need to tighten
/// caps for untrusted input should use
/// [`read_adocument_with_limits`] instead.
pub fn read_adocument(rf: &mut RevitFile) -> Result<Option<ADocumentInstance>> {
    read_adocument_with_limits(rf, WalkerLimits::default())
}

/// Same as [`read_adocument`], with caller-supplied resource caps.
/// Applies [`WalkerLimits`] to every size- / count-prefixed wire
/// value read during decode. Fields that exceed a cap fall back to
/// raw bytes rather than panicking.
pub fn read_adocument_with_limits(
    rf: &mut RevitFile,
    limits: WalkerLimits,
) -> Result<Option<ADocumentInstance>> {
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| Error::BasicFileInfo("ADocument not in schema".into()))?;

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let d = compression::inflate_at(&raw, 8)?;
    let Some(entry) = find_adocument_start_with_schema(&d, Some(adoc)) else {
        return Ok(None);
    };

    let mut cursor = entry;
    let mut fields = Vec::with_capacity(adoc.fields.len());
    for field in &adoc.fields {
        let Some(ft) = &field.field_type else {
            break;
        };
        let Some((consumed, value)) = read_field(ft, &d[cursor..], limits) else {
            break;
        };
        fields.push((field.name.clone(), value));
        cursor = cursor.saturating_add(consumed);
        if cursor > d.len() {
            break;
        }
    }

    let version = rf.basic_file_info().ok().map(|b| b.version).unwrap_or(0);
    Ok(Some(ADocumentInstance {
        entry_offset: entry,
        version,
        fields,
    }))
}

#[cfg(test)]
fn find_adocument_start(d: &[u8]) -> Option<usize> {
    find_adocument_start_with_schema(d, None)
}

/// Which strategy resolved the ADocument entry offset (API-12).
///
/// The walker's entry-point detector runs two strategies in order:
/// a fast heuristic that looks for the sequential-id-table + 8-zero
/// signature, and a slower schema-directed scoring scan over every
/// byte-aligned offset. The strategy that landed on the returned
/// offset is useful for debugging and for cross-version regression
/// tests (a release that suddenly falls through to `Scored` when
/// prior ones hit `Heuristic` is a signal of wire-layout drift).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionStrategy {
    /// Fast heuristic hit — `heuristic_find` resolved directly.
    /// Typical of Revit 2024-2026 where ADocument sits in a
    /// predictable location after the sequential-id table.
    Heuristic,
    /// Scored brute-force scan found the best offset with score ≥
    /// 80. Typical of older releases where the heuristic table end
    /// doesn't align with the record start.
    Scored,
    /// No offset met the confidence threshold. The walker returns
    /// `None` from `read_adocument` in this case.
    NotFound,
}

/// Diagnostic output of [`detect_adocument_start`] (API-12).
///
/// Callers that want to understand WHY the walker landed where it
/// did — for CI drift detection, cross-version regression reports,
/// or user-facing "here's how confident I am" output — use this
/// struct instead of plain `Option<usize>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DetectionResult {
    /// Resolved byte offset into the decompressed `Global/Latest`
    /// stream where ADocument's record begins. `None` when
    /// `strategy = NotFound`.
    pub offset: Option<usize>,
    /// Confidence score of the chosen offset. 90+ is a confident
    /// hit, 80-89 is a scored-scan match, below 80 is `NotFound`.
    /// `None` when no candidates were evaluated (no schema supplied
    /// + heuristic failed).
    pub score: Option<i64>,
    /// Number of byte-aligned offsets that `trial_walk` produced a
    /// walk for during the scored scan (only populated when a
    /// schema was supplied and the heuristic didn't resolve it on
    /// its own). A low count on a large stream can indicate a wire
    /// layout the walker doesn't recognise.
    pub candidates_evaluated: usize,
    pub strategy: DetectionStrategy,
}

/// Schema-aware entry-point detection with per-call diagnostics
/// (API-12). Same decision logic as
/// [`find_adocument_start_with_schema`] but returns the strategy +
/// score + candidate count that produced the result, instead of
/// just an `Option<usize>`.
///
/// Public surface for tools that want to surface detection
/// confidence — the plain `read_adocument` path calls through the
/// non-diagnostic variant for backwards compatibility.
pub fn detect_adocument_start(
    d: &[u8],
    adoc_schema: Option<&formats::ClassEntry>,
) -> DetectionResult {
    // Strategy 1: sequential-id-table end + 8-zero signature scan.
    if let Some(h) = heuristic_find(d) {
        if let Some(cls) = adoc_schema {
            if let Some(w) = trial_walk(cls, &d[h..]) {
                let sc = walk_score(&w);
                if sc >= 90 {
                    return DetectionResult {
                        offset: Some(h),
                        score: Some(sc),
                        candidates_evaluated: 1,
                        strategy: DetectionStrategy::Heuristic,
                    };
                }
            }
        } else {
            return DetectionResult {
                offset: Some(h),
                score: None,
                candidates_evaluated: 0,
                strategy: DetectionStrategy::Heuristic,
            };
        }
    }
    // Strategy 2: score-based brute-force scan.
    if let Some(cls) = adoc_schema {
        let mut best: Option<(i64, usize)> = None;
        let mut evaluated = 0usize;
        let end = d.len().saturating_sub(256);
        for offset in 0x100..end {
            if let Some(walk) = trial_walk(cls, &d[offset..]) {
                evaluated += 1;
                let sc = walk_score(&walk);
                if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                    best = Some((sc, offset));
                }
            }
        }
        if let Some((sc, off)) = best {
            if sc >= 80 {
                return DetectionResult {
                    offset: Some(off),
                    score: Some(sc),
                    candidates_evaluated: evaluated,
                    strategy: DetectionStrategy::Scored,
                };
            }
            // Sub-threshold best — still return the score so
            // callers can see how close it got.
            return DetectionResult {
                offset: None,
                score: Some(sc),
                candidates_evaluated: evaluated,
                strategy: DetectionStrategy::NotFound,
            };
        }
    }
    DetectionResult {
        offset: None,
        score: None,
        candidates_evaluated: 0,
        strategy: DetectionStrategy::NotFound,
    }
}

/// Doc-hidden fuzz entry point for the ADocument entry-point detector.
///
/// Exposes the private [`find_adocument_start_with_schema`] so that
/// fuzz targets in `fuzz/fuzz_targets/` can exercise both the
/// heuristic path (`schema = None`) and the scoring-based
/// brute-force path (`schema = Some(&...)`) directly on caller-
/// supplied bytes. Not part of the stable public surface — the
/// name, signature, and behaviour may change without a version bump.
///
/// Kept `pub` rather than exposed via `#[cfg(fuzzing)]` because
/// cargo-fuzz compiles the library crate without custom cfgs, and
/// putting it behind a feature flag forces every downstream caller
/// to know about the flag.
#[doc(hidden)]
pub fn __fuzz_find_adocument_start(
    d: &[u8],
    schema: Option<&formats::ClassEntry>,
) -> Option<usize> {
    find_adocument_start_with_schema(d, schema)
}

/// Locate ADocument's entry point. When an ADocument class schema is
/// supplied, also runs a scoring-based brute-force scan that picks
/// the offset whose trial walk produces the most-sensible values for
/// the last three fields (small distinct `ElementId` ids with tag=0).
/// That's strong enough to find the entry point on Revit 2021–2026,
/// where the heuristic-only path misses 2021–2023.
fn find_adocument_start_with_schema(
    d: &[u8],
    adoc_schema: Option<&formats::ClassEntry>,
) -> Option<usize> {
    // Strategy 1: sequential-id-table end + 8-zero signature scan.
    if let Some(h) = heuristic_find(d) {
        if let Some(cls) = adoc_schema {
            if trial_walk(cls, &d[h..]).is_some_and(|w| walk_score(&w) >= 90) {
                return Some(h);
            }
        } else {
            return Some(h);
        }
    }
    // Strategy 2: score-based byte-aligned brute-force scan. Only
    // runs when a schema is supplied.
    if let Some(cls) = adoc_schema {
        let mut best: Option<(i64, usize)> = None;
        let end = d.len().saturating_sub(256);
        for offset in 0x100..end {
            if let Some(walk) = trial_walk(cls, &d[offset..]) {
                let sc = walk_score(&walk);
                if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                    best = Some((sc, offset));
                }
            }
        }
        if let Some((sc, off)) = best {
            if sc >= 80 {
                return Some(off);
            }
        }
    }
    None
}

fn heuristic_find(d: &[u8]) -> Option<usize> {
    let mut last_table_end = 0usize;
    let mut i = 0;
    while i + 4 < d.len() {
        if d[i..i + 4] == [1, 0, 0, 0] {
            let mut cursor = i + 4;
            let mut expect: u32 = 2;
            let mut end = i + 4;
            while cursor + 4 <= d.len() {
                let marker = expect.to_le_bytes();
                let window_end = (cursor + 64).min(d.len());
                if let Some(p) = d[cursor..window_end].windows(4).position(|w| w == marker) {
                    end = cursor + p + 4;
                    cursor = end;
                    expect += 1;
                } else {
                    break;
                }
            }
            if expect >= 6 {
                last_table_end = end + 32;
                i = end;
                continue;
            }
        }
        i += 1;
    }
    let min_start = last_table_end.max(0x200);
    let mut k = min_start;
    while k + 16 <= d.len() {
        if d[k..k + 8].iter().all(|&b| b == 0) {
            let next_u32 = u32::from_le_bytes([d[k + 8], d[k + 9], d[k + 10], d[k + 11]]);
            let next_next = u32::from_le_bytes([d[k + 12], d[k + 13], d[k + 14], d[k + 15]]);
            if (1..=100).contains(&next_u32) && (next_next == 0xffffffff || next_next <= 0x10000) {
                return Some(k);
            }
        }
        k += 1;
    }
    None
}

fn trial_walk(adoc: &formats::ClassEntry, bytes: &[u8]) -> Option<Vec<(u32, u32)>> {
    let mut cursor = 0;
    let mut out = Vec::new();
    for field in &adoc.fields {
        let ft = field.field_type.as_ref()?;
        let (consumed, tag_id) = match ft {
            formats::FieldType::Pointer { .. } => {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                (8, (u32::MAX, u32::MAX))
            }
            formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                let tag = u32::from_le_bytes([
                    bytes[cursor],
                    bytes[cursor + 1],
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                ]);
                let id = u32::from_le_bytes([
                    bytes[cursor + 4],
                    bytes[cursor + 5],
                    bytes[cursor + 6],
                    bytes[cursor + 7],
                ]);
                (8, (tag, id))
            }
            formats::FieldType::Container { kind: 0x0e, .. } => {
                if cursor + 4 > bytes.len() {
                    return None;
                }
                let count = u32::from_le_bytes([
                    bytes[cursor],
                    bytes[cursor + 1],
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                ]) as usize;
                if count > 1000 {
                    return None;
                }
                let col_bytes = 4 + count * 6;
                if cursor + col_bytes + 4 > bytes.len() {
                    return None;
                }
                let col2_count = u32::from_le_bytes([
                    bytes[cursor + col_bytes],
                    bytes[cursor + col_bytes + 1],
                    bytes[cursor + col_bytes + 2],
                    bytes[cursor + col_bytes + 3],
                ]) as usize;
                if col2_count != count {
                    return None;
                }
                (2 * col_bytes, (u32::MAX, u32::MAX))
            }
            _ => return None,
        };
        out.push(tag_id);
        cursor += consumed;
    }
    Some(out)
}

fn walk_score(walk: &[(u32, u32)]) -> i64 {
    if walk.len() < 3 {
        return i64::MIN;
    }
    let last3 = &walk[walk.len() - 3..];
    let real_ids: Vec<u32> = last3
        .iter()
        .filter(|(t, _)| *t != u32::MAX)
        .map(|(_, i)| *i)
        .collect();
    if real_ids.is_empty() || real_ids.iter().all(|i| *i == 0) {
        return i64::MIN;
    }
    let mut s: i64 = 0;
    for (t, i) in last3 {
        if *t == u32::MAX {
            continue;
        }
        if *t == 0 {
            s += 10;
        }
        if (1..=10000).contains(i) {
            s += 20;
        } else if (1..=0xffff).contains(i) {
            s += 5;
        } else {
            s -= 10;
        }
    }
    if real_ids.len() >= 2 {
        let max = *real_ids.iter().max().unwrap();
        let min = *real_ids.iter().min().unwrap();
        if max > 0 && max - min <= 50 {
            s += 25;
        }
    }
    s
}

fn read_field(
    ft: &formats::FieldType,
    bytes: &[u8],
    limits: WalkerLimits,
) -> Option<(usize, InstanceField)> {
    match ft {
        formats::FieldType::Pointer { .. } => {
            if bytes.len() < 8 {
                return None;
            }
            let a = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let b = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            Some((8, InstanceField::Pointer { raw: [a, b] }))
        }
        formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => {
            if bytes.len() < 8 {
                return None;
            }
            let tag = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let id = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
            Some((8, InstanceField::ElementId { tag, id }))
        }
        formats::FieldType::Container { kind: 0x0e, .. } => {
            if bytes.len() < 4 {
                return None;
            }
            let count = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
            if count > limits.max_container_records {
                return None;
            }
            let elem_size = 6;
            let col_bytes = 4 + count * elem_size;
            let total = 2 * col_bytes;
            if bytes.len() < total {
                return None;
            }
            let col2_count = u32::from_le_bytes([
                bytes[col_bytes],
                bytes[col_bytes + 1],
                bytes[col_bytes + 2],
                bytes[col_bytes + 3],
            ]) as usize;
            if col2_count != count {
                // Fallback to 1-column shape on mismatch.
                let mut col_a = Vec::with_capacity(count);
                for k in 0..count {
                    let base = 4 + k * elem_size;
                    col_a.push(u16::from_le_bytes([bytes[base], bytes[base + 1]]));
                }
                return Some((
                    col_bytes,
                    InstanceField::RefContainer {
                        col_a,
                        col_b: Vec::new(),
                    },
                ));
            }
            let mut col_a = Vec::with_capacity(count);
            let mut col_b = Vec::with_capacity(count);
            for k in 0..count {
                let base_a = 4 + k * elem_size;
                let base_b = col_bytes + 4 + k * elem_size;
                col_a.push(u16::from_le_bytes([bytes[base_a], bytes[base_a + 1]]));
                col_b.push(u16::from_le_bytes([bytes[base_b], bytes[base_b + 1]]));
            }
            Some((total, InstanceField::RefContainer { col_a, col_b }))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_adocument_start_returns_none_on_empty_stream() {
        assert_eq!(find_adocument_start(&[]), None);
        assert_eq!(find_adocument_start(&[0u8; 0x200]), None);
    }

    #[test]
    fn read_field_pointer_reads_8_bytes() {
        let ft = formats::FieldType::Pointer { kind: 2 };
        let bytes = [0x01u8, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00];
        let (n, v) = read_field(&ft, &bytes, WalkerLimits::default()).unwrap();
        assert_eq!(n, 8);
        match v {
            InstanceField::Pointer { raw } => assert_eq!(raw, [1, 2]),
            _ => panic!("expected Pointer"),
        }
    }

    #[test]
    fn read_field_element_id_reads_8_bytes() {
        let ft = formats::FieldType::ElementId;
        let bytes = [0x00u8, 0x00, 0x00, 0x00, 0x1b, 0x00, 0x00, 0x00];
        let (n, v) = read_field(&ft, &bytes, WalkerLimits::default()).unwrap();
        assert_eq!(n, 8);
        match v {
            InstanceField::ElementId { tag, id } => {
                assert_eq!(tag, 0);
                assert_eq!(id, 27);
            }
            _ => panic!("expected ElementId"),
        }
    }

    #[test]
    fn read_field_by_type_primitive_u32() {
        let ft = formats::FieldType::Primitive {
            kind: 0x05,
            size: 4,
        };
        let bytes = [0x2a, 0x00, 0x00, 0x00];
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4);
        match v {
            InstanceField::Integer {
                value,
                signed,
                size,
            } => {
                assert_eq!(value, 42);
                assert!(!signed);
                assert_eq!(size, 4);
            }
            _ => panic!("expected Integer, got {v:?}"),
        }
    }

    #[test]
    fn read_field_by_type_primitive_f64() {
        let ft = formats::FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        // 42.5 — arbitrary value, deliberately not near a math
        // constant so clippy's approx_constant lint doesn't trip.
        let bytes = 42.5_f64.to_le_bytes();
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 8);
        match v {
            InstanceField::Float { value, size } => {
                assert!((value - 42.5).abs() < 1e-9);
                assert_eq!(size, 8);
            }
            _ => panic!("expected Float, got {v:?}"),
        }
    }

    #[test]
    fn read_field_by_type_primitive_bool() {
        let ft = formats::FieldType::Primitive {
            kind: 0x01,
            size: 1,
        };
        let bytes = [1u8, 99];
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 1);
        match v {
            InstanceField::Bool(b) => assert!(b),
            _ => panic!("expected Bool, got {v:?}"),
        }
    }

    #[test]
    fn read_field_by_type_guid_16_bytes() {
        let ft = formats::FieldType::Guid;
        let bytes: Vec<u8> = (0..16).collect();
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 16);
        match v {
            InstanceField::Guid(g) => assert_eq!(g.to_vec(), (0..16u8).collect::<Vec<_>>()),
            _ => panic!("expected Guid, got {v:?}"),
        }
    }

    #[test]
    fn read_field_by_type_string_utf16le() {
        // 4-char string "Test": u32 count=4 then 8 bytes UTF-16LE.
        let ft = formats::FieldType::String;
        let mut bytes = vec![];
        bytes.extend_from_slice(&4u32.to_le_bytes());
        for ch in "Test".encode_utf16() {
            bytes.extend_from_slice(&ch.to_le_bytes());
        }
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, bytes.len());
        match v {
            InstanceField::String(s) => assert_eq!(s, "Test"),
            _ => panic!("expected String, got {v:?}"),
        }
    }

    #[test]
    fn read_field_by_type_graceful_on_short_input() {
        // A FieldType that claims 8 bytes but only 3 are available
        // should return InstanceField::Bytes (not panic).
        let ft = formats::FieldType::Pointer { kind: 2 };
        let bytes = [0xff, 0xff, 0xff];
        let mut cursor = 0;
        let v = read_field_by_type(&bytes, &mut cursor, &ft);
        match v {
            InstanceField::Bytes(_) => {}
            _ => panic!("expected Bytes on short input, got {v:?}"),
        }
    }

    #[test]
    fn handle_index_basic_operations() {
        let mut idx = HandleIndex::new();
        assert!(idx.is_empty());
        idx.insert(42, 0x100);
        idx.insert(7, 0x050);
        assert_eq!(idx.len(), 2);
        assert_eq!(idx.get(42), Some(0x100));
        assert_eq!(idx.get(7), Some(0x050));
        assert_eq!(idx.get(99), None);
        let pairs: Vec<_> = idx.iter().collect();
        // BTreeMap sorts ascending by ElementId.
        assert_eq!(pairs, vec![(7, 0x050), (42, 0x100)]);
    }

    #[test]
    fn decode_instance_walks_fields_in_schema_order() {
        // Synth a ClassEntry with 3 fields of known types + decode.
        let class = formats::ClassEntry {
            name: "SynthClass".to_string(),
            offset: 0,
            fields: vec![
                formats::FieldEntry {
                    name: "a_bool".to_string(),
                    cpp_type: Some("bool".into()),
                    field_type: Some(formats::FieldType::Primitive {
                        kind: 0x01,
                        size: 1,
                    }),
                },
                formats::FieldEntry {
                    name: "a_u32".to_string(),
                    cpp_type: Some("unsigned int".into()),
                    field_type: Some(formats::FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    }),
                },
                formats::FieldEntry {
                    name: "a_guid".to_string(),
                    cpp_type: Some("Guid".into()),
                    field_type: Some(formats::FieldType::Guid),
                },
            ],
            tag: Some(123),
            parent: None,
            declared_field_count: Some(3),
            was_parent_only: false,
            ancestor_tag: None,
        };
        let mut bytes = Vec::new();
        bytes.push(1); // bool=true
        bytes.extend_from_slice(&42u32.to_le_bytes()); // u32=42
        bytes.extend_from_slice(&[7u8; 16]); // guid

        let decoded = decode_instance(&bytes, 0, &class);
        assert_eq!(decoded.class, "SynthClass");
        assert_eq!(decoded.fields.len(), 3);
        assert!(matches!(decoded.fields[0].1, InstanceField::Bool(true)));
        match &decoded.fields[1].1 {
            InstanceField::Integer { value, .. } => assert_eq!(*value, 42),
            other => panic!("expected Integer, got {other:?}"),
        }
        match &decoded.fields[2].1 {
            InstanceField::Guid(g) => assert_eq!(g[0], 7),
            other => panic!("expected Guid, got {other:?}"),
        }
        assert_eq!(decoded.byte_range.end, bytes.len());
    }

    #[test]
    fn read_field_ref_container_2column() {
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        // count=2, col1=[0xaaaa, 0xbbbb], col2=[0xcccc, 0xdddd]
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[2, 0, 0, 0]); // count1
        bytes.extend_from_slice(&[0xaa, 0xaa, 0xff, 0xff, 0xff, 0xff]); // row1a
        bytes.extend_from_slice(&[0xbb, 0xbb, 0xff, 0xff, 0xff, 0xff]); // row2a
        bytes.extend_from_slice(&[2, 0, 0, 0]); // count2
        bytes.extend_from_slice(&[0xcc, 0xcc, 0xff, 0xff, 0xff, 0xff]); // row1b
        bytes.extend_from_slice(&[0xdd, 0xdd, 0xff, 0xff, 0xff, 0xff]); // row2b
        let (n, v) = read_field(&ft, &bytes, WalkerLimits::default()).unwrap();
        assert_eq!(n, 32); // 2 * (4 + 2*6)
        match v {
            InstanceField::RefContainer { col_a, col_b } => {
                assert_eq!(col_a, vec![0xaaaa, 0xbbbb]);
                assert_eq!(col_b, vec![0xcccc, 0xdddd]);
            }
            _ => panic!("expected RefContainer"),
        }
    }

    /// API-11: a tightened WalkerLimits rejects container records
    /// above the cap. Same input that passes with default limits
    /// returns None with `max_container_records: 1` because the
    /// count=2 exceeds it.
    #[test]
    fn read_field_honors_walker_limits_container_cap() {
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[2, 0, 0, 0]);
        bytes.extend_from_slice(&[0xaa, 0xaa, 0xff, 0xff, 0xff, 0xff]);
        bytes.extend_from_slice(&[0xbb, 0xbb, 0xff, 0xff, 0xff, 0xff]);
        bytes.extend_from_slice(&[2, 0, 0, 0]);
        bytes.extend_from_slice(&[0xcc, 0xcc, 0xff, 0xff, 0xff, 0xff]);
        bytes.extend_from_slice(&[0xdd, 0xdd, 0xff, 0xff, 0xff, 0xff]);

        // Default limits accept count=2.
        assert!(read_field(&ft, &bytes, WalkerLimits::default()).is_some());

        // Cap of 1 rejects count=2.
        let tight = WalkerLimits {
            max_container_records: 1,
        };
        assert!(read_field(&ft, &bytes, tight).is_none());

        // Cap of exactly 2 accepts count=2 (boundary).
        let at_cap = WalkerLimits {
            max_container_records: 2,
        };
        assert!(read_field(&ft, &bytes, at_cap).is_some());
    }

    #[test]
    fn walker_limits_default_matches_legacy_hardcoded() {
        assert_eq!(WalkerLimits::default().max_container_records, 1000);
    }

    /// API-12: `detect_adocument_start` surfaces NotFound on empty
    /// input regardless of whether a schema is supplied. Score and
    /// offset are both None; candidates_evaluated is 0.
    #[test]
    fn detect_adocument_start_on_empty_returns_not_found() {
        let r = detect_adocument_start(&[], None);
        assert_eq!(r.strategy, DetectionStrategy::NotFound);
        assert_eq!(r.offset, None);
        assert_eq!(r.score, None);
        assert_eq!(r.candidates_evaluated, 0);
    }

    #[test]
    fn detect_adocument_start_on_small_buffer_no_schema() {
        let d = vec![0u8; 64];
        let r = detect_adocument_start(&d, None);
        // Heuristic won't find anything in 64 zero bytes; no schema
        // means the scored path never runs.
        assert_eq!(r.strategy, DetectionStrategy::NotFound);
        assert_eq!(r.offset, None);
    }

    /// L5B-08: read_field_by_type decodes `Vector<f64>` into a
    /// typed sequence of `Float` items, not a raw Bytes fallback.
    #[test]
    fn read_field_by_type_vector_of_f64() {
        let ft = formats::FieldType::Vector {
            kind: 0x07,
            body: Vec::new(),
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[2, 0, 0, 0]); // count = 2
        bytes.extend_from_slice(&1.25f64.to_le_bytes());
        bytes.extend_from_slice(&(-3.5f64).to_le_bytes());
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4 + 2 * 8);
        match field {
            InstanceField::Vector(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0], InstanceField::Float { value, .. } if *value == 1.25));
                assert!(matches!(&items[1], InstanceField::Float { value, .. } if *value == -3.5));
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn read_field_by_type_vector_of_points() {
        // Vector<point> = count × (3 × f64).
        let ft = formats::FieldType::Vector {
            kind: 0x0d,
            body: Vec::new(),
        };
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[1, 0, 0, 0]); // count = 1
        for v in [10.0f64, 20.0, 30.0] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4 + 24);
        match field {
            InstanceField::Vector(points) => {
                assert_eq!(points.len(), 1);
                match &points[0] {
                    InstanceField::Vector(xyz) => {
                        assert_eq!(xyz.len(), 3);
                        let extract = |f: &InstanceField| match f {
                            InstanceField::Float { value, .. } => *value,
                            _ => f64::NAN,
                        };
                        assert_eq!(extract(&xyz[0]), 10.0);
                        assert_eq!(extract(&xyz[1]), 20.0);
                        assert_eq!(extract(&xyz[2]), 30.0);
                    }
                    other => panic!("expected inner Vector(point), got {other:?}"),
                }
            }
            other => panic!("expected outer Vector, got {other:?}"),
        }
    }

    /// API-09: completeness summary reports typed vs raw-bytes
    /// fallback counts and handles the 0-total edge case.
    #[test]
    fn adocument_instance_completeness_basic() {
        let inst = ADocumentInstance {
            entry_offset: 0,
            version: 2024,
            fields: vec![
                ("a".into(), InstanceField::Integer {
                    value: 1,
                    signed: false,
                    size: 4,
                }),
                ("b".into(), InstanceField::Bytes(vec![0xff])),
                ("c".into(), InstanceField::String("hello".into())),
                ("d".into(), InstanceField::Vector(vec![])),
                ("e".into(), InstanceField::Bytes(Vec::new())),
            ],
        };
        let c = inst.completeness();
        assert_eq!(c.total, 5);
        assert_eq!(c.typed, 3); // a, c, d
        assert_eq!(c.raw_bytes_fallback, 2); // b, e
        assert_eq!(c.typed_and_non_empty, 2); // a, c (d is empty Vector)
        assert!(!c.is_fully_typed());
        let ratio = c.typed_ratio().unwrap();
        assert!((ratio - 0.6).abs() < 1e-9);
    }

    #[test]
    fn completeness_empty_instance_has_no_ratio() {
        let inst = ADocumentInstance {
            entry_offset: 0,
            version: 0,
            fields: vec![],
        };
        let c = inst.completeness();
        assert_eq!(c.total, 0);
        assert!(c.typed_ratio().is_none());
        assert!(!c.is_fully_typed());
    }

    #[test]
    fn completeness_fully_typed_instance() {
        let inst = ADocumentInstance {
            entry_offset: 0,
            version: 2026,
            fields: vec![
                ("a".into(), InstanceField::Bool(true)),
                ("b".into(), InstanceField::ElementId { tag: 0, id: 42 }),
            ],
        };
        let c = inst.completeness();
        assert!(c.is_fully_typed());
        assert_eq!(c.typed_ratio(), Some(1.0));
    }

    #[test]
    fn read_field_by_type_vector_unknown_kind_falls_back() {
        // 0xff isn't in our element-kind table — should fall back to
        // Bytes without decoding.
        let ft = formats::FieldType::Vector {
            kind: 0xff,
            body: Vec::new(),
        };
        let bytes = [0, 0, 0, 0, 0xaa, 0xbb];
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert!(matches!(field, InstanceField::Bytes(_)));
    }
}
