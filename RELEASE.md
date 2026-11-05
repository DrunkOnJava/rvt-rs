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
dependency on `rvt`), `rvt-mcp/Cargo.toml` (the MCP server's version, and its
`=X.Y.Z` dependency on `rvt`), `rvt-wasm/Cargo.toml`, the `rvt` dependency of
`tools/stream_evidence/Cargo.toml`, `viewer/package.json`, `CITATION.cff`, and the workspace crates' entries in
`Cargo.lock` and `fuzz/Cargo.lock`, and the version has its own section in
`CHANGELOG.md`. All of them must agree at tag time:
`tools/ci/release_preflight.py` checks every one, on every pull request
(the PII guard job) and again on the tagged commit, before `publish.yml`
builds anything. `pyproject.toml` declares the wheel's version dynamic, so
`rvt-py/Cargo.toml` is its one source; the check fails if that changes.

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
  the archives' and `SHA256SUMS`'s build provenance,
  `rvt-rs-vX.Y.Z.intoto.jsonl` (the bundle
  `gh attestation verify <archive> -R DrunkOnJava/rvt-rs` also reads from
  GitHub). Verifying `SHA256SUMS` that way and then running
  `sha256sum -c --ignore-missing SHA256SUMS` checks every archive.
- Publishes the `rvt` crate to [crates.io](https://crates.io/crates/rvt),
  then, once the repository variable `PUBLISH_RVT_RS_MCP` is `true` (step 8),
  the MCP server's crate `rvt-rs-mcp` ([docs/mcp.md](docs/mcp.md)), which
  depends on that exact version, and
  Python wheels for Linux (glibc and musl), macOS and Windows and an sdist
  to [PyPI](https://pypi.org/project/rvt/), and the container image
  `ghcr.io/drunkonjava/rvt-rs:X.Y.Z`, all from
  `.github/workflows/publish.yml`. crates.io and PyPI accept it by Trusted
  Publishing (OIDC), so no registry token is stored.
- Updates `CHANGELOG.md` with human-readable entries grouped by
  `Added` / `Changed` / `Fixed` / `Security` (Keep a Changelog format).
- Updates `CITATION.cff` (`version:` + `date-released:`) so academic
  citations resolve to the right archival state.

## The 1.0 gate

1.0.0 is tagged only when every requirement in
[`docs/release-1.0-criteria.md`](docs/release-1.0-criteria.md) has passed,
each with its evidence linked. It supports every Revit release from 2015 to
2027 and ships with no limitations. The stability commitment covers the
surfaces that page names (its S1, #873), and each of them has a check that
fails on an unannounced breaking change (S2). A milestone's completion
percentage is not part of the gate.

## What 1.0 keeps stable

From 1.0.0, a breaking change to any surface below needs a new major
version. The maintainer chose these surfaces on 2026-10-23 (#873).

| Surface | What 1.0 keeps stable | What it does not |
|---|---|---|
| The `rvt` crate | The modules the crate root's module overview lists: reading and identity, typed elements and the element walk, export, and the byte-preserving writer. Their public items keep their names, signatures and meaning | `rvt::research` and everything else behind the `unstable-research` feature: the partition, native and element-record readers behind the format findings, which change whenever a finding does. `rvt::cli`, which exists for the shipped CLIs and is hidden from the docs |
| The CLIs | The `[[bin]]` targets `Cargo.toml` declares, which the README lists: their names, flags and the JSON they print, as [`docs/schemas/`](docs/schemas/) describes it | Text meant for people to read |
| The Python package `rvt` | The classes and functions in [`python/rvt/__init__.pyi`](python/rvt/__init__.pyi) | |
| Output formats | Valid IFC4 (ISO 10303-21), binary glTF 2.0, plan SVG and CSV, written deterministically: the same file gives the same bytes, once `SOURCE_DATE_EPOCH` fixes the time an IFC records | Their content. A finding that recovers more elements, properties or geometry is a minor release |
| The viewer's wasm API | The JavaScript bindings [`docs/viewer-api.md`](docs/viewer-api.md) lists, with their arguments and the JSON shapes they return | `rvt-wasm` is not published, so code outside this repository builds it from source |

A minor release may add public items, research modules and recovered
content. It may also add a variant to a public enum, which is why they are
`#[non_exhaustive]`, and a value to a list of values in the JSON the CLIs
print, such as a status or a reason code: code that reads one handles a
value it does not know. A research module that moves into the stable modules is named in
`CHANGELOG.md` and from then on follows the table above. The writer is stable
as an API; what it can patch is rated in
[`docs/support-matrix.json`](docs/support-matrix.json) (`stream-level-writer`).

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
5. **Update `CHANGELOG.md`.** Run `python3 tools/release/changelog.py
   assemble`, which moves the fragments under `changelog.d/` into
   `[Unreleased]` and deletes them (#938). Then move items from
   `[Unreleased]` into a new
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
7. **Open the release pull request,** `chore(release): prepare vX.Y.Z`,
   with steps 4 to 6 and the README's release lines (the latest release
   and the "after X.Y.Z" fragments sentence under "Install and quick
   start") moved to the new version. crates.io and
   PyPI show the README packed into each version, which can never be
   edited once it is published, so it names the release it ships in; it
   states no channel checks, which come after (Post-release). `main`
   takes changes only through pull requests, so the release metadata
   lands like any other change. The IFC header
   carries the version (#417), so both Core Interior `rvt-rs.json`
   witness observations and their verdicts change: take them from the
   pull request's CI run (its `octetproof-core-interior` artifact) and
   push them. Only `witness_version` and the verdict hashes should
   differ. From the same run's `synthetic-fixtures` artifact, refresh
   `tests/fixtures/synthetic-project.ifc` and `synthetic-structural.ifc`,
   the samples the viewer offers for download; CI validates that output
   on every run, and no check compares it with the committed copies.
   Merging this pull request is not the release.
8. **Confirm the registries' trusted publishers.** crates.io and PyPI
   each need one for repository DrunkOnJava/rvt-rs, workflow
   `publish.yml`, environment `crates-io` or `pypi` (see
   [`docs/maintaining.md`](docs/maintaining.md#releases)). crates.io needs
   one for each of its two crates, `rvt` and `rvt-rs-mcp`. Without
   crates.io's, the crate's publish fails, and PyPI, the release
   binaries and the container image, which wait for it, never run. A
   trusted publisher can be added only to a crate that exists, so the first
   version of `rvt-rs-mcp` is published by hand from the release commit
   (`cargo publish -p rvt-rs-mcp`, after `rvt` of that version is on
   crates.io), and its trusted publisher added afterwards. Until then
   `publish.yml` skips its `rvt-rs-mcp` publish step: a publish that fails
   after `rvt` is on crates.io would leave the tag half published. Set the
   repository variable `PUBLISH_RVT_RS_MCP` to `true` (Settings, Secrets and
   variables, Actions, Variables) once the trusted publisher is in place, and
   later releases publish both crates. The `rvt-rs-mcp` binary is in the
   release archives and the container image whether or not the variable is
   set.
9. **Tag the merged commit.** `git fetch origin`, check that
    `origin/main` is the release pull request's squash commit, then
    `git tag -s vX.Y.Z -m "rvt-rs X.Y.Z" origin/main`. The `-s` signs
    the tag with the maintainer's key, which release authenticity
    requires: the workflow refuses a lightweight tag and one whose
    signature GitHub does not report as verified (so the signing key has
    to be on the maintainer's GitHub account, as a GPG or SSH signing
    key). The `release-tags` ruleset lets only a maintainer create
    a `v*` tag, and nobody can move or delete one (a repository
    administrator can bypass the ruleset, which a release never needs).
10. **Push the tag.** `git push origin vX.Y.Z` runs `publish.yml`. Its
    `preflight` job runs first, and every other job waits for it. It
    refuses, within seconds, a tag that is not named `vX.Y.Z` (or with a
    `-prerelease`), is lightweight or not verified as signed, names another
    commit than the one pushed, whose commit is on neither `main` nor a
    `hotfix/*` branch, or whose commit disagrees with the tag on any
    version file above; and it waits for `CI passed` on the tagged commit,
    up to 80 minutes, and refuses a failed, cancelled or missing one (a
    commit whose every file CI's push filter skips needs no CI of its own).
    Then the workflow builds the wheels and the sdist and checks them: each
    wheel installed and opening a model, `cargo install` of the source with
    the CLI sample smoke, the docs.rs build preview (CI's pinned nightly and
    banner check) and the viewer load smoke.
    Once they pass, the workflow publishes the crate to crates.io, with no
    approval asked for (pushing the tag is the release decision), then the
    wheels and sdist to PyPI and, through
    `release-binaries.yml`, the CLI archives and the container image with
    their attestations, both of which wait for the crate. Releases run one
    at a time, in the order their tags were pushed, so two tags pushed
    together cannot race for the container image's `latest`; GitHub keeps
    one waiting run, so a third tag pushed while two are queued cancels the
    second, which then needs `gh run rerun <run id>`. If a job fails, fix
    its cause and re-run the failed jobs (not all of them: the wheels would
    be built again, differently from any already uploaded). A re-run is safe
    after a partial publish: the crates.io steps skip a version crates.io
    already has (a yanked one fails the run), and PyPI skips the files it has
    from the second attempt on.

    **When the preflight refuses a tag,** it names every problem and nothing
    was built or published. A tag cannot be moved, so a problem in the tagged
    commit (a version file or the `CHANGELOG.md` section missing, the commit
    on no release branch, `CI passed` red) is fixed in a new commit and
    released as the next version. Two causes are not about the commit: a CI
    job that flaked (re-run CI's failed jobs on that commit, and once
    `CI passed` is green re-run `publish.yml`'s failed jobs, which read the
    check again) and GitHub being unable to verify the signature yet or the
    API being down (re-run the failed jobs). The same version-file check runs
    on every pull request, so a half-finished bump fails the release pull
    request and not the tag.
11. **Write the release page.** `release-binaries.yml` creates the
    release from the tag with GitHub's generated notes. Replace them with
    this version's `CHANGELOG.md` section, verbatim, followed by
    `User guide: https://github.com/DrunkOnJava/rvt-rs/blob/vX.Y.Z/docs/user-guide.md`
    with the real tag substituted
    (`gh release edit vX.Y.Z --notes-file <file>`).
12. **Announce (optional).** If the release is worth calling out,
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
# Once PUBLISH_RVT_RS_MCP is true (step 8); until then the archive's binary:
cargo install rvt-rs-mcp --version X.Y.Z --locked
rvt-rs-mcp --version

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

An embargoed security fix is the exception to working in public: a
pushed branch and its pull request publish the fix before the release.
Prepare and review it in its GitHub security advisory's temporary
private fork ([`SECURITY.md`](SECURITY.md)), and push the branch here,
where CI and Measure run, only when it is ready to release.

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
3. **Prepare the release on the branch.** Steps 4 to 6 above (the
   version files, `CHANGELOG.md` and `CITATION.cff`) and
   the README's release lines, committed to the branch: no pull
   request into `main` can test a branch cut from a tag.
4. **Check it.** Pushing the branch runs CI on it with the full matrix
   `main` gets (`ci.yml` runs on `hotfix/**` pushes). The new version
   changes both Core Interior witness observations and their verdicts
   (step 7): take them from that run's `octetproof-core-interior`
   artifact and check that every difference is `witness_version`, a
   verdict hash, or one the fix explains and the changelog states, then
   commit them and push again; that run must pass too. Measure the
   branch against the release it patches, with the corpora the fix
   touches:
   `gh workflow run measure.yml --ref hotfix/vX.Y.(Z+1) -f base=vX.Y.Z`.
   Skip nothing else in steps 1 to 3 above.
5. **Review it.** The branch has no pull request, and crates.io and
   PyPI show its README and changelog for good: read its whole diff
   against the tag (`git diff vX.Y.Z hotfix/vX.Y.(Z+1)`) before tagging.
   So that it gets the review a pull request gets (ADR-005), push a
   branch at `vX.Y.Z` (`review/vX.Y.Z-base`) and open a review-only pull
   request of the hotfix branch into it, as 0.6.1 did (#854). Nothing
   waits for that review, but a finding that arrives before the tag is
   acted on. A
   commit that fixes what the reading or the review finds needs step 4
   again on the new head: CI runs on any push that changes more than the paths
   `ci.yml` ignores (Markdown, `docs/`, `CITATION.cff` and a few others),
   a push of only those runs none, and a change to code needs Measure
   again.
6. **Tag the reviewed head and push the tag.** Steps 8 to 11 above,
   with that head in place of `origin/main`, once its code and witness
   files are the ones CI and Measure passed:
   `git tag -s vX.Y.(Z+1) -m "rvt-rs X.Y.(Z+1)" <its SHA>`. The preflight
   accepts the tag only while `hotfix/vX.Y.(Z+1)` holds that commit, and
   waits for CI's `CI passed` on it (the branch's push run), so keep the
   branch until the release has finished, re-runs included. Then close
   the review-only pull request unmerged and delete
   `review/vX.Y.Z-base`.
7. **Bring `main` up to it** through a pull request, as merging the
   branch back would: the fix if `main` lacks it, the new `CHANGELOG.md`
   section (its entries moved out of `[Unreleased]`), the version files
   and the README's release lines. `main` exports differently from the
   branch, so take its Core Interior witness observations and verdicts
   from that pull request's own CI run, as in step 7, not from the
   branch. Then delete the branch; the tag keeps its commits.

## Post-release

- **Update the docs.** Once every channel has been checked from what was
  published ([Post-Publish Install Verification](#post-publish-install-verification)),
  a pull request moves the other latest-release statements to the new
  version and records the checks: `docs/install.md`, `docs/python.md`,
  `docs/status.md`, `docs/supported-profile.md` and `ROADMAP.md`. The
  README's release lines moved in the release pull request (step 7).
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

- **CI on every push to `main` and on every PR** (`.github/workflows/ci.yml`,
  which a pull request runs through `ci-pr.yml`: in full once ready, only the
  fast jobs while a draft):
  fmt, clippy, the test matrix (Linux, macOS and Windows, stable, and the
  MSRV), rustdoc, `cargo deny`, `cargo audit`, the docs.rs build preview,
  the Python wheel and its integration tests, IfcOpenShell validation and
  the witness verdicts, the viewer's browser tests
  (`.github/workflows/viewer-test.yml`), the PII guard and the rest. `full / CI passed` (a
  push to `main` names it `CI passed`), the one required check, passes only
  when every one of them did (see
  [`docs/maintaining.md`](docs/maintaining.md#pull-requests)).
- **Publish workflow** (`.github/workflows/publish.yml`), on `v*` tags and
  `workflow_dispatch`: the checks and publish order of step 10, behind a
  `preflight` job (`tools/ci/release_preflight.py` and the
  `.github/actions/wait-for-check` composite action, which Deploy viewer
  waits for CI through too) that every other job needs. Its
  crates.io and PyPI jobs authenticate by Trusted Publishing in the
  `crates-io` and `pypi` environments, which only `v*` tags may deploy
  to; PyPI and the release binaries wait for the crates.io job. The
  one-time registry setup
  is described at the top of the file.
  `workflow_dispatch` has no tag, so the tag, signature, branch and CI
  checks pass with a note and the version files must still agree with each
  other; with `test-pypi: true` it runs the checks and
  publishes to TestPyPI only, as a dry run. The preflight's judgements have
  failure-first tests (`tools/ci/test_release_preflight.py`, run by the PII
  guard job), and `tools/ci/release_preflight.py check` can be run by hand
  against any tag read-only, for example
  `tools/ci/release_preflight.py check --event push --ref-type tag
  --ref-name v0.15.0 --sha <the tag's commit>` from a checkout that holds
  `origin/main`.
- **Release binaries** (`.github/workflows/release-binaries.yml`): the
  CLI archives, built with `cargo auditable`, and the multi-arch
  container image. Every archive's binaries are run before it ships; the
  Intel macOS build is cross-compiled on an arm64 runner and run there under
  Rosetta. It also runs weekly and on changes to itself, without
  uploading anything, and uploads only when the publish workflow calls
  it for a tag.
- **Dependabot** (`.github/dependabot.yml`): see
  [`docs/maintaining.md`](docs/maintaining.md#security-and-automation-settings).
- **TestPyPI:** no dry run has published to TestPyPI yet (task `Q-13`
  in the backlog); the 0.2.0 dry runs ran the checks only.
