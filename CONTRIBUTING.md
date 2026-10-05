# Contributing to rvt-rs

Thanks for your interest. This project is small and evolving quickly, so
the guidelines are intentionally light — but the gate below is the same one
CI applies, and everything in it runs without Revit, without the Autodesk SDK,
and without any private files: the synthetic fixtures under `corpus/tier1/`
drive the default checks, and corpus-gated tests skip themselves while
`RVT_PROJECT_CORPUS_DIR` is unset.

## Ten minutes to a first pull request

You need stable Rust 1.87 or newer (`rustup` installs it) and git.

1. Fork the repository on GitHub, then clone your fork:

   ```bash
   git clone git@github.com:<your-user>/rvt-rs.git
   cd rvt-rs
   ```

2. Run the local gate. The first run compiles everything, which takes a few
   minutes; later runs are incremental.

   ```bash
   tools/check-local.sh
   ```

   That runs `cargo fmt --check`, `cargo clippy` with `-D warnings`, rustdoc
   with `-D warnings`, and the integration tests under `tests/` — the same
   checks a pull request must pass. Green here means your machine is set up.

3. Pick something small. Tasks that fit in an afternoon and need no corpus
   files are labelled
   [good first issue](https://github.com/DrunkOnJava/rvt-rs/labels/good%20first%20issue);
   [`docs/contribution-map.md`](docs/contribution-map.md) lists the larger
   areas and where each one starts.

4. Branch, change, and run the gate again:

   ```bash
   git switch -c fix/short-description
   # ...edit...
   tools/check-local.sh
   ```

5. Optional: `git config core.hooksPath .githooks` turns on a pre-commit hook
   that runs only `cargo fmt --check`, so the cheapest failure never reaches CI.

6. Commit with a [Conventional Commits](#commit-messages) message
   (`docs(...)`, `fix(...)`, `test(...)`), push to your fork, and open a pull
   request against `main`. The pull-request template asks what changed and
   what you ran — fill in what applies and write "N/A" for the rest.

7. A maintainer reviews and squash-merges. GitHub does not run CI on a
   first-time contributor's pull request until a maintainer clicks
   "Approve and run", so a PR that looks idle for a while is waiting on that,
   not on you. Leave "Allow edits by maintainers" on and we can push small
   fix-ups to your branch instead of bouncing it back.
   [`docs/maintaining.md`](docs/maintaining.md) says how issues are triaged
   and labelled, what a pull request has to pass, and how releases are cut.

### Optional gates

`tools/check-local.sh` never needs the network by default. Opt into the
heavier checks when your change touches them:

```bash
tools/check-local.sh --viewer          # viewer typecheck + build
tools/check-local.sh --corpus          # corpus-backed tests (dirs must exist)
tools/check-local.sh --ifcopenshell    # require the ifcopenshell Python module
tools/check-local.sh --deny --audit    # require cargo-deny / cargo-audit
tools/check-local.sh --all-optional    # enable every optional gate
```

`tools/quality.sh` is the fuller pre-push path (optional supply-chain tools
when installed, plus `--full` bench compile); set `RVT_REQUIRE_AUDIT=1` or
`RVT_REQUIRE_DENY=1` there when missing tools should fail. Supply-chain rules
for Rust crates, viewer pnpm dependencies, advisory ignores, and GitHub Actions
pinning are documented in
[`docs/supply-chain-policy.md`](docs/supply-chain-policy.md).

Two CI behaviours are worth knowing before you read a red or amber check.
The `viewer dependency audit` job retries `pnpm audit --audit-level high` and
reports `NOT MEASURED` as a warning, passing the job, when the advisory
endpoint is unreachable — a measured high or critical advisory still fails it
(#258). And CI on a pull request is deliberately narrower than on `main`: the
bench compile-check builds with `--profile ci`, the macOS and Windows test jobs
build `--lib --bins --tests` rather than `--all-targets`, and only the Linux
Python wheel builds, with the full platform matrix reserved for pushes to
`main` (pull request
[#260](https://github.com/DrunkOnJava/rvt-rs/pull/260), merged
2026-09-19).

**Unit tests are banned.** Do not add them. Changes are verified end to
end: against real Revit files and Revit's own exports, through the built
CLIs, and by the contract checks that keep the published claims honest.
`tools/ci/test-targets.txt` classes every file under `tests/` as `real`,
`cli`, `contract` or `unit`, and `tools/ci/verify-real-files.sh` runs all
but `unit`, in CI on every OS (with the family corpus on Linux and Windows)
and again against the project corpora in the tier-2 job. The script fails
on a file the manifest does not class, so a new target cannot slip past.
Targets that read a corpus must skip cleanly when their environment
variable is unset: the macOS and MSRV runs have no corpus.

## What's welcome

- **Bug reports** with a minimal reproducer (the smallest `.rfa`
  or `.rvt` that triggers the issue). Security-sensitive reports
  go through [`SECURITY.md`](SECURITY.md), not public issues.
- **Performance regressions** caught by the benchmark harness in
  `tools/bench.sh` — open an issue with a before/after table.
- **New facts** about the file format, as a probe under `examples/` and a
  dated report under `reports/element-framing/` (see
  [Reverse-engineering findings](#reverse-engineering-findings)).
- **Documentation improvements.** The README and inline doc comments
  are fair game.
- **End-to-end checks.** A test under `tests/` that runs the real entry
  point (a CLI, the library on a file, the exporter) against a fixture or a
  corpus file and checks what a user would see. Unit tests are not accepted:
  CI and `tools/check-local.sh` do not run them, so they would check nothing.
  For a format fact, the evidence is a probe under `examples/` and its
  report.

## Where help is most wanted

What actually works today is in [`docs/status.md`](docs/status.md) and the
executable [`docs/support-matrix.json`](docs/support-matrix.json); the
per-area starting points are in
[`docs/contribution-map.md`](docs/contribution-map.md). In short:

- **Small, self-contained tasks** are labelled
  [good first issue](https://github.com/DrunkOnJava/rvt-rs/labels/good%20first%20issue)
  when there are any; the documentation and the tiny reference models below
  are always a good first contribution.
- **Corpus.** Redistributable `.rvt` / `.rfa` files with known element counts
  are the scarcest input. See [`docs/corpus-intake.md`](docs/corpus-intake.md)
  and the corpus issue form, and never send a file you are not certain you
  may share — a local probe you run yourself is the alternative.
  [`docs/reference-models.md`](docs/reference-models.md) lists the models
  every claim is measured on (`tools/fetch-reference-models.sh` fetches the
  licensed ones), five tiny models anyone with Revit can make for the open
  geometry questions, and how to report a problem with a model you cannot
  share: send `rvt-inspect --json` and `rvt-ifc --diagnostics` output, never
  the building.
- **Decoder research.** A byte probe under `examples/` plus a dated evidence
  table for one class or partition pattern (decoder issue form; the reports
  under `reports/element-framing/` show the shape). Typed extraction works on Revit 2023 to 2025 projects and not yet
  on other releases; [`ROADMAP.md`](ROADMAP.md) says what is partial and the
  reports under `reports/element-framing/` record the known negatives (RE-19,
  RE-20, RE-121 and others), so check before spending days on one.
- **Tests that prevent false-positive decode claims**, viewer accessibility,
  and plain-language documentation are always welcome.

## Questions

Questions, ideas and "is this a bug?" belong in
[Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions); issues track
work with a definite outcome. If you are not sure where something goes, a
Discussion is always fine.

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
- `tools/check-local.sh` must pass: fmt, clippy, rustdoc, a build of every
  target, and the real-file, CLI and contract checks. The CI in
  `.github/workflows/` enforces the same. No unit tests.
- **No `unsafe` in the library crate.** If you genuinely need it,
  open an issue first to discuss.
- **No panics in parsing paths.** Malformed input must return an
  `Error`, never `panic!`.
- **No PII in tests.** Use synthetic fixtures — `testuser`,
  `111111`, `FY-20XX`, etc. The redaction tests in
  `src/redact.rs` are the canonical examples.
- **Keep public status honest.** If a change affects user-visible
  capability, update [`docs/status.md`](docs/status.md),
  [`ROADMAP.md`](ROADMAP.md), or [`docs/compatibility.md`](docs/compatibility.md)
  in the same PR.
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

That isolation has a cost: `tools/check-local.sh` and the pull-request gate
never compile `fuzz/`, so a new field on a public type the targets construct
breaks them silently and the break only surfaces on the nightly Fuzz run. That
is what happened when `IfcEntity::BuildingElement` gained `predefined_type` in
#236 and `fuzz_step_writer` failed with E0063 until #259. If you add or remove
a field on a type a fuzz target builds, run
`cargo +nightly check --manifest-path fuzz/Cargo.toml` before pushing.

To add a new fuzz target:

1. Pick the parser surface you want to harden and check the
   "Current targets" list in `fuzz/README.md` so you do not duplicate
   one of the eleven existing targets.
2. Create `fuzz/fuzz_targets/<name>.rs` using the libfuzzer-sys
   template and register it as a `[[bin]]` entry in
   `fuzz/Cargo.toml`.
3. Run the target locally (`cargo +nightly fuzz run <name>`) for
   long enough to exercise mutation — a few minutes at minimum,
   longer for anything that touches decompression or XML.
4. Turn any reproducible crash into a stable regression test in
   `tests/fuzz_regressions.rs` — it runs without nightly in normal CI.
   `fuzz/corpus/` itself is git-ignored.

`.github/workflows/fuzz.yml` runs every target nightly with a bounded
budget and uploads the crash corpus as an artifact on failure.

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
   the finding from bytes: one self-contained file whose module doc states
   the fact it proves and how to check it, with corpus paths taken from
   arguments or the `RVT_*` environment variables, never hardcoded. The
   `probe_re*.rs` files are the pattern.
2. Write a dated report, `reports/element-framing/RE-NN-<topic>.md`: status
   (positive or negative), the files and their sha256, the reference export
   measured against, and counts against Revit's own export (matches,
   misses, tolerance). A negative result is worth reporting; it keeps the
   next person from repeating the search.
3. If the finding changes what rvt-rs outputs, measure the change end to end
   (the scorers under `tools/re/` compare an export with Revit's) and update
   the public status in the same pull request: `docs/status.md`,
   `docs/support-matrix.json`, the README and `CHANGELOG.md`. The Measure
   workflow runs every scorer on the licensed reference models for you,
   before and after, on GitHub's runners, and can run your probe on them
   too; the differences land in the run summary:

   ```bash
   gh workflow run measure.yml --ref <branch> -f base=main
   gh workflow run measure.yml --ref <branch> -f base=none -f probe=<example name>
   gh workflow run measure.yml --ref <branch> -f base=main -f families=true
   ```

   Every run also has `rvt-info` read each model, and lists the files whose
   output differs: that is what measures the PartAtom parser, which the IFC
   export does not reach (the six reference models carry a `ProjectInformation`
   stream, read by the same parser, and no `PartAtom`). The last command adds
   the eleven-release Autodesk family corpus, the files with a `PartAtom`
   stream. Use it for a change to a parser or to a crate it depends on (the
   PartAtom reader, an inflate or decode dependency). To measure a branch with
   a workflow file that is not yet on `main`, dispatch with `--ref <branch with
   the workflow> -f ref=<branch to measure>`.

   The summary's `rvt-ifc` wall times come from the base and head jobs, which
   run on different runners; runner speed alone moves them by up to about
   1.7x. For a speed claim, add `-f timing=true`: one runner then builds both
   refs and exports each model (and each sample, with `-f samples=true`) with
   both in turn, and reports the medians and their ratio.

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

tools/ci/verify-real-files.sh --profile ci          # real-file, CLI, contract checks
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

Questions: open a GitHub issue. Security reports go through
[`SECURITY.md`](SECURITY.md) (private vulnerability reporting) — do
not use `users.noreply.github.com` addresses.
