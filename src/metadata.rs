//! Document identity at a glance — the answer to "what is this file?".
//!
//! [`FileMetadata`] collects what a BIM manager asks of a Revit file before
//! opening it: which release saved it, whether it is workshared and where
//! its central model lives, who saved it last and when, and the document
//! identity. It comes from two small streams — `BasicFileInfo` (see
//! [`crate::basic_file_info`]) and the document Atom entry (`PartAtom` on
//! families, `ProjectInformation` on projects) — so [`read_metadata`] reads
//! only those streams from disk instead of the whole file, which is what
//! makes a folder inventory of multi-hundred-megabyte projects fast.

use crate::basic_file_info::{BasicFileInfo, FileProperty};
use crate::part_atom::PartAtom;
use crate::streams::{BASIC_FILE_INFO, PART_ATOM, PROJECT_INFORMATION};
use crate::{Error, Result, RevitFile};
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};
use std::path::Path;

/// Cap on each identity stream read by [`read_metadata`]. Observed
/// `BasicFileInfo` streams are ~2 KiB and Atom entries ~1 KiB; anything
/// near this is not a Revit identity stream.
const MAX_IDENTITY_STREAM_BYTES: u64 = 4 * 1024 * 1024;

/// Document identity recovered from `BasicFileInfo` and the Atom entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMetadata {
    /// Revit release that saved the file (e.g. `2024`).
    pub revit_version: u32,
    /// Build tag of that release, e.g. `20230509_0315(x64)`.
    pub build: Option<String>,
    /// Atom entry title — the file's base name as Revit recorded it.
    pub title: Option<String>,
    /// Atom `<updated>` timestamp (UTC, ISO 8601), written on save.
    pub last_saved: Option<String>,
    /// `Worksharing` verbatim: `Not enabled`, or the saved copy's role.
    pub worksharing: Option<String>,
    /// `false` for `Not enabled`, `true` for any other `Worksharing` value.
    pub workshared: Option<bool>,
    /// User recorded by the last save (empty on non-workshared files).
    pub username: Option<String>,
    /// Central model a workshared file syncs with.
    pub central_model_path: Option<String>,
    /// Path the file was last saved to.
    pub last_save_path: Option<String>,
    /// `Unique Document GUID`.
    pub document_guid: Option<String>,
    /// `Unique Document Increments` — grows as the document is saved.
    pub document_increments: Option<u32>,
    /// `Locale when saved`, e.g. `ENU`.
    pub locale: Option<String>,
    /// `IsSingleUserCloudModel` (Revit 2019+).
    pub single_user_cloud_model: Option<bool>,
    /// Every `BasicFileInfo` `Key: value` line, verbatim, in file order.
    pub properties: Vec<FileProperty>,
}

impl FileMetadata {
    /// Combine a parsed `BasicFileInfo` with the document Atom entry.
    pub fn from_parts(bfi: &BasicFileInfo, atom: Option<&PartAtom>) -> Self {
        let owned = |v: Option<&str>| v.map(str::to_string);
        Self {
            revit_version: bfi.version,
            build: bfi.build.clone(),
            title: atom.and_then(|a| a.entry_title.clone()),
            last_saved: atom.and_then(|a| a.updated.clone()),
            worksharing: owned(bfi.worksharing()),
            workshared: bfi.is_workshared(),
            username: owned(bfi.username()),
            central_model_path: owned(bfi.central_model_path()),
            last_save_path: owned(bfi.last_save_path()).or_else(|| bfi.original_path.clone()),
            document_guid: owned(bfi.document_guid()).or_else(|| bfi.guid.clone()),
            document_increments: bfi.document_increments(),
            locale: bfi.locale.clone(),
            single_user_cloud_model: bfi.is_single_user_cloud_model(),
            properties: bfi.properties.clone(),
        }
    }

    /// Scrub personal data for sharing: the user name becomes
    /// `<redacted>`, every path (including path-valued properties) goes
    /// through [`crate::redact::redact_sensitive`], and the recorded user
    /// name is also scrubbed inside file names — Revit names a local copy
    /// `<central>_<username>.rvt`.
    pub fn redact(&mut self) {
        let user = self.username.take().filter(|u| u.chars().count() >= 2);
        if user.is_some() {
            self.username = Some("<redacted>".to_string());
        }
        let scrub = |value: &str| {
            let redacted = crate::redact::redact_sensitive(value);
            match &user {
                Some(u) => redacted.replace(u.as_str(), "<redacted>"),
                None => redacted,
            }
        };
        for path in [&mut self.central_model_path, &mut self.last_save_path]
            .into_iter()
            .flatten()
        {
            *path = scrub(path);
        }
        for prop in &mut self.properties {
            if prop.key == "Username" && !prop.value.is_empty() {
                prop.value = "<redacted>".to_string();
            } else if prop.key.ends_with("Path") || prop.value.contains(":\\") {
                prop.value = scrub(&prop.value);
            }
        }
    }
}

impl RevitFile {
    /// [`FileMetadata`] for an already-open file. Fails only when
    /// `BasicFileInfo` is unreadable; a missing or malformed Atom entry
    /// leaves `title` / `last_saved` empty.
    pub fn metadata(&mut self) -> Result<FileMetadata> {
        let bfi = self.basic_file_info()?;
        let atom = self.document_atom().ok().flatten();
        Ok(FileMetadata::from_parts(&bfi, atom.as_ref()))
    }
}

/// [`FileMetadata`] for the file at `path`, reading only the identity
/// streams. Returns [`Error::NotACfbFile`] for anything that is not an
/// OLE/CFB container and [`Error::StreamNotFound`] for a CFB file (a `.doc`,
/// an `.msg`, …) without `BasicFileInfo`. I/O errors name the path.
pub fn read_metadata(path: impl AsRef<Path>) -> Result<FileMetadata> {
    let path = path.as_ref();
    let with_path = |e: std::io::Error| {
        Error::Io(std::io::Error::new(
            e.kind(),
            format!("{}: {e}", path.display()),
        ))
    };
    if std::fs::metadata(path).map_err(with_path)?.is_dir() {
        return Err(Error::Io(std::io::Error::new(
            std::io::ErrorKind::IsADirectory,
            format!(
                "{}: is a directory, not a .rvt / .rfa / .rte / .rft file",
                path.display()
            ),
        )));
    }
    let file = std::fs::File::open(path).map_err(with_path)?;
    read_metadata_from(std::io::BufReader::new(file))
}

/// [`read_metadata`] over any seekable reader (an open file, a
/// `Cursor<Vec<u8>>`, …).
pub fn read_metadata_from<R: Read + Seek>(mut reader: R) -> Result<FileMetadata> {
    let mut magic = [0u8; 8];
    let is_cfb = reader.read_exact(&mut magic).is_ok()
        && magic == [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    if !is_cfb {
        return Err(Error::NotACfbFile);
    }
    reader.rewind()?;
    let mut cfb = cfb::CompoundFile::open(reader).map_err(|e| Error::Cfb(e.to_string()))?;
    let bfi_bytes = read_small_stream(&mut cfb, BASIC_FILE_INFO)?
        .ok_or_else(|| Error::StreamNotFound(BASIC_FILE_INFO.to_string()))?;
    let bfi = BasicFileInfo::from_bytes(&bfi_bytes)?;
    let atom = match read_small_stream(&mut cfb, PART_ATOM) {
        Ok(Some(bytes)) => PartAtom::from_bytes(&bytes).ok(),
        _ => match read_small_stream(&mut cfb, PROJECT_INFORMATION) {
            Ok(Some(bytes)) => {
                crate::project_information::parse(&bytes, MAX_IDENTITY_STREAM_BYTES as usize).ok()
            }
            _ => None,
        },
    };
    Ok(FileMetadata::from_parts(&bfi, atom.as_ref()))
}

fn read_small_stream<R: Read + Seek>(
    cfb: &mut cfb::CompoundFile<R>,
    name: &str,
) -> Result<Option<Vec<u8>>> {
    let path = format!("/{name}");
    if !cfb.is_stream(&path) {
        return Ok(None);
    }
    let stream = cfb
        .open_stream(&path)
        .map_err(|e| Error::Cfb(e.to_string()))?;
    let mut bytes = Vec::new();
    stream
        .take(MAX_IDENTITY_STREAM_BYTES + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IDENTITY_STREAM_BYTES {
        return Err(Error::Cfb(format!(
            "stream '{name}' exceeds {MAX_IDENTITY_STREAM_BYTES} bytes"
        )));
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf16le(s: &str) -> Vec<u8> {
        s.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn bfi() -> BasicFileInfo {
        let mut bytes = utf16le("2024  20230509_0315(x64) ");
        bytes.extend(utf16le(
            "\u{0A0D}Worksharing: Local\r\n\
             Username: testuser\r\n\
             Central Model Path: \\\\fileserver\\projects\\Tower_Central.rvt\r\n\
             Last Save Path: C:\\Users\\testuser\\Documents\\Tower_testuser.rvt\r\n\
             Unique Document GUID: d713e470-abcd-4321-9876-123456789012\r\n\
             Unique Document Increments: 12\u{0A0D}",
        ));
        BasicFileInfo::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn from_parts_prefers_the_text_block() {
        let atom = PartAtom {
            title: Some("Other".into()),
            entry_title: Some("Tower".into()),
            updated: Some("2023-09-07T12:50:35Z".into()),
            ..Default::default()
        };
        let m = FileMetadata::from_parts(&bfi(), Some(&atom));
        assert_eq!(m.revit_version, 2024);
        assert_eq!(m.title.as_deref(), Some("Tower"));
        assert_eq!(m.last_saved.as_deref(), Some("2023-09-07T12:50:35Z"));
        assert_eq!(m.worksharing.as_deref(), Some("Local"));
        assert_eq!(m.workshared, Some(true));
        assert_eq!(m.username.as_deref(), Some("testuser"));
        assert_eq!(m.document_increments, Some(12));
        assert_eq!(
            m.document_guid.as_deref(),
            Some("d713e470-abcd-4321-9876-123456789012")
        );
    }

    #[test]
    fn redact_scrubs_user_and_paths_everywhere() {
        let mut m = FileMetadata::from_parts(&bfi(), None);
        m.redact();
        assert_eq!(m.username.as_deref(), Some("<redacted>"));
        assert_eq!(
            m.last_save_path.as_deref(),
            Some("C:\\Users\\<redacted>\\Documents\\Tower_<redacted>.rvt")
        );
        let json = serde_json::to_string(&m).unwrap();
        assert!(!json.contains("testuser"), "{json}");
    }

    #[test]
    fn read_metadata_from_rejects_non_cfb_bytes() {
        let err = read_metadata_from(std::io::Cursor::new(b"hello".to_vec())).unwrap_err();
        assert!(matches!(err, Error::NotACfbFile), "{err}");
        let err = read_metadata_from(std::io::Cursor::new(Vec::new())).unwrap_err();
        assert!(matches!(err, Error::NotACfbFile), "{err}");
    }

    #[test]
    fn read_metadata_names_a_missing_path() {
        let err = read_metadata("definitely/not/here.rvt").unwrap_err();
        assert!(err.to_string().contains("definitely/not/here.rvt"), "{err}");
    }
}
