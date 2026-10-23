//! The crate's error type.
//!
//! Reading, decoding, exporting and writing a Revit file return [`Result`],
//! whose error is [`Error`](enum@Error); the few functions that only convert
//! JSON, such as `CapabilityManifest::to_json_string`, return
//! `serde_json::Error`. Input is untrusted, so a file rvt-rs cannot read is
//! reported as an [`Error`](enum@Error); the parsers are written not to panic
//! on it and are fuzzed for it. `Error` is `#[non_exhaustive]`: a release may add
//! variants, so a `match` on it needs a wildcard arm.

use std::io;
use thiserror::Error;

/// `std::result::Result` with [`Error`](enum@Error) as the default error.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Why an operation on a Revit file failed.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading or writing the file failed.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// The file is not an OLE compound file, so it cannot be a Revit file.
    #[error("Not a Revit file (magic bytes did not match CFB/OLE)")]
    NotACfbFile,

    /// The OLE container could not be opened, read or written.
    #[error("CFB {op}{}: {source}", for_path(.path))]
    Cfb {
        /// What was being done: `open`, `open_stream`, `create`,
        /// `create_storage`, `create_stream`, `write`, `flush` or `rename`.
        op: &'static str,
        /// The stream or file it was done to, where there is one.
        path: Option<String>,
        /// The I/O error, also [`std::error::Error::source`].
        #[source]
        source: io::Error,
    },

    /// A file, buffer or stream is larger than the limit set for it, such as
    /// [`crate::reader::OpenLimits::max_file_bytes`].
    #[error("{what} is {size} bytes, over the limit of {limit}")]
    TooLarge {
        /// What was too large, such as `"the file"` or `"stream 'Global/Latest'"`.
        what: String,
        /// Its size in bytes, or as far as it was read.
        size: u64,
        /// The limit in bytes.
        limit: u64,
    },

    /// The file has no stream of this name.
    #[error("Stream not found: {0}")]
    StreamNotFound(String),

    /// A stream's compressed bytes did not inflate.
    #[error("Decompression failed: {0}")]
    Decompress(String),

    /// A bounded-decompression call refused to continue because its
    /// output budget would be exceeded. Raised by
    /// [`crate::compression::inflate_at_with_limits`] and its
    /// aggregate-budget cousin. Distinct from the generic
    /// `Decompress` variant so callers can distinguish "corrupt
    /// input" from "well-formed input that would inflate beyond the
    /// configured DoS ceiling."
    #[error("Decompression output limit exceeded: {0}")]
    DecompressLimitExceeded(String),

    /// The `BasicFileInfo` stream does not parse.
    #[error("Malformed BasicFileInfo: {0}")]
    BasicFileInfo(String),

    /// The `PartAtom` stream is not the Atom XML it should be.
    #[error("Malformed PartAtom XML: {0}")]
    PartAtom(String),

    /// A UTF-16 string in the file does not decode.
    #[error("Invalid UTF-16: {0}")]
    Utf16(String),

    /// A record or stream does not have the shape its reader expects.
    #[error("malformed {what}: {detail}")]
    Malformed {
        /// What was being read, such as `"ArcWall record"` or
        /// `"Global/ElemTable"`.
        what: &'static str,
        /// What did not match.
        detail: String,
    },

    /// The file's schema (`Formats/Latest`) declares no class of this name.
    #[error("the schema declares no {0} class")]
    MissingClass(&'static str),

    /// A record the reader looks for could not be located with confidence,
    /// as when the `ADocument` entry-point detector, validated on Revit 2024
    /// to 2026, finds no offset in an earlier release's layout. The file
    /// may be valid.
    #[error("the {0} record could not be located")]
    NotLocated(&'static str),

    /// A typed decoder was handed the schema entry of another class.
    #[error("{decoder} decoder received the schema of {schema}")]
    SchemaMismatch {
        /// The class the decoder decodes.
        decoder: &'static str,
        /// The schema entry it was given.
        schema: String,
    },

    /// No typed decoder is registered for the class.
    #[error("no typed decoder for class {0}")]
    NoDecoder(String),

    /// The writer refused a patch it cannot write as Revit stores it: a
    /// stream kept in checksummed pages whose new bytes would span one, when
    /// rvt-rs cannot compute the page checksum.
    // Tracker: B69.
    #[error("write refused: {0}")]
    WriteRefused(String),

    /// A caller-supplied [`crate::control::CancelToken`] was cancelled
    /// while a scan was running. Partial results are discarded; the
    /// file itself is fine.
    #[error("operation cancelled by caller")]
    Cancelled,
}

fn for_path(path: &Option<String>) -> String {
    path.as_deref().map(|p| format!(" {p}")).unwrap_or_default()
}

impl Error {
    /// Name the file a container error happened in, when it was raised by
    /// code that had only the bytes or a reader.
    pub(crate) fn in_file(self, file: &std::path::Path) -> Self {
        match self {
            Error::Cfb {
                op,
                path: None,
                source,
            } => Error::Cfb {
                op,
                path: Some(file.display().to_string()),
                source,
            },
            other => other,
        }
    }
}
