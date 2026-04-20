# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed — IFC exporter now emits the full spatial hierarchy

- **`rvt-ifc` output now includes `IfcSite → IfcBuilding → IfcBuildingStorey`**
  with `IfcLocalPlacement` per container and `IfcRelAggregates`
  binding each level to its parent. Previous output was a valid-but-
  empty `IfcProject`; BlenderBIM and IfcOpenShell-based viewers
  accepted it but couldn't render anything because there was no
  spatial structure for them to attach geometry to. The minimal
  `Default Site / Default Building / Level 1` hierarchy now opens as
  a navigable scene directly. Once the walker surfaces real
  `BasePoint` / `Level` / `Building` records from the Revit file,
  these placeholder names and the zero-elevation storey will be
  replaced with the actual values.
- **`make_guid(index)` deterministic GUID generator** — replaces the
  constant `random_guid_stub()` placeholder. Emits 22-character
  strings in the IFC-GUID alphabet (`0-9A-Za-z_$`), prefix `0rvtrs`
  + base-64 big-endian-encoded entity index. Every entity in one
  export now has a distinct GUID; identical models produce
  byte-identical STEP output (STEP text diffs now work).
- **`IfcClassification` + `IfcClassificationReference` +
  `IfcRelAssociatesClassification` emission.** `RvtDocExporter`
  already extracted OmniClass codes from PartAtom (e.g.
  `23.45.12.34`) into `model.classifications`; the STEP writer now
  actually emits them. Each classification source (OmniClass,
  Uniformat, …) gets one `IfcClassification`; each coded item gets
  an `IfcClassificationReference` linked back to its source; the
  project gets one `IfcRelAssociatesClassification` per reference
  binding the code to the root `IfcProject`. BIM consumers that
  track code/category provenance (Solibri, IfcOpenShell
  classification viewer) can now read those codes directly from
  the exported IFC.
- 7 new unit tests total pinning spatial-hierarchy presence,
  entity counts, GUID alphabet, GUID determinism, per-file GUID
  uniqueness, OmniClass classification emission with items + names
  + edition, and a guard that empty classifications produce no
  classification entities. Existing `ifc_roundtrip` integration
  tests continue to pass across the 11-release corpus.

### Added — Python bindings via pyo3 + maturin

- **`rvt` Python package** — `pip install rvt` produces a single wheel
  per OS/arch that works on every Python ≥ 3.8 (via pyo3 `abi3-py38`).
  Pure-Python `rvt` package wraps the compiled `rvt._rvt` extension
  and ships a PEP 561 `py.typed` marker + hand-maintained
  `__init__.pyi` stubs so mypy, pyright, and IDE autocomplete work
  out of the box.
- **`rvt.RevitFile` class** — Python surface onto `RustRevitFile`.
  Properties: `version`, `original_path`, `build`, `guid`,
  `part_atom_title`. Methods: `stream_names()`,
  `missing_required_streams()`, `schema_summary()`,
  `read_adocument()` (returns a dict with the walker's
  `ADocumentInstance` serialised to native Python types), and
  `write_ifc()` (returns the IFC4 STEP text).
- **`rvt.rvt_to_ifc(path)`** one-shot helper — equivalent to
  `RevitFile(path).write_ifc()` for callers that just want the IFC
  string and never touch the intermediate object.
- **`RevitFile.schema_json()`** — returns the full schema as a JSON
  string (parse with `json.loads` to get a dict equivalent to
  Rust's `SchemaTable`). Zero-copy relative to the decoded schema;
  ~1-2 MB per typical Revit family. `schema_summary()` remains the
  cheap counts-only variant. Two new pytest tests cross-check that
  summary counts match `schema_json()`'s full-parse counts and that
  the `ADocument` class (the walker's target) is always present.
- **`RevitFile.basic_file_info_json()`** — `BasicFileInfo` as JSON
  in one call. Single-call equivalent of the four individual
  getters (`version` / `original_path` / `build` / `guid`) plus
  any future fields. Returns `None` when the stream is unparseable.
- **`RevitFile.part_atom_json()`** — `PartAtom` as JSON in one
  call. Superset of `part_atom_title` — also carries `id`,
  `updated`, `taxonomies`, `categories`, `omniclass`, and `raw_xml`
  (the original XML for lossless downstream reuse). Returns `None`
  when the stream is absent (common on project `.rvt` files).
- Two new pytest tests pin `basic_file_info_json` ↔ individual
  getters agreement, and `part_atom_json` ↔ `part_atom_title`
  agreement plus presence of the structural keys.
- **`RevitFile.read_stream(name)`** — return the raw bytes of an
  OLE stream by name as a Python `bytes` object. Accepts either
  path form (`"/Formats/Latest"` or `"Formats/Latest"`). Raises
  `IOError` for unknown streams. Use `stream_names()` to enumerate
  what's available. Opens up forensic-inspection use cases the
  announcement draft calls out (reading raw bytes without the
  Rust-API dependency). Three new pytest tests pin bytes
  round-trip, path-normalisation equivalence, and
  missing-stream-raises semantics.
- **CI wheel build matrix** (`.github/workflows/ci.yml` `python-wheel`
  job) — `PyO3/maturin-action@v1` builds a release wheel on Ubuntu,
  macOS, and Windows runners, installs it into the runner's Python,
  runs the pytest integration suite (`tests/python/test_rvt.py`), and
  uploads the wheel as a workflow artifact. Any regression in the
  Python surface fails CI across all three OSes.
- **38 pytest integration tests** covering module surface, error
  handling on missing / non-CFB files, happy-path reads against every
  one of the 11 corpus releases (2016–2026), cross-version
  `read_adocument` consistency-band checks, and `write_ifc` output
  shape. Gracefully skips with a clear message when
  `_corpus/rac_basic_sample_family` is absent so local runs work
  without LFS fetches.
- **`docs/python.md`** — full Python API reference (install, quick
  start, tables per method, return shapes, error handling,
  limitations, troubleshooting, contribution notes).
- **`docs/rvt-python-quickstart.ipynb`** — 15-cell Jupyter notebook
  mirror of `docs/python.md` for anyone who prefers an interactive
  walkthrough.
- **`.github/workflows/publish.yml`** — PyPI release workflow. Fires
  on tag push (`v*`) or `workflow_dispatch`. Builds wheels on
  Ubuntu / macOS / Windows via `PyO3/maturin-action@v1`, builds the
  sdist on Ubuntu, downloads every artifact into one `dist/`, and
  publishes via `pypa/gh-action-pypi-publish` using PyPI's Trusted
  Publisher flow (OIDC) — no `PYPI_API_TOKEN` secret stored in the
  repo. Supports `workflow_dispatch` with `test-pypi: true` for
  TestPyPI dry runs. Per-tag releases will cover every Python ≥ 3.8
  on mainstream OSes with one wheel each.

Design principle: expose only the stable high-level surface
(metadata, walker-read ADocument, IFC export). The low-level
byte-pattern / `FieldType` machinery stays in Rust; Python callers
get dicts and strings, no wrapper types to learn. To rebuild the
wheel locally: `maturin build --release --features python`.

### Added — Layer 5: first end-to-end `rvt → ifc` pipeline

- **`rvt::ifc::step_writer::write_step`** — pure-Rust IFC4 STEP
  serializer. Takes an `IfcModel`, produces spec-valid ISO-10303-21
  text with all required framework entities (IfcPerson,
  IfcOrganization, IfcApplication, IfcOwnerHistory, IfcSIUnit×4,
  IfcUnitAssignment, IfcGeometricRepresentationContext, IfcProject).
  No IfcOpenShell dependency. No `unsafe`. 4 new unit tests pinning
  envelope shape, escaping, and required entities.
- **`rvt::ifc::RvtDocExporter`** — concrete `Exporter` that
  populates `IfcModel` from a `RevitFile`. Extracts project name
  from PartAtom (falls back to BasicFileInfo path), builds a
  description string from version + id, pulls OmniClass codes into
  `ClassificationSource::OmniClass`.
- **`rvt-ifc` CLI** — ninth shipped binary. `rvt-ifc input.rfa`
  writes `input.ifc` next to the input. `rvt-ifc -o path input.rfa`
  overrides. `--null` uses the empty-project exporter for
  STEP-writer testing.

First end-user deliverable for Layer 5: `cargo run --release --bin
rvt-ifc -- sample.rfa` produces a ~1 KB IFC4 file that
IfcOpenShell, BlenderBIM, and buildingSMART validators can read.
Geometry and per-element entities are pending walker expansion;
this v1 covers document-level metadata.

### Fixed
- **Windows CFB stream-name path separator.** `RevitFile::stream_names()`
  returned backslash-separated paths on Windows (`Formats\Latest`)
  because `Path::display()` uses host-native separators. Now
  normalises to forward-slashes across all OSes so
  `has_revit_signature()` and equivalent cross-stream comparisons
  work uniformly. This was the root cause of the Windows-only
  integration-test failures on the 2016 sample.
- **MSRV compliance.** Removed a `if let ... && ...` let-chain that
  crept in; let-chains require Rust 1.88+ and the crate's MSRV is
  1.85. Rewrote as nested `if let { if cond { ... } }`.

### Added — Layer 5a walker + rvt-doc CLI

- **`src/walker.rs` module** — first end-to-end schema-directed
  instance reader. Exposes `read_adocument(&mut RevitFile) ->
  Result<Option<ADocumentInstance>>` returning `ADocumentInstance {
  entry_offset, version, fields }` where each field is one of
  `InstanceField::{Pointer, ElementId, RefContainer, Bytes}`.
- **`rvt-doc` CLI** — eighth shipped binary. Dumps ADocument's
  instance fields as human-readable text or machine-readable JSON
  with `--json`. Respects `--redact` for user-path scrubbing.
- **Cross-version detection** — hybrid entry-point finder that
  combines a sequential-id-table heuristic with a scoring-based
  brute-force fallback. Works on all 11 releases (Revit 2016–2026)
  with five cross-version-consistent bands:
  2016–17 / 2018 (solo) / 2019–20 / 2021–23 / 2024–26.
- **`RevitFile::missing_required_streams()`** — diagnostic form of
  `has_revit_signature`. Returns the list of required stream names
  not found in the file, so "signature invalid" errors can point
  at the specific missing stream.

### Research progress

- **Q6.3**: refuted Q6.2's "post-history bytes are ADocument"
  hypothesis. The 131-record table at the post-history boundary
  is a multi-table directory, not ADocument's instance.
- **Q6.4**: directory u16 body values are not cross-stream
  references. Two sequential-id tables (Table A + Table B) exist
  in Global/Latest.
- **Q6.5-A/B**: post-Table-B region at 0x0f67 (2024) is where
  ADocument's actual instance data lives. 33× class-tag density
  vs uniform-random baseline.
- **Q6.5-C**: first-pass walker drifts after field 2 because
  Container wire encoding was wrong.
- **Q6.5-D**: Container wire is two-column `[u32 count][12 × 6B
  ids][u32 count][12 × 6B masks]` = 152 bytes for count=12.
- **Q6.5-E**: walker reads 8/13 fields cleanly on Revit 2024.
- **Q6.5-F**: walker reads ADocument on all 11 releases with
  cross-version-byte-identical output within each version band.

## [0.1.1] — 2026-04-19

### Added
- **CI-enforced 100% schema-field classification.** New integration
  test `tests/field_type_coverage.rs` opens every file in the 11-version
  `rac_basic_sample_family` corpus, parses the schema, and asserts zero
  fields decode to `FieldType::Unknown`. Fails if any release regresses
  or if the corpus is incomplete — no silent-skip. CI job fetches the
  corpus from [phi-ag/rvt](https://github.com/phi-ag/rvt) at build time
  via `actions/checkout@v4` with LFS (rvt-rs does not redistribute the
  Autodesk-owned sample files; see SECURITY.md).
- `FieldType` enum with 8 variants (`Primitive`, `String`, `Guid`,
  `ElementId`, `ElementIdRef`, `Pointer`, `Vector`, `Container`) —
  classifies **100.00% of all 13,570 schema fields** across the 11-version
  reference corpus (Revit 2016–2026). Zero fields decode to `Unknown`.
  Evidence: `examples/unknown_bytes_deep.rs` against every sample file.
- `ClassEntry.tag`, `.parent`, `.ancestor_tag`, `.declared_field_count`,
  `.was_parent_only` — richer schema metadata with cross-release stability.
- `writer::write_with_patches` + `StreamPatch` / `StreamFraming` types —
  stream-level modifying writer; verified end-to-end round-trip on
  `Formats/Latest`.
- `compression::truncated_gzip_encode` + `truncated_gzip_encode_with_prefix8`
  — inverse of `inflate_at`, producing Revit-compatible gzip bytes.
- `redact` module with `redact_path_str` + `redact_sensitive` —
  shared PII scrubber used by every CLI's `--redact` flag.
- `rvt-analyze` CLI — one-shot forensic analysis. 7 subsystems: identity,
  history, format anchors, schema, schema→data link, content metadata,
  disclosure scan. `--json`, `--section`, `--redact`, `--quiet`,
  `--no-color`.
- `rvt-info --redact` and `rvt-history --redact` — PII propagation to the
  other shipped CLIs.
- `elem_table` + `partitions` modules — Global/ElemTable + Partitions/NN
  header parsers.
- `ifc` module — Layer 5 scaffold: `IfcModel`, `Exporter` trait,
  `NullExporter`, full Revit-class → IFC-entity mapping plan.
- `writer::copy_file` — byte-preserving OLE round-trip (13 streams
  identical, verified).
- 14 new reproducible probes under `examples/` covering every FACT in
  the reconnaissance report.
- `tools/bench.sh` hyperfine benchmark harness + `docs/benchmarks.md`.
- First publicly-available RVT tag-drift table — `docs/data/tag-drift-2016-2026.csv`
  (122 classes × 11 releases) + `tag-drift-heatmap.svg`.
- First publicly-documented Revit format-identifier GUID
  (`3529342d-e51e-11d4-92d8-0000863f27ad`) — stable across every Revit
  release 2016-2026.

### Changed
- Library surface reorganised; `src/lib.rs` has a proper crate-level
  doc with a quickstart example, moat-layer table, and module inventory.
- `FieldType::Primitive` now carries `{kind, size}` instead of
  `{size_hint}`.
- `FieldType::Container` now carries a `kind: u8` field marking the
  element base type (so `Container<u32>` is distinguishable from
  `Container<f64>` / `Container<ref>`). Existing consumers that
  destructure with `..` continue to work.
- `FieldType::decode` is now panic-safe on short inputs: 0/1/2/3-byte
  slices produce either `Unknown` or a typed variant with an empty body
  rather than a bounds-check panic.
- `scan_fields_until_next_class_bounded` respects `declared_field_count`
  — fixes the over-reader that bled from HostObjAttr into Symbol's
  fields.

### Research findings (Phase 4c)

- **Q4**: The u16 "flag" in each tagged-class preamble is an
  **ancestor-class reference**, not a bitmask. 9/9 non-zero values in
  the 2024 sample resolve to named classes in the same schema.
- **Q5**: Decoded the field `type_encoding` byte sequence. 9 category
  discriminators + sub-type variants.
- **Q5.1**: Extended to 84% coverage — wider primitive discriminators
  (`0x01 bool`, `0x02 u16`, `0x05 u32`, `0x06 f32`, `0x07 f64`,
  `0x08 string`, `0x09 GUID`, `0x0b u64`).
- **Q5.2**: Extended to **100.00%** coverage across the 11-version
  corpus. Generalized `{scalar_base} 0x0010 ...` → `Vector<base>` and
  `{scalar_base} 0x0050 ...` → `Container<base>` for every scalar base
  (previously only `0x07 0x10` and `0x0e 0x50` were mapped). Added the
  `0x0d` point/transform base (seen only in composite form), the
  `0x08 0x60 ...` alternate string encoding, the `ElementIdRef { tag,
  sub }` variant (for references that carry a specific referenced-class
  tag — 80+ fields per release use this), the deprecated `0x03` i32-
  alias (2016–2018 only, 5 fields), and robust handling of truncated
  2-byte `{kind}{modifier}` headers (schema-parse boundary artifacts).
- **Q6**: `Global/Latest` is **not** an index + heap. It's a flat
  TLV stream.
- **Q6.1**: Instance data is **schema-directed** (tag-less, protobuf-
  style). Decoding requires schema-first sequential walk from a known
  entry point.
- **Q6.2**: Initial hypothesis — entry point located at offset `0x363`
  in the 2024 sample (right after the document-upgrade-history
  UTF-16LE block). Confidence 0.6. **Refuted by Q6.3.**
- **Q6.3 CORRECTION**: The Q6.2 entry-point hypothesis is refuted by
  rigorous validation against the 11-version corpus. The bytes at the
  post-history boundary are NOT ADocument's 13-field instance — they
  are a multi-table directory / reference-pool with ~131 sequentially
  numbered records per release (stable count across all 11 years,
  unchanged from the 13 that would be expected if this were
  ADocument). Body-size does not correlate with FieldType; body u16
  values do not resolve to schema class tags (0/131 hit). ADocument's
  actual location in `Global/Latest` (or another stream) is not yet
  known — decoding the directory table format is the next open
  research question (Q6.4+). Probes: `examples/adocument_walk.rs`,
  `examples/post_directory.rs`, `examples/directory_class_lookup.rs`.
  See `docs/rvt-moat-break-reconnaissance.md` §Q6.3 for full evidence.
- **Q7**: `Partitions/NN` trailer u32 fields are **not** per-chunk
  offsets. Gzip-magic scan remains correct.

## [0.1.0] — 2026-04-19

Initial public release.

- OLE2/MS-CFB container reader (via `cfb`) — Layer 1.
- Truncated-gzip decompression (via `flate2`) — Layer 2.
- Per-stream framing for `Formats/Latest`, `Global/Latest`,
  `Global/ElemTable`, `Partitions/NN`, `Contents`, `PartitionTable`,
  `RevitPreview4.0` — Layer 3.
- Schema table parser: class names + fields + tags + parent classes
  + declared field counts + cross-release tag-drift map — Layer 4a.
- Phase D moat proof: class tags from `Formats/Latest` occur in
  `Global/Latest` at ~340× uniform-random rate — Layer 4b.
- `FieldType` enum with 7 initial variants (Primitive, ElementId,
  Pointer, Vector, Container, String, Guid). **84% field-type
  classification** on a typical Revit 2024 sample family — Layer 4c.
- Stream-level modifying writer (`write_with_patches`) with
  byte-preserving round-trips verified on all 13 streams — Layer 6.
- Seven shipped CLIs: `rvt-analyze`, `rvt-info`, `rvt-schema`,
  `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`.
- Full PII-redaction (`--redact`) across every CLI.
- First publicly-documented Revit format-identifier GUID
  (`3529342d-e51e-11d4-92d8-0000863f27ad`), stable across every
  Revit release 2016–2026.
- First public RVT tag-drift table: 122 classes × 11 releases CSV
  plus SVG heatmap.
