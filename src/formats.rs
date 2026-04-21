//! Parse `Formats/Latest` into a real schema table.
//!
//! **This is the key file.** The decompressed `Formats/Latest` stream
//! contains Autodesk's complete on-disk serialization schema — every class
//! name, every field name, every C++ type signature (including full
//! `std::pair< ElementId, double >` generics). It is in effect a bundled
//! `.proto` file for the entire Revit object graph.
//!
//! Every Revit release since at least 2016 embeds this schema in the file
//! itself. Class IDs are UUIDv1 values whose MAC suffixes (e.g.
//! `0000863f27ad`, `0000863de970`) are visible in Autodesk Forge JSON
//! outputs — strong evidence the schema identifiers have been stable since
//! Revit was built ca. 2000.
//!
//! # Wire format (inferred from the 11-version RFA corpus)
//!
//! Each class record starts with:
//!
//! ```text
//! [uint16 LE name_len] [name_len bytes ASCII class_name]
//! [uint16 LE type_tag]                     // bit 0x8000 = flag; low byte = secondary length
//! [padding zeros]                          // variable — see field parser
//! ```
//!
//! Followed by a field table. Each field entry:
//!
//! ```text
//! [uint16 LE fieldname_len] [fieldname_len bytes ASCII field_name]
//! [uint16 LE typename_len]  [typename_len bytes ASCII cpp_type]    // optional
//! ```
//!
//! The parser below is best-effort. The regex-fallback mode still works
//! even when the wire layout has a variation we haven't yet documented.

use crate::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTable {
    pub classes: Vec<ClassEntry>,
    /// Every unique C++ type signature seen in the schema (e.g.
    /// `std::pair< ElementId, double >`, `ElementId`, `Identifier`).
    pub cpp_types: Vec<String>,
    /// Raw count of parse-candidates skipped for validation reasons.
    pub skipped_records: usize,
}

/// Derived diagnostic counters for a parsed [`SchemaTable`] (API-10).
///
/// These counters are computed on demand from the parsed table — the
/// `SchemaTable` itself stores only the raw record lists to keep the
/// serialized JSON format stable. Callers who want structured
/// quality metadata (for CLI output, CI drift detection, or
/// cross-release regression checks) use [`SchemaTable::diagnostics`]
/// and inspect the returned `SchemaDiagnostics`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct SchemaDiagnostics {
    /// Total number of classes in the schema.
    pub class_count: usize,
    /// Total number of declared fields across all classes (summed
    /// `fields.len()`). Not the same as `declared_field_count_sum` —
    /// the parser may have missed a field or two; this is the
    /// actually-parsed total.
    pub parsed_field_count: usize,
    /// Sum of `declared_field_count` across classes that set it.
    /// Compare against `parsed_field_count` to spot parser coverage
    /// gaps (expected equal when parsing is 100%-complete).
    pub declared_field_count_sum: u64,
    /// Classes that declared a field count that the parser couldn't
    /// walk all the way through. Non-zero means the parser missed
    /// fields; inspect those class names for what encoding slipped.
    pub field_count_mismatches: usize,
    /// Classes that carry a serialization `tag` (i.e. are top-level
    /// serializable). The complement is mixins / embedded types that
    /// only appear as members of other classes.
    pub tagged_class_count: usize,
    /// Classes that appear only because another class referenced
    /// them as a parent (`was_parent_only = true`). A non-zero count
    /// usually means the full schema for that class lives in another
    /// stream or is implicit.
    pub parent_only_class_count: usize,
    /// Classes that carry a non-zero `ancestor_tag` (Q4 addendum
    /// finding — a mixin / protocol / category reference distinct
    /// from direct `parent`).
    pub ancestor_tag_count: usize,
    /// Parse candidates skipped for validation reasons. Copied from
    /// the underlying `skipped_records` field for convenience.
    pub skipped_records: usize,
    /// Count of unique C++ type signatures. Rough proxy for schema
    /// complexity — stable across releases for a given Revit major.
    pub cpp_type_count: usize,
}

impl SchemaTable {
    /// Compute derived diagnostic counters. O(n) in the number of
    /// classes + fields; cheap enough to call ad-hoc from CLIs.
    pub fn diagnostics(&self) -> SchemaDiagnostics {
        let mut d = SchemaDiagnostics {
            class_count: self.classes.len(),
            skipped_records: self.skipped_records,
            cpp_type_count: self.cpp_types.len(),
            ..SchemaDiagnostics::default()
        };
        for c in &self.classes {
            d.parsed_field_count += c.fields.len();
            if let Some(declared) = c.declared_field_count {
                d.declared_field_count_sum += declared as u64;
                if (declared as usize) != c.fields.len() {
                    d.field_count_mismatches += 1;
                }
            }
            if c.tag.is_some() {
                d.tagged_class_count += 1;
            }
            if c.was_parent_only {
                d.parent_only_class_count += 1;
            }
            if c.ancestor_tag.is_some() {
                d.ancestor_tag_count += 1;
            }
        }
        d
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEntry {
    pub name: String,
    /// Stream offset where this class entry begins.
    pub offset: usize,
    /// Fields declared by this class (best-effort).
    pub fields: Vec<FieldEntry>,
    /// Serialization tag if this class has one set (u16, 0x8000 flag stripped).
    /// Absent = the class is not top-level serializable; it's an embedded type.
    pub tag: Option<u16>,
    /// Parent / superclass name if present. Determined by the `[u16 len][name]`
    /// block that follows the tag. For e.g. HostObjAttr → Some("Symbol").
    pub parent: Option<String>,
    /// Field-count value the schema itself declares (may disagree with
    /// `fields.len()` if the walker missed one).
    pub declared_field_count: Option<u32>,
    /// True when this entry was synthesized from a parent-class
    /// reference inside another class's record rather than from a
    /// dedicated top-level declaration. Such entries carry the name
    /// (and possibly offset where the reference appeared) but no
    /// fields or tag — the full declaration may appear elsewhere in
    /// `Formats/Latest`, or may be implicit.
    #[serde(default)]
    pub was_parent_only: bool,
    /// Class-tag reference found in the preamble's "flag" word slot
    /// (the u16 immediately before the field count). Non-zero values
    /// match known class tags — likely a mixin / protocol / category
    /// ancestor distinct from the direct `parent`. See §Q4 addendum
    /// in `docs/rvt-moat-break-reconnaissance.md`.
    ///
    /// `None` when the slot was 0x0000 (no reference). 55% of tagged
    /// classes in the 2024 sample have no ancestor_tag.
    #[serde(default)]
    pub ancestor_tag: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldEntry {
    pub name: String,
    pub cpp_type: Option<String>,
    /// Best-effort decode of the field's type encoding. See
    /// `FieldType::decode` for the byte-level pattern this maps onto.
    pub field_type: Option<FieldType>,
}

/// Best-effort classification of a field's type encoding (the byte block
/// that follows a field name in `Formats/Latest`). Derived from the
/// 2026-04-19 Phase 4c.2 sweeps (Q5 + Q5.1) — see the §Q5 / §Q5.1
/// addenda in `docs/rvt-moat-break-reconnaissance.md` for evidence.
///
/// The primary discriminator is the first byte of the encoding:
///
/// | Byte | Semantic | Wire size |
/// |---|---|---|
/// | `0x01` | `bool` | 1 (padded) |
/// | `0x02` | `u16` / `i16` | 2 |
/// | `0x04` | `u32` / `i32` (legacy) | 4 |
/// | `0x05` | `u32` / `i32` | 4 |
/// | `0x06` | `f32` | 4 |
/// | `0x07` | `f64` (double) | 8 |
/// | `0x08` | UTF-16LE string, length-prefixed | variable |
/// | `0x09` | `GUID` (UUID) | 16 |
/// | `0x0b` | `u64` / `i64` | 8 |
/// | `0x0e` | reference / pointer / container | variable (see sub-type) |
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    /// A fixed-size numeric or boolean primitive. `size` is the wire
    /// size in bytes; `kind` is the type category byte (0x01, 0x02,
    /// 0x04, 0x05, 0x06, 0x07, 0x0b).
    Primitive { kind: u8, size: u8 },
    /// UTF-16LE string, length-prefixed. `0x08` family. Two equivalent
    /// wire encodings exist: `0x08 0x00 0x60 0x00` (sub=0x6000) and
    /// `0x08 0x60 0x00 0x00` (sub=0x0060). Both decode to `String`.
    String,
    /// 16-byte GUID / UUID. `0x09` family.
    Guid,
    /// `0x0e 0x00 0x00 0x00 0x14 0x00 0x00 0x00` — the canonical ElementId,
    /// referencing the root `ElementId` class (tag 0x0014). This is the
    /// common case; see `ElementIdRef` for references that carry a
    /// specific referenced-class tag.
    ElementId,
    /// `0x0e 0x00 0x00 0x00 <tag:u16> <sub:u16>` where `<tag>` is the
    /// referenced class's 15-bit tag (high bit stripped). Revit encodes
    /// "pointer to element of known class" by embedding the target's
    /// tag here. The `sub` u16 refines the reference kind (0x0000
    /// bare-ref, 0x0009/0x0014/0x0080 observed modifiers).
    ElementIdRef {
        /// The tag of the referenced class, as it appears in the schema.
        referenced_tag: u16,
        /// Secondary discriminator (meaning unknown; empirically varies
        /// per-field and per-version).
        sub: u16,
    },
    /// `0x0e 0xNN 0x00 0x00` where NN ∈ {0x01, 0x02, 0x03} — a pointer
    /// or singular reference to another class instance. The low byte
    /// marks the reference-kind (e.g. pointer vs. non-owning ref).
    Pointer { kind: u8 },
    /// `{kind} 0x10 0x00 0x00 ...` — a vector/array. Body (not fully
    /// decoded) contains an element-count hint and, for reference
    /// vectors, a reference to the element class's tag. Handled bases:
    /// `0x01` bool, `0x02` u16, `0x04` u32-legacy, `0x05` u32, `0x06`
    /// f32, `0x07` f64, `0x0b` u64, `0x0d` point/transform, `0x0e` ref.
    Vector {
        /// The outer type byte — e.g. `0x07` (`vector<double>`), `0x0e`
        /// (`vector<reference>`), `0x0d` (`vector<point>`).
        kind: u8,
        /// Raw body bytes after the 4-byte header.
        body: Vec<u8>,
    },
    /// `{kind} 0x50 0x00 0x00 ...` — a map / set / other associative
    /// container. Body for reference-containers (`kind == 0x0e`)
    /// typically embeds an ASCII C++ type signature
    /// (`std::pair< K, V >`, `std::map< K, V >`); scalar-base
    /// containers (e.g. `kind == 0x04` for `Container<u32>`) rarely do.
    Container {
        /// The outer type byte marking the element base type.
        kind: u8,
        /// Embedded ASCII C++ signature, if one was recovered.
        cpp_signature: Option<String>,
        /// Raw body bytes after the 4-byte header.
        body: Vec<u8>,
    },
    /// Anything we haven't classified yet. Preserves the raw bytes so
    /// downstream tools can reanalyse.
    Unknown { bytes: Vec<u8> },
}

impl FieldType {
    /// Decode a field's type-encoding block. Input is the raw bytes
    /// starting immediately after the `[u32 name_len][name]` record.
    pub fn decode(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return FieldType::Unknown { bytes: Vec::new() };
        }
        // `sub` is the two bytes following the outer-kind byte, little-
        // endian. The schema parser occasionally delivers a 2-byte slice
        // (kind + modifier only) at a record boundary — treat a single
        // trailing byte as the low byte of sub, high byte implied zero.
        let sub = if bytes.len() >= 3 {
            u16::from_le_bytes([bytes[1], bytes[2]])
        } else if bytes.len() == 2 {
            bytes[1] as u16
        } else {
            0
        };
        // Scalar base types share a single "base → size" table used by
        // both the plain-primitive arm (sub == 0x0000) and the
        // vector/container modifier arms (sub == 0x0010 / 0x0050). The
        // `0x0d` base only appears inside Vector/Container — its bare
        // wire size is unobserved in the corpus, so we do not emit it
        // as a plain Primitive.
        fn scalar_size(kind: u8) -> Option<u8> {
            match kind {
                0x01 => Some(1), // bool
                0x02 => Some(2), // u16
                0x03 => Some(4), // deprecated i32-alias (seen in 2016–2018 only,
                // fields `UserID.m_id` and `ElementRegenerationInfo.m_nAtomType`;
                // superseded by `0x05` from 2019 onward)
                0x04 => Some(4), // legacy u32
                0x05 => Some(4), // u32
                0x06 => Some(4), // f32
                0x07 => Some(8), // f64 / double
                0x0b => Some(8), // u64
                _ => None,
            }
        }
        let is_vector_capable = matches!(
            bytes[0],
            0x01 | 0x02 | 0x04 | 0x05 | 0x06 | 0x07 | 0x0b | 0x0d
        );
        // Body-after-header is bytes[4..], but the schema parser
        // occasionally delivers 2- or 3-byte slices where a record
        // boundary clipped the header mid-word. Use a safe slice.
        let body_after_header: Vec<u8> = bytes.get(4..).unwrap_or(&[]).to_vec();

        match bytes[0] {
            // Scalar primitives — 4-byte header `XX 00 00 00`
            k if scalar_size(k).is_some() && sub == 0x0000 => FieldType::Primitive {
                kind: k,
                size: scalar_size(k).unwrap(),
            },
            // Scalar vectors — `XX 10 00 00 ...` for each scalar base,
            // plus the composite `0x0d` point/transform base.
            k if is_vector_capable && sub == 0x0010 => FieldType::Vector {
                kind: k,
                body: body_after_header.clone(),
            },
            // Scalar containers — `XX 50 00 00 ...`. Reference containers
            // (0x0e 0x50 / 0x0e 0x51) are handled further down because
            // they may carry embedded C++ signatures.
            k if is_vector_capable && sub == 0x0050 => extract_container(k, &body_after_header),
            // UTF-16LE string. Two equivalent wire encodings are known:
            //   `08 00 60 00` (sub=0x6000)  — common form
            //   `08 60 00 00` (sub=0x0060)  — alternate form
            0x08 if sub == 0x6000 || sub == 0x0060 => FieldType::String,
            0x09 if sub == 0x0000 => FieldType::Guid,
            // Reference / pointer family (`0x0e` base). Per §Q5.2 of the
            // recon report, truncated 2-byte headers (`0e 50`, `0e 10`)
            // are accepted as Container / Vector with empty bodies.
            0x0e => match sub {
                0x0000 if bytes.len() >= 8 => {
                    let ref_tag = u16::from_le_bytes([bytes[4], bytes[5]]);
                    let sub2 = u16::from_le_bytes([bytes[6], bytes[7]]);
                    if ref_tag == 0x0014 && sub2 == 0x0000 {
                        FieldType::ElementId
                    } else {
                        FieldType::ElementIdRef {
                            referenced_tag: ref_tag,
                            sub: sub2,
                        }
                    }
                }
                0x0000 if bytes.len() >= 6 && bytes[4] == 0x14 && bytes[5] == 0x00 => {
                    FieldType::ElementId
                }
                0x0001..=0x0003 if bytes.len() >= 4 => FieldType::Pointer { kind: bytes[1] },
                0x0010 | 0x0011 => FieldType::Vector {
                    kind: 0x0e,
                    body: body_after_header.clone(),
                },
                0x0050 | 0x0051 => extract_container(0x0e, &body_after_header),
                _ => FieldType::Unknown {
                    bytes: bytes.to_vec(),
                },
            },
            _ => FieldType::Unknown {
                bytes: bytes.to_vec(),
            },
        }
    }

    /// Encode (WRT-01) a `FieldType` back to its wire type-encoding
    /// block — the inverse of [`FieldType::decode`]. Used by the
    /// write path (`write_with_patches` field-level edits and the
    /// forthcoming `ADocument` writer) to serialise schema records.
    ///
    /// Canonical-form guarantees:
    ///
    /// - `String` always emits the common `08 00 60 00` encoding,
    ///   even when the source file used `08 60 00 00`.
    /// - `Vector`-with-base-`0x0e` emits `sub = 0x0010` (the 0x0011
    ///   alias collapses to 0x0010 on round-trip).
    /// - `Container`-with-base-`0x0e` emits `sub = 0x0050` (0x0051
    ///   alias similarly collapses).
    /// - `Unknown` variants emit their captured bytes unchanged.
    ///
    /// Round-trip: `FieldType::decode(&x.encode()) == x` for every
    /// canonical-form variant. The two collapsing cases (String alt
    /// encoding, Vector/Container 0x0011/0x0051 aliases) round-trip
    /// to the canonical form, never back to the alt.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            FieldType::Primitive { kind, .. } => vec![*kind, 0x00, 0x00, 0x00],
            FieldType::String => vec![0x08, 0x00, 0x60, 0x00],
            FieldType::Guid => vec![0x09, 0x00, 0x00, 0x00],
            FieldType::ElementId => {
                vec![0x0e, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00]
            }
            FieldType::ElementIdRef {
                referenced_tag,
                sub,
            } => {
                let tag_bytes = referenced_tag.to_le_bytes();
                let sub_bytes = sub.to_le_bytes();
                vec![
                    0x0e,
                    0x00,
                    0x00,
                    0x00,
                    tag_bytes[0],
                    tag_bytes[1],
                    sub_bytes[0],
                    sub_bytes[1],
                ]
            }
            FieldType::Pointer { kind } => vec![0x0e, *kind, 0x00, 0x00],
            FieldType::Vector { kind, body } => {
                let mut out = vec![*kind, 0x10, 0x00, 0x00];
                out.extend_from_slice(body);
                out
            }
            FieldType::Container { kind, body, .. } => {
                // The cpp_signature is embedded inside body (the
                // decoder lifts it out as a convenience, but body
                // retains the raw bytes verbatim). Emit body as-is.
                let mut out = vec![*kind, 0x50, 0x00, 0x00];
                out.extend_from_slice(body);
                out
            }
            FieldType::Unknown { bytes } => bytes.clone(),
        }
    }
}

fn extract_container(kind: u8, body: &[u8]) -> FieldType {
    let mut cpp_signature = None;
    let mut k = 0;
    while k + 2 < body.len() {
        let slen = u16::from_le_bytes([body[k], body[k + 1]]) as usize;
        if (3..=120).contains(&slen) && k + 2 + slen <= body.len() {
            let sig = &body[k + 2..k + 2 + slen];
            if sig.iter().all(|b| b.is_ascii_graphic() || *b == b' ')
                && sig.iter().any(|b| *b == b':' || *b == b'<')
            {
                cpp_signature = Some(std::str::from_utf8(sig).unwrap_or("").to_string());
                break;
            }
        }
        k += 1;
    }
    FieldType::Container {
        kind,
        cpp_signature,
        body: body.to_vec(),
    }
}

/// Parse the decompressed `Formats/Latest` bytes into a schema table.
///
/// # Caveat
///
/// The real schema lives in the first ~64 KB of the decompressed stream.
/// Beyond that, `Formats/Latest` contains binary object data whose bit
/// patterns incidentally trip our class-name heuristic. We cap scanning at
/// 64 KB to avoid emitting false-positive garbage classes.
pub fn parse_schema(decompressed: &[u8]) -> Result<SchemaTable> {
    let mut classes = Vec::new();
    let mut cpp_types = std::collections::BTreeSet::new();
    let mut skipped = 0usize;

    // Schema section is in the early portion of the stream. Scanning
    // beyond this produces false-positive class records from compressed
    // binary noise.
    const SCHEMA_SCAN_LIMIT: usize = 64 * 1024;
    let data = if decompressed.len() > SCHEMA_SCAN_LIMIT {
        &decompressed[..SCHEMA_SCAN_LIMIT]
    } else {
        decompressed
    };
    let mut i = 0;

    while i + 2 < data.len() {
        // Find next candidate length-prefixed string of length 3..=60.
        // Candidates that don't match our alphabet are skipped.
        let len = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if !(3..=60).contains(&len) {
            i += 1;
            continue;
        }
        let str_start = i + 2;
        if str_start + len > data.len() {
            i += 1;
            continue;
        }
        let name_bytes = &data[str_start..str_start + len];
        if !looks_like_class_name(name_bytes) {
            i += 1;
            continue;
        }

        // Got a class-name candidate. Parse its fields until we hit
        // another likely class boundary (another length-prefixed name
        // matching our heuristic).
        let class_name = std::str::from_utf8(name_bytes).unwrap().to_string();
        let class_offset = i;

        // Move cursor past the class name header.
        let mut cursor = str_start + len;

        // Try to parse the tag word (u16) immediately after the name.
        // If its 0x8000 bit is set, this is a TAGGED (top-level) class.
        // For tagged classes we also try to recognise the following
        // `[u16 pad=0][u16 parent_name_len][parent_name]` block and the
        // `[u16 flag][u32 field_count][u32 field_count]` preamble that
        // precede the field list. See FACT F3 in
        // docs/rvt-moat-break-reconnaissance.md §Phase 4c findings.
        let mut tag: Option<u16> = None;
        let mut parent: Option<String> = None;
        let mut declared_field_count: Option<u32> = None;
        let mut ancestor_tag: Option<u16> = None;
        if cursor + 2 <= data.len() {
            let raw_tag = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
            if raw_tag & 0x8000 != 0 {
                tag = Some(raw_tag & 0x7fff);
                cursor += 2;
                // Skip the 2-byte pad, then read u16 parent-name-length.
                if cursor + 4 <= data.len() {
                    let pad = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
                    let plen = u16::from_le_bytes([data[cursor + 2], data[cursor + 3]]) as usize;
                    if pad == 0 && (3..=40).contains(&plen) && cursor + 4 + plen <= data.len() {
                        let p = &data[cursor + 4..cursor + 4 + plen];
                        if looks_like_class_name(p) {
                            // Peek at what follows the candidate parent name
                            // to confirm the preamble validates. Only commit
                            // (record parent, advance cursor) if both the
                            // parent name AND the following
                            // `[u16 flag][u32 fc][u32 fc_dup]` preamble look
                            // plausible. This avoids misreading the NEXT
                            // class's declaration as this class's parent.
                            let preamble_at = cursor + 4 + plen;
                            if preamble_at + 10 <= data.len() {
                                let flag =
                                    u16::from_le_bytes([data[preamble_at], data[preamble_at + 1]]);
                                let fc = u32::from_le_bytes([
                                    data[preamble_at + 2],
                                    data[preamble_at + 3],
                                    data[preamble_at + 4],
                                    data[preamble_at + 5],
                                ]);
                                let fc2 = u32::from_le_bytes([
                                    data[preamble_at + 6],
                                    data[preamble_at + 7],
                                    data[preamble_at + 8],
                                    data[preamble_at + 9],
                                ]);
                                if flag & 0x8000 == 0 && fc == fc2 && fc <= 200 {
                                    parent = Some(std::str::from_utf8(p).unwrap().to_string());
                                    declared_field_count = Some(fc);
                                    if flag != 0 {
                                        ancestor_tag = Some(flag);
                                    }
                                    cursor = preamble_at + 10;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Walk forward until we find the next class-name candidate OR
        // we've seen the declared number of fields, whichever comes
        // first. The declared_field_count bound prevents bleeding into
        // the parent class's field list when a subclass has few own
        // fields but the parent has many.
        let mut fields = Vec::new();
        let (next_class_offset, found_fields) = scan_fields_until_next_class_bounded(
            data,
            cursor,
            &mut cpp_types,
            declared_field_count,
        );
        fields.extend(found_fields);
        cursor = next_class_offset;

        // Validate: at least class name parsed successfully.
        if class_name.is_empty() {
            skipped += 1;
        } else {
            classes.push(ClassEntry {
                name: class_name,
                offset: class_offset,
                fields,
                tag,
                parent,
                declared_field_count,
                was_parent_only: false,
                ancestor_tag,
            });
        }
        i = cursor.max(i + 1);
    }

    // Second pass: for every `parent` reference that doesn't appear as its
    // own top-level declaration, synthesize a stub entry. Keeps the
    // schema table closed over the class graph.
    let declared_names: std::collections::BTreeSet<String> =
        classes.iter().map(|c| c.name.clone()).collect();
    let parent_names: std::collections::BTreeSet<String> =
        classes.iter().filter_map(|c| c.parent.clone()).collect();
    for parent_name in parent_names.difference(&declared_names) {
        classes.push(ClassEntry {
            name: parent_name.clone(),
            offset: 0,
            fields: Vec::new(),
            tag: None,
            parent: None,
            declared_field_count: None,
            was_parent_only: true,
            ancestor_tag: None,
        });
    }

    Ok(SchemaTable {
        classes,
        cpp_types: cpp_types.into_iter().collect(),
        skipped_records: skipped,
    })
}

fn looks_like_class_name(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    // First char must be uppercase ASCII letter
    let first = bytes[0];
    if !first.is_ascii_uppercase() {
        return false;
    }
    // Remaining chars: alphanumeric or underscore only
    bytes[1..]
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn looks_like_field_name(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0];
    // field names often start with m_ (C++ convention), or lowercase,
    // or uppercase if it's a nested class or enum
    if !(first.is_ascii_alphanumeric() || first == b'_') {
        return false;
    }
    bytes
        .iter()
        .all(|c| c.is_ascii_alphanumeric() || *c == b'_')
}

fn looks_like_cpp_type(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let s = match std::str::from_utf8(bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };
    // Basic sanity: must be printable ASCII, reasonable chars
    s.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                ':' | '<' | '>' | ',' | ' ' | '_' | '*' | '&' | '[' | ']' | '(' | ')'
            )
    }) && (s.chars().any(|c| c.is_ascii_uppercase())
        || s.contains("std::")
        || s.contains("int")
        || s.contains("double")
        || s.contains("long"))
}

/// Scan the buffer starting at `cursor` for field records until we hit
/// either end-of-stream or another class-name candidate. Returns
/// `(new_cursor_position, discovered_fields)`.
///
/// Field names use u32 LE length prefix (distinct from class names which use
/// u16). Type signatures that follow also use u32 LE. Example (from the
/// 2024 reference file, offset 0x80):
///
/// ```text
///   0080  00 0d 00 41 43 44 50 74 72 57 72 61 70 70 65 72    0  13  A C D P t r W r a p p e r
///         pad  u16=13 ^------------ "ACDPtrWrapper" --------^
///                     (class name)
///         00 00                                              class tag / pad
///         01 00 00 00  01 00 00 00                           field count, field index
///         06 00 00 00  6d 5f 70 41 43 44                     u32=6, "m_pACD" (field name)
///         0e 03 00 00 00 00 00 00 00 00                      field type code block
/// ```
#[allow(dead_code)]
fn scan_fields_until_next_class(
    data: &[u8],
    start: usize,
    cpp_types: &mut std::collections::BTreeSet<String>,
) -> (usize, Vec<FieldEntry>) {
    scan_fields_until_next_class_bounded(data, start, cpp_types, None)
}

/// Same as `scan_fields_until_next_class` but stops early once
/// `max_fields` fields have been emitted. Used when the caller already
/// knows the declared field count from the class's preamble, preventing
/// the scanner from bleeding into the parent class's field list.
fn scan_fields_until_next_class_bounded(
    data: &[u8],
    start: usize,
    cpp_types: &mut std::collections::BTreeSet<String>,
    max_fields: Option<u32>,
) -> (usize, Vec<FieldEntry>) {
    let mut fields = Vec::new();
    let mut i = start;
    let hard_stop = (start + 4096).min(data.len());

    while i + 4 < hard_stop {
        if let Some(max) = max_fields {
            if fields.len() as u32 >= max {
                return (i, fields);
            }
        }
        // First: is this a u16-prefixed class-name candidate?
        let u16_len = u16::from_le_bytes([data[i], data[i + 1]]) as usize;
        if (4..=60).contains(&u16_len) && i + 2 + u16_len <= hard_stop {
            let slice = &data[i + 2..i + 2 + u16_len];
            if looks_like_class_name(slice) {
                return (i, fields);
            }
        }

        // Field record candidate: u32 length prefix.
        let u32_len = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if (2..=60).contains(&u32_len) && i + 4 + u32_len <= hard_stop {
            let slice = &data[i + 4..i + 4 + u32_len];
            if looks_like_field_name(slice) {
                let field_name = std::str::from_utf8(slice).unwrap().to_string();
                let post_name = i + 4 + u32_len;

                // Optional C++ type follows. Try u32 prefix first, then u16.
                let mut cpp_type = None;
                let consumed = post_name;

                // Type signatures in the corpus sometimes have u16 prefix,
                // sometimes u32. Try u32 first.
                let mut type_consumed_bytes = 0usize;
                for (prefix_len, is_u32) in [(4usize, true), (2usize, false)] {
                    if consumed + prefix_len >= hard_stop {
                        continue;
                    }
                    let type_len = if is_u32 {
                        u32::from_le_bytes([
                            data[consumed],
                            data[consumed + 1],
                            data[consumed + 2],
                            data[consumed + 3],
                        ]) as usize
                    } else {
                        u16::from_le_bytes([data[consumed], data[consumed + 1]]) as usize
                    };
                    if (3..=120).contains(&type_len)
                        && consumed + prefix_len + type_len <= hard_stop
                    {
                        let type_slice =
                            &data[consumed + prefix_len..consumed + prefix_len + type_len];
                        if looks_like_cpp_type(type_slice) {
                            let ts = std::str::from_utf8(type_slice)
                                .unwrap_or_default()
                                .trim()
                                .to_string();
                            cpp_types.insert(ts.clone());
                            cpp_type = Some(ts);
                            type_consumed_bytes = prefix_len + type_len;
                            break;
                        }
                    }
                }

                // Decode the type_encoding byte pattern from the bytes
                // immediately after the field name. We cap at 32 bytes
                // because field_type's Unknown variant preserves the raw
                // input, and we don't want to accidentally swallow the
                // next field's header.
                let enc_end = (post_name + 32).min(hard_stop);
                let field_type = if enc_end > post_name {
                    Some(FieldType::decode(&data[post_name..enc_end]))
                } else {
                    None
                };

                // Harvest embedded C++ signatures from Container fields —
                // they contain the only reliable source of ASCII C++ type
                // strings (e.g. "std::pair< int, X >") in the schema
                // stream. Preserves the `cpp_types` set that was broken
                // when we stopped reading explicit type prefixes.
                if let Some(FieldType::Container {
                    cpp_signature: Some(sig),
                    ..
                }) = &field_type
                {
                    cpp_types.insert(sig.clone());
                    if cpp_type.is_none() {
                        cpp_type = Some(sig.clone());
                    }
                }

                fields.push(FieldEntry {
                    name: field_name,
                    cpp_type,
                    field_type,
                });
                i = if type_consumed_bytes > 0 {
                    consumed + type_consumed_bytes
                } else {
                    post_name
                };
                continue;
            }
        }
        i += 1;
    }
    (i, fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn class_name_heuristic() {
        assert!(looks_like_class_name(b"ADocument"));
        assert!(looks_like_class_name(b"A3PartyObject"));
        assert!(looks_like_class_name(b"APIEventHandlerStatus"));
        assert!(!looks_like_class_name(b"lowercaseStart"));
        assert!(!looks_like_class_name(b""));
        assert!(!looks_like_class_name(b"Has-Dash"));
    }

    #[test]
    fn field_name_heuristic() {
        assert!(looks_like_field_name(b"m_id"));
        assert!(looks_like_field_name(b"m_id64"));
        assert!(looks_like_field_name(b"first"));
        assert!(looks_like_field_name(b"second"));
        assert!(!looks_like_field_name(b"Has Space"));
    }

    #[test]
    fn cpp_type_heuristic() {
        assert!(looks_like_cpp_type(b"std::pair< ElementId, double >"));
        assert!(looks_like_cpp_type(b"ElementId"));
        assert!(looks_like_cpp_type(b"int"));
        assert!(!looks_like_cpp_type(b"m_id")); // lowercase only = field name territory
    }

    #[test]
    fn parses_sample_schema_snippet() {
        // Realistic snippet mirroring the observed wire format:
        //  [u16 LE 13] "ACDPtrWrapper"   (class name)
        //  [u16 LE 0]                     (class tag / pad)
        //  [u32 LE 1]                     (field count)
        //  [u32 LE 1]                     (index or secondary count)
        //  [u32 LE 6] "m_pACD"            (field name with u32 prefix)
        //  [u32 LE 0]                     (no cpp type)
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&[0x0d, 0x00]); // u16 len=13
        buf.extend_from_slice(b"ACDPtrWrapper"); // 13 ASCII bytes
        buf.extend_from_slice(&[0x00, 0x00]); // class tag
        buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // field count u32
        buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // index u32
        buf.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]); // field name len u32
        buf.extend_from_slice(b"m_pACD"); // 6 ASCII bytes

        let schema = parse_schema(&buf).unwrap();
        assert!(
            schema.classes.iter().any(|c| c.name == "ACDPtrWrapper"),
            "expected class ACDPtrWrapper, got {:?}",
            schema.classes.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        let class = schema
            .classes
            .iter()
            .find(|c| c.name == "ACDPtrWrapper")
            .unwrap();
        assert!(
            class.fields.iter().any(|f| f.name == "m_pACD"),
            "expected field m_pACD, got {:?}",
            class.fields.iter().map(|f| &f.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn decodes_field_type_element_id() {
        // 6-byte pattern for ElementId: 0e 00 00 00 14 00
        let bytes = [0x0e, 0x00, 0x00, 0x00, 0x14, 0x00];
        assert_eq!(FieldType::decode(&bytes), FieldType::ElementId);
    }

    #[test]
    fn decodes_field_type_pointer() {
        // 4-byte pattern for pointer: 0e 02 00 00
        let bytes = [0x0e, 0x02, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(ft, FieldType::Pointer { kind: 0x02 }));
    }

    #[test]
    fn decodes_field_type_primitive_u32_legacy() {
        // Legacy 0x04 pattern (pre-2021 u32 discriminator)
        let bytes = [0x04, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(
            ft,
            FieldType::Primitive {
                kind: 0x04,
                size: 4
            }
        ));
    }

    #[test]
    fn decodes_field_type_primitive_bool() {
        let bytes = [0x01, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(
            ft,
            FieldType::Primitive {
                kind: 0x01,
                size: 1
            }
        ));
    }

    #[test]
    fn decodes_field_type_primitive_f64() {
        let bytes = [0x07, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(
            ft,
            FieldType::Primitive {
                kind: 0x07,
                size: 8
            }
        ));
    }

    #[test]
    fn decodes_field_type_string() {
        let bytes = [0x08, 0x00, 0x60, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(ft, FieldType::String));
    }

    #[test]
    fn decodes_field_type_guid() {
        let bytes = [0x09, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(ft, FieldType::Guid));
    }

    #[test]
    fn decodes_field_type_u64() {
        let bytes = [0x0b, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(
            ft,
            FieldType::Primitive {
                kind: 0x0b,
                size: 8
            }
        ));
    }

    #[test]
    fn decodes_field_type_container_with_cpp_signature() {
        // 0e 50 00 00 + class-tag + u16 len + "std::pair< int, X >"
        let sig = b"std::pair< int, X >";
        let mut bytes = vec![0x0e, 0x50, 0x00, 0x00];
        // class-tag stand-in
        bytes.extend_from_slice(&0x0000814au32.to_le_bytes());
        bytes.extend_from_slice(&(sig.len() as u16).to_le_bytes());
        bytes.extend_from_slice(sig);
        let ft = FieldType::decode(&bytes);
        match ft {
            FieldType::Container {
                kind,
                cpp_signature,
                ..
            } => {
                assert_eq!(kind, 0x0e);
                assert_eq!(cpp_signature.as_deref(), Some("std::pair< int, X >"));
            }
            other => panic!("expected Container, got {other:?}"),
        }
    }

    // ─────────────────────────────────────────────────────────────────
    // Pattern pins: every FieldType::decode arm added in Phase Q5.2 has
    // a unit test that captures its exact byte signature. If the decoder
    // ever regresses, these tests fail with a single precise diff.
    // Evidence for each pattern is in examples/unknown_bytes_deep.rs,
    // which runs against the 11-version corpus.
    // ─────────────────────────────────────────────────────────────────

    #[test]
    fn decodes_field_type_string_alt_encoding() {
        // `08 60 00 00` — alternate string discriminator seen in 2016–2026,
        // e.g. `Identifier.second`, `AStringWrapper.m_str`.
        let bytes = [0x08, 0x60, 0x00, 0x00];
        assert!(matches!(FieldType::decode(&bytes), FieldType::String));
    }

    #[test]
    fn decodes_field_type_primitive_deprecated_0x03() {
        // `03 00 00 00` — deprecated i32-alias; seen in 2016–2018 only.
        // Fields: `UserID.m_id`, `ElementRegenerationInfo.m_nAtomType`.
        let bytes = [0x03, 0x00, 0x00, 0x00];
        assert!(matches!(
            FieldType::decode(&bytes),
            FieldType::Primitive {
                kind: 0x03,
                size: 4,
            }
        ));
    }

    #[test]
    fn decodes_field_type_scalar_vector_f32() {
        // `06 10 00 00 03 00 00 00` — vector<f32>, e.g. APropertyFloat3.m_value
        let bytes = [0x06, 0x10, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        match ft {
            FieldType::Vector { kind, body } => {
                assert_eq!(kind, 0x06);
                assert_eq!(body, [0x03, 0x00, 0x00, 0x00]);
            }
            other => panic!("expected Vector<f32>, got {other:?}"),
        }
    }

    #[test]
    fn decodes_field_type_scalar_vector_i32() {
        // `04 10 00 00` — vector<u32/i32>, e.g. Int64Wrapper.m_value
        let bytes = [0x04, 0x10, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Vector { kind: 0x04, .. }));
    }

    #[test]
    fn decodes_field_type_scalar_vector_bool() {
        // `01 10 00 00` — vector<bool>, e.g. AnalyticalAdjustmentGStep.m_bLock
        let bytes = [0x01, 0x10, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Vector { kind: 0x01, .. }));
    }

    #[test]
    fn decodes_field_type_point_vector_0x0d() {
        // `0d 10 00 00` — vector<point3d>, e.g. Outline.m_minmax (3D pair)
        let bytes = [0x0d, 0x10, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Vector { kind: 0x0d, .. }));
    }

    #[test]
    fn decodes_field_type_point_container_0x0d() {
        // `0d 50 00 00` — container<point3d>, e.g. Dimension.m_refPnts
        let bytes = [0x0d, 0x50, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Container { kind: 0x0d, .. }));
    }

    #[test]
    fn decodes_field_type_scalar_container_u32() {
        // `04 50 00 00` — container<u32>, e.g. RbsSystemNavigatorColumn.m_rgColWidths
        let bytes = [0x04, 0x50, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Container { kind: 0x04, .. }));
    }

    #[test]
    fn decodes_field_type_scalar_container_f64() {
        // `07 50 00 00` — container<f64>, e.g. AnalysisResultSchema.m_unitsMultipliers
        let bytes = [0x07, 0x50, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Container { kind: 0x07, .. }));
    }

    #[test]
    fn decodes_field_type_element_id_ref() {
        // `0e 00 00 00 58 00 09 00` — reference to class with tag=0x58, sub=0x09
        // e.g. ADocument.m_devBranchInfo (2026 sample)
        let bytes = [0x0e, 0x00, 0x00, 0x00, 0x58, 0x00, 0x09, 0x00];
        let ft = FieldType::decode(&bytes);
        match ft {
            FieldType::ElementIdRef {
                referenced_tag,
                sub,
            } => {
                assert_eq!(referenced_tag, 0x0058);
                assert_eq!(sub, 0x0009);
            }
            other => panic!("expected ElementIdRef, got {other:?}"),
        }
    }

    #[test]
    fn decodes_field_type_element_id_tag_0x14_8byte_still_canonical() {
        // The canonical ElementId (tag=0x0014, sub=0x0000) in 8-byte form
        // should still resolve to the unit `ElementId` variant, not
        // `ElementIdRef`.
        let bytes = [0x0e, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00];
        assert_eq!(FieldType::decode(&bytes), FieldType::ElementId);
    }

    #[test]
    fn decodes_field_type_truncated_ref_container() {
        // `0e 50` — 2-byte clipped `0x0e 0x50` header seen once in 2025
        // (RadialConstraint.m_witnessRefs). Schema parser clipped at a
        // record boundary. Still valid Container<ref>, empty body.
        let bytes = [0x0e, 0x50];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Container { kind: 0x0e, .. }));
    }

    #[test]
    fn decodes_field_type_truncated_point_vector() {
        // `0d 10` — 2-byte clipped `0x0d 0x10` header seen once in 2018
        // (BoundedSpace.m_boundActive). Still valid Vector<point>.
        let bytes = [0x0d, 0x10];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(&ft, FieldType::Vector { kind: 0x0d, .. }));
    }

    #[test]
    fn decode_is_safe_on_short_inputs() {
        // Pathological short inputs must never panic. 0, 1, 2, 3-byte
        // slices should all produce either Unknown or a typed variant.
        for len in 0..=3 {
            let input: Vec<u8> = (0..len).map(|_| 0xff).collect();
            let _ = FieldType::decode(&input); // must not panic
        }
    }

    #[test]
    fn parses_tagged_class_with_parent() {
        // Mirrors the observed HostObjAttr record at offset 0x7238 in the
        // 2024 reference file:
        //   [u16 11] "HostObjAttr"
        //   [u16 0x806b]          (tag 0x006b, 0x8000 flag set)
        //   [u16 0]               (pad)
        //   [u16 6] "Symbol"      (parent class)
        //   [u16 0x0025]          (flag)
        //   [u32 3] [u32 3]       (field count x 2)
        //   [u32 12] "m_symbolInfo" [u32 0x0000020e]        (field 1)
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&[0x0b, 0x00]);
        buf.extend_from_slice(b"HostObjAttr");
        buf.extend_from_slice(&[0x6b, 0x80]); // tag 0x006b with 0x8000 flag
        buf.extend_from_slice(&[0x00, 0x00]); // pad
        buf.extend_from_slice(&[0x06, 0x00]); // parent name len = 6
        buf.extend_from_slice(b"Symbol"); // parent
        buf.extend_from_slice(&[0x25, 0x00]); // flag
        buf.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]); // field count = 3
        buf.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]); // duplicate
        buf.extend_from_slice(&[0x0c, 0x00, 0x00, 0x00]); // field 1 name len = 12
        buf.extend_from_slice(b"m_symbolInfo"); // field 1 name
        buf.extend_from_slice(&[0x0e, 0x02, 0x00, 0x00]); // type encoding
        // pad out to 64KB-ish so schema parser doesn't bail on the last record
        buf.resize(512, 0);

        let schema = parse_schema(&buf).unwrap();
        let class = schema
            .classes
            .iter()
            .find(|c| c.name == "HostObjAttr")
            .expect("HostObjAttr class not parsed");
        assert_eq!(class.tag, Some(0x006b));
        assert_eq!(class.parent.as_deref(), Some("Symbol"));
        assert_eq!(class.declared_field_count, Some(3));
    }

    /// API-10: diagnostic counters reflect the parsed schema state
    /// accurately — parsed field count, declared-vs-parsed mismatch
    /// detection, tagged / parent-only / ancestor-tag breakdowns.
    #[test]
    fn schema_diagnostics_on_synthetic_table() {
        let table = SchemaTable {
            classes: vec![
                ClassEntry {
                    name: "Wall".into(),
                    offset: 0,
                    fields: vec![
                        FieldEntry {
                            name: "m_id".into(),
                            cpp_type: None,
                            field_type: None,
                        },
                        FieldEntry {
                            name: "m_name".into(),
                            cpp_type: None,
                            field_type: None,
                        },
                    ],
                    tag: Some(0x01),
                    parent: None,
                    declared_field_count: Some(2),
                    was_parent_only: false,
                    ancestor_tag: Some(0x42),
                },
                ClassEntry {
                    name: "Door".into(),
                    offset: 128,
                    fields: vec![FieldEntry {
                        name: "m_id".into(),
                        cpp_type: None,
                        field_type: None,
                    }],
                    tag: Some(0x02),
                    parent: Some("FamilyInstance".into()),
                    // Declares 3 but we only parsed 1 → mismatch.
                    declared_field_count: Some(3),
                    was_parent_only: false,
                    ancestor_tag: None,
                },
                ClassEntry {
                    name: "FamilyInstance".into(),
                    offset: 256,
                    fields: vec![],
                    tag: None,
                    parent: None,
                    declared_field_count: None,
                    was_parent_only: true,
                    ancestor_tag: None,
                },
            ],
            cpp_types: vec!["ElementId".into(), "double".into()],
            skipped_records: 7,
        };
        let d = table.diagnostics();
        assert_eq!(d.class_count, 3);
        assert_eq!(d.parsed_field_count, 3);
        assert_eq!(d.declared_field_count_sum, 5);
        assert_eq!(d.field_count_mismatches, 1); // Door declared 3 but parsed 1
        assert_eq!(d.tagged_class_count, 2);
        assert_eq!(d.parent_only_class_count, 1);
        assert_eq!(d.ancestor_tag_count, 1);
        assert_eq!(d.skipped_records, 7);
        assert_eq!(d.cpp_type_count, 2);
    }

    #[test]
    fn schema_diagnostics_empty_table_is_zero() {
        let table = SchemaTable {
            classes: vec![],
            cpp_types: vec![],
            skipped_records: 0,
        };
        let d = table.diagnostics();
        assert_eq!(d.class_count, 0);
        assert_eq!(d.parsed_field_count, 0);
        assert_eq!(d.declared_field_count_sum, 0);
        assert_eq!(d.field_count_mismatches, 0);
        assert_eq!(d.tagged_class_count, 0);
    }

    // ---- WRT-01: FieldType::encode round-trip tests ----

    #[test]
    fn encode_primitive_round_trips() {
        for kind in [0x01_u8, 0x02, 0x04, 0x05, 0x06, 0x07, 0x0b] {
            let original = FieldType::decode(&[kind, 0x00, 0x00, 0x00]);
            let encoded = original.encode();
            let decoded = FieldType::decode(&encoded);
            assert_eq!(original, decoded, "primitive kind 0x{kind:02x} round-trip");
        }
    }

    #[test]
    fn encode_string_emits_canonical_form() {
        let original = FieldType::String;
        let encoded = original.encode();
        assert_eq!(encoded, vec![0x08, 0x00, 0x60, 0x00]);
        assert_eq!(FieldType::decode(&encoded), FieldType::String);
    }

    #[test]
    fn encode_string_alt_form_collapses_on_round_trip() {
        // Alt form 08 60 00 00 also decodes to String; encoder should
        // emit the canonical form instead.
        let alt = FieldType::decode(&[0x08, 0x60, 0x00, 0x00]);
        assert_eq!(alt, FieldType::String);
        let canonical = alt.encode();
        assert_eq!(canonical, vec![0x08, 0x00, 0x60, 0x00]);
    }

    #[test]
    fn encode_guid_round_trips() {
        let g = FieldType::Guid;
        assert_eq!(g.encode(), vec![0x09, 0x00, 0x00, 0x00]);
        assert_eq!(FieldType::decode(&g.encode()), FieldType::Guid);
    }

    #[test]
    fn encode_element_id_round_trips() {
        let id = FieldType::ElementId;
        let bytes = id.encode();
        assert_eq!(bytes.len(), 8);
        assert_eq!(FieldType::decode(&bytes), FieldType::ElementId);
    }

    #[test]
    fn encode_element_id_ref_round_trips() {
        let refed = FieldType::ElementIdRef {
            referenced_tag: 0x0150,
            sub: 0x0009,
        };
        let bytes = refed.encode();
        let decoded = FieldType::decode(&bytes);
        assert_eq!(decoded, refed);
    }

    #[test]
    fn encode_pointer_round_trips() {
        for kind in [0x01_u8, 0x02, 0x03] {
            let p = FieldType::Pointer { kind };
            let bytes = p.encode();
            assert_eq!(FieldType::decode(&bytes), p);
        }
    }

    #[test]
    fn encode_vector_round_trips_for_scalar_bases() {
        for kind in [0x01_u8, 0x02, 0x05, 0x06, 0x07, 0x0b] {
            let body = vec![0xAA, 0xBB, 0xCC];
            let v = FieldType::Vector {
                kind,
                body: body.clone(),
            };
            let bytes = v.encode();
            assert_eq!(FieldType::decode(&bytes), v, "vector kind 0x{kind:02x}");
        }
    }

    #[test]
    fn encode_vector_reference_round_trips() {
        let body = vec![0x14, 0x00, 0x00, 0x00];
        let v = FieldType::Vector {
            kind: 0x0e,
            body: body.clone(),
        };
        let bytes = v.encode();
        assert_eq!(FieldType::decode(&bytes), v);
    }

    #[test]
    fn encode_container_round_trips_with_cpp_signature() {
        // Build a body that embeds a plausible C++ signature so the
        // decoder recovers cpp_signature on the other side.
        let sig = b"std::map< unsigned, double >";
        let mut body = Vec::new();
        body.extend_from_slice(&(sig.len() as u16).to_le_bytes());
        body.extend_from_slice(sig);
        body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]); // trailing data
        let c = FieldType::Container {
            kind: 0x0e,
            cpp_signature: Some("std::map< unsigned, double >".to_string()),
            body: body.clone(),
        };
        let bytes = c.encode();
        let decoded = FieldType::decode(&bytes);
        assert_eq!(decoded, c);
    }

    #[test]
    fn encode_unknown_emits_captured_bytes() {
        let raw = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x42];
        let u = FieldType::Unknown { bytes: raw.clone() };
        assert_eq!(u.encode(), raw);
    }
}
