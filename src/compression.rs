//! Truncated-gzip decompression for Revit streams.
//!
//! Revit writes standard gzip file headers (magic `1F 8B 08` + 10-byte
//! minimum header) but omits the trailing 8-byte CRC32 + ISIZE that
//! RFC 1952 requires. `flate2::read::GzDecoder` validates those trailing
//! bytes and refuses truncated streams, so we skip the gzip header manually
//! and pump the raw DEFLATE body through `flate2::read::DeflateDecoder`.

use crate::{Error, Result};
use flate2::read::DeflateDecoder;
use std::io::Read;

pub const GZIP_MAGIC: [u8; 3] = [0x1F, 0x8B, 0x08];

/// Returns `true` iff `data` starts with the gzip magic at the given offset.
pub fn has_gzip_magic(data: &[u8], offset: usize) -> bool {
    data.get(offset..offset + 3) == Some(GZIP_MAGIC.as_slice())
}

/// Length of the gzip header starting at `offset`, or `None` if no magic.
///
/// Standard gzip: magic(3) + method(1) + flags(1) + mtime(4) + xfl(1) + os(1) = 10 bytes.
/// Plus optional FEXTRA / FNAME / FCOMMENT / FHCRC fields when flags are set.
pub fn gzip_header_len(data: &[u8], offset: usize) -> Option<usize> {
    if !has_gzip_magic(data, offset) {
        return None;
    }
    let flags = *data.get(offset + 3)?;
    let mut pos = offset + 10;
    if flags & 0x04 != 0 {
        // FEXTRA: 2-byte LE length, then that many bytes
        let xlen = u16::from_le_bytes([*data.get(pos)?, *data.get(pos + 1)?]) as usize;
        pos += 2 + xlen;
    }
    if flags & 0x08 != 0 {
        // FNAME: null-terminated string
        pos = data[pos..].iter().position(|&b| b == 0).map(|i| pos + i + 1)?;
    }
    if flags & 0x10 != 0 {
        // FCOMMENT: null-terminated string
        pos = data[pos..].iter().position(|&b| b == 0).map(|i| pos + i + 1)?;
    }
    if flags & 0x02 != 0 {
        // FHCRC: 2-byte header CRC
        pos += 2;
    }
    Some(pos - offset)
}

/// Inflate the DEFLATE stream that follows a gzip header starting at `offset`.
///
/// Returns the decompressed bytes. Unused tail (any garbage / next chunk /
/// missing CRC+ISIZE) is silently ignored.
pub fn inflate_at(data: &[u8], offset: usize) -> Result<Vec<u8>> {
    let header_len =
        gzip_header_len(data, offset).ok_or_else(|| Error::Decompress("no gzip header".into()))?;
    let body = &data[offset + header_len..];
    let mut out = Vec::with_capacity(body.len() * 4);
    DeflateDecoder::new(body)
        .read_to_end(&mut out)
        .map_err(|e| Error::Decompress(format!("DEFLATE at offset {offset}: {e}")))?;
    Ok(out)
}

/// Find every gzip magic byte-triple in `data`.
/// Used for streams like `Partitions/NN` that pack multiple GZIP segments.
pub fn find_gzip_offsets(data: &[u8]) -> Vec<usize> {
    let mut hits = Vec::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        if data[i..i + 3] == GZIP_MAGIC {
            hits.push(i);
            i += 3;
        } else {
            i += 1;
        }
    }
    hits
}

/// Inflate every GZIP chunk in `data`, concatenating the outputs.
/// Silently skips chunks that fail to inflate (some trailing "magic-like"
/// byte triples in random compressed data are false positives).
pub fn inflate_all_chunks(data: &[u8]) -> Vec<Vec<u8>> {
    find_gzip_offsets(data)
        .into_iter()
        .filter_map(|off| inflate_at(data, off).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_magic() {
        assert!(has_gzip_magic(&[0x1F, 0x8B, 0x08, 0, 0, 0], 0));
        assert!(!has_gzip_magic(&[0x1F, 0x8B, 0x07], 0));
        assert!(has_gzip_magic(&[0xFF, 0x1F, 0x8B, 0x08], 1));
    }

    #[test]
    fn minimal_gzip_header_len() {
        // magic(3) + method(1) + flags=0(1) + mtime(4) + xfl(1) + os(1) = 10
        let hdr = [0x1F, 0x8B, 0x08, 0x00, 0, 0, 0, 0, 0, 0x0B];
        assert_eq!(gzip_header_len(&hdr, 0), Some(10));
    }

    #[test]
    fn gzip_header_with_fname() {
        // flags = 0x08 (FNAME), followed by "foo\0" after 10-byte base
        let hdr = [
            0x1F, 0x8B, 0x08, 0x08, 0, 0, 0, 0, 0, 0x0B, b'f', b'o', b'o', 0,
        ];
        assert_eq!(gzip_header_len(&hdr, 0), Some(14));
    }
}
