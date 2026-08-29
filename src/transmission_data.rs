//! `TransmissionData` stream — encoding detection only (Phase 1 hygiene).
//!
//! Project files carry a `TransmissionData` OLE stream with linked-model /
//! transmission metadata. This module **detects** whether the payload looks
//! like UTF-16LE text suitable for further research. It does **not** decode
//! field layouts, link graphs, or Autodesk transmission schemas.
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

/// Detect-only summary for `TransmissionData`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransmissionDataProbe {
    pub encoding: TransmissionEncoding,
    pub byte_len: usize,
    /// Lossy UTF-16LE preview (truncated) when encoding is [`TransmissionEncoding::Utf16Le`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_preview: Option<String>,
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
                notes: vec![
                    "TransmissionData empty or absent — no linked-model metadata to inspect."
                        .into(),
                ],
            };
        }

        let looks_utf16 = looks_like_utf16_le(bytes);
        if looks_utf16 {
            let (cow, _, _) = UTF_16LE.decode(bytes);
            let preview: String = cow.chars().take(240).collect();
            return Self {
                encoding: TransmissionEncoding::Utf16Le,
                byte_len,
                text_preview: Some(preview),
                notes: vec![
                    "Detected UTF-16LE-shaped payload only — no field/layout decode.".into(),
                    "Linked-model resolution remains unsupported.".into(),
                ],
            };
        }

        Self {
            encoding: TransmissionEncoding::OpaqueBinary,
            byte_len,
            text_preview: None,
            notes: vec![
                "TransmissionData present but not confidently UTF-16LE; left opaque.".into(),
                "No transmission schema decoder is claimed.".into(),
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

    #[test]
    fn empty_is_empty() {
        let p = TransmissionDataProbe::detect(&[]);
        assert_eq!(p.encoding, TransmissionEncoding::Empty);
    }

    #[test]
    fn utf16_le_ascii_pairs_detected() {
        // "IsTransmitted" as UTF-16LE without BOM
        let mut bytes = Vec::new();
        for c in "IsTransmitted".encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
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
        for c in "hi".encode_utf16() {
            bytes.extend_from_slice(&c.to_le_bytes());
        }
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
}
