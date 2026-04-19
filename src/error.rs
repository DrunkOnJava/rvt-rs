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

    #[error("Malformed BasicFileInfo: {0}")]
    BasicFileInfo(String),

    #[error("Malformed PartAtom XML: {0}")]
    PartAtom(String),

    #[error("Invalid UTF-16: {0}")]
    Utf16(String),
}
