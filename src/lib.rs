//! # rvt-rs · open reader for Autodesk Revit files
//!
//! Reads `.rvt`, `.rfa`, `.rte` and `.rft` files without Revit or any
//! Autodesk component: metadata, previews and the embedded schema of every
//! release from 2016 to 2027, and, on Revit 2024 and 2025 project files, the
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
//! The examples below run in CI against the synthetic project file the
//! repository ships, `corpus/tier1/architectural-2024/architectural-2024.rvt`.
//!
//! ## Read a file's identity
//!
//! ```
//! use rvt::RevitFile;
//!
//! let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
//! let summary = rf.summarize_strict()?;
//! println!("Revit {} ({})", summary.version, summary.build.as_deref().unwrap_or("-"));
//! # Ok::<(), rvt::Error>(())
//! ```
//!
//! ## List the elements of a project
//!
//! ```
//! use rvt::RevitFile;
//! use rvt::ifc::RvtDocExporter;
//! use rvt::ifc::entities::IfcEntity;
//!
//! let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
//! let result = RvtDocExporter.export_with_diagnostics(&mut rf)?;
//! for entity in &result.model.entities {
//!     if let IfcEntity::BuildingElement { ifc_type, name, type_guid, .. } = entity {
//!         // `type_guid` carries the Revit ElementId (the IFC Tag).
//!         println!("{ifc_type} {name} {}", type_guid.as_deref().unwrap_or("-"));
//!     }
//! }
//! // What was approximated or left out, as the CLI's --diagnostics writes it.
//! println!("{}", serde_json::to_string_pretty(&result.diagnostics)?);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Write IFC4 and glTF
//!
//! ```
//! use rvt::RevitFile;
//! use rvt::ifc::{Exporter, RvtDocExporter, gltf::model_to_glb, write_step};
//!
//! let mut rf = RevitFile::open("corpus/tier1/architectural-2024/architectural-2024.rvt")?;
//! let model = RvtDocExporter.export(&mut rf)?;
//! let out = std::env::temp_dir().join(format!("rvt-doc-example-{}", std::process::id()));
//! std::fs::create_dir_all(&out)?;
//! std::fs::write(out.join("project.ifc"), write_step(&model))?;
//! std::fs::write(out.join("project.glb"), model_to_glb(&model))?;
//! # std::fs::remove_dir_all(&out)?;
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
//!   bounding box and references (`research::partition_element_records`).
//!
//! ## Module overview
//!
//! These modules are the stable API: from 1.0 they change only in a major
//! release (`RELEASE.md`, "What 1.0 keeps stable").
//!
//! - [`reader`]: [`RevitFile`], the entry point: open a file from a path or bytes, with resource limits
//! - [`error`]: [`Error`] and [`Result`]
//! - [`streams`]: names of the OLE streams every Revit file carries
//! - [`compression`]: truncated-gzip and checksum-paged stream decoding
//! - [`basic_file_info`]: `BasicFileInfo`: version, GUID, build, creator path, worksharing
//! - [`metadata`]: [`metadata::FileMetadata`]: what a file is, without reading the whole model
//! - [`part_atom`]: `PartAtom` Atom XML: family identity, OmniClass and taxonomies
//! - [`project_information`]: a project file's `ProjectInformation` Atom entry
//! - [`transmission_data`]: `TransmissionData`: encoding detection and the links it records
//! - [`formats`]: `Formats/Latest`: every serialized class, its tag, parent and field types
//! - [`class_index`]: a fast inventory of the class names a file declares
//! - [`elem_table`]: `Global/ElemTable`: the ElementIds a file declares
//! - [`partitions`]: the `Partitions/NN` stream header and chunk splitter
//! - [`object_graph`]: document history and string records
//! - [`elements`]: typed elements and the per-class decoders
//! - [`arc_wall_record`]: the Revit 2023 ArcWall partition record
//! - [`walker`]: the element walk: every element a file yields, with its provenance
//! - [`parse_mode`]: strict and lossy parse results with diagnostics
//! - [`control`]: cooperative cancellation and progress for long scans
//! - [`geometry`]: points and curves, and the recovery of element geometry from the typed views
//! - [`level_bind`]: fail-closed binding of Levels to building storeys
//! - [`ifc`]: IFC4, glTF, plan SVG and CSV export, with diagnostics
//! - [`capability`]: the capability manifest built from `docs/support-matrix.json`
//! - [`corpus`]: cross-version comparison behind `rvt-corpus`
//! - [`writer`]: byte-preserving copy and stream-level patching
//! - [`round_trip`]: verification that a written file round-trips
//! - [`redact`]: PII scrubbers for anything a report prints
//!
//! The research modules behind the format findings in `reports/` (the
//! partition, native and element-record readers) are under `research`, behind
//! the `unstable-research` feature. They change whenever a finding does and
//! are outside the semver promise.
//!
//! ## Reading the docs
//!
//! `RE-NN` names a reverse-engineering finding, positive or negative. Each
//! is written up in `reports/element-framing/RE-NN-*.md` in the
//! [repository](https://github.com/DrunkOnJava/rvt-rs/tree/main/reports/element-framing),
//! which says what was examined, on which files, and what was measured;
//! most compare against Revit's own export of the same model.
//! `#NNN` is an issue or pull request there.
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
#![warn(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
// Tracker: SEC-11, SEC-12, SEC-13.
// Forbid `unsafe` in the core library.
// Revit files come from untrusted sources, so any raw-pointer or
// uninit-memory manoeuvre is a potential parser vulnerability.
// `forbid` is one step stronger than `deny` — it cannot be locally
// overridden with `#[allow(unsafe_code)]`, so nobody can sneak an
// `unsafe` block through review.
//
// The pyo3 Python bindings are the one legitimate place where
// `unsafe fn` / `unsafe impl` appear (pyo3's `#[pyclass]` macros
// expand into them), so those bindings live in the separate
// `rvt-py` workspace member crate, which has its own `unsafe_code`
// allowance. This root crate is unconditionally `forbid`-ed —
// there is no feature flag or cfg that can turn this off.
#![forbid(unsafe_code)]

pub mod arc_wall_record;
pub mod basic_file_info;
pub mod capability;
pub mod class_index;
#[doc(hidden)]
pub mod cli;
pub mod compression;
pub mod control;
pub mod corpus;
pub mod elem_table;
pub mod element_id;
pub mod elements;
pub mod error;
pub mod formats;
pub mod geometry;
pub mod ifc;
mod le_bytes;
pub mod level_bind;
pub mod metadata;
pub mod object_graph;
pub mod parse_mode;
pub mod part_atom;
pub mod partitions;
pub mod project_information;
pub mod reader;
pub mod redact;
pub mod round_trip;
pub mod streams;
pub mod transmission_data;
mod vector_math;
pub mod walker;
pub mod writer;

#[cfg(feature = "unstable-research")]
pub mod research;
#[cfg(not(feature = "unstable-research"))]
#[allow(
    dead_code,
    unused_imports,
    reason = "without unstable-research, research code only the probes call is unreachable"
)]
mod research;

#[allow(
    unused_imports,
    reason = "keeps every crate::<research module> path working; not every module is named outside research"
)]
pub(crate) use research::{
    element_record_beam_cuts, element_record_column_cuts, element_record_level_refs,
    element_record_plan_profiles, element_record_storeys, element_record_wall_joins, es_refs,
    evidence, identity, native_column_joins, native_connectors, native_document,
    native_duct_shapes, native_element, native_element_parameters, native_element_types,
    native_empty_faces, native_es_catalog, native_extensible_storage, native_family_forms,
    native_graphics_traversal, native_index, native_instance_materials, native_json,
    native_metadata, native_mullion_hosts, native_parameter_definitions, native_parameters,
    native_parametric_mesh, native_parametric_surface, native_phases, native_rendered_content,
    native_saved_mesh, native_saved_scene, native_segments, native_stairs, native_symbol_cuts,
    native_type_materials, native_wall_types, native_walls, partition_arc_walls,
    partition_area_records, partition_beam_axes, partition_column_nodes,
    partition_compound_structure, partition_connector_pairs, partition_curtain_materials,
    partition_curve_fields, partition_design_options, partition_element_records,
    partition_element_records_2023, partition_fitting_sizes, partition_id_objects,
    partition_ifc_export_overrides, partition_instance_transforms, partition_level_parameters,
    partition_level_records, partition_materials, partition_mep_systems, partition_name_candidates,
    partition_names, partition_phases, partition_pipe_axes, partition_roof_slopes,
    partition_room_boundaries, partition_room_parameters, partition_scanner, partition_schema_mvp,
    partition_stairs, partition_type_materials, partition_type_parameters, partition_type_records,
    rect_opening_index, revit_global_ids, schema_registry,
};

// Python bindings live in their own workspace member crate
// (`rvt-py/`), so pyo3's unavoidable
// `unsafe impl` / `unsafe fn` macro-expansions don't require
// this crate to relax its unconditional `#![forbid(unsafe_code)]`.
// The wheel on PyPI is still called `rvt`; build it with
// `maturin build --manifest-path rvt-py/Cargo.toml`.

// Tracker: VW1-01, C6.
// The WASM bindings are their own crate, `rvt-wasm`.

pub use element_id::ElementId;
pub use error::{Error, Result};
pub use reader::RevitFile;
