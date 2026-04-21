# Roadmap

This is the public roadmap for rvt-rs. It tracks what ships today, what is in flight, and where the project is going. Detailed internal sequencing and audit findings live in planning docs outside this repo; this file is the subset that is useful to external contributors and observers. Every "Shipped" claim below is checked against current source — if a claim here contradicts the code, the code wins and this file is wrong. Open an issue.

## Vision

rvt-rs aims to be the complete, Apache-2.0, clean-room reader and IFC4 exporter for Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`). No production-quality open reader exists today — the openBIM community has worked around that gap for years using Revit's own in-process exporter, whose output is widely described as "very limited" and "data loss" because it can only emit what the Revit API chooses to surface. rvt-rs decodes the on-disk bytes directly, giving downstream tools (BIM pipelines, viewers, analysis, estimators, archival indexers) a path that does not require a Revit installation, the Revit API, or the ODA SDK, and is not bounded by what Autodesk chooses to expose through those APIs.

## Shipped

Everything in this section works today on `main` and is exercised by tests against the 11-release `rac_basic_sample_family` reference corpus (Revit 2016 through 2026). See [`docs/compatibility.md`](docs/compatibility.md) for the full matrix.

### File reading

- OLE2 / Microsoft Compound File Binary (MS-CFB) container open — no Revit installation required, no platform dependency.
- Truncated-gzip stream decompression (gzip header without trailing CRC/ISIZE, which breaks `gzip.GzipFile` and `flate2::read::GzDecoder` by default).
- `.rvt`, `.rfa`, `.rte`, and `.rft` all dispatch on CFB magic, not file extension, so the same read path handles all four.
- Revit releases 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 — all open and enumerate the full invariant stream set.
- `BasicFileInfo` (version / build / GUID / original path / locale), `PartAtom` XML (title / OmniClass / taxonomies), `RevitPreview4.0` PNG thumbnail extraction with wrapper stripped.
- Stable Revit format-identifier GUID (`3529342d-e51e-11d4-92d8-0000863f27ad`) decoded from the 167-byte invariant region of `Global/PartitionTable` — useful as a magic number for file-type sniffers.
- Bounded decompression (`InflateLimits`), bounded file/stream reads (`OpenLimits`), and `Error::DecompressLimitExceeded` to mitigate compressed-bomb DoS.

### Schema enumeration

- `Formats/Latest` parsed into `{name, tag, parent, ancestor_tag, declared_field_count, fields}` records — 395 classes, 13,570 fields on the 2024 sample.
- `FieldType` enum with 8 variants (`Primitive`, `String`, `Guid`, `ElementId`, `ElementIdRef`, `Pointer`, `Vector`, `Container`) classifies **100.00 %** of all fields across the 11-release corpus. CI regression gate fails on any regression.
- Cross-release tag-drift table — first publicly available 122-class × 11-release dataset, emitted as CSV and an SVG heat-map.

### Element decoding

- **54 `ElementDecoder` implementations registered in `elements::all_decoders()`** covering: walls + wall types, floors + floor types, roofs + roof types, ceilings + ceiling types, doors, windows, curtain walls + grids + mullions + panels, stairs + stair types, railings + railing types, columns + structural columns, beams + structural framing, structural foundations, rebar, furniture + furniture systems + casework, generic model + mass + family instances, levels, grids + grid types, base / survey / project position, reference planes, materials + fill patterns + line patterns + line styles, categories + subcategories, phases + design options + worksets, symbols, views + sheets + schedules + schedule views, rooms + areas + spaces. Each decoder handles camelCase / snake_case / `m_`-prefixed field-name variants across all 11 releases.
- `walker::read_adocument` is reliable on Revit 2024–2026 and returns `Ok(None)` on 2016–2023 when the entry-point detector cannot find a high-confidence offset (no wrong answers).

### IFC4 export

- Full spatial tree: `IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` (one per Revit `Level`, with name + elevation, ft → m at the 0.3048 boundary).
- Per-element entities — `IfcWall`, `IfcSlab`, `IfcRoof`, `IfcCovering`, `IfcDoor`, `IfcWindow`, `IfcColumn`, `IfcBeam`, `IfcStair`, `IfcRailing`, `IfcFurniture`, `IfcFooting`, `IfcReinforcingBar`, `IfcSpace`, `IfcBuildingElementProxy` — each wired to its storey via `IfcRelContainedInSpatialStructure` with its own `IfcLocalPlacement` and deterministic GUID.
- Extrusion geometry helpers for wall, slab, roof, ceiling, and column — `IfcRectangleProfileDef` + `IfcExtrudedAreaSolid` + `IfcShapeRepresentation` + `IfcProductDefinitionShape`.
- Materials via `IfcMaterial` + `IfcRelAssociatesMaterial` when the caller populates `BuilderOptions.materials`.
- Property sets via `IfcPropertySet` + `IfcPropertySingleValue` + `IfcRelDefinesByProperties` (helpers exist for wall, door, window, stair).
- Openings via `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement` for doors and windows.
- OmniClass / Uniformat from `PartAtom` emitted as `IfcClassification` + `IfcClassificationReference` + `IfcRelAssociatesClassification`.
- Deterministic STEP output under `StepOptions { timestamp }` — fixed timestamp produces byte-identical files. ISO-10303-21-correct Unicode escaping (`\X2\HHHH\X0\`, `\X4\HHHHHHHH\X0\`).
- A committed sample output — [`tests/fixtures/synthetic-project.ifc`](tests/fixtures/synthetic-project.ifc) — opens cleanly in BlenderBIM and IfcOpenShell.

### Extrusion-geometry assembly from decoded fields

- Wall: location-line endpoints + layer thicknesses → `IfcExtrudedAreaSolid` (GEO-27).
- Floor: boundary polygon + thickness → `IFCARBITRARYCLOSEDPROFILEDEF` extrusion + `Qto_SlabBaseQuantities` (GEO-28).
- Roof: gable triangular profile + hip faceted-brep with computed pitch (GEO-29).
- Door + Window: body + opening-void extrusions with configurable z-fighting margin, plus sill-height placement helper (GEO-30, GEO-31).
- Stair: sawtooth cross-section profile (back stringer + alternating riser/tread) + tread + landing rectangular extrusions (GEO-32).
- Column: I-shape / circular / rectangular-hollow / arbitrary-closed profiles + level-delta height resolution (GEO-33).
- Beam: same profile variants + axis yaw/pitch + 3D span helpers (GEO-34).

### Stream-level write path

- `FieldType::encode` — inverse of `FieldType::decode`, canonical-form emitting (WRT-01).
- `encode_instance` + `ElementEncoder` trait + `write_field_by_type` — schema-driven inverse of the walker reader (WRT-03).
- `encode_adocument_fields` + `write_adocument_field` — ADocument-level inverse including the 2-column Container variant (WRT-02, WRT-09).
- `round_trip::verify_instance_round_trip` — decode → re-encode → byte-compare harness with per-field divergence location (WRT-04).
- `validate_truncated_gzip_round_trip` + `..._prefix8_..` — pre-write gzip encoder validators (WRT-11).
- `BasicFileInfo::encode` / `PartAtom::encode` — per-stream writers (WRT-07, WRT-08).
- `writer::file_guid` + `guid_preserved` — file-identity invariant check after any write (WRT-05).
- `writer::file_history_entries` + `history_entries_preserved` — upgrade-history invariant check (WRT-06).
- `writer::decompress_stream` + `verify_patches_applied` + `write_with_patches_verified` — atomic temp+rename write with per-stream decompression verification (WRT-12, WRT-13).
- `rvt-write` CLI — applies a JSON manifest of `StreamPatch` entries + verifies the round-trip (WRT-14).

### Viewer data model (Rust side — `rvt::ifc`)

A feature-complete Rust data model for a browser/desktop 3D viewer. All modules are serde-serializable so a WASM build passes JSON across the FFI boundary cleanly.

- `scene_graph::build_scene_graph` — project → storey → element tree with hosted doors/windows nested under their wall (VW1-05).
- `pbr::PbrMaterial` — name-driven Revit `Material` → glTF-2.0 metallic-roughness translation (glass double-sided, metal metallic=1, wood/concrete roughness tuned) (VW1-06).
- `camera::CameraState` + orbit/pan/zoom/frame-bbox controls (VW1-07).
- `scene_graph::element_info_panel` — click-to-inspect JSON payload with storey + material + formatted property-set values (VW1-08).
- `scene_graph::CategoryFilter` — hide-by-IFC-type pruning of the scene graph (VW1-09).
- `clipping::ViewMode` (Plan / ThreeD / Section) + default section-box per mode (VW1-10).
- `sheet::render_plan_svg` — 2D plan-view SVG with per-category colour (walls black, doors blue, columns red, …) (VW1-11).
- `annotation::AnnotationLayer` — Note / Leader / Polyline / Pin user markups (VW1-12).
- `measure` — distance / angle / polygon-area primitives + tagged `Measurement` enum (VW1-13).
- `clipping::ClippingPlane` + `SectionBox` — oriented half-space + AABB spatial culling (VW1-14).
- `scene_graph::Schedule` + CSV export with RFC 4180 escaping (VW1-15).
- `gltf::model_to_glb` — dep-free binary glTF-2.0 exporter (VW1-04). The `rvt-gltf` CLI wraps this (VW1-16).
- `rvt-ifc` CLI emits STEP IFC4 (VW1-17).
- `share::ViewerState` + `encode_to_fragment` / `decode_from_fragment` — whole-viewer-state URL sharing (VW1-24).

### Tooling

- Eleven CLI binaries: `rvt-analyze`, `rvt-info`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`, `rvt-write`, `rvt-gltf`. Every CLI supports `--redact` for PII-safe output where relevant.
- Python bindings via pyo3 + maturin — `pip install rvt`. Surface: `RevitFile` class with `version`, `original_path`, `build`, `guid`, `part_atom_title`, `stream_names()`, `read_stream(name)`, `schema_json()`, `basic_file_info_json()`, `part_atom_json()`, `read_adocument()`, `write_ifc()` and a one-shot `rvt.rvt_to_ifc(path)` helper. PEP 561 typed via `__init__.pyi`. CI builds wheels on Ubuntu, macOS, Windows (Python ≥ 3.8, abi3).
- Stream-level modifying writer (`writer::write_with_patches`) — 13/13 streams byte-preserving round-trip on the 2024 sample; atomic temp-file + rename; pre-flight validation that every patch's stream name exists.
- 30+ reproducible probes under [`examples/`](examples/), one per documented finding in the recon report.
- Integration tests against the 11-release corpus (skipped gracefully when LFS samples are absent).
- CI: Ubuntu / macOS / Windows matrix, 100 % field-type coverage regression gate, Python wheel build + test, `cargo deny check`, `cargo audit`.

### Docs

- [`README.md`](README.md) — project overview with "What works today" table aligned to source.
- [`docs/compatibility.md`](docs/compatibility.md) — per-release, per-file-type, per-element-class, per-IFC-helper matrix.
- [`docs/python.md`](docs/python.md) + [`docs/rvt-python-quickstart.ipynb`](docs/rvt-python-quickstart.ipynb) — Python API reference and Jupyter walkthrough.
- [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) — dated addenda for every reverse-engineering finding.
- [`docs/compatibility.md`](docs/compatibility.md), [`docs/benchmarks.md`](docs/benchmarks.md), [`docs/extending-layer-5b.md`](docs/extending-layer-5b.md).
- Blog posts: [schema discovery](docs/blog/2026-04-schema-discovery.md), [IFC4 exporter walkthrough](docs/blog/2026-04-ifc4-exporter.md).
- Governance: [`CONTRIBUTING.md`](CONTRIBUTING.md), [`SECURITY.md`](SECURITY.md), [`THREAT_MODEL.md`](THREAT_MODEL.md), [`CLEANROOM.md`](CLEANROOM.md), [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md), [`RELEASE.md`](RELEASE.md).

## In flight

Current open work, ordered by the commit stream. Task IDs reference the internal decomposition; external contributors should track this list rather than the IDs.

- **Annotation decoders** — Dimension, Tag, TextNote, Annotation, Revision per-class decoders (tasks L5B-44..48).
- **Parameter value extraction** — typed parameter reads (Length, Area, Volume, Angle, URL, YesNo, Material, Currency, Count, …) with type / instance override resolution (tasks L5B-53..56).
- **MEP family-instance decoders** — LightingFixture, ElectricalEquipment / Fixture, MechanicalEquipment, PlumbingFixture, SpecialtyEquipment. The IFC category map already routes these (L5B-36, L5B-37); the element decoders are the missing half.
- **Symbol + generic `FamilyInstance`** — the container nearly every loadable family inherits from (L5B-20, L5B-21).
- **Curtain-wall geometry decomposition** — grids, mullions, panels already enumerated at the class level; per-element geometry wiring pending.
- **Fuzz target workspace** — `fuzz/` with `cargo-fuzz` targets for `open_bytes`, `gzip_header_len`, `inflate_at_with_limits`, `parse_schema`, `find_chunks`, `basic_file_info`, `part_atom`, `walker_entry_detect`, `step_writer` (tasks SEC-14..25).
- **Workspace split** — `rvt-core` (pure parser, `forbid(unsafe_code)`) / `rvt-cli` (binaries) / `rvt-py` (pyo3) / `rvt-ifc` (exporter) so the unsafe escape hatch for PyO3 macro-gen doesn't leak into the core parser (tasks SEC-11..13).

## Next up

Ordered by dependency and by how much each unblocks downstream. Earlier items clear the path for later ones.

1. **Composite wall materials** — emit `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` from Wall's layer stack (task IFC-28, IFC-29). Blocks real-world wall fidelity; every wall today is single-material.
2. **Revit unit → `IfcSIUnit` mapping** — read `autodesk.unit.*` identifiers from Partitions/NN and emit the correct `IfcUnitAssignment` (tasks IFC-39, IFC-40). Blocks correct scale in consumer viewers — today a mis-unit file can render at the wrong physical size.
3. **Sweep geometry for curved walls and non-rectangular extrusions** — `IfcSweptSolid` + `IfcRevolvedAreaSolid` + `IfcArbitraryClosedProfileDef` (tasks IFC-17, IFC-18, IFC-24). Unblocks curved walls, non-rectangular doors and windows, and profile-shape columns.
4. **`IfcBooleanResult` for arbitrary voids** — beyond the door/window `IfcOpeningElement` pattern, for penetrations that don't align to a nominal opening (task IFC-19).
5. **Reader-side geometry extraction** — the assembly helpers (GEO-27..34) exist and take caller-supplied dimensions. What's still pending is the *reader* surfacing those dimensions: wall base curve + height, floor boundary polygon, roof pitch, stair run/landing data — all currently require the caller to measure from a Revit UI screenshot or accept the walker's fallback. Unlocks end-to-end "open RVT → accurate 3D model" without human-supplied dimensions.
6. **Real-world RVT corpus** — the `rac_basic_sample_family` family is a validated baseline but families are a narrower slice of the format than projects. A set of community-donated `.rvt` project files (with redistribution rights) widens what we can assert (task Q-01).
7. **`IfcRepresentationMap` type instancing** — emit each wall / door / window type once as a shared representation and reference it from every instance, dropping file size dramatically on projects with many repeated types (tasks IFC-21, IFC-22).
8. **Walker extension to Revit 2016–2023** — `walker::read_adocument` detects the entry-point band across all 11 releases but cleanly decodes all 13 ADocument fields only on 2024–2026. Per-band entry-point heuristics for the older releases (tasks L5B-11, recon report §Q6.5-F).
9. **IFC export validation in CI** — install IfcOpenShell, run `rvt-ifc` on the corpus, assert the output loads and that `by_type("IfcWall")` returns non-empty for files with walls. Regression-fixture RVTs with known element counts (tasks IFC-41..44).

## Longer horizon

These depend on multiple foundation pieces landing and are named for calibration, not as commitments.

- **CFB structural writer** (`WRT-10`) — current output uses the `cfb` crate's default OLE2 sector ordering, which differs from Revit's own writer. Decompressed streams round-trip byte-for-byte on read, but raw file bytes don't. Needs sector-layout reverse engineering from the corpus.
- **Browser-based 3D viewer frontend** — the Rust-side data model is feature-complete (scene graph, PBR, camera, filter, view modes, annotations, measurement, clipping, schedule, glTF exporter, URL share). What remains is the frontend integration layer: WASM build of `rvt-core` (VW1-01), wasm-bindgen JS bindings (VW1-02), Three.js wrapper (VW1-03), WebWorker offloading for large files (VW1-19), progressive streaming (VW1-20), GitHub Pages static-site deploy (VW1-18), demo RVT catalogue (VW1-22), drag-and-drop file upload (VW1-23), client-side-only privacy posture (VW1-21). All Rust-side primitives are in `src/ifc/` and fully tested.
- **Workspace split** (`SEC-12`, `SEC-13`) — `rvt-core` (pure parser, `forbid(unsafe_code)`) / `rvt-cli` (binaries) / `rvt-py` (pyo3 with isolated `unsafe`) / `rvt-ifc` (exporter). The split doesn't block anything today but tightens the clean-room posture for the PyO3 macro expansion's `unsafe` escape hatch.
- **Performance work** — criterion benchmark suite (open, schema-parse, read_adocument, ifc_export per file size), memory profiling with dhat, CI performance regression gate. Tasks Q-05..09.
- **IFC2X3 / IFC4.3 target selection** — the category map is structured to make this a table swap. Not scheduled; waiting on consumer demand.
- **Revit link resolution** — following `.rvt` → `.rvt` / `.rfa` / `.ifc` references to produce a unified IFC. Large undertaking; deliberately out of the near-term plan.

## Out of scope

rvt-rs explicitly will not:

- Reverse-engineer Autodesk-proprietary binary formats in a way that could be read as circumventing a technical protection measure (DMCA §1201) or breaching Autodesk's terms of service. rvt-rs reads the on-disk bytes of files created by publicly-shipped Revit sample content, the publicly-documented OLE/CFB container format, standard gzip/DEFLATE, the `Formats/Latest` schema that Revit itself serialises into every file, and the public `RevitAPI.dll` NuGet symbol export. See [`CLEANROOM.md`](CLEANROOM.md) for the accepted/forbidden sources policy.
- Ship a rendering engine or game-engine integration. That is BlenderBIM and IfcOpenShell territory. rvt-rs produces IFC4 STEP; consumers render.
- Read or write non-Revit file formats. DWG lives in the sibling project `dwg-rs`. IFC lives in IfcOpenShell. STEP / STL / OBJ / glTF / FBX live elsewhere.
- Interpret or enforce Revit's licensing / worksharing / transmission / cloud-workshared model semantics. The byte-level streams are readable; what Autodesk's cloud-workshared ecosystem does with them is out of scope.
- Provide a Revit API-compatible surface. The surface is schema-driven Rust / Python, not a port of the Autodesk `RevitAPI.Elements.*` namespace.

## How to contribute

- Start with [`CONTRIBUTING.md`](CONTRIBUTING.md). It lists the contribution patterns (bug reports, new FACTs, tests, per-class decoders, reverse-engineering findings) and the clean-room legal note.
- Use the issue templates under [`.github/ISSUE_TEMPLATE/`](.github/ISSUE_TEMPLATE/) — bug report, feature request, corpus submission, reverse-engineering finding.
- rvt-rs is audit-honest: the README, this roadmap, and the compatibility matrix distinguish shipped from scaffolded from pending. If a claim overstates capability, open a bug — the source is the source of truth and docs that drift from it are a defect, not a style preference.
- For reverse-engineering findings: reproduce as a probe under `examples/`, add a dated addendum to [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md), and add a unit test pinning the byte pattern if the finding is a decoding rule. This keeps every claim independently verifiable, which is the whole point.
