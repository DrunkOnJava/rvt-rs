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
/// 2026-04-19 Phase 4c.2 sweep — see the §Q5 addendum in
/// `docs/rvt-moat-break-reconnaissance.md` for the evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FieldType {
    /// `0x04` prefix. Fixed-size primitive (integer-like). The tail
    /// bytes encode size / default / flags — not fully decoded yet.
    Primitive { size_hint: u32 },
    /// `0x0e 0x00 0x00 0x00 0x14 0x00` — 6-byte pattern for ElementId.
    ElementId,
    /// `0x0e 0xNN 0x00 0x00` where NN ∈ {0x01, 0x02, 0x03} — a pointer
    /// or singular reference to another class instance. The low byte
    /// marks the reference-kind (e.g. pointer vs. non-owning ref).
    Pointer { kind: u8 },
    /// `0x0e 0x10 0x00 0x00 ...` — a vector/array. Body (not fully
    /// decoded) contains an element-count hint and a reference to
    /// the element class's tag.
    Vector {
        /// Raw body bytes after the 4-byte header.
        body: Vec<u8>,
    },
    /// `0x0e 0x50 0x00 0x00 ...` — a map / set. Body typically embeds
    /// a class-tag reference AND an ASCII C++ type signature
    /// (`std::pair< K, V >`, `std::map< K, V >`).
    Container {
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
        match bytes[0] {
            0x04 => {
                // Numeric primitive. The next u32 is the size hint.
                if bytes.len() >= 4 {
                    let size = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
                    FieldType::Primitive { size_hint: size }
                } else {
                    FieldType::Unknown { bytes: bytes.to_vec() }
                }
            }
            0x0e if bytes.len() >= 4 => {
                let sub = u16::from_le_bytes([bytes[1], bytes[2]]);
                match sub {
                    0x0000 if bytes.len() >= 6 && bytes[4] == 0x14 && bytes[5] == 0x00 => {
                        FieldType::ElementId
                    }
                    0x0001 | 0x0002 | 0x0003 => FieldType::Pointer { kind: bytes[1] },
                    0x0010 => FieldType::Vector {
                        body: bytes[4..].to_vec(),
                    },
                    0x0050 => {
                        // Extract any embedded ASCII C++ signature
                        let body = &bytes[4..];
                        let mut cpp_signature = None;
                        let mut k = 0;
                        while k + 2 < body.len() {
                            let slen = u16::from_le_bytes([body[k], body[k + 1]]) as usize;
                            if (3..=120).contains(&slen) && k + 2 + slen <= body.len() {
                                let sig = &body[k + 2..k + 2 + slen];
                                if sig.iter().all(|b| b.is_ascii_graphic() || *b == b' ')
                                    && sig.iter().any(|b| *b == b':' || *b == b'<')
                                {
                                    cpp_signature = Some(
                                        std::str::from_utf8(sig).unwrap_or("").to_string(),
                                    );
                                    break;
                                }
                            }
                            k += 1;
                        }
                        FieldType::Container {
                            cpp_signature,
                            body: bytes[4..].to_vec(),
                        }
                    }
                    _ => FieldType::Unknown { bytes: bytes.to_vec() },
                }
            }
            _ => FieldType::Unknown { bytes: bytes.to_vec() },
        }
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
                                let flag = u16::from_le_bytes([
                                    data[preamble_at],
                                    data[preamble_at + 1],
                                ]);
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
                                    parent = Some(
                                        std::str::from_utf8(p).unwrap().to_string(),
                                    );
                                    declared_field_count = Some(fc);
                                    cursor = preamble_at + 10;
                                }
                            }
                        }
                    }
                }
            }
        }

        // Walk forward until we find the next class-name candidate OR end
        // of stream. While walking, pick up any embedded field records.
        let mut fields = Vec::new();
        let (next_class_offset, found_fields) =
            scan_fields_until_next_class(data, cursor, &mut cpp_types);
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
            });
        }
        i = cursor.max(i + 1);
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
    bytes[1..].iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
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
    bytes.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_')
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
            || matches!(c, ':' | '<' | '>' | ',' | ' ' | '_' | '*' | '&' | '[' | ']' | '(' | ')')
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
fn scan_fields_until_next_class(
    data: &[u8],
    start: usize,
    cpp_types: &mut std::collections::BTreeSet<String>,
) -> (usize, Vec<FieldEntry>) {
    let mut fields = Vec::new();
    let mut i = start;
    let hard_stop = (start + 4096).min(data.len());

    while i + 4 < hard_stop {
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
                    if (3..=120).contains(&type_len) && consumed + prefix_len + type_len <= hard_stop {
                        let type_slice = &data[consumed + prefix_len..consumed + prefix_len + type_len];
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

                fields.push(FieldEntry {
                    name: field_name,
                    cpp_type,
                    field_type,
                });
                i = if type_consumed_bytes > 0 { consumed + type_consumed_bytes } else { post_name };
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
        assert!(!looks_like_cpp_type(b"m_id"));   // lowercase only = field name territory
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
        buf.extend_from_slice(&[0x0d, 0x00]);     // u16 len=13
        buf.extend_from_slice(b"ACDPtrWrapper");  // 13 ASCII bytes
        buf.extend_from_slice(&[0x00, 0x00]);     // class tag
        buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // field count u32
        buf.extend_from_slice(&[0x01, 0x00, 0x00, 0x00]); // index u32
        buf.extend_from_slice(&[0x06, 0x00, 0x00, 0x00]); // field name len u32
        buf.extend_from_slice(b"m_pACD");        // 6 ASCII bytes

        let schema = parse_schema(&buf).unwrap();
        assert!(
            schema.classes.iter().any(|c| c.name == "ACDPtrWrapper"),
            "expected class ACDPtrWrapper, got {:?}",
            schema.classes.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        let class = schema.classes.iter().find(|c| c.name == "ACDPtrWrapper").unwrap();
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
    fn decodes_field_type_primitive() {
        // 4-byte pattern for int-like primitive: 04 00 00 00
        let bytes = [0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let ft = FieldType::decode(&bytes);
        assert!(matches!(ft, FieldType::Primitive { size_hint: 4 }));
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
            FieldType::Container { cpp_signature, .. } => {
                assert_eq!(cpp_signature.as_deref(), Some("std::pair< int, X >"));
            }
            other => panic!("expected Container, got {other:?}"),
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
        buf.extend_from_slice(&[0x6b, 0x80]);        // tag 0x006b with 0x8000 flag
        buf.extend_from_slice(&[0x00, 0x00]);        // pad
        buf.extend_from_slice(&[0x06, 0x00]);        // parent name len = 6
        buf.extend_from_slice(b"Symbol");            // parent
        buf.extend_from_slice(&[0x25, 0x00]);        // flag
        buf.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]); // field count = 3
        buf.extend_from_slice(&[0x03, 0x00, 0x00, 0x00]); // duplicate
        buf.extend_from_slice(&[0x0c, 0x00, 0x00, 0x00]); // field 1 name len = 12
        buf.extend_from_slice(b"m_symbolInfo");      // field 1 name
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
}
