//! rvt-rs · Open reader for Autodesk Revit files (.rvt, .rfa, .rte, .rft)
//!
//! Parses the OLE2 Compound File container used by every Revit release
//! from 2016 through 2026 without any Autodesk dependency.
//!
//! # Format overview
//!
//! - Container: Microsoft Compound File Binary Format 3.0 (`[MS-CFB]`).
//! - Compression: truncated-gzip — standard 10-byte gzip header followed
//!   by raw DEFLATE, but without the trailing CRC32+ISIZE that conforming
//!   gzip writers emit. The standard gzip parsers in most languages refuse
//!   to parse this; we handle it via raw-inflate on the post-header bytes.
//! - Streams: 12 invariant across all versions (2016-2026) + one
//!   `Partitions/NN` stream whose NN increments per year.
//!
//! # Moat layers (for context)
//!
//! ```text
//! Layer 1 (container)     → SOLVED by cfb crate.
//! Layer 2 (compression)   → SOLVED by flate2 raw DEFLATE.
//! Layer 3 (stream frames) → PARTIAL — known for most streams, fully reverse
//!                           engineered from the 11-version sample corpus.
//! Layer 4 (object graph)  → UNSOLVED — plaintext class names exposed in
//!                           Formats/Latest but field layouts are proprietary.
//! ```
//!
//! See `docs/rvt-moat-break-reconnaissance.md` in the repo root for the
//! full moat analysis.

pub mod basic_file_info;
pub mod class_index;
pub mod compression;
pub mod corpus;
pub mod elem_table;
pub mod error;
pub mod formats;
pub mod object_graph;
pub mod part_atom;
pub mod partitions;
pub mod reader;
pub mod streams;
pub mod writer;

pub mod ifc;

pub use error::{Error, Result};
pub use reader::RevitFile;
