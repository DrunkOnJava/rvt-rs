//! # rvt-rs · open reader for Autodesk Revit files
//!
//! Reads `.rvt`, `.rfa`, `.rte` and `.rft` files without Revit or any
//! Autodesk component: metadata, previews and the embedded schema of every
//! release from 2016 to 2026, and, on Revit 2024 and 2025 project files, the
//! building itself (typed elements with Revit's ElementIds, names, types,
//! storeys, materials, layers, GlobalIds and some type parameters), written
//! out as IFC4, glTF or plan SVG.
//!
//! What it recovers is measured against Revit's own IFC export of the same
//! file, model by model; it is a reader, not a converter for every model.
//! `docs/supported-profile.md` and `docs/support-matrix.json` in the
//! repository say exactly what is read on which input, and every export
//! comes with diagnostics naming what was approximated or left out.
//!
//! ## Read a file's identity
//!
//! ```no_run
//! use rvt::RevitFile;
//!
//! let mut rf = RevitFile::open("model.rvt")?;
//! let summary = rf.summarize()?;
//! println!("Revit {} ({})", summary.version, summary.build.as_deref().unwrap_or("—"));
//! # Ok::<(), rvt::Error>(())
//! ```
//!
//! ## List the elements of a Revit 2024 or 2025 project
//!
//! ```no_run
//! use rvt::RevitFile;
//! use rvt::ifc::RvtDocExporter;
//! use rvt::ifc::entities::IfcEntity;
//!
//! let mut rf = RevitFile::open("project.rvt")?;
//! let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
//! for entity in &result.model.entities {
//!     if let IfcEntity::BuildingElement { ifc_type, name, type_guid, .. } = entity {
//!         // `type_guid` carries the Revit ElementId (the IFC Tag).
//!         println!("{ifc_type} {name} {}", type_guid.as_deref().unwrap_or("-"));
//!     }
//! }
//! // What was approximated or left out, as the CLI's --diagnostics writes it.
//! println!("{}", serde_json::to_string_pretty(&result.diagnostics).unwrap());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Write IFC4 and glTF
//!
//! ```no_run
//! use rvt::RevitFile;
//! use rvt::ifc::{Exporter, RvtDocExporter, gltf::model_to_glb, write_step};
//!
//! let mut rf = RevitFile::open("project.rvt")?;
//! let model = RvtDocExporter.export(&mut rf)?;
//! std::fs::write("project.ifc", write_step(&model))?;
//! std::fs::write("project.glb", model_to_glb(&model))?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Format overview
//!
//! - **Container**: Microsoft Compound File Binary Format 3.0 (`[MS-CFB]`)
//! - **Compression**: *truncated-gzip*, a 10-byte gzip header followed by raw
//!   DEFLATE, and, for the database streams and `Formats/Latest`, stored in
//!   checksum pages that are stripped before inflating
//!   ([`compression::inflate_stream_at`]).
//! - **Schema**: `Formats/Latest` declares every serialized class and its
//!   fields ([`formats::parse_schema`]).
//! - **Elements**: `Global/ElemTable` declares the ElementIds; each element's
//!   record in the `Partitions/*` streams carries its category, class,
//!   bounding box and references ([`partition_element_records`]).
//!
//! ## Module overview
//!
//! - [`reader`] — [`RevitFile`], the main entry point
//! - [`basic_file_info`] — version, GUID, build tag, creator path, worksharing text block
//! - [`metadata`] — [`metadata::FileMetadata`]: document identity, readable without loading the whole file
//! - [`part_atom`] — Atom XML with OmniClass + taxonomies
//! - [`project_information`] — project-file `ProjectInformation` ZIP → Atom entry
//! - [`formats`] — class schema with tags, parents, field types
//! - [`object_graph`] — document history, string-record extraction
//! - [`elem_table`] — `Global/ElemTable` parser
//! - [`partitions`] — `Partitions/NN` header + chunk splitter
//! - [`partition_scanner`] — version-gated generic partition record candidates
//! - [`compression`] — truncated-gzip decode
//! - [`class_index`] — fast class-name inventory
//! - [`corpus`] — cross-version delta analysis
//! - [`identity`] — document-scoped ElementId / UniqueId contracts (Phase 1)
//! - [`evidence`] — evidence tiers + research edge ledgers (Phase 1)
//! - [`es_refs`] — ES reference occurrence locator contracts (Phase 1; no decoder)
//! - [`relations`] — experimental relation-domain registry + SCC/quarantine stubs
//! - [`capability`] — honest capability manifest snapshot for doctor/CLI
//! - [`transmission_data`] — TransmissionData UTF-16LE detect + opportunistic XML/UUID/path extract
//! - [`compound_framing`] — compound ArcWall `0x0821` marker tokenization (research; no opening decode)
//! - [`writer`] — byte-preserving OLE round-trip
//! - [`redact`] — shared PII scrubbers for all CLIs
//! - [`cli`] — behaviour shared by the shipped CLIs (quiet exit on a closed pipe)
//! - [`ifc`] — IFC4 / glTF / plan SVG / CSV export and diagnostics
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

pub mod arc_wall_record;
pub mod basic_file_info;
pub mod capability;
pub mod class_index;
pub mod class_tag_map;
pub mod cli;
pub mod compound_framing;
pub mod compression;
pub mod control;
pub mod corpus;
pub mod elem_table;
pub mod element_record_beam_cuts;
pub mod element_record_column_cuts;
pub mod element_record_level_refs;
pub mod element_record_plan_profiles;
pub mod element_record_storeys;
pub mod element_record_wall_joins;
pub mod elements;
pub mod error;
pub mod es_refs;
pub mod evidence;
pub mod formats;
pub mod geometry;
pub mod identity;
pub mod ifc;
pub mod level_bind;
pub mod metadata;
pub mod native_document;
pub mod native_element;
pub mod native_empty_faces;
pub mod native_es_catalog;
pub mod native_extensible_storage;
pub mod native_graphics_traversal;
pub mod native_index;
pub mod native_metadata;
pub mod native_parameter_definitions;
pub mod native_parameters;
pub mod native_parametric_mesh;
pub mod native_parametric_surface;
pub mod native_saved_glb;
pub mod native_saved_materials;
pub mod native_saved_mesh;
pub mod native_saved_scene;
pub mod native_segments;
pub mod object_graph;
pub mod parse_mode;
pub mod part_atom;
pub mod partition_arc_walls;
pub mod partition_beam_axes;
pub mod partition_compound_structure;
pub mod partition_connector_pairs;
pub mod partition_curtain_materials;
pub mod partition_curve_fields;
pub mod partition_design_options;
pub mod partition_element_records;
pub mod partition_element_records_2023;
pub mod partition_fitting_sizes;
pub mod partition_id_objects;
pub mod partition_ifc_export_overrides;
pub mod partition_instance_transforms;
pub mod partition_level_records;
pub mod partition_materials;
pub mod partition_mep_systems;
pub mod partition_name_candidates;
pub mod partition_names;
pub mod partition_pipe_axes;
pub mod partition_roof_slopes;
pub mod partition_room_boundaries;
pub mod partition_room_parameters;
pub mod partition_scanner;
pub mod partition_schema_mvp;
pub mod partition_stairs;
pub mod partition_type_materials;
pub mod partition_type_parameters;
pub mod partition_type_records;
pub mod partitions;
pub mod project_information;
pub mod reader;
pub mod rect_opening_index;
pub mod redact;
pub mod relations;
pub mod revit_global_ids;
pub mod round_trip;
pub mod schema_registry;
pub mod streams;
pub mod transmission_data;
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
