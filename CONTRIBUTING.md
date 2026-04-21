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

## Where help is most wanted

**Current shipped surface** (as of April 2026 / post-Phase-5a):

- 395 classes · 13,570 fields · 100% type classification
- **80 typed element decoders** (every major element class:
  walls, floors, roofs, ceilings, doors, windows, columns, beams,
  stairs, railings, rooms, furniture, 11 MEP classes, annotations,
  parameters — see `src/elements/mod.rs`'s `all_decoders()` for
  the full list).
- **Schema-directed ADocument walker** across all 11 releases
  (fully on 2024-2026; graceful `Decoded::partial` on 2016-2023).
- **Full IFC4 STEP export** (`rvt-ifc input.rfa input.ifc`): spatial
  hierarchy + per-element entities + 8 IfcProfileDef subclasses
  (IFC-24) + extruded / revolved / boolean / faceted-brep / swept-
  path solids (IFC-16/17/18/19/20) + material layer sets + material
  profile sets + property sets + opening/fill rels + IfcMember
  secondary-member routing + IfcRepresentationMap shared geometry
  (IFC-21) + ForgeUnit → IfcSIUnit/IfcConversionBasedUnit (IFC-39/40).
- **Three-layer validation CI** on every commit: ifc-smoke
  (substring counts) + IfcOpenShell (spec-level parse) + 382+
  lib tests (per-feature coverage). See `docs/validation-evidence.md`.
- **Python bindings**: `pip install rvt` (pyo3 + maturin wheel,
  abi3-py38, one per OS/arch).
- **Parameter system**: `ParameterElement` + `SharedParameter`
  definitions, AProperty* value-carrier decoders (L5B-54), typed
  `ParameterValue` enum, type-instance inheritance resolution
  (L5B-55), calculated/reporting flag detection (L5B-56).

**Most wanted right now:**

1. **§L5B-11 — extend walker to Revit 2016–2023.** Walker finds
   entry points across all 11 releases but fully decodes ADocument
   fields only on 2024–2026. Older releases need per-band heuristics
   (see recon report §Q6.5). Needs corpus byte-inspection.
2. **§L5B-09 — generalize Container 2-column decoder.** Current
   implementation handles `kind: 0x0e` with 6-byte records; other
   container kinds need reverse-engineering against live instance
   data.
3. **§GEO — geometry extraction from the object graph.** Writer
   + bridge can already emit per-element geometry when the caller
   supplies dimensions; the *extraction* side (reading location
   curves / profile shapes / arbitrary brep from the Revit element
   bytes) is the open research frontier. GEO-27..35 tasks are the
   breakdown.
4. **§WRT — write path.** Byte-preserving stream-level patching
   round-trips (`write_with_patches`), but field-level semantic
   writes (edit a Wall's height and round-trip to a Revit-openable
   .rvt) are the big next subsystem. WRT-01..14.
5. **§VW1 — web viewer (WASM + Three.js).** VW1-01..24.
6. **Corpus expansion.** We only have the `rac_basic_sample_family`
   11-release corpus. Donations of real-world project RVTs (with
   redistribution rights) would dramatically widen validation —
   tracked as Q-01.

Each layer has a clear validation oracle: rvt-info extracts
document title + GUID via metadata; rvt-history gives the upgrade
timeline via Phase D string scanning; IfcOpenShell validates IFC
output against the full IFC4 schema (enforced in CI per IFC-41).

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

## Fuzzing

`rvt-rs` parses untrusted on-disk byte streams, so coverage-guided
fuzzing is part of the safety story. A `cargo-fuzz` workspace
lives at `fuzz/` — see [`fuzz/README.md`](fuzz/README.md) for the
full reference.

`cargo-fuzz` drives libFuzzer against a single entry-point per
target: you write a small `fuzz_target!` that takes a `&[u8]` and
feeds it into one parser surface, and libFuzzer mutates a corpus
looking for any input that makes the target panic, abort, time
out, or OOM. The fuzz crate is a standalone workspace so that the
main `cargo build` does not need nightly Rust.

To add a new fuzz target:

1. Pick the parser surface you want to harden and check whether
   it already has a tracked task in the `SEC-14..SEC-23` series
   (listed in `fuzz/README.md`). If it does, claim that task.
2. Create `fuzz/fuzz_targets/<name>.rs` using the libfuzzer-sys
   template and register it as a `[[bin]]` entry in
   `fuzz/Cargo.toml`.
3. Run the target locally (`cargo +nightly fuzz run <name>`) for
   long enough to exercise mutation — a few minutes at minimum,
   longer for anything that touches decompression or XML.
4. Commit any reproducible crashes to `fuzz/corpus/<name>/` as
   regression inputs (tracked separately under Q-04).

The scaffold itself is tracked as SEC-14; the individual targets
are SEC-15 through SEC-23, and a nightly CI runner is SEC-25.

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

## Corpus env vars

Tests, benchmarks, and probes that need real Revit files resolve
their paths from environment variables so no contributor's home
directory leaks into the repo (the CI `PII guard` job enforces
this). Three variables are recognised, in decreasing specificity:

- `RVT_FAMILY_2024` — full path to a single `.rfa` sample
  (family-file probes).
- `RVT_SAMPLES_DIR` — directory holding the 11-release
  [`phi-ag/rvt`](https://github.com/phi-ag/rvt) corpus. Defaults
  to `../../samples` relative to the crate root.
- `RVT_PROJECT_CORPUS_DIR` — directory holding `.rvt` project
  files. Defaults to `/private/tmp/rvt-corpus-probe/magnetar/Revit`
  (the path the main contributor uses locally for the
  [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets)
  MIT-licensed corpus).

Tests and benches that need these files skip gracefully if the
path doesn't exist, so a fresh clone runs all non-corpus-dependent
suites green without any env setup. To enable the corpus suites:

```bash
# Family corpus (LFS-tracked, 11 releases 2016-2026)
git clone https://github.com/phi-ag/rvt /tmp/phiag
export RVT_SAMPLES_DIR=/tmp/phiag/examples/Autodesk

# Project corpus (LFS-tracked, 2023 and 2024 real .rvt files)
git clone https://github.com/magnetar-io/revit-test-datasets /tmp/magnetar
export RVT_PROJECT_CORPUS_DIR=/tmp/magnetar/Revit

cargo test                                          # full suite
cargo bench --bench project_file                    # Q-07 multi-MB
cargo run --release --example probe_latest_framing  # any probe
```

Never hardcode absolute paths in test or probe code — the PII
guard job scans for `/Users/<name>/` and `/home/<name>/`
patterns on every push.

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
