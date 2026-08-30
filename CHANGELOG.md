# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

Post-`0.1.2` work toward an **inspection-focused `0.2.0` alpha** (see
[`docs/release-0.2.0-plan.md`](docs/release-0.2.0-plan.md)). rvt-rs remains a
Revit inspection / reverse-engineering toolkit with experimental export —
**not** a production Revit→IFC converter for arbitrary projects.

### Added

- **Column instance recovery — 256 of 256 `IFCCOLUMN` on the Core Interior
  full-project export (#204).** `src/partition_element_records.rs` decodes the
  Revit 2024 partition *element-record header*, a fixed 88-byte prologue whose
  first field is the record's own `ElementId` (`u64`), whose sixth is the
  element's Revit `BuiltInCategory` (`i64`, negative, at `+0x12`), and which is
  followed at `+0x50` by a fixed marker and at `+0x58` by the element's model
  bounding box as six `f64` feet. 24880 records of this shape exist on
  `2024_Core_Interior.rvt`; every `ElementId` they carry is declared in
  `Global/ElemTable`, and the decoder requires that join, so a stray byte match
  cannot become an element. `partition_schema_mvp::columns_from_partition_category_records`
  keeps the `OST_Columns` (`-2000100`) records, drops those whose bounding box
  is centred on the plan origin (family-local type envelopes, not placed
  instances), and collapses co-located footprint groups to their highest
  `ElementId` — Revit allocates ids monotonically, so the newest of a
  superseded pair is the live element. The result is exactly the 256 columns
  Revit's own exporter emits: no false positives, no misses, gated at
  tolerance 0. Emitted as `IfcColumn` with a project placement and a
  bounding-box `IfcExtrudedAreaSolid` (an envelope, not a recovered family
  profile) plus an `RvtColumnGeometry` property set that says so.
  `building_elements_with_geometry` on that file goes 0 → 256.
- **`columns` is now a scored, cross-witness-agreed category.**
  `tests/fixtures/project-counts/2024-core-interior-slim.json` moves `columns`
  from `known_gap` (decoder 0) to `known` at 256/256, tolerance 0, so the
  OctetProof verdict for `magnetar-2024-core-interior-slim` compares
  `entity_counts.IFCCOLUMN` across all three lineages instead of excluding it —
  the claimed surface widens from four fields to five, and `IFCCOLUMN` is the
  first field in it with a non-zero expectation (the other four are agreements
  about absence). Both committed verdicts still `PASS`; both `rvt-rs.json`
  observations were regenerated, so their `observation_hash_sha256` changed. A
  new `column-instance-recovery` row in `docs/support-matrix.json` records the
  capability as `verified` with its scope stated: one `BuiltInCategory`, one
  release band, one recorded edge, envelope geometry, Level binding still open.
- **Third independent IFC witness — IFClite** (`tools/ci/witness-ifc-lite`):
  a small Rust binary over the crates.io crate `ifc-lite-core`, pinned at
  `=7.1.1` (`LTplus-AG/ifc-lite`, MPL-2.0 — verified against crates.io and the
  GitHub API on 2026-08-30; the registry previously recorded `MIT` and a bare
  repo slug, both corrected). It counts every manifest `source_ifc_type` by
  exact STEP keyword with its own scanner — no IfcOpenShell code — and emits
  an OctetProof §6.2 observation in the same shape as the Python bridge
  witness. Its canonical payload hashes byte-identically to IfcOpenShell's on
  the element fixture. The crate is its own workspace root and is run as a
  separate process, so MPL code is never linked into the Apache-2.0 tree;
  `tests/witness_registry.rs` now fails if the Cargo pin, the binary's
  `WITNESS_VERSION`, and the registry entry drift apart (spec §9.6). The
  Core Interior verdict now lists three lineages —
  `["ifc-lite", "ifcopenshell", "rvt-rs"]` — one source reader and two
  unrelated bridge readers.
- **Second recorded edge: the full-project export** —
  `tests/fixtures/project-counts/2024-core-interior-slim.json` registers
  `IFC Exports/2024_Core_Interior_slim.ifc` (bfdf36ff…, 1665968 bytes, IFC4,
  Autodesk Revit 24.0.20.20 via ODA SDAI 23.12, 19879 entities), which had
  been an artifact with no manifest. Counts measured with IfcOpenShell 0.8.5
  and independently reproduced by IFClite 7.1.1 and the STEP-constructor
  count: 360 `IFCWALL`, 132 `IFCDOOR`, 256 `IFCCOLUMN`, 116 `IFCSPACE`, 80
  `IFCSLAB`, 15 `IFCBUILDINGSTOREY`, 10 `IFCMATERIAL`, 6 `IFCWINDOW`, 1
  `IFCUNITASSIGNMENT`, 0 `IFCROOF` / `IFCBEAM` / `IFCFLOWTERMINAL` /
  `IFCPROPERTYSET` — all three readers agree exactly. rvt-rs recovers 0 / 0 /
  0 / 18 / 64 / 12, so nine categories are recorded as `known_gap` or
  `decoder_baseline` against #30, #31, #32, #33, #34, #35 and the new #204
  (columns, which had no tracking issue), leaving a four-field claimed
  surface. Observations and a `PASS` verdict are committed under
  `research/witness/magnetar-2024-core-interior-slim/` and gated in the
  `ifcopenshell-validate` job; the registry edge moves from
  `recorded-ungated` to `recorded`. No capability status changed.
- **OctetProof 1.0.0 specification** — `docs/octetproof-spec.md` (CC-BY-4.0,
  2026-08-30), the citable Layer-1 artifact of the cross-witness verification
  protocol. Supersedes the received draft, which stays verbatim at
  `docs/octetproof-spec-draft.md`; §19 lists every correction (jDwgParser is
  GPL-3.0, ACadSharp's coverage is undeclared, LGPL is weak copyleft, ifc-lite's
  license needed verification — resolved in note 4 to MPL-2.0 once the exact
  upstream was pinned, the ACIS reader landed in ACadSharp v3.6.51). §6's
  examples are now the real committed observation/verdict files, §7.2 states the
  geometry tolerance as relative 1e-6 with a manifest-stated absolute floor,
  §10.5 adds `REJECTED_INPUT` and `REPLAY_DRIFT`, and §5.3.1 maps the spec's
  registry vocabulary onto `research/witness-registry.json`, marking
  `ci_eligible`, coverage declarations, determinism attestation, and exact
  version pinning as umbrella scope.
- **Observation and verdict JSON Schemas** —
  `docs/schemas/witness-observation.schema.json` and
  `docs/schemas/witness-verdict.schema.json` (JSON Schema 2020-12) make the
  OctetProof §6.2 / §6.3 shapes machine-checkable; the committed files under
  `research/witness/magnetar-2024-core-interior/` validate against them.
- OctetProof witness mode and verdict gate (docs/octetproof-spec.md,
  docs/verification-protocol.md): `rvt-ifc --observation PATH --artifact-id ID`
  emits a canonical, hashed observation of what the decoder wrote (STEP
  constructor histogram, input SHA-256); `tools/ci/witness-ifcopenshell.py
  --observation` does the same for IfcOpenShell reading the Revit-authored
  reference IFC; `tools/ci/witness-verdict.py` compares them fail-closed
  (`PASS` / `DISAGREE` / `INSUFFICIENT_WITNESSES` /
  `INSUFFICIENT_INDEPENDENT_WITNESSES` / `REJECTED_INPUT` / `MANIFEST_ERROR` /
  `REPLAY_DRIFT`), enforcing the §9.3 independence set from
  `research/witness-registry.json` (`lineage`). First committed artifact:
  `research/witness/magnetar-2024-core-interior/` (two observations + a
  `PASS` verdict on eight categories, four excluded as tracked decoder
  gaps); replayed byte-for-byte in the `ifcopenshell-validate` CI job and
  pinned by `tests/witness_verdict.rs`. `sha2` moves from dev- to regular
  dependency for the input hash.
- **Cooperative cancellation + progress** — `rvt::control::{CancelToken,
  WalkerControl, Stage, ProgressEvent}` and additive `*_with_control` entry
  points beside the existing `*_with_limits` ones
  (`walker::scan_candidates_with_control`, `walker::iter_elements_with_control`,
  `partition_scanner::scan_partitions_with_control`); new `Error::Cancelled`
  variant. `rvt-elements --progress` prints each stage to stderr. Output is
  unchanged when no control is attached.
- **Property-based test suite** — `tests/proptest_parsers.rs` (proptest)
  asserts never-panic / in-bounds behaviour for the public byte parsers and
  JSON round-trips for the ES / fixture research record types on stable Rust.
- **Explicit corpus-path strictness** — when `RVT_PROJECT_CORPUS_DIR` is set,
  a missing directory or zero matching tier-two manifests now fails
  `project_count_fixtures` loudly instead of silently dropping coverage.
- **Revit-hosted oracle runner (untested skeleton)** —
  `tools/oracle/runner/pyrevit/` builds the ES-remap-00 seed and runs
  N1–N4 / R1–R2 / C1–C2 / C3a–C4a, writing `es-observation` records. Written
  without Revit; no ES on-disk layout is asserted.
- **Contributor onboarding** — clone-to-first-PR quickstart in
  `CONTRIBUTING.md`, README "For contributors", `good first issue` label,
  opt-in `.githooks/pre-commit` (`cargo fmt --check`), and
  `tests/binary_inventory.rs` pinning the shipped-binary count.
- **Experimental relation domains + capability doctor** — `relations`
  registry (ES / BIM / ElemTable isolation), Tarjan SCC + condensation +
  quarantine stubs with unit tests; `capability::CapabilityManifest`
  honest snapshot (ArcWall 2023 verified, compound/`es.elementid_remap`
  unsupported); `rvt-capabilities` CLI; evidence ledger JSON round-trip
  helpers; `docs/schemas/capability-manifest.schema.json`. Architecture
  only — not wired to production IFC/topology claims.
- **TransmissionData opportunistic extracts** — UTF-16LE probe now harvests
  UUID / path-like / XML node-name triage tokens when present; empty extract
  list explicitly means unknown, not “no links”. Still no linked-model
  resolution or schema rewrite.
- **Compound `0x0821` framing harness** — `compound_framing` marker
  tokenizer + stamp classification + adversarial f64 collision seed;
  docs under `reports/element-framing/RE-compound-0821-harness.md`. Does
  **not** decode compound openings.
- **IFC `Pset_` validation stub** — `ifc::pset_validate` allow-list /
  reserved-unknown classifier + `docs/ifc/pset-mapping-examples.yaml`
  (ES omitted by default).
- **TransmissionData UTF-16 detect stub** — `transmission_data::TransmissionDataProbe`
  classifies empty / UTF-16LE / opaque without inventing a field layout;
  `RevitFile::transmission_data_probe` exposes it. Linked-model resolution
  still unsupported. `rvt-history` clap/docs honesty: DocumentHistory is a
  UTF-16LE `"Revit "` scan, not a full history object model.
- **Preview PNG IEND trim** — `RevitFile::preview_png` truncates at the
  `IEND` chunk CRC when present (drops trailing OLE junk);
  `preview_png_untrimmed` keeps the forensic full tail.
- **Phase 1 research contracts (H-ES5 prep)** — `DocumentIdentity` /
  `ScopedElementRef` / `SourceSpan`, named `EvidenceTier` + evidence /
  edge ledgers, `EsReferenceOccurrence` + fixture transition types,
  `docs/research/unified-research-report.md`, `research/es-remap/`
  scaffold, and `es-observation` / `es-capability` JSON schemas.
  **Does not** claim ES ElementId remapping works; Phase 2 fixture
  generation remains blocked on a Revit-hosted API oracle.
- **Finding 1 / checksum-page framing (#151, Discussion #112)** —
  gated strip of trailing page checksums before inflate on
  `Partitions/*` and `Global/*` streams; Formats/Latest stays ungated
  by default (#162). Wave 1 stream-evidence harness + Wave 2 narrowed
  paged decompress, writer audit, and evidence matrix under
  `docs/recon/` and `docs/re/`.
- **A10 source_coverage fractions** — export diagnostics measure
  `exported_element_fraction` / `geometry_element_fraction` (and
  `decoded_element_fraction` when ElemTable header `element_count` is
  a trusted denominator). Fail closed with `status: unset` / nulls when
  unknown — never invents percentages.
- **Decode confidence + provenance (M3-07 / #150)** — every
  `DecodedElement` carries confidence/provenance; CLI, Python, and
  viewer expose it; default IFC emission hides low-confidence rows
  (threshold documented with the export modes).
- **Production typed / partition MVP path** — `walker::iter_elements`
  prefers fail-closed typed MVP decoders on `Global/Latest`, merges
  version-gated 2023 ArcWall partition recovers, and merges partition
  MVP recovers for Level / Material / Room / Floor plan-loops plus
  2024 ArcWallRectOpening index rows (ElemTable-confirmed related ids;
  never invents typed Door/Window). IFC maps recovered Levels →
  storeys, Floors → boundary-annotated slabs, Rooms → spaces, Material
  display names → `IfcMaterial`.
- **Viewer confidence UI** — File Status shows export readiness /
  confidence, recovered storey names and material samples, honest
  Parameters row (empty until AProperty host joins); scene tree groups
  under `IFCBUILDINGSTOREY` when elevation evidence allows.
- **Corpus + intake** — redistributable project corpus lanes, community
  corpus open→schema→scaffold validation executed
  (`docs/corpus-hunt-2026-04-21.md`: 222/223), corpus intake checklist
  (`docs/corpus-intake.md`), and CI corpus matrix trimming.
- **Inspect / compare tooling** — `rvt-inspect` user-facing status;
  `rvt-ifc-compare` for export QA (M5-05); IFC export `--mode` gates
  (scaffold / typed / geometry / strict) with diagnostics sidecar.
- **Supporting research docs** — RE-19 (no Door/Window discriminator /
  no schema-field Wall on magnetar corpora) and RE-20 (no recoverable
  Level ElementId map; Floors/Rooms stay Unassigned by evidence).
- **Earlier post-0.1.2 foundations still in tree** — ElemTable layout
  detection + `rvt-elem-table` CLI; walker public APIs; scalar-base
  Container decode (synthetic-verified); sector-preserving CFB
  identity roundtrip; always-on stream patch corpus; Python decoded
  API / `rvt-elements`; generic partition scanner.

### Changed

- **API (breaking, pre-`0.2.0`)** — `elem_table::ElemTableLayout` gains a
  `marker_offset` field (offset of the `FF` marker *within* a record), so
  struct-literal construction needs updating. `start` now means record 0's
  first byte rather than the first `0xFF` run. `rvt-elem-table`'s text
  summary reports the marker's in-record offset instead of sniffing byte 0.
- **crates.io packaging works again** — `stream-evidence` is a path-only
  dev-dependency (Cargo strips it at publish) and `docs/data/*.csv` is no
  longer excluded from the package (`src/class_tag_map.rs` include_str!s
  the tag-drift CSV). `cargo publish -p rvt --dry-run` packages and verifies.
- **Release path proven by dry-run** — `publish.yml`'s viewer smoke now uses
  the same magnetar Einhoven sample as `deploy-viewer.yml` (the old phi-ag
  family sample could never satisfy `projectSampleTest`) and installs wabt so
  the WASM import audit runs; the CLI smoke keeps phi-ag. Three
  `workflow_dispatch` dry-runs: all verification jobs green.
- **wasm-pack builds straight into `viewer/pkg`** via `--out-dir` (no
  `rm`/`mv` shuffle) in every workflow and doc.
- **Formats/Latest page-strip stays disabled**; the walker now records when
  the 64 KiB schema scan cap applies only through #188 (open).
- **Honesty sync** — README, `docs/status.md`, compatibility, and
  supported-profile language emphasize inspection + narrow MVP
  recovers; generic converter claims removed.
- **ADR-004** — desktop distribution wrappers deferred.

### Fixed

- **Geometry-coverage diagnostics no longer read "solved" from a partial
  export.** `unsupported_features` used to carry `real_file_element_geometry`
  only while *no* exported element had a body, so an export where one category
  gained geometry silently dropped the gap for every other category. It now
  reports `real_file_element_geometry` when nothing has a body and the new
  `partial_element_geometry` when some do and some do not — on Core Interior,
  256 `IFCCOLUMN` with bodies against 82 slabs/spaces without. The slim
  manifest's `rooms_spaces` gap points at the new code accordingly. Documented
  in `docs/export-diagnostics.md`; both `rvt-rs` observations record the new
  code in `unsupported_entities`.
- **`Global/ElemTable` record origin on the 40-byte project variant (#206)** —
  `elem_table::parse_records` walked 26,424 of the 26,425 declared records on
  `2024_Core_Interior.rvt`. `detect_layout` took the first `0xFF` run as
  record 0's first byte, but on that variant the run is a sentinel-valued
  *field inside* the record: each record opens with a zero `u32` and only then
  carries `FF`×8. The four-byte shift ran the walk out of bytes just short of
  the last record. The decompressed stream is 1,057,030 bytes (gzip CRC32 and
  `ISIZE` both verify) = `0x1E` + 26,425 × 40 exactly, and the `u32` ahead of
  every marker is zero on all 26,425 records, so the origin is recovered as
  `len − record_count × stride`; new `ElemTableLayout::marker_offset` keeps
  field extraction anchored to the marker. Id values on the previously-parsed
  records are byte-identical; `ElemRecord::offset`/`raw` shift by −4 and the
  final record is no longer lost. The 28-byte 2023 variant is unchanged — its
  end-anchored origin would fall five bytes ahead of the marker, which the
  `u32`-alignment guard rejects, so it still walks 2,614 of 2,615 with a
  23-byte tail (now pinned exactly rather than asserted as `<=`). Byte
  evidence and record-count semantics:
  `docs/elem-table-record-layout-2026-04-21.md` § "Where the record array
  starts". Committed 270-byte MIT regression fixture under
  `tests/fixtures/elem-table/`. No OctetProof observation changed
  (`rvt-rs.json` stays `b6d9b67c…` on both recorded edges).
- **Corpus gate gap that hid it (#206)** — `elem_table_corpus` read
  `RVT_PROJECT_CORPUS_DIR` but was named by no CI job, so it took its skip
  path on every run. It is now named by the `test` matrix job (family half,
  `RVT_REQUIRE_CORPUS=1`) and by `corpus-tier2` (project half), together with
  the six other corpus-reading targets that were equally ungated
  (`arc_wall_corpus`, `partition_scanner`, `iter_elements_typed`,
  `re15_geometry_invariants`, `re19_door_window_wall_negative`,
  `walker_to_ifc_integration`). `elem_table_corpus` now fails instead of
  skipping when the corpus is configured but a file is missing, and carries
  one always-on committed-bytes test so the target is never wholly vacuous.
- libFuzzer-caught panics in `compression::gzip_header_len`,
  `basic_file_info::extract_path`, and fuzz harness string truncation
  (nightly fuzz matrix unblocked after upload-artifact pin repair).
- Finding 1 strip gate narrowed to exclude Formats/Latest (#162).
- **CI / Deploy viewer baselines** — re-pin project-count
  `material_count` expectations after Finding 1 recoveries (Core
  Interior 102, Einhoven 42); `cargo-deny` path-dep version pins for
  `stream-evidence`; stream-evidence fails closed on explicit stream
  filter / `--all-paged` misses instead of silent first-stream fallback.

### Security

- **CI Actions SHA pins** — remaining floating third-party tags
  (`actions/checkout@v4`, `setup-python@v5/@v6`, `upload-artifact@v7`,
  `maturin-action@v1`, `dtolnay/rust-toolchain@{stable,nightly,master}`)
  pinned to full commit SHAs with version comments across workflows
  (supply-chain follow-up to publish.yml pins).
- Viewer toolchain bumps (vite major line) for Dependabot advisories on
  optimized-deps / transitive esbuild CORS (dev-server class; production
  Pages build still uses `vite build`).
- pyo3 line bumps for advisory GHSA-pph8-gcv7-4qj5 (bindings crate).

## [0.1.2] — 2026-04-19

First tagged release since 0.1.0. Bundles the Python bindings,
document-level IFC4 export, Layer 5a ADocument walker, and the
spatial-hierarchy / classification extensions that land between
`v0.1.0` (initial public release) and the PyPI debut. Changelog
entries previously accumulated under `[Unreleased]` move here
verbatim.

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
  brute-force fallback. **Reliable on Revit 2024–2026**; older
  releases (2016–2023) need further entry-point detection work.
  Observed version bands if/when older releases land:
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
- **Q6.5-F**: walker reads ADocument on Revit 2024–2026 with
  cross-version-byte-identical output within each version band.
  Older releases (2016–2023) identified the entry-point band but
  still need hardening — tracked as L5B-11.

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
