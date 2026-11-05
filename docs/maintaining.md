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
  wheels, the audits and the viewer's browser tests included) passed or was
  skipped, so auto-merge waits for all of them. A draft runs only the fast
  jobs, as `fast / ...`, and never a check of the required name; marking it
  ready runs the full set (#1237). A push to `main` runs `ci.yml` directly, as
  `CI passed`. Measure and the release and deploy workflows are not part of
  it. The viewer's browser tests are: `ci.yml` calls `viewer-test.yml` as its
  `viewer` job in the full set (#1374), in three groups of spec files on a
  WASM built without LTO, and a change that touches only documentation skips
  it. Deploy viewer builds and publishes the site, and runs the suite and the
  saved-mesh measurement again on the release WASM it publishes; `CI passed`
  does not wait for those, but the deploy job does, and it waits for `CI
  passed` on the pushed commit. To check a draft's viewer, dispatch
  `viewer-test.yml` on its branch.
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

## Measure and the MCP server's answers

Measure also scores what `rvt-rs-mcp` tells an agent about each model it
exports (`tools/re/mcp_answers_vs_ifc.py`, #1327). A scripted client with no
language model (`tools/re/mcp_client.py`, JSON-RPC over the server's stdio, the
2026-07-28 lifecycle) opens the model's `.rvt` in the built server and asks a
fixed query set: the summary, the levels, every element's row, the totals per
IFC entity, Revit class and level, the rooms with their areas, `get_element`
for the five lowest ElementIds of each IFC entity, the diagnostics and an
`export_ifc`. Run it by name on any model:

    gh workflow run measure.yml --ref <branch> -f ref=<sha> -f base=none \
      -f models="core-interior einhoven" -f scorers=mcp_answers_vs_ifc

- **Consistency** is a check, not a measurement: every answer must equal the
  IFC export of the same run (the elements and spaces by GlobalId, the
  storeys, the rooms' number, name, storey and plan area, the property and
  quantity sets of the sampled elements, their types, placement (the
  translation) and host, whether a body is none or mapped, the diagnostics
  against `diagnostics.json`, the server's export against `ifc.sha256`). A
  difference ends the output with `exit 1` and is listed under its check. The
  run summary names each model's verdict, and a model with no verdict as `no
  verdict, exit N` (a crash is never read as a pass or a fail); it fails no
  job. With `ifc_flags` the server's default decode is compared with its own
  export instead, since it takes none of the flags, and the checks that need
  the run's export (the diagnostics and the export hash) are named in the
  verdict as NOT MEASURED.
- **What consistency cannot catch.** The server and the IFC writer share the
  one `IfcModel` the decode builds. The check finds what either does to that
  model on the way out: a field the server projects wrongly, an element or a
  set the writer drops, adds or folds, a name or a type object the writer
  makes at write time, a run that does not repeat. It finds no decode error (a
  value read wrongly from the file, an element not read): both sides share it.
  The oracle is what measures those.
- **The oracle** scores the same answers against Revit's export with the
  pairing and tolerances of `class_sets_vs_ifc`, `storeys_vs_ifc`,
  `space_profiles_vs_ifc`, `psets_vs_ifc` and `qtos_vs_ifc`. Those lines are
  measurements, diffed by `base=main` like every scorer's.
- **Held-out models** keep the lines starting `mcp answers:` (fixed words,
  integers and IFC entity names, which `Report.line` enforces: it raises on
  any other value) and nothing else (`tools/ci/scrub-holdout-results.sh`).
  Nothing is cached: the step runs the server.
- **The dump** (`mcp-dump.json` in the artifact of the openly licensed models
  only: Core Interior, Einhoven, RE1, the BIM4LCA office and the Revit 2025
  models; left out of the diff) is the same answers as one deterministic
  document: sorted, floats at six decimals, no handle, time or path.
  `python3 tools/re/mcp_client.py <rvt-rs-mcp> <model.rvt> [--all]` writes it
  alone, with `get_element` for the five lowest ElementIds of each entity or
  for every element; the semantic merge check (#1334) reads it.
- The server is built (`cargo build -p rvt-rs-mcp`) only in a run that names
  no scorers or names this one. It is optional: a ref without the crate, or
  whose server does not build, leaves the step NOT MEASURED with a warning and
  fails no other scorer. Autodesk's sample projects are not asked.

## Measure and the kinds of IfcSpace

Every model and every Autodesk sample whose export is not empty is checked by
`tools/re/space_kinds_vs_ifc.py` (#1437), which reads the export alone: each
`IfcSpace` carries `RvtElementRecordGeometry.SpaceKind`, `area` holds exactly
where `SpaceOutlineSource` is an Area value, and no other kind holds one (the
values are `AREA_SOURCES` and `ROOM_SOURCES` in the scorer, the one place they
are listed, and a value in neither fails). It writes `space_kinds_vs_ifc.txt`
(fixed words and integers, so the held-out models keep it as counts;
`tools/ci/scrub-holdout-results.sh`) and exits 1 on any space without a kind,
with another value, with an outline source it does not know, or in
disagreement. The compare job sums the head side's results
(`space_kinds_vs_ifc.py --total`), prints how many `IfcSpace`s of each kind the
run saw, and fails the job when a model has no result, when any model's counts
show a failure (whatever its verdict word says), when a model's spaces are not
the number its own export counted (`diagnostics.json`, or for a model kept as
counts only the MCP server's `rooms` count), so that a check that saw none of
a model's spaces cannot pass, and when a geometry export's spaces have no
`SpaceOutlineSource` to check (an export whose diagnostics say
`typed_no_geometry` has none by design). A run of models with no spaces, such
as a filtered one, passes when every export agrees that it has none. The step
runs in a run that names no `scorers` or names this one, and a run that names
others has not asked for it. The `IfcOpenShell validates exports` job of CI
runs it on Core Interior and RE1 Architecture, shows it failing on copies of
Core Interior's export with the property renamed, every room called an area,
and every room's outline source made an Area's, runs it on a
`typed-no-geometry` export, and shows `--total` failing on a result that says
PASS over a failure count, on spaces that have lost their `SpaceOutlineSource`
in an export with geometry, and on a result that saw none of the export's
spaces.

## Bodies across saves (RE-341)

No Revit export of a 2026 or 2027 beam, tread landing, stair run or sketch
outline exists to score them against. `tools/re/bodies_across_saves.py` holds
each 2026 and 2027 save of `rac_advanced`, `rac_basic` and `rst_basic` to the
same project's earlier saves, matched by `Tag`: beams, landings and sketch
outlines to the 2024 save, stair runs to the 2025 save (the 2025 save draws no
beam or landing). Measure's script runs it beside each such sample, and the job
`same-project-gates` turns the run red on a difference, a missing file, an
unknown project, fewer bodies than the sample holds, or a check that compared
nothing. That job also holds #1428's Areas gate (rac_advanced's Areas across its
2024 to 2027 saves, `tools/ci/check-areas-across-saves.sh`), each as a step that
runs whatever the other did. It is a job of its own, so a failure of either does
not remove Compare's report.

- **A change to a reader of Revit 2024 to 2027 is measured with
  `samples=true`** (every year, or `sample_years="2024 2027"`). Without it no
  sample is exported and the check does not run; with only some years the check
  compares what it can and reads `PARTIAL`, never `PASS`. Every such skip prints
  a notice and a line in the run summary, so a green run that did not run the
  check says so.
- **A run with `ifc_flags` checks neither same-project gate.** The gates are about
  the default export, and a flag such as `--phase` makes a save refuse. Each gate
  says in a notice that it compared nothing, so the run a gate judges has no
  `ifc_flags`. A whitespace-only value is no flag.
- **What it cannot catch**: an error the earlier save shares with the 2026 or
  2027 read. Nothing in it is a Revit export. It compares bodies, not
  quantities or property sets. No 2026 or 2027 held-out model holds an
  `IfcBeam`, so the beam reader has run only on two Autodesk samples saved by
  earlier releases.
- **A difference** is a reader that is wrong or a project that really differs
  between saves. Narrow the reader until it is never wrong and let what it
  leaves out stay a box, counted (`left as boxes`); do not allowlist.
- The output carries, for rac_advanced's ceiling 147534 and floor 139803, the
  class, `BodySource`, `ProfileResolved`, volume and vertex count in every save,
  and compares sketch outlines with the 2025 save as well, without scoring.

## The semantic merge check

Measure is the byte-level oracle: it says that an export moved. The semantic
merge check (#1334) says whether the pull request says what moved, before a
pull request that changes decoder output is armed: Jev reads the semantic diff
beside the pull request's own `## Summary` and `## Testing` and is asked whether
the export moves in a way they do not state specifically (Griffin, 2026-11-05).
A described, intended change passes; an unexplained move fails, and so does a
move that only a general description covers. The check run is `semantic diff
(advisory)` on the measured commit, and nothing is posted to the pull request.
It is not in the ruleset, so merging still waits on `full / CI passed`, but a
`fail` is a red check, and a pull request is not armed over one.

    gh workflow run measure.yml --ref main -f ref=<pull request head> -f base=main \
      -f nordic=true -f r2025=true

with no `models` filter. Adding `-f scorers=mcp_answers_vs_ifc` keeps every model
and leaves the other scorers out, which is the cheap form of the same check.
Most output changes land on the BIM4LCA office (#1388, #1400, #1401 and #1405
move nothing else), so a run without it, or with a `models` list, reads "pass"
on a change it did not look at.

**Only a run of every open model writes the check.** That is a run dispatched
in this repository with a base, no `models` filter, and the office and the
Revit 2025 models on; and it compares all 34 open models
(`OPEN_MODEL_NAMES` in `tools/re/semantic_diff.py`, a list that gains a name when
a fetcher gains a model). A run with a filter, or `base=none`, gets its diffs in
the `semantic-diff` artifact and no check run, and says so in the step summary
of both its compare job and the Semantic check run: a narrower run's pass would
stand beside a fuller run's fail on the same commit, since the check has one
name. A run whose base is not main or an ancestor of main writes no check
either (`-f base=<the pull request's own head>` scores an empty diff, and its
pass would supersede an earlier fail on the same commit); the check's step summary
names the base. A run that was asked for everything and compared less (a model
whose dump is missing on one side because the server crashed on it, a diff that
failed, a group of changes dropped for size, sampled property sets that one dump
alone holds too often (see below), a count a diff does not give, a model not
fetched) writes a
check that is **NOT MEASURED at best**, naming each model and its own reason; a
warn or a fail found in what was compared stays. The check lands on the commit
Measure measured, which the compare job takes from the plan job's resolved refs
(a shard job runs the measured code, so its `commit.txt` is not trusted), and
which is found again through the API: the same-repository pull request whose
head or merge commit it is, or else the one open pull request that holds it,
since a head moves on after Measure starts (the MCP contract sync alone does).
The verdict stays on the commit that was measured, and reads the pull request's
Summary as it is then. If several open pull requests hold the commit, or it is a
fork's, or none holds it, the check is NOT MEASURED and says so, **an empty diff
included**: a pass needs that pull request first. Measure the head of the pull
request, not a branch name that has moved, and write the Summary before the run.

**Where to find the verdict.** The verdict is on the measured commit, which is
not necessarily the pull request's head, and a pull request's page and its checks
list show only the head's checks, so a `fail` can sit on an older commit where
nobody looks. Look it up on the measured commit (the commit the Measure run's
summary names as its head):

    gh api "repos/<repo>/commits/<sha>/check-runs?check_name=semantic%20diff%20(advisory)"

A later commit does not clear a fail: only a new score of the same pull request
does, which is a complete Measure run of its new head, and that run writes its own
verdict on the commit it measured. A NOT MEASURED check, on that commit or a later
one, does not clear a fail either: only a new pass or a new warn does, so read the
newest verdict that is one of those three. A filtered Measure run writes no verdict
at all, so a pull request that changes output needs its final unfiltered run (no
`models` filter), made on the head it is armed at, and the verdict that run writes
is the one to read.

**The Summary must state each change specifically.** The agent that makes a
change also writes its Summary, so "changes how walls are drawn" could cover any
wall movement, a mistake included. Name which element classes or categories
change, which property or geometry, and how many where it was measured ("the 8
shape-edited floors of each office are drawn along their sloped top; 14 and 16
Basement walls now start at it"), and put measured figures in `## Testing`: the
check counts a figure stated there. A sentence that names an area or calls the
change an improvement covers nothing, and a finding it alone covers fails like an
unstated one. A figure that counts or totals what a stated change does (the
number of entities, of property values, of bodies of a source) is stated by the
change; one that moves and that nothing stated explains is not.

**A pass means the dumps agree, not that the exports are identical.** The dump
samples five elements per entity (the five lowest ElementIds): property sets,
types and placements are compared on those elements only, and a mesh vertex or
a profile point is not in the dump. The check's summary says so every time, and
Measure's `ifc.sha256` is what says that an export is identical.

- **What is compared.** The MCP server's dump of each openly licensed model
  (Core Interior, Einhoven, RE1, the BIM4LCA office, the Revit 2025 models), on
  both sides of the run: every element's row, the rooms' numbers and areas, the
  levels, the summary and the diagnostics, and the property and quantity sets,
  type object, placement and provenance of the sampled elements (the five
  lowest ElementIds of each entity). `tools/re/semantic_diff.py` is a pure
  function of the two dumps: elements added, removed and changed with their value
  before and after; the byte offsets inside `provenance` dropped; order ignored;
  values compared to the dump's precision, which is six decimals
  (`tools/re/mcp_client.py` rounds a float there before it writes the dump), so
  0.35 and 0.3500001 are different values when a dump holds both and a change
  below the sixth decimal is not in a dump; whether an importer would notice is
  Jev's question; an empty object or list the same as
  no field; identical changes collapsed by the whole (path, before, after), so a
  bulk change of 400 walls is one line and a wall that gains another value is a
  line of its own. Its totals per IFC entity, its coverage block (how many
  elements' property sets both dumps hold) and its text summary are in the
  `semantic-diff` artifact, one `.json` and `.txt` per model.
- **What it cannot see.** A model with no dump on both sides is not compared (a
  base older than the server). A change to the property sets of an element the
  dump does not sample, and a change to a mesh vertex or a profile point, is not
  in the dump: a pass says the dumps agree, not that the exports are identical
  (Measure's `ifc.sha256` says that). A change that lands only on a release the
  open models do not cover (the 2015 to 2023 Areas of #1316 have no open model)
  shows nothing here.
- **What leaves the runner.** Only the diff of the openly licensed models. The
  held-out, spent, BIMcollab and rac_advanced models and Autodesk's samples and
  families have no dump, and the compare job fails if a model on that list
  (`RESTRICTED_PREFIXES`) or on neither list holds one; `jev_check.py` checks the
  names again before it sends anything. Beside the diff go the pull
  request's Summary and Testing and nothing else of it: the title, branch,
  number and the model's name stay out of the question. Element names and
  values are in it, and they can hold an ElementId (`Wall:Basic:20796`,
  `provenance.element_id`): what protects the data is that only openly licensed
  models have a dump, not that no identifier is sent.
- **Scoring.** `tools/ci/jev_check.py` merges findings that repeat across
  models, groups them by category into chunks of at most 8 findings and 16,000
  characters, and asks Jev `jev-1.13.0` (the version the PR queue pins) about
  each chunk with the two descriptions labelled A and B, the labels assigned at
  random per chunk from a seed of the run and the head commit. Each chunk also
  carries the first 2,500 characters (as sent) of the pull request's
  `## Summary` and of its `## Testing` as quoted data, with HTML comments, HTML
  element tags (`<br>`, `<details>`, `<span>`; angle-bracket text that is not an
  element, such as `<set>.<property>` or `Option<T>`, stays) and control,
  format and invisible characters (zero-width, variation selectors, fillers, the
  Braille blank) removed first (the pull request's
  text is untrusted: the state says it is the author's claim, and each question
  says it cannot change what is asked); the check's summary shows all of the
  text Jev read, so a person sees what it saw. Per finding, Jev chooses how they
  relate to the difference:
  stated specifically (the element class or category and the kind of change, or
  the figure), a figure that follows from a specific change they state, covered
  only by a general description, unstated, or only rounding, a reordering or a
  spelling. The probability of "general" and "unstated" together is the
  finding's score. Per chunk Jev rates how much an importer would notice, on
  five levels (0 to 4). A finding at 0.85 or more is a **fail** in a chunk rated
  1.5 or more, a **warn** in a chunk rated 1.0 or more, and nothing below that;
  else **pass**. The values are `THRESHOLDS`; the wording and the values were set
  on the calibration runs in #1419, and a change to either needs those runs
  again. A part of the diff that counts or totals elements (the diagnostics and
  the summary) is asked with the largest differences in the elements beside it,
  so that a count that follows from them can be told from one that does not; a
  list of text lines is shown by the lines that differ; and a log line whose
  words did not change and only its figures did is listed and not asked about,
  since its figures are counters of the same diff. A diff with no finding is a
  pass without a call to Jev and needs no Summary.
- **How to treat a verdict.** A `pass` needs nothing. A `warn` is answered in
  the pull request body: one line for each warned finding, saying what moved and
  why, with the evidence. A `fail` means the diff moves the export in a way the
  Summary and Testing do not state specifically: fix it by changing the code, or
  by stating the change in the pull request's Summary (which classes, which
  property or geometry, how many where measured, in the words of the finding;
  the check's summary says so on a fail), then measure the new head. A Summary
  that only names the area ("improves rooms") covers nothing. `NOT MEASURED` is
  neither: it names its cause, and the check is run again once the cause is gone.
- **NOT MEASURED, never a pass.** No key in the context, a pull request from a
  fork or no same-repository pull request that holds the measured commit, a
  pull request with no `## Summary` or an empty one (the template's comment
  alone) when the diff has findings, HTTP 402 or a 429 for exhausted quota (the
  credit is out; not retried), a key refused, a reply from another model than the
  pinned one or an answer whose probabilities do not add up to 1, a time-out or a
  service that stays busy after three tries, an error in the scoring (named by
  its type, never a message), a measured commit that is not a 40-character SHA, a
  diff with no model compared, any open model that is not compared in full (see
  above), a group of changes dropped from a diff for its size (over 20,000 groups
  in one model), sampled property sets that one of a model's two dumps alone
  holds too often (none held by both while either holds any, or more than a tenth
  of all of them held by one: the dump samples five elements per entity, the five
  lowest ElementIds, so a change that adds or removes elements of an entity moves
  the sample and a few held by one dump only are no gap, and the check's summary
  says how many there are), a measured commit that is not the commit the check is written on (the
  keyless run of a pull request that changes the check writes on the pull
  request's head, whatever commit Measure measured), a base that is missing or could
  not be checked against main, and any chunk not scored (more than 150 chunks, a run past 7
  minutes, or a chunk that does not fit in 16,000 characters as sent, a
  character outside ASCII counting six) when no scored chunk is a warn or a fail
  already. A fork's run and a run that was not dispatched are not NOT MEASURED:
  they post nothing (see the trust bullet). Run it
  again with `gh run rerun <semantic check run>`, or score a Measure run from
  main with `gh workflow run semantic-check.yml --ref main -f run_id=<Measure run
  id>`.
- **Cost.** Jev bills $0.042 per million input tokens; a diff of a few hundred
  kilobytes costs well under a cent, so the results are not cached (the Actions
  cache is not private to the workflows that write it, and the artifact of the
  run, kept 7 days, holds the result file).
- **The key and the trust.** `TYPESAFE_API_KEY` is a secret of the Actions
  environment `jev`, whose deployment branch policy allows only `main`. The
  scoring job is a `workflow_run` workflow, which always runs from main's
  definition, and it runs no measured code: it downloads the `semantic-diff`
  artifact and reads it as JSON. It scores only a Measure run whose event is
  `workflow_dispatch` and whose head repository is this repository: a pull
  request from a fork runs its own `measure.yml` and controls everything in its
  artifact, the measured commit included, so any other run posts nothing, with a
  notice. (A fork's `measure.yml` is not read by the job; main's workflow decides
  what it scores.) The Jev call does not follow a redirect, so the key goes to
  `api.typesafe.ai` and nowhere else. A pull request that changes the check runs the
  same job without the environment, so without the key, and writes its check on
  itself (NOT MEASURED where the diff has findings, a pass where it has none).
  The text of the diff and the pull request's Summary are data from files and
  from the author, so a crafted name or Summary could try to steer Jev: both go
  in as quoted state, the questions say neither can change what is asked, and
  the check is advisory for that reason; the diff itself is in the artifact for a
  human to read.
- **The check's conclusion.** pass is `success`, warn and NOT MEASURED are
  `neutral`, fail is `failure`: a red check on the measured commit, which an agent does not arm over
  and the watcher counts as failing, as intended.

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
`rvt-gltf`, `rvt-info`; and `rvt-rs-mcp` in a run that scores the MCP server,
under a key ending `-mcp`) under `rvt-bin-v2-<full SHA>-<OS>-<arch>-<toolchain
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
