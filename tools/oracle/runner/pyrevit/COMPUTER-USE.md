# Driving the oracle with a computer-use agent

The runner does the Revit-side work; a computer-use agent can do the rest of
the grind so the maintainer only ratifies. This is the recipe. It assumes a
Windows machine or VM — Revit does not run on macOS or Linux — with a Revit
2023 or 2024 install (a trial license is enough for owned synthetic
fixtures).

## Preconditions the agent must find or report missing

1. Revit 2023 or 2024 installed and licensed (trial OK).
2. Git and a checkout of `DrunkOnJava/rvt-rs` at a known commit.
3. pyRevit installed for that Revit release (https://pyrevitlabs.io/).
4. `tools/oracle/runner/pyrevit` added to pyRevit's extension search paths
   (pyRevit Settings → Custom Extension Directories), then pyRevit reloaded.
5. A writable output root; set `RVT_ORACLE_OUT` before launching Revit if
   the default `tools/oracle/out/` is not wanted.

If any precondition is missing the agent stops and reports it; it does not
improvise a different toolchain.

## The run

1. Launch Revit. Dismiss the start screen.
2. **New → Project → default project template** (Architectural is fine).
   Do not add anything to the model — the runner builds the seed itself.
3. Click **RvtOracle → ES Remap → Run ES-remap-00**. Expect a modal at the
   end with the observation count and the output directory. A transition
   that raises is logged and recorded as `operation: Rejected`; the run
   continues.
4. Record the session: screen recording or the agent's own action transcript,
   plus the Revit **Help → About** build string.
5. Close Revit without saving the active document again (the runner already
   saved the seed under its own name).

## Hand-off

1. Copy `out/<revit-version>-<timestamp>/` into
   `research/es-remap/golden/<revit-version>-<yyyymmdd>-<rvt-rs short sha>/`.
2. Write `SESSION.md` in that directory (see the golden README for the
   required contents); link the recording/transcript.
3. `tools/check-local.sh` — `tests/es_remap_golden.rs` validates the run.
4. Open a PR titled `research(es-remap): oracle run <dir>` with the checklist
   from the PR template. Do not touch `docs/support-matrix.json` or any
   capability status in the same PR: a run is evidence, not a claim.

## What the agent must never do

- Use Autodesk sample projects, customer files, or anything from
  `_project_corpus` as the seed. Owned synthetics only.
- Edit `observations.json`, the truth files, or `bundle.json` by hand.
- Claim ES remapping works, or change `es.elementid_remap` from its current
  status, on the strength of a run.
- Commit `tools/oracle/out/` (git-ignored) or any `.vhdx`/VM images.

## Replay

A second agent (or the same one on a later day) repeats the run on the same
Revit build and compares the classification columns of `observations.json`
against the committed run. Differences are findings — record them, do not
average them away.
