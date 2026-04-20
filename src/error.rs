use std::io;
use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Not a Revit file (magic bytes did not match CFB/OLE)")]
    NotACfbFile,

    #[error("CFB: {0}")]
    Cfb(String),

    #[error("Stream not found: {0}")]
    StreamNotFound(String),

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

    #[error("Malformed BasicFileInfo: {0}")]
    BasicFileInfo(String),

    #[error("Malformed PartAtom XML: {0}")]
    PartAtom(String),

    #[error("Invalid UTF-16: {0}")]
    Utf16(String),
}
