//! Partitions/NN stream — Revit's bulk BIM content.
//!
//! Each `.rfa` / `.rvt` file has exactly one `Partitions/NN` stream where
//! NN is the version-specific partition number (58 for 2016; 60-69 for
//! 2018-2026; 59 is skipped). Inside that stream is:
//!
//! 1. A header of ~44 bytes (exact size varies per release).
//! 2. 5–10 concatenated raw-gzip chunks, each decompressible independently.
//!
//! The bulk BIM data — category names, OmniClass / Uniformat codes,
//! Autodesk unit / spec / parameter-group identifiers, localized format
//! strings, asset-library references — lives in the concatenated
//! decompressed payload.
//!
//! This module currently exposes `chunks_from_stream` which finds gzip
//! magic markers and returns the chunk-byte-ranges. It does NOT yet parse
//! the header's explicit chunk table; that is a Phase 4d task. The
//! gzip-magic scanner is conservative enough to work across all 11
//! releases we have samples for.

use crate::{Result, RevitFile};

/// Size of the Partitions/NN header in bytes. Constant across all Revit
/// releases observed (2016–2026).
pub const HEADER_SIZE: usize = 44;

/// Structured view of the 44-byte Partitions/NN header. Field semantics
/// are partially reverse-engineered; see FACT F7/F8 in
/// `docs/rvt-moat-break-reconnaissance.md` for the evidence trail.
#[derive(Debug, Clone)]
pub struct PartitionHeader {
    /// First u32 LE — appears to equal `chunk_count + 1` in most releases.
    pub declared_count_plus_one: u32,
    /// Second u32 LE — observed to be 0 across every release we checked.
    pub reserved_zero: u32,
    /// Raw 12-byte block observed at offset 0x08..0x14. Encodes sizes
    /// that align with per-stream counts seen in Global/ElemTable.
    pub size_block: [u8; 12],
    /// Trailing 4 u32 fields at offsets 0x14..0x24 — likely offsets and
    //// or sizes related to individual chunk records.
    pub trailer_u32: [u32; 4],
}

/// A raw-gzip chunk located by magic-byte scan. Does NOT decompress; the
/// caller can pass the slice through `compression::inflate_at` (offset 0)
/// to get the decompressed payload.
#[derive(Debug, Clone)]
pub struct PartitionChunk {
    /// Starting offset of the chunk within the raw Partitions/NN stream.
    pub raw_offset: usize,
    /// Length in bytes (magic byte to the next magic byte or end of
    /// stream), including the gzip header.
    pub raw_len: usize,
}

/// Parse the fixed 44-byte Partitions/NN header into a structured view.
pub fn parse_header(raw: &[u8]) -> Option<PartitionHeader> {
    if raw.len() < HEADER_SIZE {
        return None;
    }
    let declared_count_plus_one = u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
    let reserved_zero = u32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
    let mut size_block = [0u8; 12];
    size_block.copy_from_slice(&raw[8..20]);
    let mut trailer_u32 = [0u32; 4];
    for (i, slot) in trailer_u32.iter_mut().enumerate() {
        let base = 0x14 + i * 4;
        *slot = u32::from_le_bytes([raw[base], raw[base + 1], raw[base + 2], raw[base + 3]]);
    }
    Some(PartitionHeader {
        declared_count_plus_one,
        reserved_zero,
        size_block,
        trailer_u32,
    })
}

/// Locate every gzip chunk inside the version-specific Partitions/NN
/// stream of the given Revit file.
pub fn chunks_from_stream(rf: &mut RevitFile) -> Result<Vec<PartitionChunk>> {
    let name = rf
        .partition_stream_name()
        .ok_or_else(|| crate::Error::StreamNotFound("no Partitions/NN stream".into()))?;
    let raw = rf.read_stream(&name)?;
    Ok(find_chunks(&raw))
}

/// Split a Partitions/NN byte slice into chunk ranges by scanning for gzip
/// magic `1F 8B 08`.
pub fn find_chunks(raw: &[u8]) -> Vec<PartitionChunk> {
    let mut positions: Vec<usize> = Vec::new();
    // `raw.len().saturating_sub(2)` gives the exclusive upper bound
    // such that `i + 2 <= raw.len() - 1`, i.e. the magic triplet at
    // the LAST valid starting position is still scanned. The prior
    // `saturating_sub(3)` missed offset raw.len()-3 entirely.
    for i in 0..raw.len().saturating_sub(2) {
        if raw[i] == 0x1f && raw[i + 1] == 0x8b && raw[i + 2] == 0x08 {
            positions.push(i);
        }
    }
    let mut chunks = Vec::with_capacity(positions.len());
    for (n, &start) in positions.iter().enumerate() {
        let end = positions.get(n + 1).copied().unwrap_or(raw.len());
        chunks.push(PartitionChunk {
            raw_offset: start,
            raw_len: end - start,
        });
    }
    chunks
}

/// Header-bytes preview: everything before the first gzip magic.
/// Reserved for when we decode the explicit chunk table.
pub fn header_bytes(raw: &[u8]) -> &[u8] {
    // Same off-by-one fix as find_chunks.
    for i in 0..raw.len().saturating_sub(2) {
        if raw[i] == 0x1f && raw[i + 1] == 0x8b && raw[i + 2] == 0x08 {
            return &raw[..i];
        }
    }
    raw
}

// Note: `partitions::stream_name()` was removed in v0.1.3. It had
// returned `streams::GLOBAL_LATEST` as a dummy, which was wrong — the
// actual Partitions/NN stream name is per-file. Callers should use
// `RevitFile::partition_stream_name()` instead.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_three_gzip_chunks() {
        let mut buf = Vec::<u8>::new();
        buf.extend_from_slice(&[0, 0, 0, 0]); // fake header
        buf.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00]);
        buf.extend_from_slice(&[0xaa; 10]);
        buf.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00]);
        buf.extend_from_slice(&[0xbb; 5]);
        buf.extend_from_slice(&[0x1f, 0x8b, 0x08, 0x00]);
        buf.extend_from_slice(&[0xcc; 3]);

        let chunks = find_chunks(&buf);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].raw_offset, 4);
        assert_eq!(chunks[0].raw_len, 14);
        assert_eq!(chunks[1].raw_offset, 18);
        assert_eq!(chunks[1].raw_len, 9);
        assert_eq!(chunks[2].raw_offset, 27);
    }

    #[test]
    fn header_bytes_returns_prefix() {
        let buf = [0u8, 0, 0, 0, 1, 2, 3, 0x1f, 0x8b, 0x08, 4, 5, 6];
        let h = header_bytes(&buf);
        assert_eq!(h, &[0u8, 0, 0, 0, 1, 2, 3]);
    }

    #[test]
    fn find_chunks_detects_magic_at_last_valid_offset() {
        // Regression for off-by-one: gzip magic at offset len-3 was
        // previously missed because the scan used saturating_sub(3)
        // as an exclusive upper bound instead of saturating_sub(2).
        let buf = [0u8, 0, 0, 0, 0x1f, 0x8b, 0x08]; // len=7, magic at offset 4 = len-3
        let chunks = find_chunks(&buf);
        assert_eq!(
            chunks.len(),
            1,
            "gzip magic at the last valid starting offset must be found"
        );
        assert_eq!(chunks[0].raw_offset, 4);
    }

    #[test]
    fn find_chunks_handles_tiny_inputs() {
        // <3 bytes cannot contain a gzip magic. Must return empty,
        // not panic.
        assert_eq!(find_chunks(&[]).len(), 0);
        assert_eq!(find_chunks(&[0x1f]).len(), 0);
        assert_eq!(find_chunks(&[0x1f, 0x8b]).len(), 0);
        // Exactly 3 bytes with magic IS valid.
        assert_eq!(find_chunks(&[0x1f, 0x8b, 0x08]).len(), 1);
    }

    #[test]
    fn parses_44_byte_header() {
        // Pattern lifted from Revit 2016 Partitions/58 header.
        // trailer_u32 covers offsets 0x14..0x24 (four u32 values).
        let mut h = vec![0u8; 44];
        h[0..4].copy_from_slice(&7u32.to_le_bytes()); // declared count
        h[0x14..0x18].copy_from_slice(&400u32.to_le_bytes());
        h[0x20..0x24].copy_from_slice(&131060u32.to_le_bytes()); // trailer[3]
        let parsed = parse_header(&h).unwrap();
        assert_eq!(parsed.declared_count_plus_one, 7);
        assert_eq!(parsed.reserved_zero, 0);
        assert_eq!(parsed.trailer_u32[0], 400);
        assert_eq!(parsed.trailer_u32[3], 131060);
    }
}
