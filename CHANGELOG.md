# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
