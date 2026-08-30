# Committed oracle runs (`research/es-remap/golden/`)

Every Revit-hosted run of the ES-remap oracle
([`tools/oracle/runner/pyrevit`](../../../tools/oracle/runner/pyrevit/README.md))
that is worth keeping goes here as one directory, committed as-is:

```text
research/es-remap/golden/
  2024-20260901-a1b2c3d/          # <revit-version>-<yyyymmdd>-<short git sha of rvt-rs at run time>
    bundle.json                    # file hashes, seed roles, Revit version/build, transitions, fixture law
    observations.json              # one record per (owner, reference path, transition)
    truth-seed.json … truth-C4a.json
    seed-ES-remap-00.rvt, N1.rvt … C4a.rvt, C3a-source.rvt, C4a-source.rvt
    runner.log
    SESSION.md                     # who/what drove Revit and how (see below)
```

`tests/es_remap_golden.rs` runs in normal CI and, for every run directory
that has a `bundle.json`, checks that each referenced file is present with
its recorded SHA-256, that every observation carries the schema's required
fields with valid `kind` / `evidence_tier` / path-segment values, that
transition ids are ones `manifest.yaml` names, and that the Revit build is
recorded. It does **not** check that rvt-rs decodes anything: `oracle_agrees`
stays `null` until an ES decoder exists, and committing a run is not a
capability claim (unified report §15.15: promotion needs API ↔ decoded
agreement, save/reopen confirmation, several id magnitudes, fresh Revit
processes, negative controls, and normalized-record localization).

## What a run must include

- **The files.** All of them — the seed and every transition output. They
  are owned synthetics built from an empty default template (Lane A,
  Apache-2.0); no Autodesk sample content is involved. Keep a run under
  64 MiB; the test enforces that.
- **`SESSION.md` (provenance).** Written by whoever drove Revit — a person or
  a computer-use agent. Minimum: Revit release + build (from `bundle.json`),
  the pyRevit version, the rvt-rs commit the runner came from, the machine
  (VM/hardware, OS build), who ratified the run, and a pointer to the
  session recording or agent transcript if one exists. If any transition was
  re-run or hand-corrected, say so per transition.
- **Untouched runner output.** Do not edit `observations.json` or the truth
  files by hand; fix the runner and re-run.

## Naming and tagging

Directory name: `<revit-version>-<yyyymmdd>-<rvt-rs short sha>`. The
`revit_build` string inside `bundle.json` (e.g. `24.0.20.20`) is the
authoritative build tag; every observation carries `revit_version` and
`revit_build` too, so records stay attributable after they are merged into
larger evidence tables.

## Replay

Re-running the runner on the same Revit build from the same seed manifest
must reproduce the same observation *classifications* (the `kind`,
transitions, and reference transitions) — ElementId numbers and file hashes
will differ per session. A replay that changes a classification is a
finding, not noise: record it in `docs/disc-112-coordination.md` and open an
issue before touching the manifest.

## Next step after the first run lands

Run the stream-evidence harness over each before/after pair, normalize the
changed records, and attempt localization — those attempts write `span`
into copies of the observations and set `oracle_agrees` from the API truth.
Only then does the ES capability move off E0/E1.
