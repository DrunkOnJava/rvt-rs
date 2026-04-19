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

## Where help is most wanted (as of v0.1.1)

The schema layer (Layer 4) is complete and regression-proofed: 100%
field-type classification across every Revit release 2016–2026,
enforced by CI. The instance layer (Layer 5) is wide open, with
three stacked sub-questions any of which is a genuine contribution:

1. **§Q6.4 — decode the `Global/Latest` directory-table format.**
   Right after the document-upgrade-history block in
   `Global/Latest`, there is a contiguous sequential-ID TLV table
   of ~131 records (stable across all 11 releases). Record bodies
   are 2, 6, 12, 14, or 16 bytes; u16 body values are not schema
   class tags (0/131 hit — see `examples/directory_class_lookup.rs`).
   Strong candidates: ElementId references, indices into
   `Global/ElemTable`, or a reference-pool. Cross-reference against
   `Global/ElemTable`'s records and/or test on a non-sample-family
   `.rvt` (any real project) to see whether the count is
   per-document or format-structural.
2. **§Q6.5 — locate ADocument's actual instance bytes.** Blocked on
   Q6.4 — once we know what the directory indexes, we can follow
   its entries to ADocument's real location (not the post-history
   offset, as originally hypothesised and refuted in §Q6.3).
3. **Layer 5a — schema-directed walker.** Blocked on Q6.5. The
   skeleton has been scoped; implementation is bounded once Q6.5
   produces the entry-point offset.

Each step has clear validation oracles: rvt-info already extracts
the document title + GUID via a second path, rvt-history extracts
the upgrade timeline via Phase D string scanning. Anything the
walker pulls must cross-check.

See `docs/rvt-moat-break-reconnaissance.md` §Q6 for the full state
of play, including the documented refutation of the original Q6.2
entry-point hypothesis (this is the first project section a
contributor should read before starting on Layer 5).

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
