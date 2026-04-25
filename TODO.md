# TODO: S-tier rvt-rs

This file decomposes the remaining work required to turn `rvt-rs`
from a strong research-grade Revit reader into a first-class,
open-source utility that non-technical BIM/AEC users can trust.

The project already has a serious foundation: safe Rust core, bounded
reads/inflate, schema extraction, Python bindings, CLIs, IFC writer
infrastructure, a WASM viewer, and a growing corpus. The main gap is
not polish. The main gap is reliable real-project model extraction:
typed elements, placement, geometry, units, materials, parameters, and
validated exports from actual `.rvt` project files.

## GitHub Operating Rules

Every task below should become a GitHub issue or a tracked checklist
item inside a parent issue. Follow these rules:

- One issue = one independently reviewable change.
- Every issue must include: problem, scope, acceptance criteria,
  test plan, affected subsystem, and links to source evidence.
- Every PR must link an issue with `Closes #NNN` or `Refs #NNN`.
- No PR may claim a capability in docs unless tests or reproducible
  probes demonstrate it.
- Use draft PRs for reverse-engineering work until a reproducible
  probe and dated report are included.
- Prefer small vertical slices over broad scaffolding.
- Keep user-facing docs audit-honest: shipped, partial, experimental,
  and unsupported states must be clearly separated.
- Do not commit proprietary Autodesk implementation code, decompiled
  internals, NDA material, private project files, or PII.
- Any new corpus fixture must include license/provenance metadata and
  must pass the corpus validation gate before it is referenced in docs.
- CI must be green before merge unless the PR is explicitly marked
  `ci-quarantine` and only changes CI diagnostics.

Recommended labels:

- `priority:P0`, `priority:P1`, `priority:P2`, `priority:P3`
- `type:bug`, `type:feature`, `type:docs`, `type:test`,
  `type:research`, `type:security`, `type:release`
- `area:reader`, `area:partitions`, `area:walker`, `area:elements`,
  `area:geometry`, `area:ifc`, `area:viewer`, `area:python`,
  `area:cli`, `area:docs`, `area:ci`, `area:corpus`
- `status:blocked`, `status:needs-corpus`, `status:needs-repro`,
  `status:ready-for-review`

Definition of Done for any implementation PR:

- `cargo fmt --all -- --check` passes.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passes on the supported toolchain.
- `cargo test --workspace --all-targets` passes.
- If viewer code changed: `npm run typecheck` and `npm run build`
  pass in `viewer/`.
- If IFC changed: committed synthetic IFC fixtures either remain
  byte-stable or the fixture diff is documented and independently
  validated by IfcOpenShell.
- If parsing changed: fuzz regression tests or corpus regression tests
  cover the new case.
- If docs changed: README, ROADMAP, compatibility docs, Python docs,
  and release notes agree with source behavior.

## Release Gates

Use these gates to decide whether the project is ready for the next
public positioning level.

### Gate A: Alpha, honest developer utility

- Local and CI health are clean.
- Docs do not overclaim real-project model extraction.
- Metadata, schema, Python, CLI, and viewer basics work from source.
- Real-project IFC export is described as partial/scaffolded unless
  the specific element classes are validated.

### Gate B: Public beta, useful to BIM developers

- At least two licensed real `.rvt` project files produce non-empty
  typed element output for Levels, Walls, Floors, Doors/Windows, Rooms,
  Materials, and core parameters.
- At least one Revit 2023 and one Revit 2024 project have known element
  count fixtures and regression tests.
- IFC output from those projects opens in IfcOpenShell and contains
  typed elements with placements and geometry.
- Viewer shows those projects with correct category filtering, element
  selection, and export status.

### Gate C: First-class industry utility

- Non-technical user can open a hosted viewer, drop a supported Revit
  file, inspect a meaningful 3D/2D model, export IFC/glTF/SVG, and
  understand any limitations without reading source code.
- The CLI and Python API expose the same model data as the viewer.
- Installation is documented and tested for `cargo install`, `pip
  install`, and browser usage.
- Corpus coverage includes architectural, structural, and MEP projects
  across multiple Revit versions.
- All unsupported cases fail with clear diagnostics, not silent empty
  exports.

## M0: Repository Health and Merge Readiness

### M0-01: Restore formatter cleanliness

Labels: `priority:P0`, `type:bug`, `area:ci`

- Run `cargo fmt --all -- --check`.
- Format the current arc-wall/probe worktree without changing behavior.
- Keep unrelated files untouched.

Acceptance criteria:

- `cargo fmt --all -- --check` exits 0.
- `git diff` shows formatting-only changes for files touched by fmt,
  unless paired with a separate behavior PR.

### M0-02: Restore clippy cleanliness on current Rust

Labels: `priority:P0`, `type:bug`, `area:ci`

- Fix the current `clippy::unnecessary_get_then_check` failure in
  `src/formats.rs`.
- Audit for other Rust 1.95 clippy warnings.
- Decide whether CI should pin the MSRV/stable combo or allow newest
  stable warnings to block merges.

Acceptance criteria:

- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  exits 0.
- CI toolchain policy is documented in `README.md` or `CONTRIBUTING.md`.

### M0-03: Finish or park current uncommitted arc-wall work

Labels: `priority:P0`, `type:research`, `area:partitions`

- Review current dirty files:
  - `src/arc_wall_record.rs`
  - `examples/probe_arcwall_2024.rs`
  - `examples/probe_arcwall_trailer.rs`
  - `reports/element-framing/RE-13-synthesis.md`
- Decide whether they form one PR or should be split.
- Ensure the PR title states exact scope, for example:
  `research: scope ArcWall decoder to Revit 2023`.

Acceptance criteria:

- No untracked research files remain outside an intentional PR.
- If merged, the PR includes a reproducible probe, report, and tests.
- If parked, the files are moved to a branch or issue attachment rather
  than lingering in the working tree.

### M0-04: Add a local quality script

Labels: `priority:P1`, `type:feature`, `area:ci`

- Add `tools/check-local.sh` that runs the standard local gates:
  fmt, clippy, tests, docs, and optional viewer checks.
- Include flags for expensive checks:
  - `--viewer`
  - `--corpus`
  - `--ifcopenshell`
  - `--deny`
  - `--audit`
- Do not require network by default.

Acceptance criteria:

- `tools/check-local.sh` exits non-zero on any failed required gate.
- `CONTRIBUTING.md` points contributors to the script.
- Script works from repo root and fails clearly when optional tools are
  missing.

### M0-05: Make cargo-audit availability explicit

Labels: `priority:P1`, `type:ci`, `type:security`, `area:ci`

- Decide whether `cargo-audit` is required locally or CI-only.
- Add installation notes for maintainers.
- Ensure CI still runs the RustSec advisory check.

Acceptance criteria:

- `CONTRIBUTING.md` documents how to run advisory checks locally.
- CI has a passing advisory job.
- Local absence of `cargo audit` is not confused with source failure.

## M1: Audit-Honest Documentation and Positioning

### M1-01: Reconcile README, ROADMAP, compatibility matrix, and docs

Labels: `priority:P0`, `type:docs`, `area:docs`

- Align claims across:
  - `README.md`
  - `ROADMAP.md`
  - `docs/compatibility.md`
  - `docs/python.md`
  - `docs/viewer-privacy-posture.md`
  - `docs/viewer-build-pipeline.md`
  - `docs/demos.md`
  - `CHANGELOG.md`
- Resolve contradictions around:
  - viewer URL/path and shipped status
  - 72 vs 80 decoder count
  - "walker dispatch table" vs registry not reached on real project
    data
  - corpus hunt "not executed" vs documented 222/223 real-file pass
    rate
  - real IFC export scope
  - Python build layout and module name

Acceptance criteria:

- Each public doc uses the same capability table.
- "What works" and "What does not work yet" agree with source and
  tests.
- Docs explicitly say generic real-project typed model extraction is
  not solved yet.

### M1-02: Add a single source-of-truth status matrix

Labels: `priority:P0`, `type:docs`, `area:docs`

- Create `docs/status.md` as the authoritative status table.
- Include rows for:
  - container open
  - stream reads
  - gzip decode
  - schema parse
  - `ElemTable`
  - `ADocument`
  - partition record decode
  - per-class decoders
  - geometry extraction
  - IFC export
  - glTF export
  - SVG plan export
  - browser viewer
  - Python API
  - write path
- Columns:
  - status: shipped, partial, experimental, unsupported
  - validated on synthetic data
  - validated on family corpus
  - validated on project corpus
  - user-visible caveat
  - primary tests

Acceptance criteria:

- README links to `docs/status.md`.
- ROADMAP links to `docs/status.md` instead of restating detailed
  claims.
- Compatibility docs reference it for high-level status.

### M1-03: Rewrite public positioning for non-technical clarity

Labels: `priority:P1`, `type:docs`, `area:docs`

- Replace jargon-heavy first-screen text with concrete user outcomes.
- Clearly separate:
  - "Inspect file metadata and schema"
  - "Export partial IFC scaffold"
  - "Experimental real-wall extraction for limited Revit 2023 cases"
  - "Full RVT to BIM model conversion not ready yet"
- Add screenshots or terminal snippets that show both successful and
  partial outputs.

Acceptance criteria:

- A non-technical reader can tell whether their use case is supported
  within 60 seconds.
- No phrase implies full Revit replacement unless supported by corpus
  tests.

### M1-04: Document diagnostic semantics

Labels: `priority:P1`, `type:docs`, `area:reader`

- Document strict vs lossy APIs.
- Explain "empty model", "scaffold", "proxy", "typed element",
  "geometry-free element", and "unsupported record layout".
- Include examples of warnings users should see for partial exports.

Acceptance criteria:

- CLI docs and Python docs explain when empty output is success vs
  partial failure.
- Viewer copy reuses the same terms.

## M2: Real Project Corpus and Ground Truth

### M2-01: Curate a redistributable project corpus

Labels: `priority:P0`, `type:test`, `area:corpus`

- Select a minimal public corpus for CI:
  - at least one architectural project
  - at least one structural project
  - at least one MEP-heavy project
  - at least one Revit 2023 project
  - at least one Revit 2024+ project
- For each file, add a metadata sidecar:
  - source URL
  - license
  - commit SHA or release archive
  - SHA256
  - file size
  - Revit version
  - redistribution notes

Acceptance criteria:

- Corpus can be fetched by a documented script.
- CI can run a smoke subset without proprietary files.
- No committed sample lacks license/provenance metadata.

### M2-02: Create known-count fixtures

Labels: `priority:P0`, `type:test`, `area:corpus`, `area:elements`

- For each curated project, create a known-count manifest:
  - levels
  - walls
  - floors
  - roofs
  - doors
  - windows
  - rooms/spaces
  - columns/beams where present
  - MEP categories where present
  - materials
  - units
- Source counts from authoritative exports where possible:
  - Revit schedules supplied by file owner
  - IFC exported by Revit
  - manually reviewed schedule screenshots
  - contributor-signed manifest

Acceptance criteria:

- `tests/fixtures/project-counts/*.json` contains count manifests.
- Tests compare decoded output against counts with documented
  tolerances.
- Any unknown count is explicit, not silently skipped.

### M2-03: Add corpus health CI tiers

Labels: `priority:P1`, `type:ci`, `area:corpus`

- Tier 1: lightweight open/schema/summary test.
- Tier 2: project model decode count tests.
- Tier 3: IFC/glTF/SVG export validation.
- Tier 4: full slow corpus and benchmark run.

Acceptance criteria:

- PR CI runs Tier 1 and synthetic tests.
- Nightly CI runs all tiers.
- Failures identify file, stage, and subsystem.

### M2-04: Add corpus triage tooling

Labels: `priority:P1`, `type:feature`, `area:corpus`

- Add `rvt-corpus doctor` or extend `rvt-corpus` to classify failures:
  - not CFB
  - missing Revit streams
  - corrupt gzip
  - schema parse failure
  - unsupported version
  - partial walker decode
  - empty IFC export
- Emit JSON for issue creation.

Acceptance criteria:

- Maintainer can run one command over a corpus and get actionable
  buckets.
- Output includes suggested labels for GitHub issues.

## M3: Partition Wire Format and Element Extraction

### M3-01: Version-gate current ArcWall decoder

Labels: `priority:P0`, `type:bug`, `area:partitions`, `area:ifc`

- Ensure the current raw `ArcWallRecord` decoder is only invoked for
  versions/layouts it supports.
- Current evidence says Revit 2023 tag `0x0191` and variant `0x07fa`
  should not be assumed for 2024.
- Add explicit version/layout guard before scanning partitions.

Acceptance criteria:

- 2023 Einhoven still yields ArcWall records.
- 2024 Core Interior does not silently run the 2023 decoder.
- Unsupported versions produce diagnostics, not false positives.

### M3-02: Reverse-engineer Revit 2024 ArcWall records

Labels: `priority:P0`, `type:research`, `area:partitions`, `area:elements`

- Use `examples/probe_arcwall_2024.rs` or successor probe.
- Identify:
  - 2024 class tag
  - record envelope
  - variant markers
  - coordinate fields
  - element id fields
  - type/material/level references if present
- Document findings in `reports/element-framing/`.

Acceptance criteria:

- Probe reproduces findings from a known Revit 2024 project.
- New decoder has unit tests and corpus tests.
- False-positive rate is measured against at least one non-wall
  partition.

### M3-03: Build a generic partition record scanner

Labels: `priority:P0`, `type:feature`, `area:partitions`

- Replace class-specific ad hoc scanning with a versioned scanner.
- Output a neutral `PartitionRecordCandidate`:
  - stream name
  - chunk index
  - offset
  - candidate class tag
  - variant/envelope fields
  - confidence score
  - consumed byte range
  - raw excerpt hash
- Keep raw bytes accessible for downstream decoders.

Acceptance criteria:

- Existing ArcWall path can be expressed through the scanner.
- Scanner has a documented confidence model.
- Scanner emits JSON through a CLI/probe for issue attachments.

### M3-04: Link `ElemTable` ids to partition record offsets

Labels: `priority:P0`, `type:research`, `area:partitions`, `area:walker`

- Determine how `Global/ElemTable` ids map to partition records.
- Investigate:
  - direct id storage
  - chunk-local handles
  - content document ids
  - cross-stream indexes
  - sort order correlations
- Build `ElementId -> PartitionRecordRef`.

Acceptance criteria:

- At least one project shows >80% coverage for declared wall ids or
  a documented reason why not.
- Tests fail if coverage regresses on known corpus files.
- CLI can print declared-but-unlocated ids.

### M3-05: Implement typed partition decoders for the MVP classes

Labels: `priority:P0`, `type:feature`, `area:elements`, `area:partitions`

MVP class order:

- Level
- Wall / ArcWall
- WallType
- Floor
- FloorType
- Door
- Window
- Room
- Material
- Category
- FamilyInstance
- Symbol
- ParameterElement / AProperty*

Acceptance criteria:

- Each decoder includes:
  - wrong-schema rejection
  - empty/short input behavior
  - synthetic happy-path test
  - real corpus test
  - mapping from raw record to typed struct
- Decoders produce enough fields to support IFC placement, geometry,
  material, parameters, and viewer labels.

### M3-06: Replace `iter_elements` false-positive behavior

Labels: `priority:P0`, `type:bug`, `area:walker`

- Current `iter_elements` on real projects can produce permissive
  parent-class hits like `HostObjAttr`.
- Rework it so production APIs return only validated records by
  default.
- Keep broad scanning available under an explicit diagnostic/probe API.

Acceptance criteria:

- Production `iter_elements` does not return low-confidence parent-only
  false positives.
- Diagnostic scanner can still report candidates for research.
- IFC exporter no longer emits misleading `HostObjAttr-*` proxies as
  user-visible model elements.

### M3-07: Add decode confidence and provenance to every element

Labels: `priority:P1`, `type:feature`, `area:elements`

- Extend decoded elements with:
  - source stream
  - source offset
  - record kind
  - decoder name/version
  - confidence
  - missing required fields
  - warnings

Acceptance criteria:

- CLI, Python, and viewer can show why an element exists.
- Low-confidence elements can be hidden by default in viewer/export.

## M4: Geometry, Units, Materials, and Parameters

### M4-01: Recover project units from Revit bytes

Labels: `priority:P0`, `type:feature`, `area:reader`, `area:ifc`

- Extract `autodesk.unit.*` identifiers from relevant streams.
- Map to IFC units:
  - length
  - area
  - volume
  - angle
  - mass where present
- Preserve unknown unit ids as diagnostics.

Acceptance criteria:

- IFC no longer defaults blindly to millimeters for real files.
- Unit tests cover metric and imperial inputs.
- Real corpus files report their unit assignment.

### M4-02: Recover wall geometry

Labels: `priority:P0`, `type:feature`, `area:geometry`, `area:ifc`

- Decode wall location curve, base level, top constraint/height,
  thickness, flip/orientation, and curve type.
- Support initially:
  - straight wall
  - arc wall
  - rectangular extrusion
- Defer complex BRep with explicit diagnostics.

Acceptance criteria:

- Known corpus walls export as `IfcWall` with `IfcShapeRepresentation`.
- Viewer shows walls in correct rough positions.
- Revit 2023 and 2024 wall count tests pass.

### M4-03: Recover floor and slab geometry

Labels: `priority:P0`, `type:feature`, `area:geometry`, `area:ifc`

- Decode floor boundary loops, thickness, level, and type.
- Export `IfcSlab` with polygon profile extrusion.

Acceptance criteria:

- Known corpus floors export as typed slabs with shape.
- Degenerate or unsupported boundaries are reported, not silently
  dropped.

### M4-04: Recover doors and windows with host relationships

Labels: `priority:P0`, `type:feature`, `area:geometry`, `area:ifc`

- Decode family instance placement, host wall id, width, height, sill
  height, hand/facing flips.
- Emit:
  - `IfcDoor`
  - `IfcWindow`
  - `IfcOpeningElement`
  - `IfcRelVoidsElement`
  - `IfcRelFillsElement`

Acceptance criteria:

- Known corpus host walls contain openings for doors/windows.
- Door/window counts match manifest.
- Viewer selection shows host relationship.

### M4-05: Recover levels and storey assignment

Labels: `priority:P0`, `type:feature`, `area:elements`, `area:ifc`

- Decode Level names/elevations from real project files.
- Assign elements to storeys via level references.
- Remove fallback-only `Level 1` behavior for files with real levels.

Acceptance criteria:

- IFC storeys match project levels.
- Elements are grouped under correct storey.
- Viewer scene tree groups by actual levels.

### M4-06: Recover materials and compound assemblies

Labels: `priority:P1`, `type:feature`, `area:elements`, `area:ifc`

- Decode Material records.
- Decode wall/floor/roof layer stacks.
- Emit:
  - `IfcMaterial`
  - `IfcMaterialLayerSet`
  - `IfcMaterialLayerSetUsage`
- Include colors/transparency where available.

Acceptance criteria:

- Material counts match known fixtures where available.
- IFC material associations validate with IfcOpenShell.
- Viewer applies material names/colors.

### M4-07: Recover common parameters

Labels: `priority:P1`, `type:feature`, `area:elements`, `area:python`

- Decode instance/type parameter values:
  - text
  - integer
  - double
  - length
  - area
  - volume
  - yes/no
  - URL
  - material
  - element id references
- Resolve type-vs-instance override precedence.

Acceptance criteria:

- CLI and Python expose parameters consistently.
- IFC property sets include common parameters.
- Viewer info panel displays them in readable form.

### M4-08: Add unsupported-geometry diagnostics

Labels: `priority:P1`, `type:feature`, `area:geometry`, `area:viewer`

- For unsupported geometry, emit explicit warnings:
  - unsupported curve
  - unsupported profile
  - unresolved host
  - missing level
  - missing dimensions
- Surface warnings through Rust, Python, CLI, and viewer.

Acceptance criteria:

- Empty geometry is never indistinguishable from successful geometry.
- Exports include a machine-readable diagnostic report.

## M5: IFC Export Quality

### M5-01: Split IFC export modes

Labels: `priority:P0`, `type:feature`, `area:ifc`, `area:cli`

- Add explicit export modes:
  - `scaffold`
  - `typed-no-geometry`
  - `geometry`
  - `strict`
- Default user-facing CLI/viewer mode should warn when output is
  scaffold-only.

Acceptance criteria:

- `rvt-ifc --mode strict` fails if required real model data is missing.
- Viewer labels export quality before download.
- Python exposes the same mode option.

### M5-02: Remove misleading generic proxies from default export

Labels: `priority:P0`, `type:bug`, `area:ifc`

- Do not emit `IFCBUILDINGELEMENTPROXY` for low-confidence scan hits
  by default.
- Allow diagnostic exports to include them with provenance.

Acceptance criteria:

- Einhoven 2023 no longer emits `HostObjAttr-*` as normal model
  elements.
- Diagnostic mode can still show them.

### M5-03: Validate real-file IFC outputs

Labels: `priority:P0`, `type:test`, `area:ifc`, `area:corpus`

- For curated real projects, run:
  - `rvt-ifc`
  - IfcOpenShell open
  - entity count checks
  - spatial hierarchy checks
  - geometry presence checks
  - material/property checks where supported

Acceptance criteria:

- CI validates at least one real-project IFC fixture in addition to
  synthetic fixtures.
- Failures show which IFC entity class regressed.

### M5-04: Add export diagnostics sidecar

Labels: `priority:P1`, `type:feature`, `area:ifc`, `area:cli`

- Add `--diagnostics out.json` to export commands.
- Include:
  - input metadata
  - decoded element counts
  - exported element counts
  - skipped elements
  - unsupported features
  - warnings
  - confidence summary

Acceptance criteria:

- CLI and viewer can generate/share diagnostics for bug reports.
- Diagnostics schema is documented.

### M5-05: Add comparison tooling against Revit IFC exports

Labels: `priority:P2`, `type:feature`, `area:ifc`, `area:corpus`

- Build `rvt-ifc-compare` or add `rvt-ifc --compare`.
- Compare rvt-rs output against a reference IFC:
  - entity counts
  - storeys
  - bounding boxes
  - object names/types
  - material counts
  - property keys

Acceptance criteria:

- Tool emits JSON and human summary.
- Known divergences are documented and linked to issues.

## M6: Viewer as a Non-Technical Product

### M6-01: Show decode/export confidence in the viewer

Labels: `priority:P0`, `type:feature`, `area:viewer`

- Add a status panel that clearly says:
  - file opened
  - schema parsed
  - elements decoded
  - geometry decoded
  - IFC export quality
  - warnings/errors
- Use plain user language.

Acceptance criteria:

- Empty/scaffold-only output is clearly labelled before export.
- Users can open diagnostics for details.

### M6-02: Add supported-file guidance in the viewer

Labels: `priority:P1`, `type:feature`, `area:viewer`

- Add a compact support matrix in the UI:
  - supported Revit versions
  - supported element classes
  - experimental versions/classes
  - unsupported cases
- Do not bury this behind docs only.

Acceptance criteria:

- User sees limitations without leaving the app.
- Matrix content is generated from or linked to `docs/status.md`.

### M6-03: Add demo gallery with redistributable files

Labels: `priority:P1`, `type:feature`, `area:viewer`, `area:corpus`

- Implement demo loading from `docs/viewer-demos.json` or successor.
- Only include redistributable files.
- Add thumbnails, expected decoded counts, and export quality labels.

Acceptance criteria:

- Viewer can load at least one demo without local file selection.
- Demo metadata includes license/provenance.
- Network behavior still complies with privacy posture.

### M6-04: Add browser regression tests

Labels: `priority:P1`, `type:test`, `area:viewer`

- Use Playwright to test:
  - viewer loads
  - dropzone visible
  - demo/sample load path
  - canvas nonblank after model load
  - category toggles work
  - element info panel opens
  - export buttons enable/disable correctly
  - diagnostics show partial-export status

Acceptance criteria:

- Tests run in CI for viewer PRs.
- Screenshots are captured on failure.

### M6-05: Improve viewer accessibility and responsiveness

Labels: `priority:P2`, `type:feature`, `area:viewer`

- Keyboard support for file picker, panels, tree navigation, and export
  buttons.
- Mobile/tablet layout for inspection.
- Color contrast checks.
- Avoid text overflow in panels.

Acceptance criteria:

- Lighthouse or equivalent accessibility checks pass reasonable
  thresholds.
- Manual keyboard-only smoke test passes.

### M6-06: Add desktop distribution investigation

Labels: `priority:P3`, `type:research`, `area:viewer`

- Evaluate whether a Tauri/Electron wrapper is worth maintaining for
  non-technical users.
- Compare:
  - install complexity
  - file size
  - privacy
  - auto-update
  - code-signing burden

Acceptance criteria:

- Decision record under `docs/decisions/`.
- No desktop wrapper is started until maintenance cost is accepted.

## M7: CLI and Python Developer Experience

### M7-01: Add a single user-facing inspect command

Labels: `priority:P1`, `type:feature`, `area:cli`

- Add or refine a command that gives a non-technical summary:
  - file health
  - supported/unsupported version
  - decoded counts
  - export readiness
  - warnings
- Consider `rvt inspect file.rvt` as a future unified CLI.

Acceptance criteria:

- Output is useful without knowing Revit internals.
- `--json` has stable schema.

### M7-02: Expose decoded model API in Python

Labels: `priority:P1`, `type:feature`, `area:python`, `area:elements`

- Add Python methods for:
  - `decoded_elements()`
  - `element_counts()`
  - `export_diagnostics()`
  - `write_ifc(mode="strict" | "geometry" | "scaffold")`
- Return typed dictionaries or dataclasses with type stubs.

Acceptance criteria:

- Python can reproduce viewer/CLI element counts.
- `__init__.pyi` and runtime API match.
- Tests cover every public Python method.

### M7-03: Add stable JSON schemas

Labels: `priority:P1`, `type:feature`, `area:cli`, `area:python`

- Define JSON schemas for:
  - summary
  - schema diagnostics
  - element records
  - export diagnostics
  - corpus report
- Commit schemas under `docs/schemas/`.

Acceptance criteria:

- CLI JSON output validates against schemas.
- Python docs link to schemas.

### M7-04: Improve install paths

Labels: `priority:P1`, `type:release`, `area:python`, `area:cli`

- Verify and document:
  - `cargo install rvt`
  - `pip install rvt`
  - source build with `maturin`
  - viewer static deployment
- Add smoke tests for published artifacts where feasible.

Acceptance criteria:

- Fresh-machine install instructions are tested.
- Release checklist includes post-publish verification commands.

## M8: Security, Fuzzing, and Robustness

### M8-01: Enforce panic-free parsing in public APIs

Labels: `priority:P0`, `type:security`, `area:reader`

- Audit all public parsing entry points for panics on adversarial input.
- Expand fuzz regression tests for any path with indexing/slicing.
- Add `RUST_BACKTRACE=1` repro notes for security reports.

Acceptance criteria:

- Fuzz regression suite covers all public byte parsers.
- Any panic found gets a minimized regression input.

### M8-02: Add WalkerLimits to production scanning

Labels: `priority:P1`, `type:security`, `area:walker`

- Ensure brute-force scans have configurable limits:
  - max scan bytes
  - max candidates
  - max trial offsets
  - max per-record decode bytes
- Apply limits in CLI, Python, and WASM.

Acceptance criteria:

- Crafted large files cannot trigger unbounded candidate scanning.
- Limit hits return diagnostics.

### M8-03: Add memory/performance budgets

Labels: `priority:P1`, `type:test`, `area:ci`

- Define budget targets for:
  - open
  - summarize
  - schema parse
  - element decode
  - IFC export
  - viewer parse/render
- Track against small, medium, and large corpus files.

Acceptance criteria:

- Benchmarks emit machine-readable results.
- CI or nightly flags regressions above threshold.

### M8-04: Keep supply-chain policy enforceable

Labels: `priority:P1`, `type:security`, `area:ci`

- Keep `cargo deny check` green.
- Keep RustSec advisory checks green.
- Review JS viewer dependencies separately with npm audit or a
  documented alternative.
- Pin or justify GitHub Actions versions.

Acceptance criteria:

- CI covers Rust and viewer dependency checks.
- Any ignored advisory has an issue, rationale, and expiry.

### M8-05: Verify no-network viewer invariant

Labels: `priority:P1`, `type:security`, `area:viewer`

- Keep WASM import audit.
- Add browser network test:
  - load viewer
  - open sample/demo
  - assert no network calls after initial static assets
- Document any allowed requests.

Acceptance criteria:

- Privacy posture is tested, not just documented.
- Network failures include request URL and initiator.

## M9: Write Path and Editing

### M9-01: Keep stream-level writer honest

Labels: `priority:P1`, `type:test`, `area:writer`

- Expand stream patch tests for:
  - grow
  - shrink
  - multi-stream
  - missing stream
  - corrupt gzip
  - unchanged identity patch
- Confirm GUID/history preservation behavior.

Acceptance criteria:

- Tests cover real family and project fixtures.
- Docs distinguish byte-preserving copy, stream patching, and semantic
  editing.

### M9-02: Design semantic write API, do not ship early

Labels: `priority:P2`, `type:research`, `area:writer`

- Draft design for field-level edits:
  - supported field types
  - transaction model
  - validation
  - backup/atomic write
  - Revit-openability checks
- Do not implement user-facing semantic writes until reader coverage is
  strong enough to avoid corrupting files.

Acceptance criteria:

- ADR documents risks and proposed API.
- Any prototype is behind an explicit experimental feature.

### M9-03: Add Revit-openability validation path

Labels: `priority:P3`, `type:research`, `area:writer`

- Define how to validate modified files without requiring every
  contributor to have Revit.
- Explore:
  - community validation volunteers
  - self-hosted Windows runner with licensed Revit, if legally allowed
  - checksum/structural invariants as partial substitute

Acceptance criteria:

- Documented decision on whether Revit-in-the-loop validation is in
  scope.

## M10: Governance, Releases, and Community

### M10-01: Turn this TODO into GitHub milestones

Labels: `priority:P0`, `type:project-management`

- Create milestones:
  - `0.2.0: audit-clean alpha`
  - `0.3.0: real-project wall/floor MVP`
  - `0.4.0: IFC geometry beta`
  - `0.5.0: viewer beta`
  - `1.0.0: first-class utility`
- Link issues from this TODO to milestones.

Acceptance criteria:

- Every P0/P1 task has a GitHub issue.
- Milestones have due criteria, not arbitrary dates.

### M10-02: Add issue forms for decoder work

Labels: `priority:P1`, `type:docs`, `area:github`

- Add `.github/ISSUE_TEMPLATE/element_decoder.yml`.
- Fields:
  - Revit class
  - target Revit versions
  - corpus file
  - source stream/offset evidence
  - expected IFC mapping
  - test plan
  - clean-room source declaration

Acceptance criteria:

- New decoder requests arrive with reproducible evidence.
- Template links to `docs/extending-layer-5b.md`.

### M10-03: Add issue form for corpus submissions

Labels: `priority:P1`, `type:docs`, `area:github`, `area:corpus`

- Ensure corpus issue form requires:
  - file license
  - redistributability
  - Revit version
  - file size
  - SHA256
  - expected contents
  - whether PII has been reviewed

Acceptance criteria:

- Corpus submissions are legally triageable without back-and-forth.

### M10-04: Publish an honest contribution map

Labels: `priority:P2`, `type:docs`, `area:docs`

- Add `docs/contribution-map.md`.
- Separate tasks for:
  - new contributors
  - Rust parser contributors
  - BIM/IFC experts
  - reverse-engineering contributors
  - documentation contributors
  - UI contributors

Acceptance criteria:

- Contributors can find a suitable task without reading the entire
  roadmap.

### M10-05: Establish release artifact verification

Labels: `priority:P1`, `type:release`, `area:ci`

- For each release, verify:
  - crate installs
  - Python wheel installs/imports on Linux/macOS/Windows
  - CLIs run on a sample
  - viewer build artifact loads
  - docs.rs builds
- Record output in release notes.

Acceptance criteria:

- Release checklist has exact commands.
- Failed verification blocks release publication or triggers hotfix.

## M11: First-Class Utility MVP

The smallest release that creates meaningful value for real industry
users should target this vertical slice.

### M11-01: Supported MVP input profile

Labels: `priority:P0`, `type:feature`, `area:product`

- Define a narrow support statement:
  - Revit versions: choose exact versions based on corpus, likely
    2023 and 2024 first.
  - File types: `.rvt` project files first; `.rfa` family support
    remains metadata/schema unless elements are validated.
  - Disciplines: architectural core first.
  - Classes: levels, walls, floors, doors, windows, rooms, materials.

Acceptance criteria:

- Support statement is in README and viewer.
- Unsupported files produce clear diagnostics.

### M11-02: End-to-end MVP workflow

Labels: `priority:P0`, `type:feature`, `area:product`

Workflow:

- User opens viewer.
- User drops a supported `.rvt`.
- Viewer parses locally.
- Viewer shows levels and model elements.
- User can click wall/door/window/room and inspect fields.
- User can export IFC.
- IFC opens in IfcOpenShell/BlenderBIM with typed geometry-bearing
  elements.
- User can export diagnostics for bug reports.

Acceptance criteria:

- Playwright test covers the workflow with a demo fixture.
- CLI and Python can reproduce the same decoded counts.
- README has a short "supported MVP workflow" section.

### M11-03: User-facing failure modes

Labels: `priority:P0`, `type:feature`, `area:product`

- For unsupported inputs, show one of:
  - unsupported Revit version
  - supported file but unsupported model layout
  - corrupt file
  - partial decode
  - scaffold-only export
  - parser bug, please report
- Include a diagnostics download button.

Acceptance criteria:

- No user gets an apparently successful empty export without a warning.
- CLI exits non-zero in strict mode for unsupported real-model export.

### M11-04: Documentation for non-technical users

Labels: `priority:P1`, `type:docs`, `area:docs`

- Add `docs/user-guide.md`.
- Include:
  - what the tool does
  - what stays private
  - how to open a file
  - how to export IFC
  - how to understand warnings
  - what file types/versions are supported
  - how to report a bad file

Acceptance criteria:

- User guide is linked from viewer, README, and release page.
- It avoids implementation jargon unless a term is explained.

## Backlog: Valuable After MVP

These should not block the MVP unless a real user need moves them up.

- Revit link resolution across host and linked models.
- IFC2X3 and IFC4.3 output modes.
- Type instancing through `IfcRepresentationMap` for all repeated
  families.
- Detailed MEP route geometry.
- Annotation/sheet graphics.
- Worksharing/cloud model semantics.
- Semantic `.rvt` write-back.
- Desktop wrapper.
- buildingSMART certification tool integration.

## Audit Snapshot

Last local audit basis: 2026-04-25.

Observed local checks:

- `cargo test --workspace --all-targets` passed.
- `cargo check --lib --features wasm --no-default-features` passed.
- `npm run typecheck` in `viewer/` passed.
- `npm run build` in `viewer/` passed with a bundle-size warning.
- `cargo deny check` passed with allowlist warnings.
- `cargo fmt --all -- --check` failed due formatting drift in the
  current worktree.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  failed on current Rust due one lint in `src/formats.rs`.
- `cargo audit` was not installed locally.

Observed model extraction state:

- `Revit_IFC5_Einhoven.rvt` generic walker: 9 `HostObjAttr` candidates.
- `Revit_IFC5_Einhoven.rvt` ArcWall path: Revit-2023-specific wall
  records emit `IFCWALL` entries, currently geometry-free.
- `2024_Core_Interior.rvt`: IFC scaffold only; no typed elements.

Do not use this snapshot as a substitute for current CI. Re-run checks
before closing any issue.
