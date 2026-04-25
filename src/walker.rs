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

/// Mirror of [`ElementDecoder`] for the write path (WRT-03). A per-
/// class encoder takes a `DecodedElement` (typically produced by the
/// decoder, possibly with a field or two mutated via the `writer`
/// module's patch API) and a schema entry, and emits the
/// instance-data bytes.
///
/// The default implementation [`encode_instance`] is schema-driven
/// and works for any class whose fields all map to a canonical
/// `FieldType` pattern — the same space the generic decoder covers.
/// Per-class encoders override [`Self::encode`] when a class needs
/// out-of-schema framing (ADocument's preamble, Container 2-column
/// layout, etc.).
///
/// ```
/// use rvt::walker::{ElementEncoder, DecodedElement, encode_instance};
/// use rvt::formats::ClassEntry;
///
/// struct WallEncoder;
/// impl ElementEncoder for WallEncoder {
///     fn class_name(&self) -> &'static str { "Wall" }
///     // default encode() uses encode_instance() — no override needed
/// }
/// ```
pub trait ElementEncoder: Sync + Send {
    /// The class's name as it appears in `Formats/Latest`. Must
    /// match the paired `ElementDecoder::class_name` for round-trip.
    fn class_name(&self) -> &'static str;

    /// Serialise a single instance of this class back to its wire
    /// bytes. The default implementation walks `decoded.fields` in
    /// schema order and calls [`write_field_by_type`] for each,
    /// producing a byte sequence identical to what the reader
    /// originally consumed.
    ///
    /// Field count mismatch between `decoded.fields` and
    /// `schema.fields` is tolerated: the encoder pairs fields by
    /// schema index, so extra decoded fields are ignored and
    /// missing ones emit nothing (consistent with the reader's
    /// best-effort philosophy).
    fn encode(&self, decoded: &DecodedElement, schema: &formats::ClassEntry) -> Vec<u8> {
        encode_instance(decoded, schema)
    }
}

/// Inverse of [`read_field_by_type`] (WRT-03). Serialises a single
/// `InstanceField` value into `out` using the declared
/// [`formats::FieldType`] to pick the wire layout.
///
/// Mismatches between `value` and `ty` (e.g. `value = String` but
/// `ty = Primitive`) fall back to emitting whatever bytes the
/// variant already carries when the decoder encountered an
/// untypable field — [`InstanceField::Bytes`] is written verbatim,
/// other mismatches emit an empty slice rather than panicking.
/// The write path stays round-trip-safe for every field the
/// decoder understood cleanly.
pub fn write_field_by_type(value: &InstanceField, ty: &formats::FieldType, out: &mut Vec<u8>) {
    use formats::FieldType;

    // Catch-all: if the reader fell back to Bytes because the
    // FieldType / layout wasn't covered, re-emit the bytes verbatim
    // regardless of `ty`. This makes any round-trip
    // decode→encode→decode stable even for unknown fields.
    if let InstanceField::Bytes(raw) = value {
        out.extend_from_slice(raw);
        return;
    }

    match (ty, value) {
        (FieldType::Primitive { kind, size }, v) => {
            let n = *size as usize;
            match (*kind, *size, v) {
                (0x01, _, InstanceField::Bool(b)) => {
                    out.push(if *b { 1 } else { 0 });
                    // Primitive bool on disk is always one byte — even
                    // when schema's declared size is greater (padding).
                    for _ in 1..n {
                        out.push(0);
                    }
                }
                (0x02, 2, InstanceField::Integer { value, .. }) => {
                    out.extend_from_slice(&(*value as u16).to_le_bytes());
                }
                (0x04, 4, InstanceField::Integer { value, .. })
                | (0x05, 4, InstanceField::Integer { value, .. }) => {
                    out.extend_from_slice(&(*value as u32).to_le_bytes());
                }
                (0x06, 4, InstanceField::Float { value, .. }) => {
                    out.extend_from_slice(&(*value as f32).to_le_bytes());
                }
                (0x07, 8, InstanceField::Float { value, .. }) => {
                    out.extend_from_slice(&value.to_le_bytes());
                }
                (0x0b, 8, InstanceField::Integer { value, .. }) => {
                    out.extend_from_slice(&value.to_le_bytes());
                }
                _ => {} // shape mismatch — nothing to emit.
            }
        }
        (FieldType::String, InstanceField::String(s)) => {
            // UTF-16LE length-prefixed. Char count is (u32) number of
            // UTF-16 code units, then that many × 2 bytes.
            let utf16: Vec<u16> = s.encode_utf16().collect();
            out.extend_from_slice(&(utf16.len() as u32).to_le_bytes());
            for code in utf16 {
                out.extend_from_slice(&code.to_le_bytes());
            }
        }
        (FieldType::Guid, InstanceField::Guid(g)) => {
            out.extend_from_slice(g);
        }
        (
            FieldType::ElementId | FieldType::ElementIdRef { .. },
            InstanceField::ElementId { tag, id },
        ) => {
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&id.to_le_bytes());
        }
        (FieldType::Pointer { .. }, InstanceField::Pointer { raw }) => {
            out.extend_from_slice(&raw[0].to_le_bytes());
            out.extend_from_slice(&raw[1].to_le_bytes());
        }
        (FieldType::Vector { kind, .. }, InstanceField::Vector(items)) => {
            out.extend_from_slice(&(items.len() as u32).to_le_bytes());
            for item in items {
                match (*kind, item) {
                    (0x01, InstanceField::Bool(b)) => out.push(if *b { 1 } else { 0 }),
                    (0x04, InstanceField::Integer { value, .. })
                    | (0x05, InstanceField::Integer { value, .. }) => {
                        out.extend_from_slice(&(*value as u32).to_le_bytes());
                    }
                    (0x07, InstanceField::Float { value, .. }) => {
                        out.extend_from_slice(&value.to_le_bytes());
                    }
                    (0x0b, InstanceField::Integer { value, .. }) => {
                        out.extend_from_slice(&value.to_le_bytes());
                    }
                    (0x0d, InstanceField::Vector(point)) => {
                        // point = 3 × f64 — walk up to three floats
                        // (shape was emitted by the reader).
                        for p in point.iter().take(3) {
                            if let InstanceField::Float { value, .. } = p {
                                out.extend_from_slice(&value.to_le_bytes());
                            } else {
                                // Point component missing; emit zero
                                // to preserve the 24-byte stride.
                                out.extend_from_slice(&0.0_f64.to_le_bytes());
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        (FieldType::Container { kind, .. }, InstanceField::Vector(_))
            if matches!(*kind, 0x01 | 0x02 | 0x04 | 0x05 | 0x07 | 0x0b | 0x0d) =>
        {
            // L5B-09.5: scalar-base Container round-trips through the
            // same `[u32 count][count × element]` wire layout as
            // Vector (reader delegates in the opposite direction).
            // Recurse with a synthesised Vector FieldType so the
            // reader's decoded Vector of items serialises back to
            // the same bytes. Reference Containers (kind=0x0e) still
            // need the explicit ADocument-side `write_adocument_field`
            // path (2-column layout).
            let fake_vec = FieldType::Vector {
                kind: *kind,
                body: Vec::new(),
            };
            write_field_by_type(value, &fake_vec, out);
        }
        (FieldType::Container { .. } | FieldType::Unknown { .. }, _) => {
            // Decoder uses Bytes fallback for these — the Bytes arm
            // at the top caught the Bytes case; shape-mismatched
            // values (typed value but container/unknown FieldType)
            // are dropped. The writer stays safe — no bytes emitted
            // for ambiguous pairings.
        }
        // All other type+value mismatches: emit nothing.
        _ => {}
    }
}

/// Encode an `ADocument`-level field (WRT-02). Mirrors
/// `read_field` — handles `Pointer`, `ElementId` / `ElementIdRef`
/// (8-byte `tag,id` layout), and `Container { kind: 0x0e, .. }`
/// (2-column layout via [`encode_ref_container`]). All other
/// field types route through [`write_field_by_type`], which covers
/// the general wire shape.
///
/// Use this when encoding ADocument instance bytes back into
/// Global/Latest, because the ADocument reader uses `read_field`
/// rather than the generic `read_field_by_type`.
pub fn write_adocument_field(value: &InstanceField, ft: &formats::FieldType, out: &mut Vec<u8>) {
    match (ft, value) {
        (formats::FieldType::Pointer { .. }, InstanceField::Pointer { raw }) => {
            out.extend_from_slice(&raw[0].to_le_bytes());
            out.extend_from_slice(&raw[1].to_le_bytes());
        }
        (
            formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. },
            InstanceField::ElementId { tag, id },
        ) => {
            out.extend_from_slice(&tag.to_le_bytes());
            out.extend_from_slice(&id.to_le_bytes());
        }
        (
            formats::FieldType::Container { kind: 0x0e, .. },
            InstanceField::RefContainer { col_a, col_b },
        ) => {
            out.extend_from_slice(&encode_ref_container(col_a, col_b));
        }
        // Any Bytes fallback is emitted verbatim regardless of ft.
        (_, InstanceField::Bytes(raw)) => {
            out.extend_from_slice(raw);
        }
        // Everything else delegates to the generic writer.
        _ => write_field_by_type(value, ft, out),
    }
}

/// Serialise an ADocument instance back to its wire bytes (WRT-02).
/// Inverse of the read_adocument decode path.
///
/// Walks `adoc.fields` in schema order (not decoded order, so the
/// caller's decoded `InstanceField` list is paired by index with
/// `schema.fields`) and uses [`write_adocument_field`] per field.
///
/// Callers producing a new Global/Latest payload append the result
/// to the decoded prefix bytes (everything before `entry_offset`),
/// then re-encode with `truncated_gzip_encode_with_prefix8` and
/// pass through `write_with_patches` as a `CustomPrefix8` stream
/// patch.
pub fn encode_adocument_fields(
    schema: &formats::ClassEntry,
    fields: &[(String, InstanceField)],
) -> Vec<u8> {
    let mut out = Vec::new();
    for (idx, schema_field) in schema.fields.iter().enumerate() {
        let Some((_, value)) = fields.get(idx) else {
            break;
        };
        if let Some(ft) = schema_field.field_type.as_ref() {
            write_adocument_field(value, ft, &mut out);
        } else if let InstanceField::Bytes(raw) = value {
            out.extend_from_slice(raw);
        }
    }
    out
}

/// Encode a 2-column reference container (WRT-09) — the inverse of
/// `read_field`'s `Container { kind: 0x0e, .. }` path. The on-
/// disk layout for this field shape is:
///
/// ```text
/// [u32 LE count_a] [count_a × (u16 LE id + 4 bytes padding)]
/// [u32 LE count_b] [count_b × (u16 LE id + 4 bytes padding)]
/// ```
///
/// When `col_a.len() != col_b.len()` the reader falls back to a
/// single-column shape. This writer always emits the full two-
/// column form — if callers want the single-column fallback, they
/// should pass an empty `col_b` (which emits an empty column with
/// `count_b = 0`, matching the on-disk shape for a missing pair).
///
/// Per-element padding is zero-filled — the reader ignores those
/// four bytes, so the value is write-time free.
pub fn encode_ref_container(col_a: &[u16], col_b: &[u16]) -> Vec<u8> {
    const ELEM_SIZE: usize = 6;
    let count_a = col_a.len();
    let count_b = col_b.len();
    let mut out = Vec::with_capacity(2 * (4 + ELEM_SIZE * count_a.max(count_b)));
    out.extend_from_slice(&(count_a as u32).to_le_bytes());
    for id in col_a {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&[0u8; 4]); // padding
    }
    out.extend_from_slice(&(count_b as u32).to_le_bytes());
    for id in col_b {
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&[0u8; 4]); // padding
    }
    out
}

/// Serialise a whole `DecodedElement` back to its wire bytes
/// (WRT-03). Inverse of [`decode_instance`]. Walks the schema's
/// declared fields in order and, for each, calls
/// [`write_field_by_type`] with the matching `InstanceField` from
/// `decoded.fields`.
///
/// Round-trip: `encode_instance(&decoded_instance(b, 0, schema),
/// schema) == b` for any `b` where the decoder produces a
/// non-fallback `InstanceField` for every schema field. Callers
/// should sanity-check `Completeness::typed_ratio()` before
/// relying on round-trip equality.
pub fn encode_instance(decoded: &DecodedElement, schema: &formats::ClassEntry) -> Vec<u8> {
    let mut out = Vec::with_capacity(decoded.byte_range.len());
    for (idx, schema_field) in schema.fields.iter().enumerate() {
        let Some((_, value)) = decoded.fields.get(idx) else {
            continue;
        };
        if let Some(ft) = schema_field.field_type.as_ref() {
            write_field_by_type(value, ft, &mut out);
        } else if let InstanceField::Bytes(raw) = value {
            // No FieldType declared — emit whatever the reader
            // captured as raw bytes.
            out.extend_from_slice(raw);
        }
    }
    out
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
        FieldType::Container { kind, .. } => {
            // L5B-09.4: scalar-base Container (kinds 0x01/0x02/0x04/
            // 0x05/0x07/0x0b/0x0d) use the same `[u32 count][count ×
            // element]` wire layout as Vector — the sub=0x0050 tag
            // only distinguishes semantic role (std::set / std::map
            // keyset / …), not the scalar encoding. We reuse the
            // Vector decode path by recursing with a synthesised
            // Vector FieldType of the same kind.
            //
            // Reference containers (kind=0x0e) have a 2-column
            // layout handled by the ADocument-walker-facing
            // `read_field` (see `FieldType::Container { kind: 0x0e,
            // .. }` arm there). Generic callers hitting the 0x0e
            // arm here get 4 bytes as a fallback.
            if matches!(*kind, 0x01 | 0x02 | 0x04 | 0x05 | 0x07 | 0x0b | 0x0d) {
                let fake_vec = FieldType::Vector {
                    kind: *kind,
                    body: Vec::new(),
                };
                return read_field_by_type(bytes, cursor, &fake_vec);
            }
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
    /// Pointer bytes from the `m_elem_table` field, when the schema
    /// and decoded payload surfaced it (L5B-01).
    ///
    /// ADocument carries a pointer to the document-wide element
    /// index table — the map from `ElementId` to per-element record
    /// location in `Global/Latest`. The pointer appears in the
    /// schema as a field named `m_elem_table` (sometimes
    /// `m_elemTable` or a normalised variant), typed as
    /// [`crate::formats::FieldType::Pointer`]. The walker decodes
    /// the field to [`InstanceField::Pointer`] carrying the raw
    /// 8-byte `[u32 slot_a][u32 slot_b]` payload.
    ///
    /// This helper looks up that field by name (accepting the
    /// common spelling variants) and returns the raw pointer
    /// tuple. Returns `None` when:
    ///
    /// - No field named `m_elem_table` (or its variants) is
    ///   present. This happens on older Revit releases where the
    ///   ADocument wire layout hasn't been fully decoded.
    /// - The field IS present but wasn't decoded to
    ///   `InstanceField::Pointer` — e.g. the typed walker fell
    ///   back to raw `Bytes` when the `FieldType` classification
    ///   didn't produce a pointer. This is a sign of schema drift
    ///   or cross-version wire changes.
    /// - The pointer payload is the sentinel NULL `[0, 0]` or
    ///   `[0xFFFF_FFFF, 0xFFFF_FFFF]` value (Revit uses both to
    ///   mean "no element table"; treating them uniformly as
    ///   `None` matches the walker's semantic).
    ///
    /// Non-None return means the pointer exists and is non-null.
    /// Consuming it (following into the referenced bytes and
    /// parsing an actual element index) is separate — see the
    /// `walk_elem_table_*` entry points on `RevitFile`.
    pub fn elem_table_pointer(&self) -> Option<[u32; 2]> {
        const NAMES: &[&str] = &[
            "m_elem_table",
            "m_elemtable",
            "elem_table",
            "elemtable",
            "elementtable",
            "m_element_table",
        ];
        for (field_name, value) in &self.fields {
            let norm = field_name
                .trim_start_matches('m')
                .trim_start_matches('_')
                .to_lowercase();
            let compacted = norm.replace('_', "");
            let matched = NAMES
                .iter()
                .any(|n| n.eq_ignore_ascii_case(field_name) || *n == compacted);
            if !matched {
                continue;
            }
            if let InstanceField::Pointer { raw } = value {
                // Revit sentinels: [0, 0] = NULL, [!0, !0] = "not
                // yet set." Both mean "no element table to walk."
                if *raw == [0, 0] || *raw == [u32::MAX, u32::MAX] {
                    return None;
                }
                return Some(*raw);
            }
        }
        None
    }

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
    let (_, d) = compression::inflate_at_auto(&raw)?;
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

/// Strict variant of [`read_adocument`] (API-07). Returns `Err` if
/// the walker's `Completeness` summary flags any raw-bytes
/// fallback, OR if the ADocument entry-point detector couldn't
/// confidently land on the record.
///
/// Use when downstream code can't tolerate a partial decode —
/// e.g. CI gates, round-trip verification, cross-version
/// correctness checks. The contract is: if this returns `Ok`, the
/// `ADocumentInstance` has every declared field decoded into a
/// typed `InstanceField` (no `Bytes` fallbacks).
pub fn read_adocument_strict(rf: &mut RevitFile) -> Result<ADocumentInstance> {
    let Some(inst) = read_adocument(rf)? else {
        return Err(Error::BasicFileInfo(
            "ADocument entry-point detector returned None".into(),
        ));
    };
    let c = inst.completeness();
    if !c.is_fully_typed() {
        return Err(Error::BasicFileInfo(format!(
            "ADocument decode incomplete: {} of {} fields fell back to raw bytes",
            c.raw_bytes_fallback, c.total
        )));
    }
    Ok(inst)
}

/// Lossy variant of [`read_adocument`] (API-08). Returns a
/// [`crate::parse_mode::Decoded<ADocumentInstance>`] that surfaces
/// the per-instance completeness summary as diagnostics rather
/// than short-circuiting.
///
/// Contract:
/// - `Ok(Decoded { complete: true, diagnostics: empty, .. })` —
///   every field decoded cleanly (same bar as the strict variant).
/// - `Ok(Decoded { complete: false, diagnostics, .. })` — record
///   was reached but at least one field fell back to raw bytes.
///   Each partial field appears in `diagnostics.partial_fields`;
///   `diagnostics.confidence` is set to `Completeness::typed_ratio`
///   so callers can threshold on it.
/// - `Err(_)` — stream-level failure (BasicFileInfo unreadable,
///   Global/Latest inflate failure, schema parse failure). A
///   stream-level error is still fatal even for the lossy path.
/// - `Ok(Decoded { value, diagnostics })` where
///   `diagnostics.failed_streams` contains "ADocument" — entry
///   detector returned None. Value is a default `ADocumentInstance`
///   placeholder (`entry_offset=0, version=0, fields=vec![]`).
pub fn read_adocument_lossy(
    rf: &mut RevitFile,
) -> Result<crate::parse_mode::Decoded<ADocumentInstance>> {
    use crate::parse_mode::{Decoded, Diagnostics};

    let Some(inst) = read_adocument(rf)? else {
        let mut d = Diagnostics::default();
        d.fail_stream("ADocument");
        let placeholder = ADocumentInstance {
            entry_offset: 0,
            version: 0,
            fields: Vec::new(),
        };
        return Ok(Decoded::partial(placeholder, d));
    };

    let c = inst.completeness();
    if c.is_fully_typed() {
        return Ok(Decoded::complete(inst));
    }

    let mut diagnostics = Diagnostics::default();
    for (name, value) in &inst.fields {
        if matches!(value, InstanceField::Bytes(_)) {
            diagnostics.partial_field(name.clone());
        }
    }
    diagnostics.confidence = c.typed_ratio().map(|r| r as f32);
    Ok(Decoded::partial(inst, diagnostics))
}

/// Decode every element recoverable from `rf`'s `Global/Latest`
/// stream, returning an iterator over [`DecodedElement`] values
/// with their `id` field populated when a self-id was resolvable.
///
/// This is L5B-11.6 — the high-level walker entry point that
/// [`crate::ifc::RvtDocExporter`] will call to emit per-element IFC
/// entities. Pipeline:
///   1. Read + decompress `Formats/Latest` → [`crate::formats::SchemaTable`].
///   2. Read + decompress `Global/Latest` → raw bytes.
///   3. Run [`scan_candidates`] with `min_score = 0` — every offset
///      where `trial_walk` produced a non-degenerate score. The
///      `80` threshold is calibrated for the 16-field ADocument
///      entry-point detector and filters out simple element
///      classes whose `walk_score` is structurally bounded (most
///      have fewer than 3 trailing ElementIds, which caps the
///      score below the ADocument band).
///   4. For each candidate, run [`decode_instance`] to produce a
///      typed `DecodedElement`, then extract the self-id via
///      [`find_self_id_field`].
///   5. Dedup: first-seen (highest-score) wins — if two candidates
///      claim the same `ElementId`, only the higher-score offset
///      survives. Scoreless / id-zero candidates are still yielded
///      only by the explicit diagnostic path.
///
/// Returns a materialised `Vec<DecodedElement>::into_iter()` rather
/// than a lazy iterator — each element requires upfront schema +
/// stream reads that are stateful (mut reader), so lazy iteration
/// would leak lifetimes. `impl Iterator` preserves the combinator-
/// friendly return type without committing to a concrete type.
///
/// Default `min_score = 80` is conservative by design. It avoids
/// surfacing low-score parent-class artifacts such as `HostObjAttr`
/// as production elements. Use [`iter_elements_with_options`] with
/// [`DIAGNOSTIC_ELEMENT_MIN_SCORE`] for reverse-engineering probes
/// that need the broad candidate set.
pub fn iter_elements(rf: &mut RevitFile) -> Result<impl Iterator<Item = DecodedElement>> {
    iter_elements_with_options(rf, PRODUCTION_ELEMENT_MIN_SCORE)
}

/// Same as [`iter_elements`], with caller-supplied candidate score
/// threshold. Pass `i64::MIN + 1` to yield every
/// `scan_candidates`-matched offset (useful for audit / coverage
/// reporting against the `ElemTable` declared set).
///
/// This is the diagnostic/probe API. Production callers should use
/// [`iter_elements`], which applies [`PRODUCTION_ELEMENT_MIN_SCORE`]
/// and avoids low-confidence parent-only matches.
pub fn iter_elements_with_options(
    rf: &mut RevitFile,
    min_score: i64,
) -> Result<impl Iterator<Item = DecodedElement>> {
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_at(&formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, d) = compression::inflate_at_auto(&raw)?;

    let class_by_name: std::collections::HashMap<&str, &formats::ClassEntry> = schema
        .classes
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();

    let candidates = scan_candidates(&schema, &d, min_score);
    let mut out = Vec::with_capacity(candidates.len());
    let mut seen_ids = std::collections::HashSet::<u32>::new();

    for cand in candidates {
        let Some(cls) = class_by_name.get(cand.class_name.as_str()).copied() else {
            continue;
        };
        let mut decoded = decode_instance(&d, cand.offset, cls);

        // Extract the self-id without holding a borrow of
        // `decoded` when we later assign `decoded.id`. The
        // find_self_id_field index is stable and cheap to recompute.
        let self_id = find_self_id_field(cls)
            .and_then(|idx| decoded.fields.get(idx))
            .and_then(|(_, field)| match field {
                InstanceField::ElementId { id, .. } if *id != 0 => Some(*id),
                _ => None,
            });

        if let Some(id) = self_id {
            // First-seen wins — scan_candidates iterates in score-
            // desc order, so the highest-score offset for each id
            // is the one that ends up in the output.
            if !seen_ids.insert(id) {
                continue;
            }
            decoded.id = Some(id);
        }

        out.push(decoded);
    }

    Ok(out.into_iter())
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
/// (API-12). Same decision logic as the internal
/// `find_adocument_start_with_schema`, but returns the strategy +
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

/// Candidate scan result — one plausible `(offset, class)` pair
/// produced by [`scan_candidates`].
///
/// The `score` reflects how well the `trial_walk` of `class_name`
/// at `offset` lined up with real-looking ElementId values (see
/// [`walk_score`]). Callers apply a threshold (typically ≥ 80)
/// before trusting the match.
#[derive(Debug, Clone)]
pub struct ScanCandidate {
    /// Byte offset in the buffer where the candidate instance starts.
    pub offset: usize,
    /// Schema class this candidate claims to be an instance of.
    pub class_name: String,
    /// Class-tag u16 the pre-filter matched at `offset - 2`.
    pub class_tag: u16,
    /// Heuristic score from `walk_score`; higher is more confident.
    pub score: i64,
}

/// Scan `bytes` for plausible element-instance starts using the
/// schema's class-tag table as a pre-filter.
///
/// Pipeline:
/// 1. Build a map `u16 class_tag -> &ClassEntry` from the schema.
/// 2. At each even-byte-aligned offset in `bytes`, read a `u16`
///    and check whether it matches any known class tag.
/// 3. For each hit, run `trial_walk(class, &bytes[offset + 2..])`
///    — the instance data starts **after** the 2-byte class-tag
///    prefix. If the walk succeeds, score it.
/// 4. Return all candidates above `min_score`, sorted by score
///    descending.
///
/// Cost: `O(bytes.len() / 2 × avg_classes_per_tag)`. On a 1 MB
/// `Global/Latest` with ~400 classes, typical tag-table lookup is
/// `O(1)` (no collisions observed) so this is effectively linear.
///
/// Caveat: this is the *raw* candidate list. A downstream pass
/// (see `build_handle_index`) still needs to extract the self-id
/// from each candidate and dispute-resolve when multiple candidates
/// at different offsets claim the same id.
pub fn scan_candidates(
    schema: &formats::SchemaTable,
    bytes: &[u8],
    min_score: i64,
) -> Vec<ScanCandidate> {
    // Index the schema by class tag. Classes without an explicit
    // tag (parent-only entries) are skipped — they won't appear as
    // instance headers anyway.
    let mut tag_to_class: std::collections::HashMap<u16, Vec<&formats::ClassEntry>> =
        std::collections::HashMap::with_capacity(schema.classes.len());
    for cls in &schema.classes {
        if let Some(tag) = cls.tag {
            tag_to_class.entry(tag).or_default().push(cls);
        }
    }

    let mut out: Vec<ScanCandidate> = Vec::new();
    if bytes.len() < 4 {
        return out;
    }

    // 2-byte-aligned scan — class tags are u16 on the wire and are
    // always aligned to the containing record's start. A 1-byte
    // shift wouldn't give a valid instance.
    let end = bytes.len().saturating_sub(2);
    let mut i = 0usize;
    while i + 2 <= end {
        let tag = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if let Some(classes) = tag_to_class.get(&tag) {
            // Instance data starts AFTER the tag. Give trial_walk
            // the post-tag slice.
            let instance_start = i + 2;
            if instance_start < bytes.len() {
                for cls in classes {
                    if let Some(walk) = trial_walk(cls, &bytes[instance_start..]) {
                        let score = walk_score(&walk);
                        if score >= min_score {
                            out.push(ScanCandidate {
                                offset: instance_start,
                                class_name: cls.name.clone(),
                                class_tag: tag,
                                score,
                            });
                        }
                    }
                }
            }
        }
        i += 2;
    }

    // Sort by score descending so the highest-confidence matches
    // surface first. Stable sort keeps within-score order == scan
    // order, which is also byte-ascending — useful for downstream
    // deduplication.
    out.sort_by_key(|c| std::cmp::Reverse(c.score));
    out
}

/// Minimum candidate score used by production element iteration.
///
/// The threshold matches the ADocument detector's "confident enough
/// to trust" band and filters low-score parent-class artifacts such
/// as `HostObjAttr` hits observed on real project files.
pub const PRODUCTION_ELEMENT_MIN_SCORE: i64 = 80;

/// Minimum candidate score for diagnostic element scans.
///
/// This intentionally keeps broad, noisy candidate output available
/// for reverse-engineering probes without using it in production
/// APIs or user-facing IFC export.
pub const DIAGNOSTIC_ELEMENT_MIN_SCORE: i64 = 0;

/// Locate the field index in `cls` that carries the instance's own
/// `ElementId` (the "self-id"). Used by [`build_handle_index`] to
/// extract the id from each scan_candidates hit so the
/// `ElementId → byte offset` map can be populated.
///
/// Detection order (first match wins):
///   1. Field named exactly `m_id` with type `ElementId` or
///      `ElementIdRef`. This is the canonical Revit convention —
///      `src/formats.rs` line 1186 pins it as the name used by
///      `UserID` and other concrete element classes.
///   2. Field named `m_id64`, `m_handle`, or `m_elementId` with a
///      matching ElementId type. These appear on a handful of
///      classes (e.g. history records) where the primary id uses an
///      alternate name.
///   3. First field whose type is the bare `ElementId` variant
///      (unit). `ElementIdRef { referenced_tag, .. }` fields point
///      to *other* classes — the self-id is always a bare `ElementId`
///      because a class can't statically name its own tag in the
///      schema record.
///
/// Returns `None` for:
///   - parent-only classes that have no `ElementId` field at all,
///   - classes where every `ElementId` field is actually a
///     `ElementIdRef` (meaning every id is a pointer to another
///     element's id, never the instance's own).
///
/// Callers treat `None` as "this class cannot be indexed" — it is
/// not an error, just a signal that `build_handle_index` should
/// skip candidates for this class.
pub fn find_self_id_field(cls: &formats::ClassEntry) -> Option<usize> {
    use formats::FieldType;

    // Priority 1/2: canonical self-id names, in preference order.
    const CANONICAL_NAMES: &[&str] = &["m_id", "m_id64", "m_handle", "m_elementId"];
    for canonical in CANONICAL_NAMES {
        if let Some((idx, _)) = cls.fields.iter().enumerate().find(|(_, f)| {
            f.name == *canonical
                && matches!(
                    f.field_type,
                    Some(FieldType::ElementId) | Some(FieldType::ElementIdRef { .. })
                )
        }) {
            return Some(idx);
        }
    }

    // Priority 3: first bare ElementId (not ElementIdRef). A bare
    // ElementId is the only type that could hold the instance's own
    // id — ElementIdRef embeds a *referenced* tag, which by
    // construction cannot match the owning class's own tag.
    for (idx, f) in cls.fields.iter().enumerate() {
        if matches!(f.field_type, Some(FieldType::ElementId)) {
            return Some(idx);
        }
    }

    None
}

/// Build a [`HandleIndex`] from a decompressed `Global/Latest` buffer
/// by running [`scan_candidates`] + [`find_self_id_field`] and
/// extracting each candidate's self-id.
///
/// Pipeline:
///   1. `scan_candidates(schema, bytes, min_score)` → coarse list,
///      sorted by score descending.
///   2. For each candidate (score-desc):
///      - Look up the `ClassEntry` by name.
///      - Find the self-id field via [`find_self_id_field`]. If the
///        class has no indexable self-id, skip it.
///      - Decode the instance via [`decode_instance`] and read the
///        self-id field's `(tag, id)` pair.
///      - Skip id == 0 (Revit's sentinel for "no id").
///      - Insert `(id → offset)` using "first seen wins" semantics
///        — since candidates are in score-desc order, this keeps
///        the highest-scoring offset for each id when multiple
///        candidates claim the same id.
///
/// Recommended `min_score`:
///   - `80` — matches the ADocument detector's production threshold;
///     filters most false-positive matches on 3-field parent classes.
///   - `0` — debug: surface every candidate that trial-walked
///     successfully, including trivial ones.
///   - `i64::MIN + 1` — exhaustive: no filtering. Useful for
///     comparing scan coverage against `ElemTable`'s declared
///     ElementId set.
///
/// Cost: `O(N × decode)` where N is the number of scan_candidates
/// hits. For a typical 1 MB `Global/Latest` with the default
/// `min_score = 80`, N ≈ 2000–6000 depending on the file's class
/// density.
pub fn build_handle_index(
    schema: &formats::SchemaTable,
    bytes: &[u8],
    min_score: i64,
) -> HandleIndex {
    let mut index = HandleIndex::new();
    let candidates = scan_candidates(schema, bytes, min_score);

    // Fast name → ClassEntry lookup so the per-candidate hot loop
    // doesn't re-scan `schema.classes`.
    let class_by_name: std::collections::HashMap<&str, &formats::ClassEntry> = schema
        .classes
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();

    for cand in &candidates {
        let Some(cls) = class_by_name.get(cand.class_name.as_str()).copied() else {
            continue;
        };
        let Some(id_idx) = find_self_id_field(cls) else {
            continue;
        };

        // Decode the whole instance — it's cheap (schema fields are
        // short and read_field_by_type has no allocation beyond the
        // field values). We discard everything but the self-id.
        let decoded = decode_instance(bytes, cand.offset, cls);

        if let Some((_, InstanceField::ElementId { id, .. })) = decoded.fields.get(id_idx) {
            // Zero is Revit's "no-id" sentinel — never index it.
            // Scores above min_score but with id=0 typically come
            // from offsets where a class-tag byte pair coincides
            // with an all-zero padding region. Skipping them
            // cleans up the index without losing real elements.
            if *id != 0 {
                // First-seen-wins: because `candidates` is in score-
                // desc order, this preserves the highest-score
                // offset for each id. BTreeMap::entry + or_insert
                // gives that semantics atomically.
                index.map.entry(*id).or_insert(cand.offset);
            }
        }
    }

    index
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

/// Trial-decode every declared field of `cls` starting at byte 0 of
/// `bytes`. Returns `Some(walk)` if every field decoded without
/// running off the buffer end; `None` otherwise.
///
/// The returned `walk` pairs are `(tag, id)` per field — only the
/// Pointer / ElementId / ElementIdRef / Container{0x0e} cases
/// populate meaningful values (used by `walk_score`). All other
/// field types push a `(u32::MAX, u32::MAX)` sentinel that the
/// scorer filters out.
///
/// Generalised 2026-04-21 from the ADocument-only version — now
/// handles Primitive / String / Guid / Vector / Container{non-0x0e}
/// via `read_field_by_type`, so the function can be driven against
/// any class in the schema (not just ADocument). That's the pre-req
/// for the `scan_candidates` + walker → IFC pipeline.
pub fn trial_walk(cls: &formats::ClassEntry, bytes: &[u8]) -> Option<Vec<(u32, u32)>> {
    let mut cursor = 0;
    let mut out = Vec::new();
    let limits = WalkerLimits::default();
    for field in &cls.fields {
        let ft = field.field_type.as_ref()?;
        let tag_id = match ft {
            // Pointer: 8 bytes, no score contribution
            formats::FieldType::Pointer { .. } => {
                if cursor + 8 > bytes.len() {
                    return None;
                }
                cursor += 8;
                (u32::MAX, u32::MAX)
            }
            // ElementId / ElementIdRef: 8 bytes, captures (tag, id)
            // which walk_score uses to judge plausibility of the
            // candidate offset.
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
                cursor += 8;
                (tag, id)
            }
            // Container kind=0x0e: 2-column reference table. Size
            // depends on a count prefix. Validates count symmetry
            // across the two columns — a corrupt or mis-aligned
            // candidate offset rarely survives this check.
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
                cursor += 2 * col_bytes;
                (u32::MAX, u32::MAX)
            }
            // All other field types — Primitive, String, Guid,
            // Vector, Bool, non-0x0e Container — delegate to the
            // general field reader. It advances the cursor and
            // returns a fallback `Bytes` on short input; we detect
            // that by checking whether the cursor overran the
            // buffer (in which case the candidate isn't viable).
            other => {
                let before = cursor;
                let _value = read_field_by_type(bytes, &mut cursor, other);
                if cursor > bytes.len() {
                    return None;
                }
                // `read_field_by_type` can legitimately emit a
                // zero-advance on zero-size primitives — guard
                // against infinite loops on degenerate schema
                // entries by requiring forward progress per field
                // (exception: fields with an explicit zero size).
                if cursor == before {
                    match other {
                        formats::FieldType::Primitive { size: 0, .. } => {
                            // Legitimately zero-width — keep going.
                        }
                        _ => return None,
                    }
                }
                (u32::MAX, u32::MAX)
            }
        };
        out.push(tag_id);
    }
    // Touch `limits` so future callers can plumb caller-supplied
    // WalkerLimits through if they want to constrain container
    // sizes or string lengths from a candidate-scanning harness.
    let _ = limits;
    Some(out)
}

/// Heuristic score for a `trial_walk` result.
///
/// Uses the last three `(tag, id)` tuples emitted by the walk (which
/// correspond to the last three ElementId/ElementIdRef fields in the
/// class — ADocument has three consecutive ones at the end of its
/// record, and most element classes have a similar trailing-id
/// pattern) to judge how plausible the candidate offset is.
///
/// Scoring rubric:
/// * Sequential small ids (< 10,000) in the trailing slots: strong +
/// * Two- or three-id clustered range: strong +
/// * Tags that are zero (a common sentinel for "no tag"): mild +
/// * Ids outside u16 range: strong −
/// * Fewer than three real ids or all-zero ids: `i64::MIN`
///
/// Callers compare against thresholds (≥90 confident, 80-89 scored,
/// <80 NotFound) — see `DetectionResult::strategy`.
pub fn walk_score(walk: &[(u32, u32)]) -> i64 {
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
    fn scan_candidates_returns_empty_on_empty_schema() {
        let schema = formats::SchemaTable {
            classes: Vec::new(),
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        let out = scan_candidates(&schema, &[0u8; 100], 0);
        assert!(out.is_empty());
    }

    #[test]
    fn scan_candidates_handles_buffer_shorter_than_4_bytes() {
        let schema = formats::SchemaTable {
            classes: Vec::new(),
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        let out = scan_candidates(&schema, &[], i64::MIN);
        assert!(out.is_empty());
        let out = scan_candidates(&schema, &[0, 1], i64::MIN);
        assert!(out.is_empty());
    }

    #[test]
    fn scan_candidates_sorts_by_score_descending() {
        // Minimal synthesis: two classes both tagged — the scanner
        // should return matches for whichever class scores higher.
        // We don't assert specific scores, just that ordering holds.
        use formats::{ClassEntry, FieldEntry, FieldType};
        let simple_cls = ClassEntry {
            name: "Simple".into(),
            tag: Some(0xABCD),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(3),
            was_parent_only: false,
            fields: vec![
                FieldEntry {
                    name: "m_a".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_b".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_c".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
            ],
        };
        let schema = formats::SchemaTable {
            classes: vec![simple_cls],
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        // Build a buffer with the tag at offset 0, then 24 bytes
        // of synthetic (tag, id) pairs: (0, 1), (0, 2), (0, 3)
        // — all u32::MAX tag sentinels fall through to "real"
        // path, sequential small ids trigger the +25 cluster
        // bonus.
        let mut buf = vec![0u8; 64];
        buf[0] = 0xCD;
        buf[1] = 0xAB; // tag = 0xABCD
        buf[2..6].copy_from_slice(&0u32.to_le_bytes());
        buf[6..10].copy_from_slice(&1u32.to_le_bytes());
        buf[10..14].copy_from_slice(&0u32.to_le_bytes());
        buf[14..18].copy_from_slice(&2u32.to_le_bytes());
        buf[18..22].copy_from_slice(&0u32.to_le_bytes());
        buf[22..26].copy_from_slice(&3u32.to_le_bytes());
        let out = scan_candidates(&schema, &buf, i64::MIN);
        assert!(
            !out.is_empty(),
            "expected at least one candidate for tagged simple class"
        );
        // Within a run the scores should be non-increasing.
        for pair in out.windows(2) {
            assert!(pair[0].score >= pair[1].score);
        }
    }

    #[test]
    fn build_handle_index_empty_schema_returns_empty_index() {
        let schema = formats::SchemaTable {
            classes: Vec::new(),
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        let idx = build_handle_index(&schema, &[0u8; 100], 0);
        assert!(idx.is_empty());
    }

    #[test]
    fn build_handle_index_extracts_self_id_and_skips_zero() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        // 3 ElementId fields — walk_score requires at least 3 trailing
        // ElementIds to score above i64::MIN. m_id carries the self-id;
        // m_owner/m_other hold related references.
        let cls = ClassEntry {
            name: "Indexable".into(),
            tag: Some(0xBEEF),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(3),
            was_parent_only: false,
            fields: vec![
                FieldEntry {
                    name: "m_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_owner".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_other".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
            ],
        };
        let schema = formats::SchemaTable {
            classes: vec![cls],
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        // Instance 1 at offset 0 (tag=0xBEEF):
        //   - m_id:    (tag=0, id=42)  — self-id, becomes the key
        //   - m_owner: (tag=0, id=1)   — trailing ids for walk_score
        //   - m_other: (tag=0, id=2)
        // Instance 2 at offset 34: same tag, m_id.id=0 — skipped.
        let mut buf = vec![0u8; 128];
        buf[0] = 0xEF;
        buf[1] = 0xBE;
        // Instance 1 fields start at offset 2 (after 2-byte tag).
        buf[2..6].copy_from_slice(&0u32.to_le_bytes()); // m_id tag
        buf[6..10].copy_from_slice(&42u32.to_le_bytes()); // m_id id
        buf[10..14].copy_from_slice(&0u32.to_le_bytes()); // m_owner tag
        buf[14..18].copy_from_slice(&1u32.to_le_bytes()); // m_owner id
        buf[18..22].copy_from_slice(&0u32.to_le_bytes()); // m_other tag
        buf[22..26].copy_from_slice(&2u32.to_le_bytes()); // m_other id
        // Instance 2 at offset 34 — m_id is all zeros (skipped).
        buf[34] = 0xEF;
        buf[35] = 0xBE;
        // Populate m_owner/m_other for instance 2 so walk_score is
        // valid (>= i64::MIN). m_id stays zero — the point of the test.
        buf[36..40].copy_from_slice(&0u32.to_le_bytes()); // m_id tag (=0)
        buf[40..44].copy_from_slice(&0u32.to_le_bytes()); // m_id id (=0 → skipped)
        buf[44..48].copy_from_slice(&0u32.to_le_bytes());
        buf[48..52].copy_from_slice(&11u32.to_le_bytes());
        buf[52..56].copy_from_slice(&0u32.to_le_bytes());
        buf[56..60].copy_from_slice(&12u32.to_le_bytes());
        let idx = build_handle_index(&schema, &buf, i64::MIN + 1);
        assert_eq!(
            idx.get(42),
            Some(2),
            "instance 1 should map id 42 → offset 2 (after class tag)"
        );
        assert_eq!(
            idx.get(0),
            None,
            "id=0 is the no-id sentinel, must not be indexed"
        );
    }

    #[test]
    fn build_handle_index_skips_classes_without_self_id_field() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        // Class with no ElementId field — nothing to index.
        let cls = ClassEntry {
            name: "Parent".into(),
            tag: Some(0xABAB),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(1),
            was_parent_only: true,
            fields: vec![FieldEntry {
                name: "m_name".into(),
                cpp_type: None,
                field_type: Some(FieldType::String),
            }],
        };
        let schema = formats::SchemaTable {
            classes: vec![cls],
            cpp_types: Vec::new(),
            skipped_records: 0,
        };
        // Buffer with the class tag but no id — should insert nothing.
        let mut buf = vec![0u8; 64];
        buf[0] = 0xAB;
        buf[1] = 0xAB;
        let idx = build_handle_index(&schema, &buf, i64::MIN + 1);
        assert!(idx.is_empty());
    }

    #[test]
    fn find_self_id_field_prefers_m_id() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        let cls = ClassEntry {
            name: "Wall".into(),
            tag: Some(0x0080),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(3),
            was_parent_only: false,
            fields: vec![
                FieldEntry {
                    name: "m_ownerId".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_id".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
                FieldEntry {
                    name: "m_extraId".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
            ],
        };
        assert_eq!(find_self_id_field(&cls), Some(1));
    }

    #[test]
    fn find_self_id_field_falls_back_to_first_bare_elementid() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        let cls = ClassEntry {
            name: "Weird".into(),
            tag: Some(0x0100),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(3),
            was_parent_only: false,
            fields: vec![
                // ElementIdRef should be skipped — it points elsewhere.
                FieldEntry {
                    name: "m_ownerFamilyId".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementIdRef {
                        referenced_tag: 0x0080,
                        sub: 0,
                    }),
                },
                // First bare ElementId, no canonical name — wins.
                FieldEntry {
                    name: "m_anonId".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementId),
                },
            ],
        };
        assert_eq!(find_self_id_field(&cls), Some(1));
    }

    #[test]
    fn find_self_id_field_returns_none_when_no_elementid() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        let cls = ClassEntry {
            name: "Empty".into(),
            tag: None,
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(1),
            was_parent_only: true,
            fields: vec![FieldEntry {
                name: "m_name".into(),
                cpp_type: None,
                field_type: Some(FieldType::String),
            }],
        };
        assert_eq!(find_self_id_field(&cls), None);
    }

    #[test]
    fn find_self_id_field_rejects_elementidref_only_matches() {
        use formats::{ClassEntry, FieldEntry, FieldType};
        // A class that has ONLY ElementIdRef fields — every id is a
        // pointer to somebody else. Cannot be indexed.
        let cls = ClassEntry {
            name: "AllRef".into(),
            tag: Some(0x0200),
            parent: None,
            ancestor_tag: None,
            offset: 0,
            declared_field_count: Some(2),
            was_parent_only: false,
            fields: vec![
                FieldEntry {
                    name: "m_a".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementIdRef {
                        referenced_tag: 0x0080,
                        sub: 0,
                    }),
                },
                FieldEntry {
                    name: "m_b".into(),
                    cpp_type: None,
                    field_type: Some(FieldType::ElementIdRef {
                        referenced_tag: 0x0081,
                        sub: 0,
                    }),
                },
            ],
        };
        assert_eq!(find_self_id_field(&cls), None);
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
    fn read_field_container_u32_delegates_to_vector_layout() {
        // L5B-09.4: scalar-base Container kinds share the Vector
        // wire format `[u32 count][count × element]`. Exercise the
        // 0x05 (u32) case: 3 elements, values 10/20/30.
        let ft = formats::FieldType::Container {
            kind: 0x05,
            cpp_signature: None,
            body: Vec::new(),
        };
        let mut bytes = vec![0x03, 0x00, 0x00, 0x00]; // count = 3
        for v in [10u32, 20, 30] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4 + 3 * 4);
        match field {
            InstanceField::Vector(items) => {
                assert_eq!(items.len(), 3);
                for (i, expected) in [10i64, 20, 30].iter().enumerate() {
                    match &items[i] {
                        InstanceField::Integer { value, size, .. } => {
                            assert_eq!(*value, *expected);
                            assert_eq!(*size, 4);
                        }
                        other => panic!("items[{i}]: expected Integer, got {other:?}"),
                    }
                }
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn read_field_container_f64_delegates_to_vector_layout() {
        // Container kind 0x07 (f64) — 2 elements: 3.14, 2.71.
        let ft = formats::FieldType::Container {
            kind: 0x07,
            cpp_signature: None,
            body: Vec::new(),
        };
        let mut bytes = vec![0x02, 0x00, 0x00, 0x00]; // count = 2
        for v in [1.5_f64, 9.75] {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4 + 2 * 8);
        match field {
            InstanceField::Vector(items) => {
                assert_eq!(items.len(), 2);
                match &items[0] {
                    InstanceField::Float { value, size } => {
                        assert!((*value - 1.5).abs() < 1e-12);
                        assert_eq!(*size, 8);
                    }
                    other => panic!("items[0]: expected Float, got {other:?}"),
                }
            }
            other => panic!("expected Vector, got {other:?}"),
        }
    }

    #[test]
    fn container_scalar_round_trips_byte_identical() {
        // L5B-09.5: read-then-write a scalar Container through
        // read_field_by_type + write_field_by_type should produce
        // byte-identical output to the input. Exercise kinds 0x01,
        // 0x05, 0x07, 0x0d.
        let cases: Vec<(u8, Vec<u8>)> = vec![
            // 0x01 bool: count=3, elements 1, 0, 1
            (0x01, vec![3, 0, 0, 0, 1, 0, 1]),
            // 0x05 u32: count=2, elements 0x00000100, 0x0000000A
            (
                0x05,
                vec![2, 0, 0, 0, 0x00, 0x01, 0x00, 0x00, 0x0A, 0, 0, 0],
            ),
            // 0x07 f64: count=1, element = 1.5
            (0x07, {
                let mut v = vec![1, 0, 0, 0];
                v.extend_from_slice(&1.5_f64.to_le_bytes());
                v
            }),
            // 0x0d point: count=1, one 24-byte point (3.0, 4.0, 5.0)
            (0x0d, {
                let mut v = vec![1, 0, 0, 0];
                for f in [3.0_f64, 4.0, 5.0] {
                    v.extend_from_slice(&f.to_le_bytes());
                }
                v
            }),
        ];
        for (kind, wire) in cases {
            let ft = formats::FieldType::Container {
                kind,
                cpp_signature: None,
                body: Vec::new(),
            };
            let mut cursor = 0;
            let decoded = read_field_by_type(&wire, &mut cursor, &ft);
            assert_eq!(
                cursor,
                wire.len(),
                "kind 0x{kind:02x}: reader consumed wrong number of bytes"
            );
            let mut rewritten = Vec::new();
            write_field_by_type(&decoded, &ft, &mut rewritten);
            assert_eq!(
                rewritten, wire,
                "kind 0x{kind:02x}: round-trip produced different bytes"
            );
        }
    }

    #[test]
    fn read_field_container_0x0e_still_falls_back() {
        // Reference-typed Container (kind=0x0e) is NOT delegated to
        // Vector — it has a different 2-column wire layout handled
        // by `read_field` for the ADocument path. Ensure the
        // read_field_by_type fallback still returns 4 raw bytes for
        // external callers.
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        let bytes = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x11, 0x22];
        let mut cursor = 0;
        let field = read_field_by_type(&bytes, &mut cursor, &ft);
        assert_eq!(cursor, 4);
        match field {
            InstanceField::Bytes(b) => assert_eq!(b, vec![0xDE, 0xAD, 0xBE, 0xEF]),
            other => panic!("expected Bytes fallback, got {other:?}"),
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
                (
                    "a".into(),
                    InstanceField::Integer {
                        value: 1,
                        signed: false,
                        size: 4,
                    },
                ),
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

    // ----- L5B-01: ADocumentInstance::elem_table_pointer -----

    fn mk_adoc(fields: Vec<(String, InstanceField)>) -> ADocumentInstance {
        ADocumentInstance {
            entry_offset: 0,
            version: 2026,
            fields,
        }
    }

    #[test]
    fn elem_table_pointer_extracts_canonical_field() {
        let adoc = mk_adoc(vec![(
            "m_elem_table".into(),
            InstanceField::Pointer { raw: [42, 1337] },
        )]);
        assert_eq!(adoc.elem_table_pointer(), Some([42, 1337]));
    }

    #[test]
    fn elem_table_pointer_accepts_snake_case_variant() {
        let adoc = mk_adoc(vec![(
            "m_elemTable".into(),
            InstanceField::Pointer { raw: [7, 8] },
        )]);
        assert_eq!(adoc.elem_table_pointer(), Some([7, 8]));
    }

    #[test]
    fn elem_table_pointer_returns_none_on_null_sentinel() {
        let adoc = mk_adoc(vec![(
            "m_elem_table".into(),
            InstanceField::Pointer { raw: [0, 0] },
        )]);
        assert_eq!(adoc.elem_table_pointer(), None);
    }

    #[test]
    fn elem_table_pointer_returns_none_on_max_sentinel() {
        let adoc = mk_adoc(vec![(
            "m_elem_table".into(),
            InstanceField::Pointer {
                raw: [u32::MAX, u32::MAX],
            },
        )]);
        assert_eq!(adoc.elem_table_pointer(), None);
    }

    #[test]
    fn elem_table_pointer_returns_none_when_field_absent() {
        let adoc = mk_adoc(vec![(
            "m_something_else".into(),
            InstanceField::Pointer { raw: [1, 2] },
        )]);
        assert_eq!(adoc.elem_table_pointer(), None);
    }

    #[test]
    fn elem_table_pointer_returns_none_when_field_is_bytes_fallback() {
        // When the walker couldn't classify the field as a Pointer
        // (schema drift etc.), it lands as Bytes. Don't try to
        // interpret Bytes as a pointer.
        let adoc = mk_adoc(vec![(
            "m_elem_table".into(),
            InstanceField::Bytes(vec![0x2a, 0, 0, 0, 0x39, 0x05, 0, 0]),
        )]);
        assert_eq!(adoc.elem_table_pointer(), None);
    }

    // ---- WRT-03: ElementEncoder / encode_instance round-trip tests ----

    fn mk_schema(class_name: &str, fields: Vec<(&str, formats::FieldType)>) -> formats::ClassEntry {
        formats::ClassEntry {
            name: class_name.to_string(),
            offset: 0,
            fields: fields
                .into_iter()
                .map(|(name, ft)| formats::FieldEntry {
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
    fn write_primitive_round_trips_u32() {
        let ft = formats::FieldType::Primitive {
            kind: 0x05,
            size: 4,
        };
        let value = InstanceField::Integer {
            value: 0xDEADBEEF,
            signed: false,
            size: 4,
        };
        let mut out = Vec::new();
        write_field_by_type(&value, &ft, &mut out);
        assert_eq!(out, vec![0xEF, 0xBE, 0xAD, 0xDE]);

        let mut cursor = 0;
        let round = read_field_by_type(&out, &mut cursor, &ft);
        match round {
            InstanceField::Integer { value: v, .. } => assert_eq!(v as u32, 0xDEADBEEF),
            other => panic!("expected Integer, got {:?}", other),
        }
    }

    #[test]
    fn write_primitive_round_trips_f64() {
        let ft = formats::FieldType::Primitive {
            kind: 0x07,
            size: 8,
        };
        let value = InstanceField::Float {
            value: std::f64::consts::PI,
            size: 8,
        };
        let mut out = Vec::new();
        write_field_by_type(&value, &ft, &mut out);
        let mut cursor = 0;
        match read_field_by_type(&out, &mut cursor, &ft) {
            InstanceField::Float { value, .. } => {
                assert!((value - std::f64::consts::PI).abs() < 1e-15);
            }
            other => panic!("expected Float, got {:?}", other),
        }
    }

    #[test]
    fn write_primitive_round_trips_bool() {
        let ft = formats::FieldType::Primitive {
            kind: 0x01,
            size: 1,
        };
        for expected in [true, false] {
            let mut out = Vec::new();
            write_field_by_type(&InstanceField::Bool(expected), &ft, &mut out);
            let mut cursor = 0;
            match read_field_by_type(&out, &mut cursor, &ft) {
                InstanceField::Bool(b) => assert_eq!(b, expected),
                other => panic!("expected Bool, got {:?}", other),
            }
        }
    }

    #[test]
    fn write_string_round_trips_unicode() {
        let ft = formats::FieldType::String;
        for s in ["hello", "café", "日本語", ""] {
            let mut out = Vec::new();
            write_field_by_type(&InstanceField::String(s.to_string()), &ft, &mut out);
            let mut cursor = 0;
            match read_field_by_type(&out, &mut cursor, &ft) {
                InstanceField::String(got) => assert_eq!(got, s),
                other => panic!("expected String, got {:?}", other),
            }
        }
    }

    #[test]
    fn write_guid_round_trips() {
        let ft = formats::FieldType::Guid;
        let g = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0xfe, 0xdc, 0xba, 0x98, 0x76, 0x54,
            0x32, 0x10,
        ];
        let mut out = Vec::new();
        write_field_by_type(&InstanceField::Guid(g), &ft, &mut out);
        assert_eq!(out.len(), 16);
        let mut cursor = 0;
        match read_field_by_type(&out, &mut cursor, &ft) {
            InstanceField::Guid(got) => assert_eq!(got, g),
            other => panic!("expected Guid, got {:?}", other),
        }
    }

    #[test]
    fn write_element_id_round_trips() {
        let ft = formats::FieldType::ElementId;
        let value = InstanceField::ElementId {
            tag: 0x14,
            id: 0x1337,
        };
        let mut out = Vec::new();
        write_field_by_type(&value, &ft, &mut out);
        let mut cursor = 0;
        match read_field_by_type(&out, &mut cursor, &ft) {
            InstanceField::ElementId { tag, id } => {
                assert_eq!(tag, 0x14);
                assert_eq!(id, 0x1337);
            }
            other => panic!("expected ElementId, got {:?}", other),
        }
    }

    #[test]
    fn write_pointer_round_trips() {
        let ft = formats::FieldType::Pointer { kind: 0x01 };
        let value = InstanceField::Pointer {
            raw: [0xAABBCCDD, 0x11223344],
        };
        let mut out = Vec::new();
        write_field_by_type(&value, &ft, &mut out);
        let mut cursor = 0;
        match read_field_by_type(&out, &mut cursor, &ft) {
            InstanceField::Pointer { raw } => assert_eq!(raw, [0xAABBCCDD, 0x11223344]),
            other => panic!("expected Pointer, got {:?}", other),
        }
    }

    #[test]
    fn write_vector_of_doubles_round_trips() {
        let ft = formats::FieldType::Vector {
            kind: 0x07,
            body: Vec::new(),
        };
        let items: Vec<InstanceField> = [1.0, 2.5, -7.25]
            .iter()
            .map(|v| InstanceField::Float { value: *v, size: 8 })
            .collect();
        let mut out = Vec::new();
        write_field_by_type(&InstanceField::Vector(items), &ft, &mut out);
        let mut cursor = 0;
        match read_field_by_type(&out, &mut cursor, &ft) {
            InstanceField::Vector(values) => {
                assert_eq!(values.len(), 3);
                if let InstanceField::Float { value, .. } = values[2] {
                    assert!((value + 7.25).abs() < 1e-9);
                }
            }
            other => panic!("expected Vector, got {:?}", other),
        }
    }

    #[test]
    fn write_bytes_fallback_passes_through_verbatim() {
        // Any FieldType + Bytes value = emit bytes as-is.
        let ft = formats::FieldType::Unknown { bytes: Vec::new() };
        let payload = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let mut out = Vec::new();
        write_field_by_type(&InstanceField::Bytes(payload.clone()), &ft, &mut out);
        assert_eq!(out, payload);
    }

    #[test]
    fn encode_instance_round_trips_typed_fields() {
        // Two fields — u32 + f64 — should produce identical bytes to
        // the reader's input when fed back through decode_instance.
        let schema = mk_schema(
            "TestClass",
            vec![
                (
                    "m_id",
                    formats::FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    },
                ),
                (
                    "m_value",
                    formats::FieldType::Primitive {
                        kind: 0x07,
                        size: 8,
                    },
                ),
            ],
        );
        let decoded = DecodedElement {
            id: None,
            class: "TestClass".into(),
            fields: vec![
                (
                    "m_id".into(),
                    InstanceField::Integer {
                        value: 0x2a,
                        signed: false,
                        size: 4,
                    },
                ),
                (
                    "m_value".into(),
                    InstanceField::Float {
                        value: 9.5,
                        size: 8,
                    },
                ),
            ],
            byte_range: 0..12,
        };
        let bytes = encode_instance(&decoded, &schema);
        assert_eq!(bytes.len(), 12);
        let re = decode_instance(&bytes, 0, &schema);
        assert_eq!(re.fields.len(), 2);
    }

    #[test]
    fn encode_instance_tolerates_field_count_mismatch() {
        let schema = mk_schema(
            "TestClass",
            vec![
                (
                    "a",
                    formats::FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    },
                ),
                (
                    "b",
                    formats::FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    },
                ),
            ],
        );
        // Decoded has only one field; second is simply skipped.
        let decoded = DecodedElement {
            id: None,
            class: "TestClass".into(),
            fields: vec![(
                "a".into(),
                InstanceField::Integer {
                    value: 0x2a,
                    signed: false,
                    size: 4,
                },
            )],
            byte_range: 0..4,
        };
        let bytes = encode_instance(&decoded, &schema);
        assert_eq!(bytes.len(), 4);
    }

    // ---- WRT-09: Container 2-column framing round-trip ----

    #[test]
    fn encode_ref_container_round_trips_balanced_columns() {
        let col_a = vec![1u16, 2, 3, 4, 5];
        let col_b = vec![10u16, 20, 30, 40, 50];
        let bytes = encode_ref_container(&col_a, &col_b);
        // Length: 2 × (4 + 5×6) = 2 × 34 = 68 bytes.
        assert_eq!(bytes.len(), 68);
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        let (consumed, field) = read_field(&ft, &bytes, WalkerLimits::default()).expect("read ok");
        assert_eq!(consumed, 68);
        match field {
            InstanceField::RefContainer { col_a: a, col_b: b } => {
                assert_eq!(a, col_a);
                assert_eq!(b, col_b);
            }
            other => panic!("expected RefContainer, got {:?}", other),
        }
    }

    #[test]
    fn encode_ref_container_empty_is_two_zero_length_prefixes() {
        let bytes = encode_ref_container(&[], &[]);
        assert_eq!(bytes.len(), 8);
        assert_eq!(&bytes[..4], &[0u8; 4]);
        assert_eq!(&bytes[4..], &[0u8; 4]);
    }

    #[test]
    fn encode_ref_container_single_row_round_trips() {
        let bytes = encode_ref_container(&[42], &[99]);
        // 2 × (4 + 6) = 20 bytes.
        assert_eq!(bytes.len(), 20);
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        let (_, field) = read_field(&ft, &bytes, WalkerLimits::default()).unwrap();
        match field {
            InstanceField::RefContainer { col_a, col_b } => {
                assert_eq!(col_a, vec![42]);
                assert_eq!(col_b, vec![99]);
            }
            other => panic!("expected RefContainer, got {:?}", other),
        }
    }

    // ---- WRT-02: ADocument writer round-trip ----

    #[test]
    fn write_adocument_field_pointer_round_trips() {
        let ft = formats::FieldType::Pointer { kind: 0x01 };
        let value = InstanceField::Pointer {
            raw: [0xDEADBEEF, 0x1337],
        };
        let mut out = Vec::new();
        write_adocument_field(&value, &ft, &mut out);
        assert_eq!(out.len(), 8);
        let (consumed, decoded) = read_field(&ft, &out, WalkerLimits::default()).expect("read ok");
        assert_eq!(consumed, 8);
        match decoded {
            InstanceField::Pointer { raw } => {
                assert_eq!(raw, [0xDEADBEEF, 0x1337]);
            }
            other => panic!("expected Pointer, got {:?}", other),
        }
    }

    #[test]
    fn write_adocument_field_element_id_round_trips() {
        let ft = formats::FieldType::ElementId;
        let value = InstanceField::ElementId {
            tag: 0x14,
            id: 0x42,
        };
        let mut out = Vec::new();
        write_adocument_field(&value, &ft, &mut out);
        let (_, decoded) = read_field(&ft, &out, WalkerLimits::default()).unwrap();
        match decoded {
            InstanceField::ElementId { tag, id } => {
                assert_eq!(tag, 0x14);
                assert_eq!(id, 0x42);
            }
            other => panic!("expected ElementId, got {:?}", other),
        }
    }

    #[test]
    fn write_adocument_field_container_round_trips_two_columns() {
        let ft = formats::FieldType::Container {
            kind: 0x0e,
            cpp_signature: None,
            body: Vec::new(),
        };
        let value = InstanceField::RefContainer {
            col_a: vec![0x10, 0x20, 0x30],
            col_b: vec![0x01, 0x02, 0x03],
        };
        let mut out = Vec::new();
        write_adocument_field(&value, &ft, &mut out);
        let (_, decoded) = read_field(&ft, &out, WalkerLimits::default()).unwrap();
        match decoded {
            InstanceField::RefContainer { col_a, col_b } => {
                assert_eq!(col_a, vec![0x10, 0x20, 0x30]);
                assert_eq!(col_b, vec![0x01, 0x02, 0x03]);
            }
            other => panic!("expected RefContainer, got {:?}", other),
        }
    }

    #[test]
    fn write_adocument_field_bytes_passes_through_verbatim() {
        let ft = formats::FieldType::Unknown { bytes: Vec::new() };
        let value = InstanceField::Bytes(vec![0xAA, 0xBB, 0xCC, 0xDD]);
        let mut out = Vec::new();
        write_adocument_field(&value, &ft, &mut out);
        assert_eq!(out, vec![0xAA, 0xBB, 0xCC, 0xDD]);
    }

    #[test]
    fn encode_adocument_fields_walks_schema_order() {
        // Schema with: pointer, container, primitive u32.
        let schema = formats::ClassEntry {
            name: "ADocument".into(),
            offset: 0,
            fields: vec![
                formats::FieldEntry {
                    name: "m_ptr".into(),
                    cpp_type: None,
                    field_type: Some(formats::FieldType::Pointer { kind: 1 }),
                },
                formats::FieldEntry {
                    name: "m_elem_table".into(),
                    cpp_type: None,
                    field_type: Some(formats::FieldType::Container {
                        kind: 0x0e,
                        cpp_signature: None,
                        body: Vec::new(),
                    }),
                },
                formats::FieldEntry {
                    name: "m_version".into(),
                    cpp_type: None,
                    field_type: Some(formats::FieldType::Primitive {
                        kind: 0x05,
                        size: 4,
                    }),
                },
            ],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        let fields = vec![
            ("m_ptr".into(), InstanceField::Pointer { raw: [0x11, 0x22] }),
            (
                "m_elem_table".into(),
                InstanceField::RefContainer {
                    col_a: vec![0x1, 0x2],
                    col_b: vec![0x3, 0x4],
                },
            ),
            (
                "m_version".into(),
                InstanceField::Integer {
                    value: 2024,
                    signed: false,
                    size: 4,
                },
            ),
        ];
        let bytes = encode_adocument_fields(&schema, &fields);
        // 8 (pointer) + 32 (2-col container, 2×(4+2×6)) + 4 (u32) = 44 bytes.
        assert_eq!(bytes.len(), 44);

        // Round-trip: read each field back through the ADocument path.
        let mut cursor = 0;
        let (n1, v1) = read_field(
            schema.fields[0].field_type.as_ref().unwrap(),
            &bytes[cursor..],
            WalkerLimits::default(),
        )
        .unwrap();
        cursor += n1;
        let (n2, v2) = read_field(
            schema.fields[1].field_type.as_ref().unwrap(),
            &bytes[cursor..],
            WalkerLimits::default(),
        )
        .unwrap();
        cursor += n2;
        // Primitive u32 uses read_field_by_type, not read_field.
        let mut pcur = 0usize;
        let v3 = read_field_by_type(
            &bytes[cursor..],
            &mut pcur,
            schema.fields[2].field_type.as_ref().unwrap(),
        );

        assert!(matches!(v1, InstanceField::Pointer { raw } if raw == [0x11, 0x22]));
        assert!(matches!(v2, InstanceField::RefContainer { col_a, col_b }
                if col_a == vec![0x1, 0x2] && col_b == vec![0x3, 0x4]));
        assert!(matches!(v3, InstanceField::Integer { value, .. } if value == 2024));
    }

    #[test]
    fn encode_adocument_fields_tolerates_missing_decoded_field() {
        // Schema declares 2 fields, decoded only has 1 — writer stops
        // at the shorter one (consistent with encode_instance).
        let schema = formats::ClassEntry {
            name: "ADocument".into(),
            offset: 0,
            fields: vec![
                formats::FieldEntry {
                    name: "a".into(),
                    cpp_type: None,
                    field_type: Some(formats::FieldType::Pointer { kind: 1 }),
                },
                formats::FieldEntry {
                    name: "b".into(),
                    cpp_type: None,
                    field_type: Some(formats::FieldType::ElementId),
                },
            ],
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: false,
            ancestor_tag: None,
        };
        let fields = vec![("a".into(), InstanceField::Pointer { raw: [1, 2] })];
        let bytes = encode_adocument_fields(&schema, &fields);
        assert_eq!(bytes.len(), 8); // just the pointer
    }

    #[test]
    fn encode_ref_container_pads_each_record_to_six_bytes() {
        let bytes = encode_ref_container(&[0x1234], &[0xabcd]);
        // Expected bytes:
        //   [01 00 00 00]               count_a = 1
        //   [34 12 00 00 00 00]         id_a + 4-byte padding
        //   [01 00 00 00]               count_b = 1
        //   [cd ab 00 00 00 00]         id_b + 4-byte padding
        assert_eq!(bytes.len(), 20);
        assert_eq!(&bytes[0..4], &[0x01, 0x00, 0x00, 0x00]);
        assert_eq!(&bytes[4..10], &[0x34, 0x12, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(&bytes[10..14], &[0x01, 0x00, 0x00, 0x00]);
        assert_eq!(&bytes[14..20], &[0xcd, 0xab, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn default_element_encoder_uses_encode_instance() {
        struct MyEncoder;
        impl ElementEncoder for MyEncoder {
            fn class_name(&self) -> &'static str {
                "TestClass"
            }
        }
        let schema = mk_schema(
            "TestClass",
            vec![(
                "a",
                formats::FieldType::Primitive {
                    kind: 0x05,
                    size: 4,
                },
            )],
        );
        let decoded = DecodedElement {
            id: None,
            class: "TestClass".into(),
            fields: vec![(
                "a".into(),
                InstanceField::Integer {
                    value: 0x2a,
                    signed: false,
                    size: 4,
                },
            )],
            byte_range: 0..4,
        };
        let enc = MyEncoder;
        let bytes = enc.encode(&decoded, &schema);
        assert_eq!(bytes, vec![0x2a, 0x00, 0x00, 0x00]);
    }
}
