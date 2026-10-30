//! Parse the `ProjectInformation` OLE stream — project files only.
//!
//! Families carry their Atom metadata as plain XML in `PartAtom`. Project
//! files instead carry a `ProjectInformation` stream that is a ZIP archive
//! (`PK\x03\x04`) with a single deflated `…project.xml` entry, whose body is
//! the same `urn:schemas-autodesk-com:partatom` Atom shape, so
//! [`PartAtom::from_bytes`] reads it. Observed on both magnetar project files
//! (Revit 2023, Revit 2024): title = the file's base name, `updated` = the
//! save timestamp, plus the product version and the Project Information
//! parameter groups.
//!
//! The entry's *name* is a temp path on the saving machine that embeds the
//! Windows user folder, so it is never surfaced — only the entry body is read.

use crate::le_bytes;
use crate::part_atom::PartAtom;
use crate::{Error, Result};
use std::io::Read;

const LOCAL_FILE_HEADER: u32 = 0x0403_4b50;
const METHOD_STORED: u16 = 0;
const METHOD_DEFLATE: u16 = 8;

/// Read the first entry of the `ProjectInformation` ZIP and parse it as an
/// Atom entry. `max_inflated` caps the decompressed entry size.
pub fn parse(bytes: &[u8], max_inflated: usize) -> Result<PartAtom> {
    let body = first_entry(bytes, max_inflated)?;
    PartAtom::from_bytes(&body)
}

fn malformed(what: &str) -> Error {
    Error::Decompress(format!("ProjectInformation: {what}"))
}

/// Decompressed body of the archive's first local file entry.
fn first_entry(bytes: &[u8], max_inflated: usize) -> Result<Vec<u8>> {
    if le_bytes::u32_at(bytes, 0) != Some(LOCAL_FILE_HEADER) {
        return Err(malformed("not a ZIP archive"));
    }
    let method = le_bytes::u16_at(bytes, 8).ok_or_else(|| malformed("truncated local header"))?;
    let compressed =
        le_bytes::u32_at(bytes, 18).ok_or_else(|| malformed("truncated local header"))? as usize;
    let name_len =
        le_bytes::u16_at(bytes, 26).ok_or_else(|| malformed("truncated local header"))? as usize;
    let extra_len =
        le_bytes::u16_at(bytes, 28).ok_or_else(|| malformed("truncated local header"))? as usize;
    let data = bytes
        .get(30 + name_len + extra_len..)
        .ok_or_else(|| malformed("entry data past end of stream"))?;
    match method {
        METHOD_STORED => {
            let body = data
                .get(..compressed)
                .ok_or_else(|| malformed("stored entry past end of stream"))?;
            if body.len() > max_inflated {
                return Err(Error::DecompressLimitExceeded(format!(
                    "ProjectInformation entry of {} bytes exceeds limit {max_inflated}",
                    body.len()
                )));
            }
            Ok(body.to_vec())
        }
        METHOD_DEFLATE => {
            // DEFLATE is self-terminating, so the compressed size (which a
            // streaming writer may leave as 0 behind a data descriptor) is
            // not needed to find the end of the entry.
            let mut out = Vec::new();
            flate2::read::DeflateDecoder::new(data)
                .take(max_inflated as u64 + 1)
                .read_to_end(&mut out)
                .map_err(|e| malformed(&format!("inflate failed: {e}")))?;
            if out.len() > max_inflated {
                return Err(Error::DecompressLimitExceeded(format!(
                    "ProjectInformation entry exceeds limit {max_inflated}"
                )));
            }
            Ok(out)
        }
        other => Err(malformed(&format!(
            "unsupported compression method {other}"
        ))),
    }
}
