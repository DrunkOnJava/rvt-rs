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

/// How many decompressed `Formats/Latest` bytes [`parse_schema`] scanned
/// before #410.
///
/// The cap hid a decompression fault rather than a property of the stream:
/// `Formats/Latest` is checksum-paged, and inflated without the page strip it
/// drifts after the first page boundary, so the parse past 64 KB produced
/// garbage classes. Inflated through [`crate::compression::inflate_stream_at`],
/// which strips the pages, the whole stream is schema: 4,126 classes on
/// Revit 2024 where the first 64 KB held 395
/// (`examples/probe_schema_page_strip.rs`). Kept for
/// [`parse_schema_with_scan_limit`] callers that want the old window.
pub const SCHEMA_SCAN_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaTable {
    pub classes: Vec<ClassEntry>,
    /// Every unique C++ type signature seen in the schema (e.g.
    /// `std::pair< ElementId, double >`, `ElementId`, `Identifier`).
    pub cpp_types: Vec<String>,
    /// Raw count of parse-candidates skipped for validation reasons.
    pub skipped_records: usize,
    /// Bytes of the decompressed stream the parser actually scanned —
    /// `min(total_bytes, scan limit)`.
    #[serde(default)]
    pub scanned_bytes: usize,
    /// Total decompressed bytes handed to [`parse_schema`].
    #[serde(default)]
    pub total_bytes: usize,
    /// `true` when the scan stopped at a caller's scan limit with bytes left
    /// over ([`parse_schema_with_scan_limit`]); [`parse_schema`] scans the
    /// whole stream.
    #[serde(default)]
    pub scan_truncated: bool,
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
    /// Classes that carry a non-zero `ancestor_tag` (the Q4 addendum read
    /// it as a reference distinct from `parent`; not reproduced, RE-137).
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

    /// Resolve a class name to the first tag found by walking the
    /// `parent` chain from the class up to the root (RE-18).
    ///
    /// Walks: `class → class.parent → class.parent.parent → …` until
    /// an entry with `tag.is_some()` is found. Returns `(ancestor_name,
    /// tag)` — the ancestor is `class_name` itself when the class is
    /// directly tagged. Returns `None` when no ancestor carries a tag,
    /// the class name is not in the schema, or a parent cycle is
    /// detected (defensive — the schema has never contained one in
    /// observed corpora but cheap to guard).
    ///
    /// # Motivation
    ///
    /// Of the 405 classes in a 2023 schema, only 80 carry tags. The
    /// other 325 are abstract parents, mixins, or type-only
    /// declarations. `Wall`, `Floor`, `Door`, `Window`, `Level`,
    /// `Grid`, `FamilyInstance`, `Room` are all tagless; their tagged
    /// concrete instances appear under subtype names like `ArcWall`
    /// (`0x0191`) or `WallCGDriver` (`0x0197`). Hypothesis H7 from
    /// `reports/element-framing/RE-09-synthesis.md` (confidence 0.7)
    /// says tagless classes use their parent's tag on the wire — this
    /// function is the helper for validating that hypothesis against
    /// real partition bytes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rvt::{RevitFile, compression, formats, streams};
    /// # let mut rf = RevitFile::open("some.rvt").unwrap();
    /// # let raw = rf.read_stream(streams::FORMATS_LATEST).unwrap();
    /// # let decomp = compression::inflate_at(&raw, 0).unwrap();
    /// # let schema = formats::parse_schema(&decomp).unwrap();
    /// // `Wall` is tagless in every observed release; its nearest
    /// // tagged ancestor is a concrete subtype like `ArcWall`.
    /// let (anc, tag) = schema.tagged_ancestor("Wall").unwrap_or(("<none>", 0));
    /// ```
    pub fn tagged_ancestor(&self, class_name: &str) -> Option<(&str, u16)> {
        let mut seen: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        let mut current = class_name;
        loop {
            if !seen.insert(current) {
                return None;
            }
            let entry = self.classes.iter().find(|c| c.name == current)?;
            if let Some(t) = entry.tag {
                return Some((entry.name.as_str(), t));
            }
            current = entry.parent.as_deref()?;
        }
    }

    /// Build a full `name → (ancestor, tag)` resolution map for every
    /// class in the schema (RE-18). Useful for partition-chunk tag
    /// scanning where you want a single pass over all classes to know
    /// "given a chunk tagged 0x0191, what's the most specific class
    /// name I should call it?" without re-walking the parent chain
    /// per-record.
    ///
    /// Entries where no ancestor carries a tag are omitted — callers
    /// who need the untagged set should filter `self.classes` by
    /// checking `tagged_ancestor().is_none()`.
    pub fn tagged_ancestor_map(&self) -> std::collections::BTreeMap<String, (String, u16)> {
        let mut out = std::collections::BTreeMap::new();
        for c in &self.classes {
            if let Some((anc, tag)) = self.tagged_ancestor(&c.name) {
                out.insert(c.name.clone(), (anc.to_string(), tag));
            }
        }
        out
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
    /// The preamble's "flag" word (the u16 immediately before the field
    /// count). The Q4 addendum read non-zero values as a class-tag
    /// reference to an ancestor distinct from the direct `parent`; that
    /// reading did not reproduce under the page-stripped schema grammar
    /// (`schema_classes`, RE-137), and what the word is has not been
    /// established. See the §Q4 addendum in
    /// `docs/rvt-moat-break-reconnaissance.md`.
    ///
    /// `None` when the slot was 0x0000. 55% of tagged classes in the 2024
    /// sample have no ancestor_tag.
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
            // Scalar containers — `XX 50 00 00`. Always 4-byte headers
            // with no body (per `docs/container-wire-format-2026-04-21.md`
            // — only 0x0e carries a variable payload). Reference
            // containers (0x0e 0x50 / 0x0e 0x51) are handled further
            // down because they may embed a C++ signature.
            //
            // Historical note: this arm previously delegated to
            // `extract_container(k, &body_after_header)`, which
            // greedily captured the next ~28 bytes of the schema
            // record as "body". The L5B-09.1/.2 probes found those
            // bytes were actually the next field's header spilling
            // into this one's payload. Empty-body is the correct
            // wire semantics for scalar-base containers.
            k if is_vector_capable && sub == 0x0050 => FieldType::Container {
                kind: k,
                cpp_signature: None,
                body: Vec::new(),
            },
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
/// `decompressed` must be the page-stripped stream, as
/// [`crate::compression::inflate_stream_at`] returns it (#410). The whole
/// stream is scanned.
pub fn parse_schema(decompressed: &[u8]) -> Result<SchemaTable> {
    parse_schema_with_scan_limit(decompressed, usize::MAX)
}

/// [`parse_schema`] scanning at most `scan_limit` bytes (#410 measures the
/// whole page-stripped stream with `usize::MAX`).
pub fn parse_schema_with_scan_limit(decompressed: &[u8], scan_limit: usize) -> Result<SchemaTable> {
    let mut classes = Vec::new();
    let mut cpp_types = std::collections::BTreeSet::new();
    let mut skipped = 0usize;

    // Schema section is in the early portion of the stream. Scanning
    // beyond this produces false-positive class records from compressed
    // binary noise.
    let scan_truncated = decompressed.len() > scan_limit;
    let data = if scan_truncated {
        &decompressed[..scan_limit]
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
        scanned_bytes: data.len(),
        total_bytes: decompressed.len(),
        scan_truncated,
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

/// One class of the `Formats/Latest` schema as the grammar reads it (#154).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaClass {
    /// The class's serialization tag: its definition ordinal, 12 for the
    /// first class and one more for each further definition, top-level or
    /// inline. Element records name their class by it (+0x4a, #223).
    pub tag: u16,
    pub name: String,
    /// The tag of the class it derives from, if any.
    pub base: Option<u16>,
    pub version: u32,
    pub field_count: u32,
}

/// Every class of a page-stripped `Formats/Latest` stream, in definition
/// order (#154).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaClasses {
    pub classes: Vec<SchemaClass>,
    /// Bytes read before the stream ended or a record broke the grammar.
    pub parsed_bytes: usize,
    /// Why reading stopped early, `None` when it reached the end.
    pub stopped: Option<String>,
}

impl SchemaClasses {
    /// The class with serialization tag `tag`.
    pub fn by_tag(&self, tag: u16) -> Option<&SchemaClass> {
        let first = self.classes.first()?.tag;
        let class = self.classes.get(usize::from(tag.checked_sub(first)?))?;
        (class.tag == tag).then_some(class)
    }
}

/// Read the class records of a page-stripped `Formats/Latest` stream (#154).
///
/// The grammar (STE1200, Discussion #112, measured against 2014 to 2026
/// schemas): records are separated by a zero u16; a class is a u16-counted
/// name, a u16 base reference (bit 15 set: the base class is defined inline
/// right after a zero u16), a u32 version, a u32 field count, the fields
/// (u32-counted name and a type), and a u32-counted list of 16-byte GUIDs. A
/// type is a u32 kind word (kind byte, modifier byte): modifier high nibble
/// 0x10 adds a u32 array count; kind 0x0d adds `01 00 00 00 20` and an inner
/// type, and a class reference when that inner type has one; kind 0x0e with
/// a by-value modifier adds a class reference. A class reference is a u16,
/// bit 15 set meaning the class is defined inline after a zero u16.
///
/// The tag is not stored: it is the definition ordinal. Every inline
/// reference repeats its tag in its low 15 bits, which the reader checks, so
/// a misread stops the parse instead of shifting every later tag.
pub fn schema_classes(data: &[u8]) -> SchemaClasses {
    let mut reader = GrammarReader {
        data,
        next_tag: FIRST_SCHEMA_TAG,
        classes: Vec::new(),
    };
    let mut at = 0;
    let stopped = loop {
        if at + 2 > data.len() {
            break None;
        }
        if reader.u16(at) != Some(0) {
            break Some(format!("record separator is not 0 at {at:#x}"));
        }
        at += 2;
        if at >= data.len() || data[at..].iter().all(|&b| b == 0) {
            at = data.len();
            break None;
        }
        let (tag, count) = (reader.next_tag, reader.classes.len());
        match reader.class(at, 0) {
            Ok((_, next)) => at = next,
            Err(reason) => {
                reader.classes.truncate(count);
                reader.next_tag = tag;
                break Some(reason);
            }
        }
    };
    reader.classes.sort_by_key(|class| class.tag);
    SchemaClasses {
        classes: reader.classes,
        parsed_bytes: at,
        stopped,
    }
}

const FIRST_SCHEMA_TAG: u16 = 12;
/// Inline definitions nest; real schemas stay a few levels deep.
const MAX_SCHEMA_NESTING: usize = 64;

struct GrammarReader<'a> {
    data: &'a [u8],
    next_tag: u16,
    classes: Vec<SchemaClass>,
}

impl GrammarReader<'_> {
    fn u16(&self, at: usize) -> Option<u16> {
        let bytes = self.data.get(at..at.checked_add(2)?)?;
        Some(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&self, at: usize) -> Option<u32> {
        let bytes = self.data.get(at..at.checked_add(4)?)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn need<T>(value: Option<T>, what: &str, at: usize) -> std::result::Result<T, String> {
        value.ok_or_else(|| format!("{what} past the end at {at:#x}"))
    }

    fn name(&self, at: usize, width: usize) -> std::result::Result<(String, usize), String> {
        let len = if width == 2 {
            usize::from(Self::need(self.u16(at), "name length", at)?)
        } else {
            Self::need(self.u32(at), "name length", at)? as usize
        };
        let start = at + width;
        let raw = (1..=250)
            .contains(&len)
            .then(|| self.data.get(start..start + len))
            .flatten()
            .ok_or_else(|| format!("bad name length {len} at {at:#x}"))?;
        if !raw.iter().all(|b| (0x20..0x7f).contains(b)) {
            return Err(format!("name is not ASCII at {at:#x}"));
        }
        Ok((String::from_utf8_lossy(raw).into_owned(), start + len))
    }

    fn inline_class(
        &mut self,
        at: usize,
        reference: u16,
        depth: usize,
    ) -> std::result::Result<(u16, usize), String> {
        if self.u16(at) != Some(0) {
            return Err(format!("inline pad is not 0 at {at:#x}"));
        }
        let (tag, next) = self.class(at + 2, depth + 1)?;
        if tag != reference & 0x7fff {
            return Err(format!(
                "inline tag {tag} does not match reference {} at {at:#x}",
                reference & 0x7fff
            ));
        }
        Ok((tag, next))
    }

    fn class_ref(&mut self, at: usize, depth: usize) -> std::result::Result<(bool, usize), String> {
        let reference = Self::need(self.u16(at), "class reference", at)?;
        if reference & 0x8000 == 0 {
            return Ok((true, at + 2));
        }
        let (_, next) = self.inline_class(at + 2, reference, depth)?;
        Ok((true, next))
    }

    /// Returns whether the type carries a class reference, and the offset
    /// after it.
    fn field_type(
        &mut self,
        at: usize,
        depth: usize,
    ) -> std::result::Result<(bool, usize), String> {
        if depth > MAX_SCHEMA_NESTING {
            return Err(format!("types nest too deep at {at:#x}"));
        }
        let word = Self::need(self.u32(at), "type word", at)?;
        let mut at = at + 4;
        let (kind, modifier) = ((word & 0xff) as u8, ((word >> 8) & 0xff) as u8);
        if !matches!(modifier & 0xf0, 0x00 | 0x10 | 0x50 | 0x60) {
            return Err(format!("unknown modifier {modifier:#04x} at {:#x}", at - 4));
        }
        if modifier & 0xf0 == 0x10 {
            let count = Self::need(self.u32(at), "array count", at)?;
            if count > 65_536 {
                return Err(format!("array count {count} at {at:#x}"));
            }
            at += 4;
        }
        if kind == 0x0d {
            if self.data.get(at..at + 5) != Some(&[1, 0, 0, 0, 0x20][..]) {
                return Err(format!("array filler at {at:#x}"));
            }
            let (inner_ref, next) = self.field_type(at + 5, depth + 1)?;
            at = next;
            if inner_ref {
                return self.class_ref(at, depth);
            }
            return Ok((false, at));
        }
        if kind == 0x0e && modifier & 0x0f == 0 {
            return self.class_ref(at, depth);
        }
        Ok((false, at))
    }

    fn class(&mut self, at: usize, depth: usize) -> std::result::Result<(u16, usize), String> {
        if depth > MAX_SCHEMA_NESTING {
            return Err(format!("classes nest too deep at {at:#x}"));
        }
        let (name, mut p) = self.name(at, 2)?;
        let tag = self.next_tag;
        self.next_tag = tag
            .checked_add(1)
            .filter(|next| *next < 0x8000)
            .ok_or_else(|| format!("too many classes at {at:#x}"))?;
        let reference = Self::need(self.u16(p), "base reference", p)?;
        p += 2;
        let base = if reference & 0x8000 != 0 {
            let (base, next) = self.inline_class(p, reference, depth)?;
            p = next;
            Some(base)
        } else {
            (reference != 0).then_some(reference)
        };
        let version = Self::need(self.u32(p), "version", p)?;
        let field_count = Self::need(self.u32(p + 4), "field count", p + 4)?;
        p += 8;
        if field_count > 2_000 {
            return Err(format!("field count {field_count} at {:#x}", p - 4));
        }
        for _ in 0..field_count {
            let (_, next) = self.name(p, 4)?;
            let (_, next) = self.field_type(next, depth)?;
            p = next;
        }
        let guids = Self::need(self.u32(p), "GUID count", p)? as usize;
        p += 4;
        p = guids
            .checked_mul(16)
            .and_then(|len| p.checked_add(len))
            .filter(|end| *end <= self.data.len() && guids <= 60_000)
            .ok_or_else(|| format!("GUID count {guids} at {:#x}", p - 4))?;
        self.classes.push(SchemaClass {
            tag,
            name,
            base,
            version,
            field_count,
        });
        Ok((tag, p))
    }
}
