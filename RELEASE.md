# Release process

How rvt-rs cuts, tags, and publishes a release. Written for maintainers;
also useful for downstream packagers who want to understand what every
version number means.

## Versioning

rvt-rs follows [Semantic Versioning 2.0](https://semver.org/spec/v2.0.0.html).

Pre-1.0, the project uses the Cargo convention — breaking changes bump
the **minor** version, not the major. That is:

- **MAJOR** — reserved. `1.0.0` is the first release with a stability
  commitment. Until then, major stays at `0`.
- **MINOR** — breaking API changes (pre-1.0) / new backwards-compatible
  features (post-1.0). Any change that renames a public item, changes a
  function signature, removes a variant, or alters serialised output
  shape goes here.
- **PATCH** — backwards-compatible bug fixes, doc corrections,
  performance work, and internal refactors that don't change public
  behaviour.

The current version lives in `Cargo.toml` under `[package].version` and
is mirrored in `CITATION.cff` and `.release-please-manifest.json`. All
three must agree at tag time.

## Release cadence

There is no fixed cadence. Releases happen when material changes
stabilise — a batch of decoders lands, an audit-driven correctness
repair closes, a security fix ships, the schema layer covers a new
Revit year. The project does **not** promise monthly, quarterly, or
"every N weeks" releases.

If downstream packagers need a stable tag, pin to one. If you need a
specific fix that hasn't shipped, the [hotfix process](#hotfix-process)
below applies.

## What's in a release

Every `vX.Y.Z` release:

- Has a signed git tag `vX.Y.Z` on `main`.
- Has a GitHub release page generated from the tag, with changelog
  excerpt, a link to the [user guide](docs/user-guide.md), and pre-built
  artefacts from the `publish.yml` workflow.
- Publishes the `rvt` crate to [crates.io](https://crates.io/crates/rvt)
  via `cargo publish`.
- Publishes Python wheels (one per OS: Linux, macOS, Windows) and an
  sdist to [PyPI](https://pypi.org/project/rvt/) via the Trusted
  Publisher OIDC flow in `.github/workflows/publish.yml`.
- Updates `CHANGELOG.md` with human-readable entries grouped by
  `Added` / `Changed` / `Fixed` / `Security` (Keep a Changelog format).
- Updates `CITATION.cff` (`version:` + `date-released:`) so academic
  citations resolve to the right archival state.

## Release checklist (for maintainers)

Run in order. Don't skip steps — the audit-driven culture for this
project treats a silent skipped step the same as an untested change.

1. **CI clean on `main`.** `cargo fmt --check`, `cargo clippy
   --all-targets --all-features -- -D warnings`, and the full test
   matrix (ubuntu/macos/windows × stable + MSRV 1.85 on ubuntu) are all
   green. `cargo deny check` and `cargo audit` are green.
2. **Synthetic-project IFC integration test.** `cargo test --release
   --test ifc_synthetic_project`. This is the end-to-end regression
   gate — a synthesised document is written, re-read, and round-tripped
   through the IFC4 STEP exporter.
3. **Corpus integration tests.** If the phi-ag/rvt corpus is available
   locally, `RVT_SAMPLES_DIR=... cargo test --release --test samples
   --test ifc_roundtrip --test field_type_coverage`. CI runs these on
   every push, but running once locally against the checked-out corpus
   before tagging catches anything a stale cache might mask.
4. **Bump version.** Edit `Cargo.toml` `[package].version`. If the
   Python wheel is part of this release, `pyproject.toml` derives its
   version from `Cargo.toml`, so no manual bump is needed there —
   confirm with `grep -n version pyproject.toml`.
5. **Update `CHANGELOG.md`.** Move items from `[Unreleased]` into a new
   `[X.Y.Z] — YYYY-MM-DD` section. Group by `Added` / `Changed` /
   `Fixed` / `Security`. Write entries so a user reading just this
   section can decide whether to upgrade.
6. **Update `CITATION.cff`.** Set `version:` to `X.Y.Z` and
   `date-released:` to the ISO date. These two fields must agree with
   the git tag and the `Cargo.toml` version.
7. **Update `.release-please-manifest.json`.** Set `"."` to the new
   version. This keeps release-please in sync even though the workflow
   isn't yet wired up (see [Release automation](#release-automation)).
8. **Commit on `main`.** Commit message: `release: vX.Y.Z`. Include the
   four touched files: `Cargo.toml`, `CHANGELOG.md`, `CITATION.cff`,
   `.release-please-manifest.json`.
9. **Tag.** `git tag -s vX.Y.Z -m "rvt-rs X.Y.Z"`. The `-s` signs the
   tag with the maintainer's GPG key — required for release
   authenticity.
10. **Push.** `git push origin main --tags`. The tag push triggers
    `publish.yml` which builds wheels + sdist on all three OSes and
    uploads to PyPI via OIDC.
11. **Publish the crate.** `cargo publish`. `cargo publish` re-runs
    tests and a clean package build before upload, so a broken tag
    still can't ship to crates.io.
12. **Draft the GitHub release.** On the repo's Releases page, draft a
    new release from the `vX.Y.Z` tag. Title `rvt-rs vX.Y.Z`. Body =
    the `CHANGELOG.md` excerpt for this version, verbatim, followed by
    `User guide: https://github.com/DrunkOnJava/rvt-rs/blob/vX.Y.Z/docs/user-guide.md`
    with the real tag substituted. Publish once the `publish.yml` run has
    finished and artefacts are attached.
13. **Announce (optional).** If the release is worth calling out,
    drafts live under `docs/launch/` — currently `hn-show-hn.md` and
    `reddit-r-rust.md`, with further channels (buildingSMART forum,
    OSArch) added as drafts land. Post from the drafts rather than
    writing fresh copy; the drafts have been audit-reviewed for
    overclaiming.

## Post-Publish Install Verification

Run these from a clean shell after the GitHub release, crates.io publish, PyPI
publish, and viewer deploy finish. Replace `X.Y.Z` with the released version.

```bash
cargo install rvt --version X.Y.Z --locked
rvt-inspect --version

python -m venv /tmp/rvt-release-smoke
. /tmp/rvt-release-smoke/bin/activate
python -m pip install --upgrade pip
python -m pip install "rvt==X.Y.Z"
python -c "import rvt; print(rvt.__version__)"
```

Open <https://drunkonjava.github.io/rvt-rs/> in a fresh browser session and
confirm the viewer loads. If a redistributable sample is available, inspect it
with both `rvt-inspect` and the viewer diagnostics download and compare the
failure mode.

## Hotfix process

When a critical issue (security advisory, data-loss regression, broken
docs on docs.rs blocking adoption) surfaces between releases:

1. **Branch from the latest release tag.** `git checkout -b
   hotfix/vX.Y.(Z+1) vX.Y.Z`.
2. **Apply the minimal fix.** No refactoring, no adjacent cleanup, no
   new features. Smallest possible diff that closes the issue, plus a
   regression test that fails without the fix.
3. **Bump PATCH.** `Cargo.toml` + `CITATION.cff` +
   `.release-please-manifest.json` to `X.Y.(Z+1)`.
4. **Abbreviated checklist.** Steps 1–3 above (CI + synthetic-project
   test + corpus if available). Skip nothing else.
5. **Tag, push, publish.** Same as the full release flow — `git tag -s`,
   `git push origin --tags`, `cargo publish`, draft GitHub release.
6. **Backport to `main`** if the fix applies there too. `git checkout
   main && git cherry-pick <hotfix-commit>`, resolve any drift, push.

## Post-release

- **Monitor `docs.rs`.** The docs build is async and often trails the
  `cargo publish` by several minutes. If it fails, the package page
  shows a red banner; triage the build log and cut a `PATCH` with the
  docs fix. The `[package.metadata.docs.rs]` block in `Cargo.toml`
  already sets `all-features = true` and the `docsrs` cfg, so feature
  gating (`#[cfg(docsrs)]`) in the source is respected.
- **Watch issues.** Anything filed with the `release-blocker` label
  after the release goes out is a hotfix candidate.
- **Confirm wheel availability.** `pip install rvt==X.Y.Z` should
  resolve on Linux, macOS, and Windows within a few minutes of the
  `publish.yml` workflow finishing. If PyPI reports only an sdist,
  re-check the wheel artefact step in the workflow run.
- **Confirm the crate is queryable.** `cargo search rvt` should return
  the new version. `cargo install rvt --version X.Y.Z` should succeed.

## Release automation

The following is the actual state of release infrastructure, not the
aspirational state. Update this section when you wire up more of it.

- **CI on every push to `main` and on every PR** (`.github/workflows/ci.yml`):
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test` across `ubuntu-latest`, `macos-latest`,
    `windows-latest` × stable + MSRV 1.85 on ubuntu
  - `cargo doc --no-deps --lib` with `RUSTDOCFLAGS=-D warnings`
  - `cargo deny check` (license allowlist, RustSec advisory deny,
    crates.io-only source, wildcard deny — see `deny.toml`)
  - [`rustsec/audit-check`](https://github.com/rustsec/audit-check)
  - Python wheel build + pytest integration tests on all three OSes
  - PII-shape guard (belt-and-suspenders on top of `src/redact.rs`)
- **Publish workflow** (`.github/workflows/publish.yml`):
  - Triggered on tag pushes matching `v*` and on
    `workflow_dispatch`
  - Builds `abi3-py38` wheels on Linux / macOS / Windows via
    `PyO3/maturin-action@v1` + an sdist
  - Uploads to PyPI via OIDC **Trusted Publisher** — no PyPI API token
    stored in the repo. One-time PyPI setup instructions are inlined
    at the top of `publish.yml`.
  - `workflow_dispatch` with `test-pypi: true` publishes to TestPyPI
    instead — use this for dry runs of a new release path before the
    first real tag.
- **Dependabot** (`.github/dependabot.yml`):
  - Weekly `cargo` PRs, limit 5 open, label `dependencies` + `rust`
  - Weekly `github-actions` PRs, label `dependencies` + `github-actions`
- **release-please**: configuration is in place
  (`release-please-config.json` + `.release-please-manifest.json`
  targeting `rust` release-type for this crate) but is **not yet
  wired** to a GitHub Actions workflow. Until that lands, the
  release checklist above is run manually.
- **TestPyPI trial run** (task `Q-13` in the backlog) and **`docs.rs`
  preview build validation** (task `Q-12`) are planned but have not
  yet been exercised for any real release. The first maintainer to cut
  a release after those tasks land should update this section and
  `CHANGELOG.md` accordingly.
