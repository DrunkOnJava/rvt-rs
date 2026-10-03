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
//! | Other `FieldType` variants | not observed in current ADocument fixtures; retained as raw bytes until a validated wire shape is available |

use crate::control::{PROGRESS_BYTE_INTERVAL, Stage, WalkerControl};
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

/// Why a [`DecodedElement`] exists and how strongly we trust it (M3-07).
///
/// Surfaced on CLI / Python / viewer JSON so callers can explain an
/// element and hide low-confidence rows by default.
#[derive(Debug, Clone, PartialEq)]
pub struct ElementProvenance {
    /// Stream path inside the OLE container (e.g. `Global/Latest`,
    /// `partition:/BasicFileInfo`-adjacent partition name).
    pub source_stream: Option<std::string::String>,
    /// Absolute byte offset of the record in that stream (often
    /// matches [`DecodedElement::byte_range`].start).
    pub source_offset: Option<usize>,
    /// Record family: `schema_instance`, `partition_arcwall`,
    /// `partition_schema_mvp`, etc.
    pub record_kind: Option<std::string::String>,
    /// Decoder identity (`LevelDecoder`, `generic_decode_instance`,
    /// `ArcWallRecord`, `partition_schema_mvp::floor_plan_loop`, …).
    pub decoder: Option<std::string::String>,
    /// Trust score in `[0.0, 1.0]`. Unspecified constructors default
    /// to `1.0` so legacy test fixtures stay visible until they set
    /// an honest score.
    pub confidence: f32,
    /// Schema / MVP fields that were required but absent.
    pub missing_required_fields: Vec<std::string::String>,
    /// Non-fatal decode notes.
    pub warnings: Vec<std::string::String>,
}

impl Default for ElementProvenance {
    fn default() -> Self {
        Self {
            source_stream: None,
            source_offset: None,
            record_kind: None,
            decoder: None,
            confidence: 1.0,
            missing_required_fields: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

impl ElementProvenance {
    /// Schema-directed generic instance walk (no typed MVP validation).
    pub fn schema_generic(stream: &str, offset: usize, class: &str) -> Self {
        Self {
            source_stream: Some(stream.into()),
            source_offset: Some(offset),
            record_kind: Some("schema_instance".into()),
            decoder: Some(format!("generic_decode_instance:{class}")),
            confidence: 0.55,
            missing_required_fields: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Registered typed MVP decoder success.
    pub fn schema_typed(stream: &str, offset: usize, decoder_name: &str) -> Self {
        Self {
            source_stream: Some(stream.into()),
            source_offset: Some(offset),
            record_kind: Some("schema_instance".into()),
            decoder: Some(decoder_name.into()),
            confidence: 0.9,
            missing_required_fields: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Partition MVP / ArcWall recoveries with an explicit score.
    pub fn partition(
        stream: &str,
        offset: usize,
        kind: &str,
        decoder: &str,
        confidence: f32,
        warnings: impl IntoIterator<Item = impl Into<std::string::String>>,
    ) -> Self {
        Self {
            source_stream: Some(stream.into()),
            source_offset: Some(offset),
            record_kind: Some(kind.into()),
            decoder: Some(decoder.into()),
            confidence: confidence.clamp(0.0, 1.0),
            missing_required_fields: Vec::new(),
            warnings: warnings.into_iter().map(Into::into).collect(),
        }
    }

    pub fn to_json_value(&self) -> serde_json::Value {
        serde_json::json!({
            "source_stream": self.source_stream,
            "source_offset": self.source_offset,
            "record_kind": self.record_kind,
            "decoder": self.decoder,
            "confidence": self.confidence,
            "missing_required_fields": self.missing_required_fields,
            "warnings": self.warnings,
        })
    }
}

/// Default floor for “hide low-confidence” UI / export filters (M3-07).
pub const DEFAULT_MIN_ELEMENT_CONFIDENCE: f32 = 0.55;

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
    /// Decode confidence and provenance (M3-07).
    pub provenance: ElementProvenance,
}

impl DecodedElement {
    /// Attach provenance, preserving other fields.
    pub fn with_provenance(mut self, provenance: ElementProvenance) -> Self {
        self.provenance = provenance;
        self
    }

    /// True when provenance confidence meets `min_confidence`.
    pub fn meets_confidence(&self, min_confidence: f32) -> bool {
        self.provenance.confidence + f32::EPSILON >= min_confidence
    }
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
    decode_instance_with_limits(bytes, start, class, WalkerLimits::default())
}

pub fn decode_instance_with_limits(
    bytes: &[u8],
    start: usize,
    class: &formats::ClassEntry,
    limits: WalkerLimits,
) -> DecodedElement {
    let end = start
        .checked_add(limits.max_per_record_decode_bytes)
        .map(|n| n.min(bytes.len()))
        .unwrap_or(bytes.len());
    let bytes = bytes.get(..end).unwrap_or(bytes);
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
        provenance: ElementProvenance::schema_generic("schema", start, &class.name),
    }
}

/// Resource cap that stopped or truncated a walker scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WalkerLimitHit {
    MaxScanBytes,
    MaxCandidates,
    MaxTrialOffsets,
    MaxPerRecordDecodeBytes,
}

impl WalkerLimitHit {
    pub fn code(self) -> &'static str {
        match self {
            Self::MaxScanBytes => "walker-max-scan-bytes",
            Self::MaxCandidates => "walker-max-candidates",
            Self::MaxTrialOffsets => "walker-max-trial-offsets",
            Self::MaxPerRecordDecodeBytes => "walker-max-record-decode-bytes",
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            Self::MaxScanBytes => "walker scan stopped at max_scan_bytes",
            Self::MaxCandidates => "walker candidate list stopped at max_candidates",
            Self::MaxTrialOffsets => "walker trial scan stopped at max_trial_offsets",
            Self::MaxPerRecordDecodeBytes => {
                "walker candidate decode stopped at max_per_record_decode_bytes"
            }
        }
    }
}

/// Resource caps applied while the walker scans and decodes instances
/// (API-11). Every size- or count-prefixed wire value that could
/// drive an allocation is compared against the matching cap; values
/// above the cap trigger a graceful fallback or a diagnostic limit
/// hit, not a panic.
///
/// Defaults preserve ordinary corpus behaviour while making scan
/// work finite for adversarial files.
/// Tighten the limits when parsing adversarial / untrusted input:
///
/// ```
/// use rvt::walker::WalkerLimits;
/// let tight = WalkerLimits {
///     max_scan_bytes: 16 * 1024 * 1024,
///     max_container_records: 64,
///     ..WalkerLimits::default()
/// };
/// ```
#[derive(Debug, Clone, Copy)]
pub struct WalkerLimits {
    /// Maximum decompressed `Global/Latest` bytes considered by
    /// brute-force walker scans. Bytes beyond this cap are ignored
    /// and a [`WalkerLimitHit::MaxScanBytes`] diagnostic is surfaced
    /// by diagnostic APIs. Default 128 MiB.
    pub max_scan_bytes: usize,
    /// Maximum candidates materialised by [`scan_candidates_with_limits`].
    /// Default 100,000.
    pub max_candidates: usize,
    /// Maximum trial decodes attempted by schema-directed scans.
    /// For ADocument detection this bounds byte-by-byte fallback
    /// scanning; for element scans this bounds class-tag hits. Default
    /// 16,000,000.
    pub max_trial_offsets: usize,
    /// Maximum bytes a single candidate decode may inspect. Default
    /// 1 MiB, far above observed record sizes and low enough to bound
    /// hostile size-prefix paths.
    pub max_per_record_decode_bytes: usize,
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
            max_scan_bytes: 128 * 1024 * 1024,
            max_candidates: 100_000,
            max_trial_offsets: 16_000_000,
            max_per_record_decode_bytes: 1024 * 1024,
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
    Ok(read_adocument_internal(rf, limits)?.0)
}

fn read_adocument_internal(
    rf: &mut RevitFile,
    limits: WalkerLimits,
) -> Result<(Option<ADocumentInstance>, Option<DetectionResult>)> {
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_stream_at(streams::FORMATS_LATEST, &formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    let adoc = schema
        .classes
        .iter()
        .find(|c| c.name == "ADocument")
        .ok_or_else(|| Error::BasicFileInfo("ADocument not in schema".into()))?;

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, d) = compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw)?;
    let detection = detect_adocument_start_with_limits(&d, Some(adoc), limits);
    let Some(entry) = detection.offset else {
        return Ok((None, Some(detection)));
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
    Ok((
        Some(ADocumentInstance {
            entry_offset: entry,
            version,
            fields,
        }),
        Some(detection),
    ))
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
    read_adocument_lossy_with_limits(rf, WalkerLimits::default())
}

pub fn read_adocument_lossy_with_limits(
    rf: &mut RevitFile,
    limits: WalkerLimits,
) -> Result<crate::parse_mode::Decoded<ADocumentInstance>> {
    use crate::parse_mode::{Decoded, Diagnostics, Warning};

    let (maybe_inst, detection) = read_adocument_internal(rf, limits)?;
    let mut diagnostics = Diagnostics::default();
    if let Some(hit) = detection.and_then(|d| d.limit_hit) {
        diagnostics.warn(Warning::new(hit.code(), hit.message()));
    }

    let Some(inst) = maybe_inst else {
        let mut d = Diagnostics::default();
        d.extend(diagnostics);
        d.fail_stream("ADocument");
        let placeholder = ADocumentInstance {
            entry_offset: 0,
            version: 0,
            fields: Vec::new(),
        };
        return Ok(Decoded::partial(placeholder, d));
    };

    let c = inst.completeness();
    if c.is_fully_typed() && diagnostics.is_empty() {
        return Ok(Decoded::complete(inst));
    }

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
///
/// # Typed MVP + partition merge (M3-05 / M3-06)
///
/// For each `Global/Latest` candidate whose class is in
/// [`crate::elements::MVP_TYPED_CLASSES`], production iteration
/// prefers the registered typed decoder and **skips** the hit when
/// that decoder rejects (fail closed — no fake typed success via
/// generic fallback). Other classes keep generic
/// [`decode_instance`].
///
/// When the file's Revit year is in the ArcWall-supported set,
/// validated partition ArcWall records are merged in as
/// `DecodedElement` values with `class == "ArcWall"` (via
/// [`crate::elements::arc_wall`]). Additional fail-closed partition
/// MVP recovers ([`crate::partition_schema_mvp`]) may emit `Level`,
/// `Material`, `Room`, `Floor` (plan-loop), and — on Revit 2024 —
/// `ArcWallRectOpening` rows. Semantic `Door` / `Window` classes are
/// **not** invented from opening-index bytes.
pub fn iter_elements(rf: &mut RevitFile) -> Result<impl Iterator<Item = DecodedElement>> {
    iter_elements_with_limits(rf, PRODUCTION_ELEMENT_MIN_SCORE, WalkerLimits::default())
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
    iter_elements_with_limits(rf, min_score, WalkerLimits::default())
}

/// Same as [`iter_elements_with_options`], with explicit walker
/// resource caps.
pub fn iter_elements_with_limits(
    rf: &mut RevitFile,
    min_score: i64,
    limits: WalkerLimits,
) -> Result<impl Iterator<Item = DecodedElement>> {
    iter_elements_with_control(rf, min_score, limits, &WalkerControl::default())
}

/// Same as [`iter_elements_with_limits`] with cooperative cancellation and
/// progress reporting through [`WalkerControl`]. Stages reported, in
/// order: [`Stage::SchemaParse`], [`Stage::CandidateScan`] (per 1 MiB of
/// `Global/Latest`), [`Stage::ElementDecode`] (per 256 candidates), and
/// [`Stage::PartitionScan`] (start/end of the partition merges). Cancelling
/// returns [`Error::Cancelled`]; nothing partial is yielded.
pub fn iter_elements_with_control(
    rf: &mut RevitFile,
    min_score: i64,
    limits: WalkerLimits,
    control: &WalkerControl,
) -> Result<impl Iterator<Item = DecodedElement> + use<>> {
    control.check()?;
    control.report(Stage::SchemaParse, 0, None);
    let formats_raw = rf.read_stream(streams::FORMATS_LATEST)?;
    let formats_d = compression::inflate_stream_at(streams::FORMATS_LATEST, &formats_raw, 0)?;
    let schema = formats::parse_schema(&formats_d)?;
    control.report(Stage::SchemaParse, 1, Some(1));

    let raw = rf.read_stream(streams::GLOBAL_LATEST)?;
    let (_, d) = compression::inflate_stream_auto(streams::GLOBAL_LATEST, &raw)?;

    let class_by_name: std::collections::HashMap<&str, &formats::ClassEntry> = schema
        .classes
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();

    let candidates =
        scan_candidates_with_control(&schema, &d, min_score, limits, control)?.candidates;
    let candidate_total = candidates.len() as u64;
    let mut out = Vec::with_capacity(candidates.len());
    let mut seen_ids = std::collections::HashSet::<u32>::new();

    for (index, cand) in candidates.into_iter().enumerate() {
        if index % 256 == 0 {
            control.check()?;
            control.report(Stage::ElementDecode, index as u64, Some(candidate_total));
        }
        let Some(cls) = class_by_name.get(cand.class_name.as_str()).copied() else {
            continue;
        };
        // Typed MVP preference: reject → skip (fail closed). Generic
        // path for non-MVP classes.
        let Some(mut decoded) =
            crate::elements::decode_instance_prefer_typed_with_limits(&d, cand.offset, cls, limits)
        else {
            continue;
        };

        // Extract the self-id without holding a borrow of
        // `decoded` when we later assign `decoded.id`. The
        // find_self_id_field index is stable and cheap to recompute.
        let self_id = find_self_id_field(cls)
            .and_then(|idx| decoded.fields.get(idx))
            .and_then(|(_, field)| match field {
                InstanceField::ElementId { id, .. } if *id != 0 => Some(*id),
                _ => None,
            })
            .or(decoded.id.filter(|id| *id != 0));

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

    control.check()?;
    control.report(Stage::ElementDecode, candidate_total, Some(candidate_total));
    control.report(Stage::PartitionScan, 0, None);

    // Partition ArcWall merge (version-gated). Fail closed: only
    // records that pass the standard envelope decoder are emitted.
    if let Ok(bfi) = rf.basic_file_info() {
        if let Ok(scan) = crate::partition_arc_walls::scan_partition_arc_walls_with_limits(
            rf,
            bfi.version,
            limits,
        ) {
            for wall in scan.walls {
                let typed = crate::elements::arc_wall::from_partition_arc_wall(&wall);
                let decoded = typed.to_decoded_element(&wall.partition, wall.offset);
                if let Some(id) = decoded.id {
                    if !seen_ids.insert(id) {
                        continue;
                    }
                }
                out.push(decoded);
            }
        }

        // Partition schema MVP: Level / Material / Room / Floor plan
        // loops / 2024 ArcWallRectOpening index. Fail closed — no
        // invented Door/Window typed success.
        if let Ok(mvp) =
            crate::partition_schema_mvp::recover_partition_schema_mvp(rf, bfi.version, limits)
        {
            for decoded in mvp.into_elements() {
                if let Some(id) = decoded.id {
                    if !seen_ids.insert(id) {
                        continue;
                    }
                }
                out.push(decoded);
            }
        }
    }

    control.check()?;
    control.report(Stage::PartitionScan, 1, Some(1));
    Ok(out.into_iter())
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
    /// First resource cap reached while scanning, if any.
    pub limit_hit: Option<WalkerLimitHit>,
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
    detect_adocument_start_with_limits(d, adoc_schema, WalkerLimits::default())
}

pub fn detect_adocument_start_with_limits(
    d: &[u8],
    adoc_schema: Option<&formats::ClassEntry>,
    limits: WalkerLimits,
) -> DetectionResult {
    let (scan_bytes, scan_limit_hit) = bounded_scan_bytes(d, limits);
    // Strategy 1: sequential-id-table end + 8-zero signature scan.
    if let Some(h) = heuristic_find(scan_bytes) {
        if let Some(cls) = adoc_schema {
            if let TrialWalkOutcome::Match(w) = trial_walk_checked(cls, &scan_bytes[h..], limits) {
                let sc = walk_score(&w);
                if sc >= 90 {
                    return DetectionResult {
                        offset: Some(h),
                        score: Some(sc),
                        candidates_evaluated: 1,
                        limit_hit: scan_limit_hit,
                        strategy: DetectionStrategy::Heuristic,
                    };
                }
            }
        } else {
            return DetectionResult {
                offset: Some(h),
                score: None,
                candidates_evaluated: 0,
                limit_hit: scan_limit_hit,
                strategy: DetectionStrategy::Heuristic,
            };
        }
    }
    // Strategy 2: score-based brute-force scan.
    if let Some(cls) = adoc_schema {
        let mut best: Option<(i64, usize)> = None;
        let mut evaluated = 0usize;
        let mut limit_hit = scan_limit_hit;
        let end = scan_bytes.len().saturating_sub(256);
        for offset in 0x100..end {
            if evaluated >= limits.max_trial_offsets {
                limit_hit.get_or_insert(WalkerLimitHit::MaxTrialOffsets);
                break;
            }
            evaluated += 1;
            match trial_walk_checked(cls, &scan_bytes[offset..], limits) {
                TrialWalkOutcome::Match(walk) => {
                    let sc = walk_score(&walk);
                    if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                        best = Some((sc, offset));
                    }
                }
                TrialWalkOutcome::NoMatch => {}
                TrialWalkOutcome::Limit(hit) => {
                    limit_hit.get_or_insert(hit);
                }
            }
        }
        if let Some((sc, off)) = best {
            if sc >= 80 {
                return DetectionResult {
                    offset: Some(off),
                    score: Some(sc),
                    candidates_evaluated: evaluated,
                    limit_hit,
                    strategy: DetectionStrategy::Scored,
                };
            }
            // Sub-threshold best — still return the score so
            // callers can see how close it got.
            return DetectionResult {
                offset: None,
                score: Some(sc),
                candidates_evaluated: evaluated,
                limit_hit,
                strategy: DetectionStrategy::NotFound,
            };
        }
        return DetectionResult {
            offset: None,
            score: None,
            candidates_evaluated: evaluated,
            limit_hit,
            strategy: DetectionStrategy::NotFound,
        };
    }
    DetectionResult {
        offset: None,
        score: None,
        candidates_evaluated: 0,
        limit_hit: scan_limit_hit,
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

#[derive(Debug, Clone, Default)]
pub struct ScanCandidatesResult {
    pub candidates: Vec<ScanCandidate>,
    pub scanned_bytes: usize,
    pub trial_offsets_evaluated: usize,
    pub limit_hit: Option<WalkerLimitHit>,
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
    scan_candidates_with_limits(schema, bytes, min_score, WalkerLimits::default()).candidates
}

pub fn scan_candidates_with_limits(
    schema: &formats::SchemaTable,
    bytes: &[u8],
    min_score: i64,
    limits: WalkerLimits,
) -> ScanCandidatesResult {
    scan_candidates_with_control(schema, bytes, min_score, limits, &WalkerControl::default())
        .expect("a scan without a cancellation token cannot be cancelled")
}

/// Same as [`scan_candidates_with_limits`] with cooperative cancellation
/// and progress reporting through [`WalkerControl`]. The token is checked
/// at the start of every [`PROGRESS_BYTE_INTERVAL`] of input and once
/// after the loop, so a token flipped from inside the progress callback
/// is honoured before the result is returned.
pub fn scan_candidates_with_control(
    schema: &formats::SchemaTable,
    bytes: &[u8],
    min_score: i64,
    limits: WalkerLimits,
    control: &WalkerControl,
) -> Result<ScanCandidatesResult> {
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

    let (bytes, scan_limit_hit) = bounded_scan_bytes(bytes, limits);
    let mut out: Vec<ScanCandidate> = Vec::new();
    let mut trial_offsets_evaluated = 0usize;
    let mut limit_hit = scan_limit_hit;
    if bytes.len() < 4 {
        return Ok(ScanCandidatesResult {
            candidates: out,
            scanned_bytes: bytes.len(),
            trial_offsets_evaluated,
            limit_hit,
        });
    }

    // 2-byte-aligned scan — class tags are u16 on the wire and are
    // always aligned to the containing record's start. A 1-byte
    // shift wouldn't give a valid instance.
    let end = bytes.len().saturating_sub(2);
    let mut i = 0usize;
    while i + 2 <= end {
        if i & (PROGRESS_BYTE_INTERVAL - 1) == 0 {
            control.check()?;
            control.report(Stage::CandidateScan, i as u64, Some(bytes.len() as u64));
        }
        let tag = u16::from_le_bytes([bytes[i], bytes[i + 1]]);
        if let Some(classes) = tag_to_class.get(&tag) {
            // Instance data starts AFTER the tag. Give trial_walk
            // the post-tag slice.
            let instance_start = i + 2;
            if instance_start < bytes.len() {
                for cls in classes {
                    if trial_offsets_evaluated >= limits.max_trial_offsets {
                        limit_hit.get_or_insert(WalkerLimitHit::MaxTrialOffsets);
                        break;
                    }
                    trial_offsets_evaluated += 1;
                    match trial_walk_checked(cls, &bytes[instance_start..], limits) {
                        TrialWalkOutcome::Match(walk) => {
                            let score = walk_score(&walk);
                            if score >= min_score {
                                out.push(ScanCandidate {
                                    offset: instance_start,
                                    class_name: cls.name.clone(),
                                    class_tag: tag,
                                    score,
                                });
                                if out.len() >= limits.max_candidates {
                                    limit_hit.get_or_insert(WalkerLimitHit::MaxCandidates);
                                    break;
                                }
                            }
                        }
                        TrialWalkOutcome::NoMatch => {}
                        TrialWalkOutcome::Limit(hit) => {
                            limit_hit.get_or_insert(hit);
                        }
                    }
                }
            }
        }
        if matches!(
            limit_hit,
            Some(WalkerLimitHit::MaxCandidates | WalkerLimitHit::MaxTrialOffsets)
        ) {
            break;
        }
        i += 2;
    }

    // Sort by score descending so the highest-confidence matches
    // surface first. Stable sort keeps within-score order == scan
    // order, which is also byte-ascending — useful for downstream
    // deduplication.
    control.check()?;
    control.report(
        Stage::CandidateScan,
        bytes.len() as u64,
        Some(bytes.len() as u64),
    );
    out.sort_by_key(|c| std::cmp::Reverse(c.score));
    Ok(ScanCandidatesResult {
        candidates: out,
        scanned_bytes: bytes.len(),
        trial_offsets_evaluated,
        limit_hit,
    })
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
    build_handle_index_with_limits(schema, bytes, min_score, WalkerLimits::default())
}

pub fn build_handle_index_with_limits(
    schema: &formats::SchemaTable,
    bytes: &[u8],
    min_score: i64,
    limits: WalkerLimits,
) -> HandleIndex {
    let mut index = HandleIndex::new();
    let candidates = scan_candidates_with_limits(schema, bytes, min_score, limits).candidates;

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
        let decoded = decode_instance_with_limits(bytes, cand.offset, cls, limits);

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
    detect_adocument_start(d, adoc_schema).offset
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
    trial_walk_with_limits(cls, bytes, WalkerLimits::default())
}

pub fn trial_walk_with_limits(
    cls: &formats::ClassEntry,
    bytes: &[u8],
    limits: WalkerLimits,
) -> Option<Vec<(u32, u32)>> {
    match trial_walk_checked(cls, bytes, limits) {
        TrialWalkOutcome::Match(walk) => Some(walk),
        TrialWalkOutcome::NoMatch | TrialWalkOutcome::Limit(_) => None,
    }
}

enum TrialWalkOutcome {
    Match(Vec<(u32, u32)>),
    NoMatch,
    Limit(WalkerLimitHit),
}

fn trial_walk_checked(
    cls: &formats::ClassEntry,
    bytes: &[u8],
    limits: WalkerLimits,
) -> TrialWalkOutcome {
    let source_len = bytes.len();
    let capped = source_len > limits.max_per_record_decode_bytes;
    let bytes = &bytes[..source_len.min(limits.max_per_record_decode_bytes)];
    let mut cursor = 0;
    let mut out = Vec::new();
    for field in &cls.fields {
        let Some(ft) = field.field_type.as_ref() else {
            return TrialWalkOutcome::NoMatch;
        };
        let tag_id = match ft {
            // Pointer: 8 bytes, no score contribution
            formats::FieldType::Pointer { .. } => {
                if cursor + 8 > bytes.len() {
                    return if capped {
                        TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                    } else {
                        TrialWalkOutcome::NoMatch
                    };
                }
                cursor += 8;
                (u32::MAX, u32::MAX)
            }
            // ElementId / ElementIdRef: 8 bytes, captures (tag, id)
            // which walk_score uses to judge plausibility of the
            // candidate offset.
            formats::FieldType::ElementId | formats::FieldType::ElementIdRef { .. } => {
                if cursor + 8 > bytes.len() {
                    return if capped {
                        TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                    } else {
                        TrialWalkOutcome::NoMatch
                    };
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
                    return if capped {
                        TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                    } else {
                        TrialWalkOutcome::NoMatch
                    };
                }
                let count = u32::from_le_bytes([
                    bytes[cursor],
                    bytes[cursor + 1],
                    bytes[cursor + 2],
                    bytes[cursor + 3],
                ]) as usize;
                if count > limits.max_container_records {
                    return TrialWalkOutcome::NoMatch;
                }
                let Some(col_payload_bytes) = count.checked_mul(6) else {
                    return TrialWalkOutcome::NoMatch;
                };
                let Some(col_bytes) = 4usize.checked_add(col_payload_bytes) else {
                    return TrialWalkOutcome::NoMatch;
                };
                let Some(col2_count_end) =
                    cursor.checked_add(col_bytes).and_then(|n| n.checked_add(4))
                else {
                    return TrialWalkOutcome::NoMatch;
                };
                if col2_count_end > bytes.len() {
                    return if capped {
                        TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                    } else {
                        TrialWalkOutcome::NoMatch
                    };
                }
                let col2_count = u32::from_le_bytes([
                    bytes[cursor + col_bytes],
                    bytes[cursor + col_bytes + 1],
                    bytes[cursor + col_bytes + 2],
                    bytes[cursor + col_bytes + 3],
                ]) as usize;
                if col2_count != count {
                    return TrialWalkOutcome::NoMatch;
                }
                let Some(total) = col_bytes.checked_mul(2) else {
                    return TrialWalkOutcome::NoMatch;
                };
                let Some(total_end) = cursor.checked_add(total) else {
                    return TrialWalkOutcome::NoMatch;
                };
                if total_end > bytes.len() {
                    return if capped {
                        TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                    } else {
                        TrialWalkOutcome::NoMatch
                    };
                }
                cursor = total_end;
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
                    return TrialWalkOutcome::NoMatch;
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
                        _ => {
                            return if capped {
                                TrialWalkOutcome::Limit(WalkerLimitHit::MaxPerRecordDecodeBytes)
                            } else {
                                TrialWalkOutcome::NoMatch
                            };
                        }
                    }
                }
                (u32::MAX, u32::MAX)
            }
        };
        out.push(tag_id);
    }
    TrialWalkOutcome::Match(out)
}

fn bounded_scan_bytes(bytes: &[u8], limits: WalkerLimits) -> (&[u8], Option<WalkerLimitHit>) {
    if bytes.len() > limits.max_scan_bytes {
        (
            &bytes[..limits.max_scan_bytes],
            Some(WalkerLimitHit::MaxScanBytes),
        )
    } else {
        (bytes, None)
    }
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
