//! Phase D v0 · Object graph deserializer — starter pieces.
//!
//! This module contains partial parsers for specific record types we've
//! successfully reverse-engineered from the decompressed `Global/Latest`
//! and `Partitions/NN` streams. It grows incrementally as more classes
//! are mapped; the `rvt-rs` v0.1 ships only the pieces we can verify
//! byte-for-byte against the 11-version reference corpus.
//!
//! # Currently supported
//!
//! - `DocumentHistory` — the sequence of Revit version strings recorded
//!   every time this file has been opened and saved in a different Revit
//!   release. Lives near the top of decompressed `Global/Latest`. Format:
//!
//!   ```text
//!   [header ~0x48 bytes: size + sentinels + counts]
//!   repeated:
//!     [u32 LE unknown_tag]     // often 0x00000007
//!     [u32 LE char_length]     // number of UTF-16 code units + separator
//!     [UTF-16LE bytes]         // the version string
//!   ```
//!
//!   Strings look like `"Revit 2024  20230308_1635(x64)"` or
//!   `"Revit 2018 - Preview Pre-Release 2018 (2018.000) : 20170106_1515(x64)/"`.

use crate::{compression, reader::RevitFile, streams::GLOBAL_LATEST, Result};
use encoding_rs::UTF_16LE;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentHistory {
    /// Ordered list of Revit version strings this file has seen, oldest first.
    pub entries: Vec<String>,
    /// Offset in the decompressed Global/Latest stream where the strings begin.
    pub string_section_offset: usize,
}

impl DocumentHistory {
    /// Extract document history from a Revit file.
    pub fn from_revit_file(rf: &mut RevitFile) -> Result<Self> {
        let bytes = rf.read_stream(GLOBAL_LATEST)?;
        // Global/Latest has a custom 8-byte header followed by gzip at offset 8
        let decompressed = compression::inflate_at(&bytes, 8)?;
        Self::from_decompressed(&decompressed)
    }

    /// Extract from already-decompressed Global/Latest bytes.
    pub fn from_decompressed(decomp: &[u8]) -> Result<Self> {
        // Scan for every UTF-16LE occurrence of "Revit " and decode the
        // string that follows. Each entry lives near a length prefix but
        // the framing between entries isn't perfectly consistent across
        // versions, so we're resilient: find each marker, read until
        // terminator (null or non-printable), dedupe.
        const PROBE: [u8; 12] = [b'R', 0, b'e', 0, b'v', 0, b'i', 0, b't', 0, b' ', 0];
        let mut entries = Vec::new();
        let first = decomp.windows(PROBE.len()).position(|w| w == PROBE).ok_or_else(|| {
            crate::Error::Decompress(
                "no 'Revit ' UTF-16LE marker found in decompressed Global/Latest".into(),
            )
        })?;
        let string_section_offset = first.saturating_sub(8);

        let mut scan = 0usize;
        while scan + PROBE.len() <= decomp.len() {
            let idx = match decomp[scan..]
                .windows(PROBE.len())
                .position(|w| w == PROBE)
            {
                Some(p) => scan + p,
                None => break,
            };
            // Read UTF-16LE until we hit a null code unit (00 00) or 256 chars.
            let mut end = idx;
            let max_end = (idx + 512).min(decomp.len());
            while end + 2 <= max_end {
                let c = u16::from_le_bytes([decomp[end], decomp[end + 1]]);
                if c == 0 {
                    break;
                }
                // Accept typical string chars only — stop on control bytes.
                if c < 0x20 && c != b' ' as u16 {
                    break;
                }
                end += 2;
            }
            if end > idx {
                let (text, _, _) = UTF_16LE.decode(&decomp[idx..end]);
                let s = text.trim_end_matches(&['\0', '/', ' '] as &[_]).to_string();
                if s.starts_with("Revit ") && !entries.contains(&s) {
                    entries.push(s);
                }
            }
            scan = end.max(idx + 1);
        }

        Ok(Self { entries, string_section_offset })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_input() {
        let res = DocumentHistory::from_decompressed(&[]);
        assert!(res.is_err(), "empty input should fail");
    }

    #[test]
    fn parses_contrived_single_entry() {
        // The scanner looks for UTF-16LE "Revit " and reads until NULL.
        let version_str = "Revit 2024  20230308_1635(x64)";
        let utf16: Vec<u8> = version_str.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();

        let mut buf = Vec::new();
        buf.extend(std::iter::repeat(0u8).take(64)); // header padding
        buf.extend_from_slice(&7u32.to_le_bytes()); // tag
        buf.extend_from_slice(&(version_str.chars().count() as u32).to_le_bytes()); // length
        buf.extend_from_slice(&utf16);
        buf.extend_from_slice(&[0u8, 0u8]); // null terminator

        let history = DocumentHistory::from_decompressed(&buf).unwrap();
        assert_eq!(history.entries.len(), 1);
        assert!(
            history.entries[0].starts_with("Revit 2024"),
            "got: {:?}",
            history.entries[0]
        );
    }

    #[test]
    fn parses_multiple_entries() {
        fn push_entry(buf: &mut Vec<u8>, version: &str) {
            let utf16: Vec<u8> = version.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
            buf.extend_from_slice(&7u32.to_le_bytes());
            buf.extend_from_slice(&(version.chars().count() as u32).to_le_bytes());
            buf.extend_from_slice(&utf16);
            buf.extend_from_slice(&[0u8, 0u8]);
            // Some garbage between entries
            buf.extend_from_slice(&[0xff, 0xff, 0xab, 0xcd]);
        }
        let mut buf = Vec::new();
        buf.extend(std::iter::repeat(0u8).take(64));
        push_entry(&mut buf, "Revit 2018 (Build X)");
        push_entry(&mut buf, "Revit 2024 (Build Y)");

        let history = DocumentHistory::from_decompressed(&buf).unwrap();
        assert_eq!(history.entries.len(), 2);
        assert!(history.entries[0].starts_with("Revit 2018"));
        assert!(history.entries[1].starts_with("Revit 2024"));
    }
}
