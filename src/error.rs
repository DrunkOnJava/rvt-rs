//! The crate's error type.
//!
//! Every fallible function in the crate returns [`Result`], whose error is
//! [`Error`](enum@Error). Input is untrusted, so a file rvt-rs cannot read is
//! reported as an [`Error`](enum@Error); the parsers are written not to panic on it and
//! are fuzzed for it. `Error` is `#[non_exhaustive]`: a release may add
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
    #[error("CFB: {0}")]
    Cfb(String),

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
