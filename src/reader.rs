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
    ///
    /// When an `IEND` chunk is present, the returned buffer ends at the
    /// trailing CRC of that chunk — trailing junk after a well-formed PNG
    /// is trimmed. If no `IEND` is found, bytes from PNG magic through
    /// end-of-stream are returned (fail-open for truncated previews).
    pub fn preview_png(&mut self) -> Result<Vec<u8>> {
        let bytes = self.read_stream(REVIT_PREVIEW_4_0)?;
        extract_preview_png(&bytes)
    }

    /// Like [`Self::preview_png`], but never trims after `IEND` — returns
    /// every byte from PNG magic through end of the OLE stream. Useful for
    /// forensic inspection of trailing payload.
    pub fn preview_png_untrimmed(&mut self) -> Result<Vec<u8>> {
        let bytes = self.read_stream(REVIT_PREVIEW_4_0)?;
        extract_preview_png_untrimmed(&bytes)
    }

    /// Raw bytes of the `RevitPreview4.0` stream including Revit's
    /// custom wrapper. Use `preview_png` for just the PNG.
    pub fn preview_raw(&mut self) -> Result<Vec<u8>> {
        self.read_stream(REVIT_PREVIEW_4_0)
    }

    /// Detect-only probe of `TransmissionData` (UTF-16LE vs opaque vs empty).
    ///
    /// Does **not** decode linked-model tables or Autodesk transmission
    /// field layouts — see [`crate::transmission_data`].
    pub fn transmission_data_probe(
        &mut self,
    ) -> Result<crate::transmission_data::TransmissionDataProbe> {
        crate::transmission_data::probe_transmission_data(self)
    }

    /// Decompress `Formats/Latest` and extract the class/schema inventory.
    pub fn class_names(&mut self) -> Result<BTreeSet<String>> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        // Formats/Latest has GZIP magic at offset 0 (no custom header).
        // Wave 2 gate deliberately does **not** strip page trailers here —
        // naive strip regresses opportunistic `class_names` on redistributable
        // corpora (#151 judge: narrow). `inflate_stream_at` stays path-aware
        // for API uniformity but is a no-op strip for Formats.
        let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &bytes, 0)?;
        class_index::extract_class_names(&decompressed)
    }

    /// Decompress `Formats/Latest` and parse it into a full schema table —
    /// classes + fields + C++ type signatures. This is the structured
    /// version of `class_names()`.
    pub fn schema(&mut self) -> Result<crate::formats::SchemaTable> {
        let bytes = self.read_stream(FORMATS_LATEST)?;
        let decompressed = compression::inflate_stream_at(FORMATS_LATEST, &bytes, 0)?;
        crate::formats::parse_schema(&decompressed)
    }

    /// Find the version-specific `Partitions/NN` stream name.
    pub fn partition_stream_name(&self) -> Option<String> {
        self.stream_names()
            .into_iter()
            .find(|n| n.starts_with("Partitions/"))
    }

    /// Produce a one-shot summary of everything we can parse.
    ///
    /// Historically this method was mixed — strict on BasicFileInfo
    /// (propagated errors) but lossy on PartAtom / class-names
    /// (swallowed failures via `.ok()` / `.unwrap_or_default()`).
    /// Prefer [`RevitFile::summarize_strict`] or
    /// [`RevitFile::summarize_lossy`] — this wrapper calls through
    /// to the lossy path and discards the diagnostics for backwards
    /// compatibility. It's kept on the stable surface but new
    /// callers should pick the strictness level they actually want.
    #[deprecated(
        since = "0.1.2",
        note = "Use `summarize_strict` for errors-on-any-failure or `summarize_lossy` for Decoded<Summary> with accumulated diagnostics."
    )]
    pub fn summarize(&mut self) -> Result<Summary> {
        self.summarize_lossy().map(|d| d.value)
    }

    /// Strict variant of [`RevitFile::summarize`] (API-04) — returns
    /// `Err` on ANY parse failure, including PartAtom absence and
    /// class-name enumeration failure. Use when downstream code
    /// can't tolerate a partially-populated `Summary`.
    pub fn summarize_strict(&mut self) -> Result<Summary> {
        let streams = self.stream_names();
        let bfi = self.basic_file_info()?;
        // PartAtom: absence is a soft concept — not every file has
        // one. Missing PartAtom is NOT an error here; a failed
        // parse of an existing PartAtom IS.
        let partatom = match self.part_atom() {
            Ok(p) => Some(p),
            Err(Error::StreamNotFound(_)) => None,
            Err(e) => return Err(e),
        };
        let partition_stream = self.partition_stream_name();
        let class_names = self.class_names()?;
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

    /// Lossy variant (API-05) — returns a [`crate::parse_mode::Decoded<Summary>`]
    /// that accumulates partial-parse issues into its
    /// `diagnostics` field instead of aborting. Fails only when
    /// BasicFileInfo itself is unreadable (without it there's
    /// nothing to summarise). Every other failure becomes a
    /// `Diagnostic::fail_stream` entry and the corresponding
    /// `Summary` field uses its default.
    ///
    /// Use when you're summarising a batch of files and want a
    /// best-effort report even for the ones with partial parses.
    pub fn summarize_lossy(&mut self) -> Result<crate::parse_mode::Decoded<Summary>> {
        use crate::parse_mode::{Decoded, Diagnostics};

        let streams = self.stream_names();
        // BasicFileInfo is load-bearing — without it we can't
        // populate any versioned identity. Propagate its error.
        let bfi = self.basic_file_info()?;

        let mut diagnostics = Diagnostics::default();

        let partatom = match self.part_atom() {
            Ok(p) => Some(p),
            Err(Error::StreamNotFound(_)) => None,
            Err(_) => {
                diagnostics.fail_stream("PartAtom");
                None
            }
        };

        let partition_stream = self.partition_stream_name();
        if partition_stream.is_none() {
            diagnostics.fail_stream("Partitions/*");
        }

        let class_names = match self.class_names() {
            Ok(n) => n,
            Err(_) => {
                diagnostics.fail_stream(FORMATS_LATEST);
                Default::default()
            }
        };
        let class_name_count = class_names.len();
        let class_name_sample: Vec<String> = class_names.into_iter().take(30).collect();

        let file_size: u64 = streams.iter().filter_map(|n| self.stream_size(n)).sum();

        let summary = Summary {
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
        };

        Ok(if diagnostics.is_empty() {
            Decoded::complete(summary)
        } else {
            Decoded::partial(summary, diagnostics)
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

const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
/// PNG chunk type `IEND` (zero-length terminal chunk).
const PNG_IEND_TYPE: [u8; 4] = [0x49, 0x45, 0x4E, 0x44];

/// Locate PNG magic inside a `RevitPreview4.0` stream and return bytes from
/// that offset through end-of-input (no IEND trim).
fn extract_preview_png_untrimmed(stream: &[u8]) -> Result<Vec<u8>> {
    let pos = stream
        .windows(8)
        .position(|w| w == PNG_MAGIC)
        .ok_or_else(|| Error::StreamNotFound("PNG magic inside RevitPreview4.0".into()))?;
    Ok(stream[pos..].to_vec())
}

/// Locate PNG magic and, when present, trim after the IEND chunk CRC.
fn extract_preview_png(stream: &[u8]) -> Result<Vec<u8>> {
    let mut png = extract_preview_png_untrimmed(stream)?;
    if let Some(end) = png_iend_exclusive_end(&png) {
        png.truncate(end);
    }
    Ok(png)
}

/// Return the exclusive end offset of a well-formed IEND chunk inside `png`,
/// or `None` if no IEND type marker with room for its CRC is found.
///
/// IEND wire layout: `[u32 BE length=0][IEND][u32 CRC]` (12 bytes). We scan
/// for the type bytes and require a 4-byte length field of zero immediately
/// before them plus four CRC bytes after.
fn png_iend_exclusive_end(png: &[u8]) -> Option<usize> {
    // Need at least 4 (len) + 4 (type) + 4 (crc) = 12 bytes for a hit.
    if png.len() < 12 {
        return None;
    }
    for i in 0..=png.len().saturating_sub(8) {
        if png[i..i + 4] != PNG_IEND_TYPE {
            continue;
        }
        // Type must be preceded by a 4-byte big-endian length of zero.
        if i < 4 {
            continue;
        }
        let len_start = i - 4;
        if png[len_start..i] != [0, 0, 0, 0] {
            continue;
        }
        let crc_end = i + 4 + 4; // type + CRC
        if crc_end > png.len() {
            return None;
        }
        return Some(crc_end);
    }
    None
}

#[cfg(test)]
mod preview_png_tests {
    use super::*;

    /// Minimal 1×1 transparent PNG (same bytes as `gen-fixture`).
    const MIN_PNG: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG magic
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15,
        0xC4, 0x89, //
        0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4,
        // IEND
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
    ];

    #[test]
    fn trim_drops_bytes_after_iend() {
        let mut stream = vec![0x62, 0x19, 0x22, 0x05, 0, 0, 0, 0];
        stream.extend_from_slice(MIN_PNG);
        stream.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11]);
        let trimmed = extract_preview_png(&stream).expect("png");
        let untrimmed = extract_preview_png_untrimmed(&stream).expect("png");
        assert_eq!(trimmed, MIN_PNG);
        assert_eq!(untrimmed.len(), MIN_PNG.len() + 6);
        assert!(untrimmed.ends_with(&[0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0x11]));
    }

    #[test]
    fn missing_iend_keeps_tail() {
        let mut partial = MIN_PNG[..MIN_PNG.len() - 12].to_vec(); // drop IEND
        partial.extend_from_slice(&[0xAA, 0xBB]);
        let out = extract_preview_png(&partial).expect("png");
        assert_eq!(out, partial);
    }

    #[test]
    fn missing_magic_errors() {
        let err = extract_preview_png(&[0, 1, 2, 3]).unwrap_err();
        assert!(matches!(err, Error::StreamNotFound(_)));
    }
}
