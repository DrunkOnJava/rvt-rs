# Maintaining rvt-rs

How this repository is run, so contributors know what to expect and
maintainers do the same thing each time. The rules for contributors are in
[`CONTRIBUTING.md`](../CONTRIBUTING.md); the release procedure is in
[`RELEASE.md`](../RELEASE.md).

## Triage

Every issue and pull request gets a reply that says what happens next. There is
one maintainer and no response-time guarantee, but nothing is left without an
answer. New issues arrive from the issue forms with `needs-triage`.

1. Questions go to [Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions),
   with a link. An issue that duplicates another is closed with a link to it.
2. Every open issue carries a `type:*`, an `area:*` and a `priority:*` label.
   `needs-triage` comes off when they are set.
3. An issue with a user-facing acceptance criterion (an element, a property, a
   geometry residual against Revit's own export) goes into the milestone it
   serves. Format research with no such criterion stays open and unscheduled, so
   it neither blocks a release nor inflates one.
4. An issue is closed when its acceptance criteria are met, with the evidence
   linked, or when it is superseded, with the superseding issue linked. It is
   never closed for being old.

### Labels

| Label | Meaning |
|---|---|
| `type:bug`, `type:feature`, `type:research`, `type:docs`, `type:test`, `type:ci`, `type:release`, `type:security`, `type:project-management` | What kind of work it is. |
| `area:*` | The part of the code it touches: `elements`, `partitions`, `geometry`, `ifc`, `reader`, `writer`, `walker`, `python`, `cli`, `viewer`, `corpus`, `docs`, `ci`, `github`, `product`. |
| `priority:P0` to `priority:P3` | P0 is reserved for immediate release or security impact. P1 is work the beta path needs, P2 quality and hardening, P3 follow-up after the core milestones. |
| `re-finding`, `research-critical` | A new on-disk format finding. `research-critical` is gated on format evidence or an oracle, and is not a release or security blocker. |
| `good first issue`, `help wanted` | Small, self-contained tasks for newcomers, and work where outside help is wanted. |
| `needs-triage` | Not yet triaged. |
| `community` | A pull request or issue from outside the maintainers. |
| `needs-split` | Too large to review whole; to be split into per-capability pull requests. |
| `dependencies` with an ecosystem label | Dependabot's pull requests. |
| `question`, `duplicate`, `invalid`, `wontfix` | How an issue was resolved without work. |

### Milestones

A milestone is a release line. It is closed when its release ships, and the
issues still open move to the next milestone that serves them. The plan is the
milestones and the issues under them; [`ROADMAP.md`](../ROADMAP.md) and
[`docs/contribution-map.md`](contribution-map.md) describe them and are updated
when a release ships.

## Pull requests

- A pull request is required for everyone, and nobody can bypass the ruleset,
  the maintainer included: a bypass let an admin merge without `CI passed`
  (2026-10-05). An emergency change goes through a pull request like any other,
  or the ruleset is edited deliberately and the edit committed.
  Merges are **squash only**, commits must be **signed**, history is linear, and
  the branch must be up to date with `main`. The one required check is
  `CI passed`, CI's summary job, which passes only when every CI job (macOS
  and Windows, corpus tier 2, the writer corpus, IfcOpenShell validation,
  wheels and the audits included) passed or was skipped, so auto-merge waits
  for all of them. Measure and the release and deploy workflows are not part
  of it.
- The squash commit's title is the pull request's title plus its number and its
  body is the pull request's description, so the description is permanent
  history: fix it before merging.
- A first-time contributor's workflow runs wait for a maintainer to approve them
  (the "Approve and run" button, or
  `gh api -X POST repos/DrunkOnJava/rvt-rs/actions/runs/<run-id>/approve`).
- A contributor's work is credited: keep their commit where the squash allows
  it, and end the squash message with a `Co-authored-by:` line where it does
  not. A very large pull request is reviewed against measurements, labelled
  `needs-split`, and the parts that hold up are merged as their own pull
  requests.
- Every measured claim in a pull request links the run or command that
  produced it, and what is not established is stated. `docs/status.md`, the
  support matrix, the README and the CHANGELOG change in the same pull request
  as the capability they describe.

## Dependencies and the MSRV

`rust-version` in `Cargo.toml` is the oldest stable Rust the crate builds on, and
the MSRV job in CI checks every target (`cargo check --all-targets`) on exactly that release. It is one of the jobs
`CI passed` requires, named `build + real files / ubuntu-24.04 / msrv` so that
the name does not change when the release does. The MSRV is never lower than the
floor the 2024 edition sets (1.85), and it moves up when a dependency worth
taking needs a newer compiler: quick-xml 0.42 needs 1.86 and earcut 0.4.10 and
later call `is_multiple_of`, which is stable from 1.87, so the MSRV is 1.87. It
is raised in a minor release, never in a patch, and the CHANGELOG says so.

A Dependabot pull request that fails the MSRV job cannot merge as it is. Either
the MSRV is raised in the same pull request, with the reason and the crates it
unblocks in the description, or the update is held with a comment in
`Cargo.toml` naming the release it needs. earcut declares no `rust-version`, so
Cargo's MSRV-aware resolver cannot warn about it: the MSRV job is the only check.
Raising the MSRV also brings the lints that need it (at 1.87, clippy's
`manual_is_multiple_of`), which the same pull request fixes.

A dependency of a parser (quick-xml, flate2, encoding_rs, cfb) is measured, not
only built. Run Measure with `-f families=true` (see
[`CONTRIBUTING.md`](../CONTRIBUTING.md)): it diffs `rvt-info`'s output on the six
reference models and the eleven Autodesk families, which is the only place the
PartAtom reader is measured, since the IFC export never reaches it. For the
PartAtom reader, dispatch `fuzz.yml` on the branch as well
(`-f target=fuzz_part_atom -f duration_seconds=300`), and cite both runs in the
pull request.

## Releases

Tags `v*` are protected by the `release-tags` ruleset: only a maintainer can
create one, and none can be moved or deleted. Pushing a tag runs the publish
workflow, which builds and verifies the wheels and, through the
release-binaries workflow, the CLI archives and the container image, and
publishes them with build-provenance attestations. Each release is described in
the release notes and the CHANGELOG, its milestone is closed, and the issues
still open move on.

## Security and automation settings

Configured on the repository, with the rulesets kept as code in
[`.github/rulesets/`](../.github/rulesets/):

- Private vulnerability reporting, secret scanning with push protection,
  Dependabot alerts and security updates are on. Dependabot opens weekly version
  updates for Cargo (the crate, the fuzz workspace and `tools/ci/witness-ifc-lite`),
  GitHub Actions (the workflows and the composite actions under
  `.github/actions`), the viewer's npm packages and the release image's base in
  `docker/Dockerfile`, with minor and patch updates grouped; major updates
  arrive one at a time and are migrated by hand when they need it. A release
  waits three days (seven for a major Cargo or npm one) before it is proposed, and commit
  titles read `build(deps): ...`. The witness's `ifc-lite-core` is never
  proposed: it is pinned to an exact version on purpose, as a different version
  is a different witness.
- CodeQL (`.github/workflows/codeql.yml`) analyses the workflows, the
  viewer's TypeScript and JavaScript, the Python bindings and the Rust crate,
  on every pull request, push to main and weekly. The default setup stays
  off: it cannot scan Rust, and GitHub refuses an advanced setup's results
  while it is on.
- Every action is pinned to a full commit SHA and the repository requires it.
  The default workflow token is read-only.
- Deleting or force-pushing `main` is blocked.
- PyPI publishing uses OIDC trusted publishing, so no PyPI token is stored. The
  `pypi` environment accepts only `v*` tags, and `testpypi` only `v*` tags and
  `main`; a dry run from another branch is refused until its name is added to
  the environment's deployment policy.

To re-apply a ruleset from its file:

```bash
gh api -X PUT repos/DrunkOnJava/rvt-rs/rulesets/<id> --input .github/rulesets/main-protection.json
```

## Housekeeping

At every release, and at least monthly, run
`python3 tools/maintainer/audit.py`. It is read-only, checks most of the list
below against the repository and exits 1 if any check fails, and each failing
line names what to fix (its rulesets check needs an admin token). The list:

1. Every open issue has its labels and, where it has acceptance criteria, a
   milestone; closed milestones have no open issues.
2. No stale branches remain on the remote (`delete_branch_on_merge` is on; a
   branch of a closed pull request is deleted with a note where its commits
   stay reachable from the pull request's head).
3. The Actions cache is under its 10 GB limit. `python3
   tools/maintainer/clean_caches.py` (a dry run; `--delete` acts) removes the
   caches of closed pull requests and deleted branches and `main`'s superseded
   rust-cache generations: a new lockfile saves a new cache under a new key and
   the old ones are never read again. `main`'s newest two generations of each
   family, live branches' and open pull requests' caches are kept.
4. Workflow runs on `main` are green. A red check is a defect to fix, or an
   issue to file with its cause, and a check that fails only because a service
   is down reports "not measured" rather than failing.
5. Open Dependabot and code scanning alerts are triaged, each fixed or
   dismissed with a reason, and Dependabot watches every lockfile, Dockerfile
   and composite action in `.github/dependabot.yml` (the audit lists what is
   missing).
6. Discussions and community pull requests have an answer.
7. The rulesets and settings still match their files.
