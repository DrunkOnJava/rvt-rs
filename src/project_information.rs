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

fn u16_at(bytes: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(bytes.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn malformed(what: &str) -> Error {
    Error::Decompress(format!("ProjectInformation: {what}"))
}

/// Decompressed body of the archive's first local file entry.
fn first_entry(bytes: &[u8], max_inflated: usize) -> Result<Vec<u8>> {
    if u32_at(bytes, 0) != Some(LOCAL_FILE_HEADER) {
        return Err(malformed("not a ZIP archive"));
    }
    let method = u16_at(bytes, 8).ok_or_else(|| malformed("truncated local header"))?;
    let compressed = u32_at(bytes, 18).ok_or_else(|| malformed("truncated local header"))? as usize;
    let name_len = u16_at(bytes, 26).ok_or_else(|| malformed("truncated local header"))? as usize;
    let extra_len = u16_at(bytes, 28).ok_or_else(|| malformed("truncated local header"))? as usize;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const ATOM: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<entry xmlns="http://www.w3.org/2005/Atom" xmlns:A="urn:schemas-autodesk-com:partatom"><title>Tower</title><updated>2023-09-07T12:50:35Z</updated><A:taxonomy><term>adsk:revit</term><label>Autodesk Revit</label></A:taxonomy><A:features><A:feature><A:title>Project Information</A:title><A:group><A:title>Identity Data</A:title></A:group><A:group><A:title>Other</A:title></A:group></A:feature></A:features></entry>"#;

    fn zip_with(method: u16, body: &[u8], stored_size: u32) -> Vec<u8> {
        let name = b"tmp\\Revit0000.project.xml";
        let mut out = Vec::new();
        out.extend_from_slice(&LOCAL_FILE_HEADER.to_le_bytes());
        out.extend_from_slice(&20u16.to_le_bytes()); // version needed
        out.extend_from_slice(&2u16.to_le_bytes()); // flags
        out.extend_from_slice(&method.to_le_bytes());
        out.extend_from_slice(&[0; 8]); // time, date, crc32
        out.extend_from_slice(&stored_size.to_le_bytes());
        out.extend_from_slice(&(ATOM.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(body);
        out.extend_from_slice(b"PK\x05\x06"); // trailing directory, ignored
        out
    }

    fn deflated() -> Vec<u8> {
        let mut enc =
            flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(ATOM.as_bytes()).unwrap();
        enc.finish().unwrap()
    }

    #[test]
    fn parses_deflated_entry() {
        let body = deflated();
        let atom = parse(&zip_with(METHOD_DEFLATE, &body, body.len() as u32), 1 << 20).unwrap();
        // The entry title is the document name; the nested `A:title`
        // parameter-group names must not replace it.
        assert_eq!(atom.entry_title.as_deref(), Some("Tower"));
        assert_eq!(atom.title.as_deref(), Some("Other"));
        assert_eq!(atom.updated.as_deref(), Some("2023-09-07T12:50:35Z"));
    }

    #[test]
    fn deflated_entry_without_sizes_still_parses() {
        let atom = parse(&zip_with(METHOD_DEFLATE, &deflated(), 0), 1 << 20).unwrap();
        assert_eq!(atom.entry_title.as_deref(), Some("Tower"));
    }

    #[test]
    fn parses_stored_entry() {
        let atom = parse(
            &zip_with(METHOD_STORED, ATOM.as_bytes(), ATOM.len() as u32),
            1 << 20,
        )
        .unwrap();
        assert_eq!(atom.updated.as_deref(), Some("2023-09-07T12:50:35Z"));
    }

    #[test]
    fn enforces_the_inflate_limit() {
        let body = deflated();
        let err = parse(&zip_with(METHOD_DEFLATE, &body, body.len() as u32), 16).unwrap_err();
        assert!(matches!(err, Error::DecompressLimitExceeded(_)), "{err}");
    }

    #[test]
    fn rejects_malformed_input_without_panicking() {
        assert!(parse(b"", 1024).is_err());
        assert!(parse(b"PK\x03\x04", 1024).is_err());
        assert!(parse(b"not a zip at all", 1024).is_err());
        let mut truncated = zip_with(METHOD_STORED, ATOM.as_bytes(), ATOM.len() as u32);
        truncated.truncate(40);
        assert!(parse(&truncated, 1024).is_err());
        assert!(parse(&zip_with(12, b"bzip2", 5), 1024).is_err());
    }
}
