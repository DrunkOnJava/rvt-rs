//! `TransmissionData` stream — encoding detection + opportunistic extracts.
//!
//! Project files carry a `TransmissionData` OLE stream with linked-model /
//! transmission metadata. This module:
//!
//! 1. **Detects** UTF-16LE vs opaque vs empty.
//! 2. When UTF-16LE, opportunistically extracts XML-ish structure, UUID-like
//!    tokens, and path-like strings for research triage.
//!
//! It does **not** decode Autodesk transmission schemas, resolve linked
//! models, or rewrite the stream. An empty extract list means **unknown /
//! not recovered**, not “this file has no links”.
//!
//! See `docs/compatibility.md` (no linked-model resolution) and the unified
//! research report non-goals.

use crate::RevitFile;
use crate::error::{Error, Result};
use crate::streams::TRANSMISSION_DATA;
use encoding_rs::UTF_16LE;
use serde::{Deserialize, Serialize};

/// Coarse encoding classification for a `TransmissionData` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransmissionEncoding {
    /// Stream missing or zero-length.
    Empty,
    /// Looks like UTF-16LE text (BOM and/or high printable decode ratio).
    Utf16Le,
    /// Bytes present but not confidently UTF-16LE — leave opaque.
    OpaqueBinary,
}

/// One opportunistic extract from a UTF-16LE payload (research triage).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransmissionExtract {
    /// UUID / GUID-shaped token (8-4-4-4-12 hex).
    Uuid { value: String },
    /// Path-like fragment (drive letter, UNC, or `.rvt`/`.rfa` suffix).
    Path { value: String },
    /// XML element local name observed while scanning tags.
    XmlNode { name: String },
}

/// Detect + opportunistic extract summary for `TransmissionData`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransmissionDataProbe {
    pub encoding: TransmissionEncoding,
    pub byte_len: usize,
    /// Lossy UTF-16LE preview (truncated) when encoding is [`TransmissionEncoding::Utf16Le`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_preview: Option<String>,
    /// Opportunistic extracts — **empty does not mean “no links”**.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extracts: Vec<TransmissionExtract>,
    /// True when the decoded text looks XML-ish (`<` … `>`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub looks_like_xml: bool,
    /// Explicit honesty notes — not a decode success claim.
    pub notes: Vec<String>,
}

impl TransmissionDataProbe {
    /// Classify raw stream bytes without inventing a field map.
    pub fn detect(bytes: &[u8]) -> Self {
        let byte_len = bytes.len();
        if byte_len == 0 {
            return Self {
                encoding: TransmissionEncoding::Empty,
                byte_len,
                text_preview: None,
                extracts: Vec::new(),
                looks_like_xml: false,
                notes: vec![
                    "TransmissionData empty or absent — linked-model metadata unknown (not proof of no links)."
                        .into(),
                ],
            };
        }

        let looks_utf16 = looks_like_utf16_le(bytes);
        if looks_utf16 {
            let (cow, _, _) = UTF_16LE.decode(bytes);
            let text = cow.into_owned();
            let preview: String = text.chars().take(240).collect();
            let looks_like_xml = text.contains('<') && text.contains('>');
            let extracts = opportunistic_extracts(&text);
            let mut notes = vec![
                "Detected UTF-16LE-shaped payload — opportunistic extracts only; no schema decode."
                    .into(),
                "Linked-model resolution remains unsupported.".into(),
                "Empty extract list ≠ “no links”; recovery may be incomplete.".into(),
            ];
            if looks_like_xml {
                notes.push(
                    "Payload looks XML-ish; node names are triage labels, not a typed model."
                        .into(),
                );
            }
            return Self {
                encoding: TransmissionEncoding::Utf16Le,
                byte_len,
                text_preview: Some(preview),
                extracts,
                looks_like_xml,
                notes,
            };
        }

        Self {
            encoding: TransmissionEncoding::OpaqueBinary,
            byte_len,
            text_preview: None,
            extracts: Vec::new(),
            looks_like_xml: false,
            notes: vec![
                "TransmissionData present but not confidently UTF-16LE; left opaque.".into(),
                "No transmission schema decoder is claimed.".into(),
                "Opaque payload ≠ proof of absence of links.".into(),
            ],
        }
    }
}

/// Heuristic: UTF-16LE BOM, or even length with many ASCII NUL-padded pairs.
fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    if bytes.len() < 4 {
        return false;
    }
    if bytes.starts_with(&[0xFF, 0xFE]) {
        return true;
    }
    if bytes.len() % 2 != 0 {
        return false;
    }
    // Count code units that look like printable ASCII as `XX 00`.
    let mut ascii_nul = 0usize;
    let mut units = 0usize;
    for chunk in bytes.chunks_exact(2) {
        units += 1;
        let lo = chunk[0];
        let hi = chunk[1];
        if hi == 0 && (lo == 0x09 || lo == 0x0A || lo == 0x0D || (0x20..=0x7E).contains(&lo)) {
            ascii_nul += 1;
        }
    }
    units > 0 && (ascii_nul * 100 / units) >= 60
}

fn opportunistic_extracts(text: &str) -> Vec<TransmissionExtract> {
    let mut out = Vec::new();
    extract_uuids(text, &mut out);
    extract_paths(text, &mut out);
    if text.contains('<') {
        extract_xml_node_names(text, &mut out);
    }
    // Dedup while preserving order.
    let mut seen = std::collections::BTreeSet::new();
    out.retain(|e| {
        let key = match e {
            TransmissionExtract::Uuid { value } => format!("u:{value}"),
            TransmissionExtract::Path { value } => format!("p:{value}"),
            TransmissionExtract::XmlNode { name } => format!("x:{name}"),
        };
        seen.insert(key)
    });
    out
}

fn extract_uuids(text: &str, out: &mut Vec<TransmissionExtract>) {
    // Scan for 8-4-4-4-12 hex with separators `-` or `{}` wrappers.
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut i = 0;
    while i + 36 <= n {
        if let Some(u) = try_uuid_at(text, i) {
            out.push(TransmissionExtract::Uuid { value: u });
            i += 36;
            continue;
        }
        i += 1;
    }
}

fn try_uuid_at(text: &str, start: usize) -> Option<String> {
    let slice = text.get(start..start + 36)?;
    let b = slice.as_bytes();
    // xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
    let dashes = [8usize, 13, 18, 23];
    for (idx, &ch) in b.iter().enumerate() {
        if dashes.contains(&idx) {
            if ch != b'-' {
                return None;
            }
        } else if !ch.is_ascii_hexdigit() {
            return None;
        }
    }
    Some(slice.to_ascii_lowercase())
}

fn extract_paths(text: &str, out: &mut Vec<TransmissionExtract>) {
    for token in text.split_whitespace() {
        let t = token.trim_matches(|c: char| {
            c == '"' || c == '\'' || c == '<' || c == '>' || c == ',' || c == ';'
        });
        if t.len() < 4 {
            continue;
        }
        let lower = t.to_ascii_lowercase();
        let looks = (t.len() >= 3 && t.as_bytes()[1] == b':' && t.as_bytes()[2] == b'\\')
            || t.starts_with("\\\\")
            || lower.ends_with(".rvt")
            || lower.ends_with(".rfa")
            || lower.ends_with(".rte")
            || lower.ends_with(".rft");
        if looks {
            out.push(TransmissionExtract::Path {
                value: t.chars().take(240).collect(),
            });
        }
    }
}

fn extract_xml_node_names(text: &str, out: &mut Vec<TransmissionExtract>) {
    // Lightweight tag-name harvest — not a validating XML parse / rewrite.
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'<' {
            let start = i + 1;
            if start >= bytes.len() {
                break;
            }
            // Skip declarations / comments / closing slashes' leading slash handled below.
            let first = bytes[start];
            if first == b'!' || first == b'?' {
                i += 1;
                continue;
            }
            let name_start = if first == b'/' { start + 1 } else { start };
            let mut name_end = name_start;
            while name_end < bytes.len() {
                let c = bytes[name_end];
                if c.is_ascii_alphanumeric() || c == b'_' || c == b':' || c == b'-' || c == b'.' {
                    name_end += 1;
                } else {
                    break;
                }
            }
            if name_end > name_start {
                if let Ok(name) = std::str::from_utf8(&bytes[name_start..name_end]) {
                    // Strip namespace prefix for triage: `a:b` → keep full token.
                    if !name.is_empty() {
                        out.push(TransmissionExtract::XmlNode {
                            name: name.to_string(),
                        });
                    }
                }
            }
            i = name_end;
            continue;
        }
        i += 1;
    }
}

/// Read and classify `TransmissionData` from an open file.
pub fn probe_transmission_data(rf: &mut RevitFile) -> Result<TransmissionDataProbe> {
    match rf.read_stream(TRANSMISSION_DATA) {
        Ok(bytes) => Ok(TransmissionDataProbe::detect(&bytes)),
        Err(Error::StreamNotFound(_)) => Ok(TransmissionDataProbe::detect(&[])),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16_le(s: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        for c in s.encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
        bytes
    }

    #[test]
    fn empty_is_empty_and_does_not_claim_no_links() {
        let p = TransmissionDataProbe::detect(&[]);
        assert_eq!(p.encoding, TransmissionEncoding::Empty);
        assert!(p.notes.iter().any(|n| n.contains("not proof of no links")));
    }

    #[test]
    fn utf16_le_ascii_pairs_detected() {
        let bytes = utf16_le("IsTransmitted");
        let p = TransmissionDataProbe::detect(&bytes);
        assert_eq!(p.encoding, TransmissionEncoding::Utf16Le);
        assert!(
            p.text_preview
                .as_deref()
                .unwrap_or("")
                .contains("IsTransmitted")
        );
    }

    #[test]
    fn utf16_le_bom_detected() {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend(utf16_le("hi"));
        let p = TransmissionDataProbe::detect(&bytes);
        assert_eq!(p.encoding, TransmissionEncoding::Utf16Le);
    }

    #[test]
    fn random_binary_is_opaque() {
        let bytes = [0x01, 0x02, 0x03, 0x04, 0x80, 0x90, 0xA0, 0xB0];
        let p = TransmissionDataProbe::detect(&bytes);
        assert_eq!(p.encoding, TransmissionEncoding::OpaqueBinary);
        assert!(p.text_preview.is_none());
    }

    #[test]
    fn extracts_uuid_path_and_xml_nodes() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>",
            "<TransmissionData>",
            "<ExternalFileReference>",
            "C:\\\\Models\\\\link.rvt ",
            "guid=aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee",
            "</ExternalFileReference>",
            "<UnknownCustomNode/>",
            "</TransmissionData>"
        );
        let p = TransmissionDataProbe::detect(&utf16_le(xml));
        assert!(p.looks_like_xml);
        assert!(p.extracts.iter().any(|e| matches!(
            e,
            TransmissionExtract::Uuid { value } if value == "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        )));
        assert!(p.extracts.iter().any(
            |e| matches!(e, TransmissionExtract::Path { value } if value.contains("link.rvt"))
        ));
        assert!(p.extracts.iter().any(|e| matches!(
            e,
            TransmissionExtract::XmlNode { name } if name == "TransmissionData"
        )));
        assert!(p.extracts.iter().any(|e| matches!(
            e,
            TransmissionExtract::XmlNode { name } if name == "UnknownCustomNode"
        )));
        assert!(p.notes.iter().any(|n| n.contains("Empty extract list")));
    }

    #[test]
    fn empty_extracts_on_plain_utf16_are_not_no_links() {
        let p = TransmissionDataProbe::detect(&utf16_le("IsTransmitted true"));
        assert!(p.extracts.is_empty());
        assert!(
            p.notes
                .iter()
                .any(|n| n.contains("≠") || n.contains("no links"))
        );
    }
}
