//! High-level reader API. Opens a `.rvt` / `.rfa` / `.rte` / `.rft` file
//! and exposes its streams + parsed metadata.

use crate::{
    basic_file_info::BasicFileInfo,
    class_index, compression,
    part_atom::PartAtom,
    streams::{
        BASIC_FILE_INFO, CONTENTS, FORMATS_LATEST, GLOBAL_CONTENT_DOCUMENTS,
        GLOBAL_DOC_INCREMENT_TABLE, GLOBAL_ELEM_TABLE, GLOBAL_HISTORY, GLOBAL_LATEST,
        GLOBAL_PARTITION_TABLE, PART_ATOM, REVIT_PREVIEW_4_0, TRANSMISSION_DATA,
    },
    Error, Result,
};
use cfb::CompoundFile;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

/// Opened Revit file. Holds the CFB handle + cached stream bytes.
pub struct RevitFile {
    cfb: CompoundFile<Cursor<Vec<u8>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub file_size: u64,
    pub streams: Vec<String>,
    pub version: u32,
    pub build: Option<String>,
    pub original_path: Option<String>,
    pub guid: Option<String>,
    pub locale: Option<String>,
    pub partition_stream: Option<String>,
    pub partatom: Option<PartAtom>,
    pub class_name_count: usize,
    pub class_name_sample: Vec<String>,
}

impl RevitFile {
    /// Open a Revit file from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut f = File::open(path.as_ref())?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        Self::open_bytes(bytes)
    }

    /// Open a Revit file from an in-memory byte buffer.
    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.len() < 8 || bytes[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            return Err(Error::NotACfbFile);
        }
        let cfb =
            CompoundFile::open(Cursor::new(bytes)).map_err(|e| Error::Cfb(e.to_string()))?;
        Ok(Self { cfb })
    }

    /// List all OLE stream paths (sorted).
    pub fn stream_names(&self) -> Vec<String> {
        let mut streams: Vec<_> = self
            .cfb
            .walk()
            .filter(|e| e.is_stream())
            .map(|e| e.path().display().to_string().trim_start_matches('/').to_string())
            .collect();
        streams.sort();
        streams
    }

    /// Read a named stream's raw bytes.
    pub fn read_stream(&mut self, name: &str) -> Result<Vec<u8>> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        let mut stream = self
            .cfb
            .open_stream(&path)
            .map_err(|_| Error::StreamNotFound(name.to_string()))?;
        let mut out = Vec::new();
        stream.read_to_end(&mut out)?;
        Ok(out)
    }

    /// Parse `BasicFileInfo`.
    pub fn basic_file_info(&mut self) -> Result<BasicFileInfo> {
        let bytes = self.read_stream(BASIC_FILE_INFO)?;
        BasicFileInfo::from_bytes(&bytes)
    }

    /// Parse `PartAtom` XML.
    pub fn part_atom(&mut self) -> Result<PartAtom> {
        let bytes = self.read_stream(PART_ATOM)?;
        PartAtom::from_bytes(&bytes)
    }

    /// Extract the PNG thumbnail from `RevitPreview4.0`.
    ///
    /// The raw stream has a ~300-byte Revit-specific header (magic
    /// `62 19 22 05` — the same header magic seen at the start of the
    /// `Contents` stream). The PNG payload begins at the first occurrence
    /// of the standard PNG magic bytes.
    pub fn preview_png(&mut self) -> Result<Vec<u8>> {
        let bytes = self.read_stream(REVIT_PREVIEW_4_0)?;
        const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
        let pos = bytes
            .windows(8)
            .position(|w| w == PNG_MAGIC)
            .ok_or_else(|| Error::StreamNotFound("PNG magic inside RevitPreview4.0".into()))?;
        Ok(bytes[pos..].to_vec())
    }

    /// Raw bytes of the `RevitPreview4.0` stream including Revit's
    /// custom wrapper. Use `preview_png` for just the PNG.
    pub fn preview_raw(&mut self) -> Result<Vec<u8>> {
        self.read_stream(REVIT_PREVIEW_4_0)
    }

    /// Decompress `Formats/Latest` and extract the class/schema inventory.
    pub fn class_names(&mut self) -> Result<BTreeSet<String>> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        // Formats/Latest has GZIP magic at offset 0 (no custom header).
        let decompressed = compression::inflate_at(&bytes, 0)?;
        class_index::extract_class_names(&decompressed)
    }

    /// Decompress `Formats/Latest` and parse it into a full schema table —
    /// classes + fields + C++ type signatures. This is the structured
    /// version of `class_names()`.
    pub fn schema(&mut self) -> Result<crate::formats::SchemaTable> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        let decompressed = compression::inflate_at(&bytes, 0)?;
        crate::formats::parse_schema(&decompressed)
    }

    /// Find the version-specific `Partitions/NN` stream name.
    pub fn partition_stream_name(&self) -> Option<String> {
        self.stream_names()
            .into_iter()
            .find(|n| n.starts_with("Partitions/"))
    }

    /// Produce a one-shot summary of everything we can parse.
    pub fn summarize(&mut self) -> Result<Summary> {
        let streams = self.stream_names();
        let bfi = self.basic_file_info()?;
        let partatom = self.part_atom().ok();
        let partition_stream = self.partition_stream_name();
        let class_names = self.class_names().unwrap_or_default();
        let class_name_count = class_names.len();
        let class_name_sample: Vec<String> = class_names.into_iter().take(30).collect();

        let file_size: u64 = streams
            .iter()
            .filter_map(|n| self.stream_size(n))
            .sum();

        Ok(Summary {
            file_size,
            streams,
            version: bfi.version,
            build: bfi.build,
            original_path: bfi.original_path,
            guid: bfi.guid,
            locale: bfi.locale,
            partition_stream,
            partatom,
            class_name_count,
            class_name_sample,
        })
    }

    /// Get size of a named stream (returns `None` if missing).
    pub fn stream_size(&self, name: &str) -> Option<u64> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        self.cfb.entry(&path).ok().map(|e| e.len())
    }

    /// Check the common/invariant streams are all present. Useful for triage:
    /// if any of these is missing, the file is either corrupt or not a Revit file
    /// despite having a valid CFB container.
    pub fn has_revit_signature(&self) -> bool {
        let names: BTreeSet<String> = self.stream_names().into_iter().collect();
        let required = [
            BASIC_FILE_INFO,
            CONTENTS,
            FORMATS_LATEST,
            GLOBAL_CONTENT_DOCUMENTS,
            GLOBAL_DOC_INCREMENT_TABLE,
            GLOBAL_ELEM_TABLE,
            GLOBAL_HISTORY,
            GLOBAL_LATEST,
            GLOBAL_PARTITION_TABLE,
            PART_ATOM,
            REVIT_PREVIEW_4_0,
            TRANSMISSION_DATA,
        ];
        required.iter().all(|r| names.contains(*r))
    }
}
