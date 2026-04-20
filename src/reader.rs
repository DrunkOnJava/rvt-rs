//! High-level reader API. Opens a `.rvt` / `.rfa` / `.rte` / `.rft` file
//! and exposes its streams + parsed metadata.

use crate::{
    Error, Result,
    basic_file_info::BasicFileInfo,
    class_index, compression,
    part_atom::PartAtom,
    streams::{
        BASIC_FILE_INFO, CONTENTS, FORMATS_LATEST, GLOBAL_CONTENT_DOCUMENTS,
        GLOBAL_DOC_INCREMENT_TABLE, GLOBAL_ELEM_TABLE, GLOBAL_HISTORY, GLOBAL_LATEST,
        GLOBAL_PARTITION_TABLE, PART_ATOM, REVIT_PREVIEW_4_0, TRANSMISSION_DATA,
    },
};
use cfb::CompoundFile;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{Cursor, Read},
    path::Path,
};

/// Default maximum file size accepted by [`RevitFile::open`].
///
/// 2 GiB is above any real-world Revit project we've observed
/// (typical: few MB–a few hundred MB; worksharing extreme: ~1 GiB)
/// and well below "pathological or hostile" territory. Callers with
/// specific larger-file needs should use [`RevitFile::open_with_limits`].
pub const DEFAULT_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// Default maximum stream size accepted by [`RevitFile::read_stream`].
///
/// 256 MiB per stream is comfortably above any observed stream. The
/// largest legitimate stream we've seen in the corpus is ~40 MiB
/// (Global/Latest on a large worksharing project). Hostile input
/// with a claimed huge stream size will be rejected before a
/// multi-GB allocation.
pub const DEFAULT_MAX_STREAM_BYTES: u64 = 256 * 1024 * 1024;

/// Limits applied when opening a Revit file. Protects against
/// pathological or hostile input that would otherwise force
/// unbounded memory allocation.
///
/// See audit P0 items 4 and 5 (AUDIT-2026-04-19.md) for the
/// rationale — RVT is a file-upload target, and bounded resource
/// consumption is a DoS-safety requirement, not a nice-to-have.
#[derive(Debug, Clone, Copy)]
pub struct OpenLimits {
    /// Maximum file size accepted. File bytes are read into memory
    /// entirely on open (CFB requires random access), so this
    /// doubles as an upper bound on the initial allocation.
    pub max_file_bytes: u64,
    /// Maximum per-stream size accepted by [`RevitFile::read_stream`].
    /// Streams larger than this cause an error rather than a
    /// multi-GB alloc.
    pub max_stream_bytes: u64,
    /// Inflate limits applied to every `compression::inflate_at`
    /// call sourced from this `RevitFile`. Keeps bounded-decompression
    /// and open-limits consistent so a file opened under restrictive
    /// limits doesn't accidentally re-inflate under permissive ones.
    pub inflate_limits: compression::InflateLimits,
}

impl Default for OpenLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            max_stream_bytes: DEFAULT_MAX_STREAM_BYTES,
            inflate_limits: compression::InflateLimits::default(),
        }
    }
}

/// Opened Revit file. Holds the CFB handle + cached stream bytes.
pub struct RevitFile {
    cfb: CompoundFile<Cursor<Vec<u8>>>,
    /// Limits to apply on subsequent reads. Copied from the
    /// `OpenLimits` passed at construction; defaults to
    /// `OpenLimits::default()` for back-compat `open`/`open_bytes`.
    limits: OpenLimits,
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
    ///
    /// Returns an error if the file doesn't exist, can't be read, or
    /// doesn't start with the OLE2 / MS-CFB magic bytes
    /// (`D0 CF 11 E0 A1 B1 1A E1`).
    ///
    /// ```no_run
    /// use rvt::RevitFile;
    ///
    /// let mut rf = RevitFile::open("your-project.rfa")?;
    /// let summary = rf.summarize()?;
    /// println!("Revit {}", summary.version);
    /// # Ok::<(), rvt::Error>(())
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::open_with_limits(path, OpenLimits::default())
    }

    /// Open a Revit file from disk with explicit resource limits.
    ///
    /// Stats the file before reading; refuses if file size exceeds
    /// `limits.max_file_bytes` to prevent multi-GB allocations from
    /// a hostile input. Back-compat [`Self::open`] calls this with
    /// `OpenLimits::default()` (2 GiB file, 256 MiB per stream, 256
    /// MiB per inflate).
    ///
    /// ```no_run
    /// use rvt::reader::{RevitFile, OpenLimits};
    ///
    /// // Only accept files up to 100 MB.
    /// let limits = OpenLimits {
    ///     max_file_bytes: 100 * 1024 * 1024,
    ///     ..OpenLimits::default()
    /// };
    /// let mut rf = RevitFile::open_with_limits("your-project.rfa", limits)?;
    /// # Ok::<(), rvt::Error>(())
    /// ```
    pub fn open_with_limits(path: impl AsRef<Path>, limits: OpenLimits) -> Result<Self> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)?;
        if metadata.len() > limits.max_file_bytes {
            return Err(Error::Cfb(format!(
                "file size {} exceeds limit {}",
                metadata.len(),
                limits.max_file_bytes
            )));
        }
        let mut f = File::open(path)?;
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        f.read_to_end(&mut bytes)?;
        Self::open_bytes_with_limits(bytes, limits)
    }

    /// Open a Revit file from an in-memory byte buffer.
    ///
    /// Useful for callers that have the file bytes already (e.g. streamed
    /// over the network). Equivalent to `open` after a `read_to_end`.
    ///
    /// ```
    /// use rvt::RevitFile;
    /// // Four bytes that are definitely not a valid CFB file.
    /// let result = RevitFile::open_bytes(b"nope".to_vec());
    /// assert!(matches!(result, Err(rvt::Error::NotACfbFile)));
    /// ```
    pub fn open_bytes(bytes: Vec<u8>) -> Result<Self> {
        Self::open_bytes_with_limits(bytes, OpenLimits::default())
    }

    /// Open-bytes variant with explicit limits. The byte count check
    /// has already been done by the caller if they came through
    /// [`Self::open_with_limits`]; here it's repeated for in-memory
    /// paths that skip disk stat.
    pub fn open_bytes_with_limits(bytes: Vec<u8>, limits: OpenLimits) -> Result<Self> {
        if (bytes.len() as u64) > limits.max_file_bytes {
            return Err(Error::Cfb(format!(
                "in-memory buffer size {} exceeds limit {}",
                bytes.len(),
                limits.max_file_bytes
            )));
        }
        if bytes.len() < 8 || bytes[..8] != [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1] {
            return Err(Error::NotACfbFile);
        }
        let cfb = CompoundFile::open(Cursor::new(bytes)).map_err(|e| Error::Cfb(e.to_string()))?;
        Ok(Self { cfb, limits })
    }

    /// Resource limits this file was opened under. Use to match
    /// the limits when calling bounded-inflate on extracted stream
    /// bytes.
    pub fn limits(&self) -> OpenLimits {
        self.limits
    }

    /// List all OLE stream paths (sorted). Paths are always returned
    /// with forward-slash separators regardless of host OS — on
    /// Windows, `Path::display()` emits backslashes, but CFB stream
    /// paths are logically `/`-separated.
    pub fn stream_names(&self) -> Vec<String> {
        let mut streams: Vec<_> = self
            .cfb
            .walk()
            .filter(|e| e.is_stream())
            .map(|e| {
                e.path()
                    .display()
                    .to_string()
                    .replace('\\', "/")
                    .trim_start_matches('/')
                    .to_string()
            })
            .collect();
        streams.sort();
        streams
    }

    /// Read a named stream's raw bytes, capped at the file's
    /// configured `max_stream_bytes`.
    ///
    /// For streams larger than the limit, returns an error rather
    /// than allocating a potentially multi-GB `Vec`. Use
    /// [`Self::read_stream_with_limit`] to override per-call.
    pub fn read_stream(&mut self, name: &str) -> Result<Vec<u8>> {
        self.read_stream_with_limit(name, self.limits.max_stream_bytes)
    }

    /// Read a named stream's raw bytes, capped at an explicit
    /// byte limit.
    ///
    /// `max_bytes` is the ceiling on output size. A stream whose
    /// declared size (or read position) exceeds this returns
    /// `Error::Cfb("stream exceeds limit…")`.
    pub fn read_stream_with_limit(&mut self, name: &str, max_bytes: u64) -> Result<Vec<u8>> {
        let path = if name.starts_with('/') {
            name.to_string()
        } else {
            format!("/{name}")
        };
        let mut stream = self
            .cfb
            .open_stream(&path)
            .map_err(|_| Error::StreamNotFound(name.to_string()))?;
        // Stream size is known up-front from the CFB directory entry.
        // Reject before reading.
        let stream_size = stream.len();
        if stream_size > max_bytes {
            return Err(Error::Cfb(format!(
                "stream '{name}' size {stream_size} exceeds limit {max_bytes}"
            )));
        }
        let cap = (stream_size as usize).min(max_bytes as usize);
        let mut out = Vec::with_capacity(cap);
        // Read in bounded chunks so we can catch the case where a
        // stream's directory-entry size is a lie (malformed CFB).
        let mut buf = [0u8; 8192];
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                break;
            }
            if (out.len() as u64) + (n as u64) > max_bytes {
                return Err(Error::Cfb(format!(
                    "stream '{name}' exceeded limit {max_bytes} mid-read"
                )));
            }
            out.extend_from_slice(&buf[..n]);
        }
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

        let file_size: u64 = streams.iter().filter_map(|n| self.stream_size(n)).sum();

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
        self.missing_required_streams().is_empty()
    }

    /// Diagnostic form of `has_revit_signature` — returns the list of
    /// required streams that are missing, or an empty vec if all are
    /// present. Much more useful than the bool when triaging: "why does
    /// this 2016 file work on Linux but not Windows?" gets a concrete
    /// answer ("missing Global/DocumentIncrementTable") instead of "yes
    /// or no".
    pub fn missing_required_streams(&self) -> Vec<&'static str> {
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
        required
            .iter()
            .copied()
            .filter(|r| !names.contains(*r))
            .collect()
    }
}
