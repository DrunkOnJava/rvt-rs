# r/rust post — rvt-rs

Target: [r/rust](https://reddit.com/r/rust). Rust-first audience. No marketing copy. Technical specificity. Markdown supported.

## Title options

1. **rvt-rs: a Rust library for reading Autodesk Revit files (.rvt/.rfa, 2016–2026)** — *preferred*. Leads with what it is, scopes the format coverage, no adjectives.
2. Open-source Rust reader for Autodesk Revit (OLE/CFB + truncated-gzip + schema-directed walker)
3. Announcing rvt-rs 0.1.2 — Revit file introspection in Rust, Apache-2

Option 1 is preferred: it states the category (Rust library), the subject (Autodesk Revit), and the corpus bound (2016–2026) in one line without hyperbole. Options 2 and 3 front-load jargon or read as a release announcement rather than a "here's a library, please beat it up" post.

## Body

rvt-rs is a Rust library that opens Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) without a Revit installation, decodes the embedded `Formats/Latest` class schema, walks the object graph through a schema-directed instance reader, and emits IFC4 STEP. I've been posting about DWG in a separate thread; this is the Revit sibling project.

### Technical surface

- **`forbid(unsafe_code)` posture — honest state.** Every non-Python module contains zero `unsafe` blocks, CONTRIBUTING.md declares "no `unsafe` in the library crate", and `src/ifc/step_writer.rs` documents itself as `#![deny(unsafe_code)]`-clean. But the crate root (`src/lib.rs`) currently has only `#![warn(rust_2024_compatibility)]` — there is no top-level `forbid(unsafe_code)` attribute. Enforcing it at the root is tracked as SEC-11; the reason it's still pending is the planned workspace split (SEC-12: carve into `rvt-core` / `rvt-cli` / `rvt-py` / `rvt-ifc`) so pyo3's lone `#![allow(unsafe_op_in_unsafe_fn)]` in `src/python.rs` — gated behind the optional `python` feature — stays isolated to a leaf crate. Default Rust builds pull zero unsafe.
- **MSRV 1.85, Rust 2024 edition** (`Cargo.toml`: `edition = "2024"`, `rust-version = "1.85"`).
- **9 dependencies, no async runtime.** `cfb 0.11` (MS-CFB container), `flate2 1.0` with the pure-Rust `miniz_oxide` backend (no C toolchain), `encoding_rs 0.8` (recovers from Revit's malformed UTF-16LE surrogate pairs where stdlib panics), `quick-xml 0.36`, `serde` + `serde_json`, `clap 4`, `anyhow`, `thiserror`. `pyo3` is optional behind the `python` feature.
- **Parse-safety: bounded decompression + open limits.** Revit streams are truncated-gzip (10-byte standard header, raw DEFLATE, no trailing CRC/ISIZE — stdlib gzip parsers refuse them). `compression::inflate_at_with_limits` enforces `InflateLimits { max_output_bytes }`; `RevitFile::open_with_limits` takes `OpenLimits { max_file_bytes, max_stream_bytes, inflate_limits }` and stat-checks before reading. Defaults: 2 GiB file, 256 MiB per stream, 256 MiB per inflate. A 20-byte compressed bomb that would decompress to 1 MB is rejected as `Error::DecompressLimitExceeded` rather than allocating; unit test pins the behavior.
- **Schema-directed walker.** `Formats/Latest` decompresses to a tagged class table: 395 classes, 13,570 fields, every encoding classified across the 11-release corpus (100% of fields, zero `Unknown`). The `FieldType` enum has 8 variants (Primitive, String, Guid, ElementId, ElementIdRef, Pointer, Vector, Container) with 11 discriminator bytes mapped. `walker::decode_instance(bytes, offset, &ClassEntry) -> DecodedElement` is the generic dispatch; each of the 54 per-class decoders is a few hundred lines that projects the schema-typed byte run into a typed Rust struct.

### What's shipped end-to-end

- **54 per-class decoders** (from `elements::all_decoders()` — see `docs/compatibility.md` §3). Architectural hosts + openings (Wall, Floor, Roof, Ceiling, Door, Window, CurtainWall + grid/mullion/panel), circulation (Stair, Railing), zoning (Room, Area, Space), structural (Column, Beam, Foundation, Rebar, ReferencePlane), datum (Level, Grid, BasePoint, SurveyPoint, ProjectPosition), furnishings, styling, project organization, drafting.
- **IFC4 STEP exporter.** Every element routes through a category map to a valid IFC entity (`IfcWall` / `IfcSlab` / `IfcDoor` / …); spatial hierarchy ships (`IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey`); elements are wired with `IfcRelContainedInSpatialStructure`; doors/windows get `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement` when host + extrusion are supplied; property sets via `IfcRelDefinesByProperties`; materials via `IfcRelAssociatesMaterial`. Output is deterministic under `StepOptions { timestamp }`, ISO-10303-21 Unicode escaping is correct. `tests/fixtures/synthetic-project.ifc` opens cleanly in BlenderBIM / IfcOpenShell.
- **9 CLIs** (`rvt-analyze`, `rvt-info`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`) plus 30+ reproducible probes under `examples/`, and **281 tests**. Integration tests run against the public `rac_basic_sample_family` Git LFS corpus and skip gracefully when absent, so `cargo test` is green out of the box.

### What is honestly missing

- **No write path for Revit files** beyond byte-preserving round-trip + stream-level `write_with_patches`. Field-level semantic writes are Phase 7.
- **No pre-2016 support.** Earlier Revit releases use different compression + schema framing; no corpus coverage, no claim.
- **No encrypted / password-protected models.** CFB open succeeds; inner streams are gibberish.
- **Walker is partial on 2016–2023.** `walker::read_adocument` is reliable on 2024–2026; on older releases it returns `Ok(None)` when the entry-point detector can't find a high-confidence offset rather than emitting possibly-wrong fields.
- **MEP decoders pending.** LightingFixture / ElectricalEquipment / MechanicalEquipment / PlumbingFixture / SpecialtyEquipment are category-mapped to their IFC types but fall through to the generic walker; unknowns route to `IfcBuildingElementProxy`.
- **No annotation / dimension / tag / legend decoders.** Sheets, schedules, and views have class-level decoders; the drawn annotations themselves do not.
- **Geometry extraction is Phase 5.** IFC extrusion helpers produce valid `IfcExtrudedAreaSolid` when the caller supplies dimensions — the reader does not yet recover location curves, profile shapes, or brep from the byte stream.

### Why I built this

The AEC industry runs on proprietary file formats with a thick tooling moat. Autodesk's own `revit-ifc` exporter runs *inside* Revit on Windows, so it can only emit what the Revit API surfaces, and openBIM users have been [publicly documenting the resulting data loss](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM) for years. I wanted a byte-level reader that composes into a Rust toolchain and could in principle carry more than the API chooses to expose once per-element decoders and geometry catch up. This is the current cut: 100%-classified schema layer (CI-gated), walker beachhead on 2024–2026, IFC exporter with a real spatial tree, openings, and materials. No claim of parity with commercial tools.

Source policy: clean-room. No Autodesk SDK, no leaked docs, no decompiled proprietary code. Formal policy in `CLEANROOM.md`; legal basis (*Sega v. Accolade*, *Sony v. Connectix*, EU Directive 2009/24/EC Art. 6) in `NOTICE`.

### Review welcome

Interested in Rust-reviewer feedback on: the `InflateLimits` / `OpenLimits` surface (sane defaults? should `max_stream_bytes` be per-open or per-call?); whether top-level `forbid(unsafe_code)` should land now or after the workspace split; the `FieldType` enum and `walker::decode_instance` dispatch — the load-bearing piece of the schema layer; and the fuzzing plan — cargo-fuzz targets `fuzz_open_bytes`, `fuzz_gzip_header_len`, `fuzz_inflate_at_with_limits`, `fuzz_parse_schema`, `fuzz_walker_entry_detect`, `fuzz_step_writer` are scoped (SEC-14 through SEC-25) but none are shipped yet. Crash corpora welcome.

Issues, PRs, and "here's how the Rust version of X handles this" comments all welcome.

- **Repo:** https://github.com/DrunkOnJava/rvt-rs
- **Crate:** https://crates.io/crates/rvt
- **Docs:** https://docs.rs/rvt
- **License:** Apache-2.0
- **Not affiliated with Autodesk.** "Autodesk" and "Revit" are trademarks of Autodesk, Inc.; use here is nominative.

— Griffin Long
