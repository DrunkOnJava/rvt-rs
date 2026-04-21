//! # rvt-rs · open reader for Autodesk Revit files
//!
//! Parses `.rvt`, `.rfa`, `.rte`, and `.rft` files without any Autodesk
//! dependency. Verified against 11 Revit releases (2016 through 2026).
//!
//! ## Quickstart
//!
//! ```no_run
//! use rvt::RevitFile;
//!
//! let mut rf = RevitFile::open("model.rfa")?;
//! let summary = rf.summarize()?;
//! println!("Revit {} ({})", summary.version, summary.build.as_deref().unwrap_or("—"));
//! # Ok::<(), rvt::Error>(())
//! ```
//!
//! ## Format overview
//!
//! - **Container**: Microsoft Compound File Binary Format 3.0 (`[MS-CFB]`)
//! - **Compression**: *truncated-gzip* — standard 10-byte gzip header
//!   followed by raw DEFLATE, but **without** the trailing CRC32+ISIZE
//!   that conforming gzip writers emit. Standard `gzip` parsers in most
//!   languages refuse these streams; this crate handles them via raw
//!   inflate on the post-header bytes (see [`compression::inflate_at`]).
//! - **Streams**: 12 invariant across every release (2016-2026), plus one
//!   `Partitions/NN` stream whose `NN` varies per release (58 in 2016;
//!   60-69 for 2018-2026; 59 is skipped).
//!
//! ## Moat layers
//!
//! | Layer | Description | Status |
//! |---|---|---|
//! | 1 · Container | OLE2 / MS-CFB | **Done** (via `cfb` crate) |
//! | 2 · Compression | Truncated gzip | **Done** (via `flate2` raw DEFLATE) |
//! | 3 · Stream framing | Per-stream custom headers | **Done** |
//! | 4a · Schema table | Class names + fields + tags | **Done** ([`formats`]) |
//! | 4b · Schema→data link | Tags index instance data at 340× | **Done** |
//! | 4c · Field decoding | 11 discriminators mapped, **100% coverage** (13,570 fields across the 11-version corpus; zero `Unknown`) | **Done** ([`formats::FieldType`]) |
//! | 4d · ElemTable records | Header done; body TBD | **Partial** ([`elem_table`]) |
//! | 5 · IFC export | Scaffold + mapping plan | **Scaffolded** ([`ifc`]) |
//! | 6 · Write path | Byte-preserving round-trip verified | **Scaffolded** ([`writer`]) |
//!
//! Full analysis narrative with 13 dated addenda is in
//! `docs/rvt-moat-break-reconnaissance.md` in the repo.
//!
//! ## Module overview
//!
//! - [`reader`] — [`RevitFile`], the main entry point
//! - [`basic_file_info`] — version, GUID, build tag, creator path
//! - [`part_atom`] — Atom XML with OmniClass + taxonomies
//! - [`formats`] — class schema with tags, parents, field types
//! - [`object_graph`] — document history, string-record extraction
//! - [`elem_table`] — `Global/ElemTable` parser
//! - [`partitions`] — `Partitions/NN` header + chunk splitter
//! - [`compression`] — truncated-gzip decode
//! - [`class_index`] — fast class-name inventory
//! - [`corpus`] — cross-version delta analysis
//! - [`writer`] — byte-preserving OLE round-trip
//! - [`redact`] — shared PII scrubbers for all CLIs
//! - [`ifc`] — IFC export scaffold
//! - [`error`] — [`Error`] + [`Result`] aliases
//! - [`streams`] — named constants for every invariant OLE stream
//!
//! ## Safety
//!
//! This crate is read-only for the OLE container and performs no
//! privileged operations. Files are opened via the `cfb` crate with
//! standard POSIX `read` semantics. The [`writer::copy_file`] function
//! writes a new file at a caller-specified path.
//!
//! All decompression uses `flate2` in safe Rust (`miniz_oxide` backend
//! by default, no C toolchain required). No `unsafe` blocks in the
//! public surface.
//!
//! ## License
//!
//! Apache-2.0. See LICENSE. Not affiliated with Autodesk. "Revit" and
//! related marks are trademarks of Autodesk, Inc.
//!
//! rvt-rs is intended as a clean-room interoperability implementation.
//! It does not use Autodesk/ODA SDK sources, leaked documentation, or
//! decompiled proprietary implementation code — see CLEANROOM.md for
//! the formal policy on accepted / forbidden sources. Users with
//! specific legal constraints should evaluate the project with their
//! own counsel. Nothing in this crate's documentation is legal
//! advice.

#![warn(rust_2024_compatibility)]
// SEC-11 + SEC-12 + SEC-13: forbid `unsafe` in the core library.
// Revit files come from untrusted sources, so any raw-pointer or
// uninit-memory manoeuvre is a potential parser vulnerability.
// `forbid` is one step stronger than `deny` — it cannot be locally
// overridden with `#[allow(unsafe_code)]`, so nobody can sneak an
// `unsafe` block through review.
//
// The pyo3 Python bindings are the one legitimate place where
// `unsafe fn` / `unsafe impl` appear (pyo3's `#[pyclass]` macros
// expand into them). SEC-12 moved those bindings to the separate
// `rvt-py` workspace member crate, which has its own `unsafe_code`
// allowance. This root crate is unconditionally `forbid`-ed —
// there is no feature flag or cfg that can turn this off.
#![forbid(unsafe_code)]

pub mod basic_file_info;
pub mod class_index;
pub mod class_tag_map;
pub mod compression;
pub mod corpus;
pub mod elem_table;
pub mod elements;
pub mod error;
pub mod formats;
pub mod geometry;
pub mod ifc;
pub mod object_graph;
pub mod parse_mode;
pub mod part_atom;
pub mod partitions;
pub mod reader;
pub mod redact;
pub mod round_trip;
pub mod streams;
pub mod walker;
pub mod writer;

// Python bindings live in their own workspace member crate
// (`rvt-py/`) as of SEC-12 / SEC-13, so pyo3's unavoidable
// `unsafe impl` / `unsafe fn` macro-expansions don't require
// this crate to relax its unconditional `#![forbid(unsafe_code)]`.
// The wheel on PyPI is still called `rvt`; build it with
// `maturin build --manifest-path rvt-py/Cargo.toml`.

// WASM bindings (VW1-01). Only compiled when the `wasm` feature is
// enabled (typically via
// `wasm-pack build --target web --features wasm --no-default-features`).
// Default Rust builds and the Python wheel build are unaffected.
// The wasm bindings are pure-safe Rust — wasm-bindgen macros do not
// expand into `unsafe`, so this module is compatible with the
// root crate's `#![forbid(unsafe_code)]`.
#[cfg(feature = "wasm")]
pub mod wasm;

pub use error::{Error, Result};
pub use reader::RevitFile;
