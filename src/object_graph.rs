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

use crate::{Result, compression, reader::RevitFile, streams::GLOBAL_LATEST};
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
        // Family files prefix the gzip body with an 8-byte custom header;
        // `.rvt` project files observed in the wild sometimes have no
        // prefix and the magic sits at offset 0. `inflate_at_auto` picks
        // whichever the file actually has.
        let (_, decompressed) = compression::inflate_at_auto(&bytes)?;
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
        let first = decomp
            .windows(PROBE.len())
            .position(|w| w == PROBE)
            .ok_or_else(|| {
                crate::Error::Decompress(
                    "no 'Revit ' UTF-16LE marker found in decompressed Global/Latest".into(),
                )
            })?;
        let string_section_offset = first.saturating_sub(8);

        let mut scan = 0usize;
        while scan + PROBE.len() <= decomp.len() {
            let idx = match decomp[scan..].windows(PROBE.len()).position(|w| w == PROBE) {
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

        Ok(Self {
            entries,
            string_section_offset,
        })
    }
}

/// A length-prefixed UTF-16LE string record discovered in the decompressed
/// `Global/Latest` stream. These appear to be the canonical "string value"
/// record format in Revit's object graph — found as:
///
/// ```text
/// [u32 LE tag]  [u32 LE char_count]  [char_count * u16 LE]
/// ```
///
/// Tags observed in the 2024 reference file include:
/// - `0x0000_0007` — the original "Revit version" tag (first entry only)
/// - `0x0029_0034` (2,687,028) — later document-version string
/// - `0x0000_0001` — sheet / level / elevation identifiers
///   (examples: `"Level 1"`, `"A100"`, `"Elevation 0"`)
/// - `0xFFFF_FFFF` — tombstone / sentinel marker
/// - Many record-specific tags
///
/// The tag value is almost certainly the class-ID that binds the record
/// to an entry in `Formats/Latest`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StringRecord {
    pub offset: usize,
    pub tag: u32,
    pub value: String,
}

/// Extract every length-prefixed UTF-16LE string record from the decompressed
/// `Global/Latest` bytes. Returns records sorted by offset.
///
/// Heuristic: at every byte position, try to read `[u32 tag][u32 char_count]`
/// and validate that the following `2 * char_count` bytes decode as mostly
/// printable UTF-16LE. False-positive rate is low in practice because
/// binary bytes rarely satisfy both the length bound and the printable-text
/// check simultaneously.
pub fn extract_string_records(decomp: &[u8]) -> Vec<StringRecord> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 8 < decomp.len() {
        let tag = u32::from_le_bytes([decomp[i], decomp[i + 1], decomp[i + 2], decomp[i + 3]]);
        let cnt = u32::from_le_bytes([decomp[i + 4], decomp[i + 5], decomp[i + 6], decomp[i + 7]])
            as usize;
        if (1..=400).contains(&cnt) && i + 8 + 2 * cnt <= decomp.len() {
            let body = &decomp[i + 8..i + 8 + 2 * cnt];
            // decode UTF-16LE
            let (cow, _, _) = UTF_16LE.decode(body);
            let text = cow.into_owned();
            if !text.contains('\0') {
                let printable = text
                    .chars()
                    .filter(|c| c.is_ascii_graphic() || *c == ' ' || *c == '\t' || *c == '\n')
                    .count();
                // Require EVERY char to be printable (strict). The prior
                // 90%/integer-division check false-positived on length-1
                // records where 9/10 == 0, letting `\xFF\xFF` + a single
                // control character through and consuming the cursor past
                // real adjacent records like "Elevation 0", "A100", etc.
                let total_chars = text.chars().count();
                if total_chars >= 1 && printable == total_chars {
                    out.push(StringRecord {
                        offset: i,
                        tag,
                        value: text
                            .trim_end_matches(&['\0', '/', ' ', '\t'] as &[_])
                            .to_string(),
                    });
                    i += 8 + 2 * cnt;
                    continue;
                }
            }
        }
        i += 1;
    }
    out
}

/// Pull all string records from a Revit file's `Global/Latest` stream.
pub fn string_records_from_file(rf: &mut crate::RevitFile) -> Result<Vec<StringRecord>> {
    let bytes = rf.read_stream(GLOBAL_LATEST)?;
    let (_, decomp) = compression::inflate_at_auto(&bytes)?;
    Ok(extract_string_records(&decomp))
}

/// Pull string records from the version-specific `Partitions/NN` stream.
/// That stream contains 5-10 concatenated gzip chunks; we decompress all of
/// them and search across the concatenation. Partitions/NN is where the
/// bulk Revit content lives — category names, OmniClass/Uniformat codes,
/// Autodesk parameter-group + spec identifiers, localized format strings,
/// timestamps, asset-library references, etc.
pub fn string_records_from_partitions(rf: &mut crate::RevitFile) -> Result<Vec<StringRecord>> {
    let partition_name = rf
        .partition_stream_name()
        .ok_or_else(|| crate::Error::StreamNotFound("no Partitions/NN stream".into()))?;
    let bytes = rf.read_stream(&partition_name)?;
    // Concatenate all gzip chunks with an FF-sentinel separator so the
    // extractor sees clear boundaries between them (in case record scans
    // cross chunks accidentally).
    let chunks = compression::inflate_all_chunks(&bytes);
    let sep = [0xFFu8; 16];
    let mut joined = Vec::new();
    for (i, c) in chunks.iter().enumerate() {
        if i > 0 {
            joined.extend_from_slice(&sep);
        }
        joined.extend_from_slice(c);
    }
    Ok(extract_string_records(&joined))
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
        let utf16: Vec<u8> = version_str
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();

        let mut buf = Vec::new();
        buf.extend(std::iter::repeat_n(0u8, 64)); // header padding
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
            let utf16: Vec<u8> = version
                .encode_utf16()
                .flat_map(|c| c.to_le_bytes())
                .collect();
            buf.extend_from_slice(&7u32.to_le_bytes());
            buf.extend_from_slice(&(version.chars().count() as u32).to_le_bytes());
            buf.extend_from_slice(&utf16);
            buf.extend_from_slice(&[0u8, 0u8]);
            // Some garbage between entries
            buf.extend_from_slice(&[0xff, 0xff, 0xab, 0xcd]);
        }
        let mut buf = Vec::new();
        buf.extend(std::iter::repeat_n(0u8, 64));
        push_entry(&mut buf, "Revit 2018 (Build X)");
        push_entry(&mut buf, "Revit 2024 (Build Y)");

        let history = DocumentHistory::from_decompressed(&buf).unwrap();
        assert_eq!(history.entries.len(), 2);
        assert!(history.entries[0].starts_with("Revit 2018"));
        assert!(history.entries[1].starts_with("Revit 2024"));
    }
}
