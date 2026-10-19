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

The current version lives in `Cargo.toml` under `[package].version`. It
is repeated in `rvt-py/Cargo.toml` (the wheel's version, and its `=X.Y.Z`
dependency on `rvt`), `rvt-wasm/Cargo.toml`, the `rvt` dependency of
`tools/stream_evidence/Cargo.toml`, `viewer/package.json`, `CITATION.cff`,
`.release-please-manifest.json`, and the workspace crates' entries in
`Cargo.lock` and `fuzz/Cargo.lock`. All of them must agree at tag time.

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

- Has a signed git tag `vX.Y.Z` on `main`, or for a patch that `main`
  cannot release, on its hotfix branch ([Hotfix process](#hotfix-process)).
- Has a GitHub release page for the tag, whose body is the version's
  `CHANGELOG.md` section and a link to the [user guide](docs/user-guide.md),
  carrying the CLI archives for Linux, macOS and Windows, `SHA256SUMS` and
  their build-provenance attestations.
- Publishes the `rvt` crate to [crates.io](https://crates.io/crates/rvt),
  Python wheels for Linux (glibc and musl), macOS and Windows and an sdist
  to [PyPI](https://pypi.org/project/rvt/), and the container image
  `ghcr.io/drunkonjava/rvt-rs:X.Y.Z`, all from
  `.github/workflows/publish.yml`. crates.io and PyPI accept it by Trusted
  Publishing (OIDC), so no registry token is stored.
- Updates `CHANGELOG.md` with human-readable entries grouped by
  `Added` / `Changed` / `Fixed` / `Security` (Keep a Changelog format).
- Updates `CITATION.cff` (`version:` + `date-released:`) so academic
  citations resolve to the right archival state.

## Release checklist (for maintainers)

Run in order. Don't skip steps — the audit-driven culture for this
project treats a silent skipped step the same as an untested change.

1. **CI clean on `main`.** `cargo fmt --check`, `cargo clippy
   --all-targets --all-features -- -D warnings`, and the full test
   matrix (ubuntu/macos/windows × stable + MSRV 1.87 on ubuntu) are all
   green. `cargo deny check` and `cargo audit` are green.
2. **Synthetic-project IFC integration test.** `cargo test --release
   --test integration -- ifc_synthetic_project::`. This is the end-to-end
   regression gate — a synthesised document is written, re-read, and
   round-tripped through the IFC4 STEP exporter.
3. **Corpus integration tests.** If the phi-ag/rvt corpus is available
   locally, `RVT_SAMPLES_DIR=... cargo test --release --test integration --
   samples:: ifc_roundtrip:: field_type_coverage::`. CI runs these on
   every push, but running once locally against the checked-out corpus
   before tagging catches anything a stale cache might mask.
4. **Bump the version** in every file listed under
   [Versioning](#versioning). The wheel takes its version from
   `rvt-py/Cargo.toml` (`pyproject.toml` declares it dynamic), so
   `pyproject.toml` needs no edit.
5. **Update `CHANGELOG.md`.** Move items from `[Unreleased]` into a new
   `[X.Y.Z] — YYYY-MM-DD` section, leaving an empty `[Unreleased]` above
   it. Open the section with a short introduction and an **Upgrading from
   X.Y** list of what a user of the previous release has to change. Then
   group by `Added` / `Changed` / `Fixed` / `Security` (and `Research`),
   one heading each: merges append headings of their own, so fold them,
   keeping every entry. Credit outside contributors by name. Write
   entries so a user reading just this section can decide whether to
   upgrade.
6. **Update `CITATION.cff`.** Set `version:` to `X.Y.Z` and
   `date-released:` to the ISO date. These two fields must agree with
   the git tag and the `Cargo.toml` version.
7. **Update `.release-please-manifest.json`.** Set `"."` to the new
   version. This keeps release-please in sync even though the workflow
   isn't yet wired up (see [Release automation](#release-automation)).
8. **Open the release pull request,** `chore(release): prepare vX.Y.Z`,
   with steps 4 to 7 and the README's release section
   (`## Release X.Y.Z and main`) moved to the new version. crates.io and
   PyPI show the README packed into each version, which can never be
   edited once it is published, so it names the release it ships in; it
   states no channel checks, which come after (Post-release). `main`
   takes changes only through pull requests, so the release metadata
   lands like any other change. The IFC header
   carries the version (#417), so both Core Interior `rvt-rs.json`
   witness observations and their verdicts change: take them from the
   pull request's CI run (its `octetproof-core-interior` artifact) and
   push them. Only `witness_version` and the verdict hashes should
   differ. Merging this pull request is not the release.
9. **Confirm the registries' trusted publishers.** crates.io and PyPI
   each need one for repository DrunkOnJava/rvt-rs, workflow
   `publish.yml`, environment `crates-io` or `pypi` (see
   [`docs/maintaining.md`](docs/maintaining.md#releases)). Without
   crates.io's, the crate's publish fails, and PyPI, the release
   binaries and the container image, which wait for it, never run.
10. **Tag the merged commit.** `git fetch origin`, check that
    `origin/main` is the release pull request's squash commit, then
    `git tag -s vX.Y.Z -m "rvt-rs X.Y.Z" origin/main`. The `-s` signs
    the tag with the maintainer's key, which release authenticity
    requires. The `release-tags` ruleset lets only a maintainer create
    a `v*` tag, and nobody can move or delete one.
11. **Push the tag.** `git push origin vX.Y.Z` runs `publish.yml`. It
    builds the wheels and the sdist and checks them: each wheel installed
    and opening a model, `cargo install` of the source with the CLI
    sample smoke, the docs.rs build preview and the viewer load smoke.
    Once they pass, the workflow publishes the crate to crates.io, with no
    approval asked for (pushing the tag is the release decision), then the
    wheels and sdist to PyPI and, through
    `release-binaries.yml`, the CLI archives and the container image with
    their attestations, both of which wait for the crate. If a job fails, fix its cause and
    re-run the failed jobs.
12. **Write the release page.** `release-binaries.yml` creates the
    release from the tag with GitHub's generated notes. Replace them with
    this version's `CHANGELOG.md` section, verbatim, followed by
    `User guide: https://github.com/DrunkOnJava/rvt-rs/blob/vX.Y.Z/docs/user-guide.md`
    with the real tag substituted
    (`gh release edit vX.Y.Z --notes-file <file>`).
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

When a fix has to ship before the next release from `main`, and `main`
cannot ship it as a patch because it already holds a breaking change: a
critical issue (security advisory, data-loss regression, broken docs on
docs.rs blocking adoption), or a published README that claims more than
was measured, which crates.io and PyPI keep showing until the next
version replaces it.

1. **Branch from the latest release tag.** `git checkout -b
   hotfix/vX.Y.(Z+1) vX.Y.Z`. GitHub runs a push's workflows from the
   pushed branch, and a tag before v0.6.1 has a `ci.yml` without the
   `hotfix/**` trigger step 4 needs: commit that trigger to the branch
   first, as `hotfix/v0.6.1` did.
2. **Apply the minimal fix.** No refactoring, no adjacent cleanup, no
   new features. Smallest possible diff that closes the issue; when it
   landed on `main` first, `git cherry-pick -x` it. Where no existing
   real-file, CLI or contract check fails without the fix, add one that
   does (unit tests are banned, see `CONTRIBUTING.md`).
3. **Prepare the release on the branch.** Steps 4 to 7 above (the
   version files, `CHANGELOG.md`, `CITATION.cff` and the manifest) and
   the README's release section, committed to the branch: no pull
   request into `main` can test a branch cut from a tag.
4. **Check it.** Pushing the branch runs CI on it with the full matrix
   `main` gets (`ci.yml` runs on `hotfix/**` pushes). The new version
   changes both Core Interior witness observations and their verdicts
   (step 8): take them from that run's `octetproof-core-interior`
   artifact, check that only `witness_version` and the verdict hashes
   differ, commit them and push again, and that run must pass too.
   Measure the branch against the release it patches, with the corpora
   the fix touches:
   `gh workflow run measure.yml --ref hotfix/vX.Y.(Z+1) -f base=vX.Y.Z`.
   The branch has no pull request, so nothing reviews its text, which
   crates.io and PyPI then show for good: push a branch at `vX.Y.Z`
   (`review/vX.Y.Z-base`) and open a review-only pull request of the
   hotfix branch into it, answer the review, then close it unmerged and
   delete that base after the tag. Skip nothing else in steps 1 to 3
   above.
5. **Tag the branch and push the tag.** Steps 9 to 12 above, with the
   branch's head in place of `origin/main`:
   `git tag -s vX.Y.(Z+1) -m "rvt-rs X.Y.(Z+1)" origin/hotfix/vX.Y.(Z+1)`.
6. **Bring `main` up to it** through a pull request, as merging the
   branch back would: the fix if `main` lacks it, the new `CHANGELOG.md`
   section (its entries moved out of `[Unreleased]`), the version files
   and the README's release section. `main` exports differently from the
   branch, so take its Core Interior witness observations and verdicts
   from that pull request's own CI run, as in step 8, not from the
   branch. Then delete the branch; the tag keeps its commits.

## Post-release

- **Update the docs.** Once every channel has been checked from what was
  published ([Post-Publish Install Verification](#post-publish-install-verification)),
  a pull request moves the other latest-release statements to the new
  version and records the checks: `docs/install.md`, `docs/python.md`,
  `docs/status.md`, `docs/supported-profile.md` and `ROADMAP.md`. The
  README's release section moved in the release pull request (step 8).
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
  fmt, clippy, the test matrix (Linux, macOS and Windows, stable, and the
  MSRV), rustdoc, `cargo deny`, `cargo audit`, the docs.rs build preview,
  the Python wheel and its integration tests, IfcOpenShell validation and
  the witness verdicts, the PII guard and the rest. `CI passed`, the one
  required check, passes only when every one of them did (see
  [`docs/maintaining.md`](docs/maintaining.md#pull-requests)).
- **Publish workflow** (`.github/workflows/publish.yml`), on `v*` tags and
  `workflow_dispatch`: the checks and publish order of step 11. Its
  crates.io and PyPI jobs authenticate by Trusted Publishing in the
  `crates-io` and `pypi` environments, which only `v*` tags may deploy
  to; PyPI and the release binaries wait for the crates.io job. The
  one-time registry setup
  is described at the top of the file.
  `workflow_dispatch` with `test-pypi: true` runs the checks and
  publishes to TestPyPI only, as a dry run.
- **Release binaries** (`.github/workflows/release-binaries.yml`): the
  CLI archives, built with `cargo auditable`, and the multi-arch
  container image. It also runs weekly and on changes to itself, without
  uploading anything, and uploads only when the publish workflow calls
  it for a tag.
- **Dependabot** (`.github/dependabot.yml`): see
  [`docs/maintaining.md`](docs/maintaining.md#security-and-automation-settings).
- **release-please**: configuration is in place
  (`release-please-config.json` + `.release-please-manifest.json`
  targeting `rust` release-type for this crate) but is **not yet
  wired** to a GitHub Actions workflow. Until that lands, the
  release checklist above is run manually.
- **TestPyPI:** no dry run has published to TestPyPI yet (task `Q-13`
  in the backlog); the 0.2.0 dry runs ran the checks only.
