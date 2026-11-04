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
  `full / CI passed`, CI's summary job as a ready pull request runs it
  (`ci-pr.yml` calls `ci.yml`), which passes only when every CI job (macOS
  and Windows, corpus tier 2, the writer corpus, IfcOpenShell validation,
  wheels and the audits included) passed or was skipped, so auto-merge waits
  for all of them. A draft runs only the fast jobs, as `fast / ...`, and
  never a check of the required name; marking it ready runs the full set
  (#1237). A push to `main` runs `ci.yml` directly, as `CI passed`. Measure and the release and deploy workflows are not part
  of it.
- The squash commit's title is the pull request's title plus its number and its
  body is the pull request's description, so the description is permanent
  history: fix it before merging.
- A first-time contributor's workflow runs wait for a maintainer to approve them
  (the repository's fork pull request policy is "require approval for
  first-time contributors"; the "Approve and run" button, or
  `gh api -X POST repos/DrunkOnJava/rvt-rs/actions/runs/<run-id>/approve`).
  Read the diff first, `.github/`, build scripts and anything that runs at build
  or test time above all. Approving a run lets the pull request's checks run with
  a read-only token and no secrets; it grants no write or merge access. Held
  runs and contributions left without a reply for more than two days are
  flagged to the maintainer, because a run nobody approves gives the
  contributor no feedback at all.
- A contributor's work is credited. The squash commit's author is the pull
  request's author. A contributor's unsigned commits cannot land, since `main`
  requires signed commits, so the maintainer replaces them on the contributor's
  branch with the same change, signed as committer, keeping the contributor as
  author, checks that the change set is identical, and says so on the pull
  request. Where the squash keeps no commit of a contributor's, end its message
  with a `Co-authored-by:` line.
- RE numbers (`reports/element-framing/RE-NN-*.md`, `probe_reNN_*`) are given out
  as findings land. When two open pull requests claim the same number, the
  later one is renumbered, and the maintainer names the new number on the pull
  request. A very large pull request is reviewed against measurements, labelled
  `needs-split`, and the parts that hold up are merged as their own pull
  requests.
- Every measured claim in a pull request links the run or command that
  produced it, and what is not established is stated. `docs/status.md`, the
  support matrix, the README and a changelog fragment
  ([`changelog.d/`](../changelog.d/README.md)) change in the same pull request
  as the capability they describe.

## Measure's score cache

Measure runs about 40 scorers and four schema checks on every model, and most
pull requests move the export of only a few of them. A step whose inputs are as
they were in an earlier run takes that run's output from the score cache
(`tools/ci/measure-cache.py`) instead of running; exports, `rvt-info`,
`rvt-gltf`, the witness exports, diagnostics and probes always run. A hit
writes the cached output and a `<step> cached` line to the model's
`timings.log`; every output is the one a run would have written.

- **Key.** The sha256 of the step's name and arguments; the content of every
  file argument (the rvt-rs IFC without its `FILE_NAME` and `IFCOWNERHISTORY`
  lines, which carry the export time, so a rebuild that writes the same model
  hits; Revit's export; the glTF; the validation findings); the scorer and
  every module of `tools/re` and `tools/ci` it reaches by import, importlib
  path or name, and every file of `tools/re` and `tools/ci` that is not Python
  (the Measure script, the requirements, the schemas); the value of every
  environment variable the scorers read (apart from the two that size the mesh
  cache), which is `PSETS_MISSING` for `psets_vs_ifc` and `qtos_vs_ifc`; the
  files a scorer reads that its arguments do not name (`qtos_vs_ifc`'s
  `diagnostics.json`, `unfilled_openings_vs_ifc`'s sibling Revit exports); the
  Python minor version (not the patch: a runner image moves it, 3.12.15 to
  3.12.16 in run 41769277902, which cost two of six shards every hit), the
  platform, the sha256 of `tools/ci/requirements-ifc.txt`, the installed
  version of every package it pins, the versions IfcOpenShell and numpy
  report, `RVT_IFC_FLAGS`, and the sha256 of `measure-cache.py` itself, so a
  change to what is kept or looked up starts a cache. Each entry therefore
  hashes the rvt-rs IFC, Revit's reference IFC (or the glTF), the command-line
  arguments, the scorer and its transitive `tools/re` and `tools/ci` imports,
  the environment variables they read and the library versions.
- **What is never cached.** A step whose closure lists or walks a directory,
  takes a directory from an argument, opens a file by a literal path, runs a
  program, draws a number or a date, reads the IFC's export time or reads an
  environment variable by a computed name, or one given a directory as an
  argument: the cache tells the reason in `score-cache-<shard>.log` and runs
  it. A new scorer that reads anything but the files it is given is run every
  time until `EXTRA_INPUTS` in `measure-cache.py` names what it reads. A step
  is kept only if it finished with status 0 and neither its output nor any file
  it also wrote (the validation findings, the rules summary) is over 4 MiB or
  shows a failure a rerun could mend (`unreadable:`, a traceback, a
  `MemoryError`, a dead worker); `ifc_validate_summary.py --both` exits 1 when
  a check failed, where its separate runs print the failure and exit 0. An
  entry that lacks a file the step wrote is a miss.
  The held-out models, BIMcollab's and the pairs that were held out
  (`holdout-*`, `bimcollab-*`, `spent-*`: BIM Whale's AdvancedProject was spent
  and held out again on 2026-11-03), of which only counts leave the runner
  (`tools/ci/scrub-holdout-results.sh`), and `racadv-*` (Autodesk's rac_advanced
  sample against an IFC2X3 export with no licence, #1354, which also leaves the
  runner as counts only), are never looked up or stored, because
  the Actions cache of a public repository can be restored by a workflow that a
  fork's pull request changes: nothing the scrub removes may be in it, and a
  pair's results do not become safe by being spent. Each entry names its model,
  and the end of every run removes any entry that names one of those models or
  none and prints the entries by model prefix, so the saved cache is shown to
  hold none. That gives up caching a share of the work, not a small one: in
  the unfiltered run 41748430211 the 18 held-out models took 32.8 of the 119
  task-minutes of model steps (28%); they run only with `-f holdout=true`.
- **Where it lives.** Each shard of the run, whatever it is called, restores
  the newest `measure-scores-v2-main-<shard>.*` cache after the measured code
  has built (a build script never sees a writable cache) (a run given `models`
  is the shard `all`, with a cache of its own,
  built by earlier runs of that kind). It is saved only where rust-cache is saved: the
  workflow dispatched from `main` and the side measuring `main`'s own code
  (`inputs.ref` empty, or `base` `main`), because the build scripts of any
  other ref run on the runner and must not write what a later `main` run
  reads. A pull request run, and a run of any other ref, only reads. Keying on
  the scorers makes an entry reachable only by a run whose scorers, files and
  libraries hash alike, but that is not safe by construction: a build script of
  the measured ref could rewrite the installed IfcOpenShell, which the key
  names only by version, and the cache would then hold what that run produced.
  That is why only `main`'s own code saves it. A run dispatched with `-f
  score_cache=<name>` reads and saves a cache of its own,
  `measure-scores-v2-x-<name>-<shard>.*`, from any ref (GitHub scopes a
  branch's caches to that branch, and the shared cache never reads the name);
  this is how a change to the cache itself is proved. `-f score_cache=off`
  runs every step as before.
- **Bounds.** Entries not used for 21 days are removed, then the oldest past
  256 MiB; a run saves a new cache only if it kept an entry, pruned one or
  refreshed one unused for three days, and
  `tools/maintainer/clean_caches.py` deletes the generations a newer one
  supersedes. An entry is a scorer's text output (kilobytes), so the cache is
  a few MiB, not GiB. `results/score-cache-<shard>.log` in the artifact lists every
  hit, miss, kept and uncached step with the first twelve characters of its key.
- **Checking it.** The run after a change to an export is the check that the
  key follows the IFC: its changed models miss and the rest hit. To check a
  change to the cache or to a scorer, run Measure twice on one branch with
  `-f score_cache=<name>` (cold, then warm) and once with `-f
  score_cache=off`, and `diff -r` the three results apart from `*.log`,
  `commit.txt` and `runner*.txt` (`diagnostics.json` is not left out: it is
  byte-identical across runs of one commit, and `qtos_vs_ifc` reads it); to
  check that a changed input misses, run the same name at two commits whose
  exports differ for some model, and the second must miss on the models that
  moved and hit on the rest, and diff clean against a run with the cache off
  at that commit; a warm run with `-f
  psets_missing=Pset_WallCommon.Reference` must miss `psets_vs_ifc` on every
  model and hit everything else.

Measure also keeps the three binaries its Build step makes (`rvt-ifc`,
`rvt-gltf`, `rvt-info`) under `rvt-bin-v1-<full SHA>-<OS>-<arch>-<toolchain
hash>-ci`, so the shards of a run, and every run that measures `main`'s commit
as its base, build them once (the build is about 70 seconds of each job). What
the cache holds is a function of its key, the code of that commit built by that
toolchain, and only a run of that commit restores it, so a run of any ref may
save it (a pull request's run does not: its caches are its own). A ref that is
not a full SHA is built each time, the probe examples always are, and
`clean_caches.py` keeps the newest sixteen.

## Dependencies and the MSRV

`rust-version` in `Cargo.toml` is the oldest stable Rust the crate builds on, and
the MSRV job in CI checks every target (`cargo check --all-targets`) on exactly that release. It is one of the jobs
`full / CI passed` requires, named `full / build + real files / ubuntu-24.04 / msrv` on a pull request so that
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
publishes them with build-provenance attestations. Pushing the tag is the
release decision, so the run asks for no approval: the `crates-io` and `pypi`
environments accept only `v*` tags, with no administrator bypass, and the PyPI
job and the release binaries wait for the crates.io job, so no channel ships
a version crates.io lacks. Each release is described in
the release notes and the CHANGELOG, its milestone is closed, and the issues
still open move on.

crates.io and PyPI accept the publish workflow through trusted publishing, so no
token is stored, and a tag release fails when crates.io refuses it (C3). Each
registry needs its trusted publisher configured on its side once (repository
DrunkOnJava/rvt-rs, workflow `publish.yml`, environments `crates-io` and
`pypi`); crates.io needs one for each of its crates, `rvt` and the MCP server's
`rvt-rs-mcp`, which is added after that crate's first version is published by
hand; the workflow publishes `rvt-rs-mcp` only once the repository variable
`PUBLISH_RVT_RS_MCP` is `true` (RELEASE.md, step 8). Confirm it before pushing a
tag; `.github/workflows/publish.yml` lists the settings.

## Security and automation settings

Configured on the repository, with the rulesets kept as code in
[`.github/rulesets/`](../.github/rulesets/):

- Private vulnerability reporting, secret scanning with push protection,
  Dependabot alerts and security updates are on. Dependabot opens weekly version
  updates for Cargo (the crate, the fuzz workspace and `tools/ci/witness-ifc-lite`),
  GitHub Actions (the workflows and the composite actions under
  `.github/actions`), the viewer's npm packages, the Python packages CI pins in
  `tools/ci/requirements-*.txt` and the Codespaces notebooks pin in
  `.devcontainer/requirements-jupyter.txt`, the base images of
  `docker/Dockerfile` and `.devcontainer/Dockerfile`, and the Dev Container
  Features `.devcontainer/devcontainer.json` installs, with minor and patch
  updates grouped; major updates arrive one at a time and are migrated by hand
  when they need it. A release
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
  `crates-io` and `pypi` environments accept only `v*` tags, with no
  administrator bypass, and PyPI's job follows the crates.io job. `testpypi` accepts
  `v*` tags and `main`; a dry run from another branch is refused until its
  name is added to the environment's deployment policy. The repository stores
  no Actions secrets.

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
   stay reachable from the pull request's head). A research or Measure branch
   with no pull request is kept while its issue is open; once its finding has
   landed or been superseded, its tip is pushed to `refs/archive/<date>/` on
   GitHub, which keeps it off the branch list, and into a git bundle the
   maintainer keeps, with a list of its commits, and the branch is then
   deleted. The audit lists such a branch once its last commit is more than
   14 days old. A commit a report's reproduce commands cite (`-f ref=<sha>`)
   that no branch, archive ref or pull request head reaches, such as the head
   of a branch rebuilt before it landed, is pushed by hand to
   `refs/archive/<date>/cited/<first 12 characters of the sha>` so GitHub
   keeps it. The PII guard job's `tools/ci/cited-commits.py` fails only on a
   cited commit GitHub does not have at all; whether a ref reaches it is not
   checked automatically.
3. The Actions cache is under its 10 GB limit. `python3
   tools/maintainer/clean_caches.py` (a dry run; `--delete` acts) removes the
   caches of closed pull requests and deleted branches, and `main`'s superseded
   rust-cache generations and CodeQL overlay base databases (per CodeQL version): a new lockfile
   saves a new cache under a new key, and each CodeQL analysis of `main` saves
   a new base database, and the old ones are never read again. `main`'s newest
   two generations of each family, live branches' and open pull requests'
   caches are kept.
4. Workflow runs on `main` are green. A red check is a defect to fix, or an
   issue to file with its cause, and a check that fails only because a service
   is down reports "not measured" rather than failing.
5. Open Dependabot and code scanning alerts are triaged, each fixed or
   dismissed with a reason, and Dependabot watches every lockfile (a pip
   requirements file pinned with hashes among them), Dockerfile, composite
   action and devcontainer in `.github/dependabot.yml` (the audit lists what
   is missing).
6. Discussions and community pull requests have an answer.
7. The rulesets and settings still match their files.
8. [`release-1.0-criteria.md`](release-1.0-criteria.md) agrees with the
   1.0.0 milestone: every open issue under it serves a requirement on the
   page (the audit checks this, and that no pull request sits in the
   milestone), and every status links evidence that is still current.
