# Contributing to rvt-rs

Thanks for your interest. This project is small and evolving
quickly, so contribution guidelines are intentionally light — but
a few practices keep the repo healthy.

## What's welcome

- **Bug reports** with a minimal reproducer (the smallest `.rfa`
  or `.rvt` that triggers the issue). Security-sensitive reports
  go through [`SECURITY.md`](SECURITY.md), not public issues.
- **Performance regressions** caught by the benchmark harness in
  `tools/bench.sh` — open an issue with a before/after table.
- **New FACTs** about the file format. The reconnaissance report in
  `docs/rvt-moat-break-reconnaissance.md` is the canonical place
  for dated findings. Please mirror any new finding there AND as a
  reproducible probe under `examples/`.
- **Documentation improvements.** The README and inline doc comments
  are fair game.
- **Tests.** More coverage is always welcome, especially for
  edge-case file layouts.

## Where help is most wanted (as of v0.1.2)

**Layer 5a (ADocument walker) exists and is reliable on Revit
2024–2026.** Layer 5b (per-element walker: walls, floors, doors,
windows, columns, etc.) is the current beachhead. The schema layer
is 100%-classified and CI-gated. Per-element extraction + geometry
+ a real IFC exporter + a web viewer are the phases documented in
`TODO-BLINDSIDE.md` at the repo parent.

Most wanted right now:

1. **§L5B — per-element decoders.** The schema tells you exactly
   what bytes each element class consumes. Pick any class from the
   L5B-XX task list (Wall, Floor, Door, Level, Material, etc.) and
   implement its `ElementDecoder`. Each one is a few hundred lines
   + unit test + integration test against corpus.
2. **§L5B-11 — extend walker to Revit 2016–2023.** Walker detects
   entry points across all 11 releases but cleanly decodes all 13
   ADocument fields only on 2024–2026. Older releases need per-band
   entry-point heuristics (see recon report §Q6.5-F addendum).
3. **§GEO — geometry extraction.** Once decoders surface location
   curves, profiles, sketches — turn them into `Solid` enum values
   (Extrusion, Sweep, Revolve, Blend, Boolean). IFC exporter
   consumes these.
4. **§IFC-02+ — real IfcWall/IfcSlab/IfcDoor/... emission.** Today
   we emit `IfcProject → IfcSite → IfcBuilding → IfcBuildingStorey`
   scaffolding only. Wiring per-element entities with geometry +
   material layer sets + property sets is the blind-side unlock.
5. **§VW1 — web viewer (WASM + Three.js).** Once any geometry is
   extractable, a basic viewer showing a 3D model becomes
   immediately useful to BIM engineers. glTF export for Blender
   compatibility is the first milestone.
6. **Corpus expansion.** We only have the `rac_basic_sample_family`
   11-release corpus. Donations of real-world project RVTs (with
   redistribution rights) would dramatically widen validation.

Each layer has a clear validation oracle: rvt-info extracts
document title + GUID via metadata (for cross-checking walker
output); rvt-history gives the upgrade timeline via Phase D string
scanning; IfcOpenShell validates IFC output for free.

See `docs/rvt-moat-break-reconnaissance.md` §Q6 for Layer 5a's
research trail, including the documented refutation of the Q6.2
hypothesis. See `TODO-BLINDSIDE.md` (repo parent dir, local-only)
for the full per-task decomposition.

## What needs discussion first

Open an issue (or a draft PR) before starting work on any of:

- **Layer 5 itself** — the questions above are open research; a
  one-paragraph sketch of your approach in an issue saves everyone
  time before you spend days on a probe.
- **IFC exporter emission** (`src/ifc/`). Mapping decisions have
  to align with buildingSMART IFC schema conventions.
- **The modifying writer** (`src/writer::write_with_patches`). Any
  change to Revit's truncated-gzip framing must be verified
  against a round-trip test.
- **Layer 4c field-type decoder changes.** Coverage is at 100%
  and CI-gated. If you think a pattern is misclassified, file an
  issue with byte evidence from the corpus — do not silently
  change the decoder.

## Coding conventions

- Rust 2024 edition.
- `cargo fmt` before every commit.
- `cargo test --release` must pass. The CI in `.github/workflows/`
  enforces this.
- **No `unsafe` in the library crate.** If you genuinely need it,
  open an issue first to discuss.
- **No panics in parsing paths.** Malformed input must return an
  `Error`, never `panic!`.
- **No PII in tests.** Use synthetic fixtures — `testuser`,
  `111111`, `FY-20XX`, etc. The redaction tests in
  `src/redact.rs` are the canonical examples.
- **Every probe under `examples/`** gets a module-level doc
  comment explaining *what FACT it proves* and *how to verify*
  the result against the 11-version corpus.

## Commit messages

We use Conventional Commits:

- `feat(<scope>): ...` for new features
- `fix(<scope>): ...` for bug fixes
- `docs(<scope>): ...` for documentation
- `test(<scope>): ...` for test-only changes
- `refactor(<scope>): ...` for behavior-preserving internal changes
- `perf(<scope>): ...` for performance
- `chore(<scope>): ...` for infra / CI / build

Scopes that appear frequently: `formats`, `object_graph`,
`elem_table`, `partitions`, `writer`, `ifc`, `readme`, `cli`.

## Reverse-engineering findings

When you discover something new about the file format:

1. Write a short probe under `examples/<name>.rs` that reproduces
   the finding from bytes. One self-contained file, runs against
   the phi-ag/rvt sample corpus.
2. Add a dated addendum to `docs/rvt-moat-break-reconnaissance.md`
   with an evidence table and a confidence value.
3. If the finding is a decoding rule, also add a unit test that
   pins the byte pattern (see `FieldType::decode` tests in
   `src/formats.rs` for the pattern).

This keeps every claim independently verifiable, which is the
whole point of open reverse-engineering work.

## Legal note for contributors

rvt-rs is Apache-2.0 licensed. By submitting a contribution, you
agree that your work is licensable under Apache-2.0 and that you
have the right to grant that license.

**Please do not submit any code, comments, tests, or documentation
that contains information derived from Autodesk proprietary
sources** (NDA'd SDKs, decompiled binaries beyond what the public
`RevitAPI.dll` symbol export trivially exposes, leaked internal
documents, etc.). This project operates strictly from public
on-disk byte observations.

Questions: open an issue or email <151978260+DrunkOnJava@users.noreply.github.com>.
