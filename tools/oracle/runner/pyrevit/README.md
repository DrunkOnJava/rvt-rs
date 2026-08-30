# pyRevit oracle runner for ES-remap-00

The Revit-hosted half of Phase 2 in
[`docs/research/unified-research-report.md`](../../../../docs/research/unified-research-report.md)
§15: build the owned `S_All` seed, apply exactly one semantic mutation per
transition, save before/after files, and write API-truth observations that
rvt-rs can later be checked against.

**Status: UNTESTED.** This was written on a machine without Revit. The Revit
API members it relies on are named inline in `lib/rvt_oracle.py`; expect a
few small signature fixes on the first run and please commit them back.

## What it produces

`out/<revit-version>-<timestamp>/`

| File | Content |
|---|---|
| `seed-ES-remap-00.rvt` | the owned seed: wall `W` and DataStorage `DS` carry `Entity(S_All)`; `T` (target) and `X` (control) are level-independent DirectShape boxes |
| `N1.rvt` … `C4a.rvt` (+ `C3a-source.rvt`, `C4a-source.rvt`) | one file per transition, each produced from a fresh copy of the seed |
| `truth-<label>.json` | API truth per document state: roles, ElementIds, UniqueIds, every ElementId leaf in the entity with its path |
| `observations.json` | one record per (owner, reference path, transition) in the shape of [`docs/schemas/es-observation.schema.json`](../../../../docs/schemas/es-observation.schema.json) |
| `bundle.json` | file SHA-256s, seed roles, Revit version/build, transition list, fixture law |
| `runner.log` | timestamps and any transition that raised |

Observation `kind` per transition: `N1`–`N4` → `noop_baseline`, `R1`/`R2` →
`scalar`, `C1`/`C2`/`C3a`/`C4a` → `copy`. `evidence_tier` is `E1`
(single-environment observation) as listed in `TRANSITIONS`; promotion is the
coordinator's decision, never the runner's. `oracle_agrees` is always `null`
here — this *is* the oracle side; agreement is computed when rvt-rs decodes
the same files.

## Schema under test

`S_All` (GUID `9c6a8b9e-…-1c01`):

| Field | Type | Seed value |
|---|---|---|
| `F_ref` | ElementId | `T` |
| `F_list` | ElementId[] | `[T, X]` |
| `F_key_map` | Map<ElementId, Int32> | `{T: 1001, X: 1002}` (values are immutable markers so an entry survives a key remap) |
| `F_value_map` | Map<String, ElementId> | `{"target": T, "control": X}` |
| `F_child` | nested Entity(`S_Child`) | `F_child_ref = T` |
| `F_note` | String | `"control"` |

Role markers live in a separate schema `S_Role` (never inside `S_All`), so
copies can be told apart from originals without touching the schema under test
(`CopyCorrespondence::RoleMarker`). Copies get their role suffixed with `'`.

## Running it

1. Install [pyRevit](https://pyrevitlabs.io/) for the Revit release you are
   targeting (2023 and 2024 are the versions the fixture law names).
2. Add this folder's `RvtOracle.extension` to pyRevit's extension search paths
   (pyRevit Settings → Custom Extension Directories → add
   `tools/oracle/runner/pyrevit`), then reload pyRevit.
3. Open a **new, empty project** from the default project template.
4. Optional environment variables (set before starting Revit):
   - `RVT_ORACLE_OUT` — output root (default `tools/oracle/out/`, git-ignored)
   - `RVT_ORACLE_ONLY` — comma-separated transition ids to run, e.g. `N1,R1,C1`
5. Click **RvtOracle → ES Remap → Run ES-remap-00**.

Per-transition isolation: every transition copies `seed-ES-remap-00.rvt` to a
`work-<id>.rvt`, opens that, mutates, and saves the result as `<id>.rvt`. The
seed is never mutated after it is saved.

## Feeding results back to rvt-rs

- Copy `out/<run>/` somewhere the corpus policy allows (owned synthetics are
  Lane A material; they contain no Autodesk sample content).
- Validate: `python3 -c 'import json; json.load(open("observations.json"))'`
  and check against the schema with any JSON-Schema validator.
- The next step in the report's sequence is stream-evidence + record
  normalization over the before/after pairs (`stream-evidence` binary on
  `main`), then localization attempts that write `span` into the observation
  and set `oracle_agrees` from the API truth.

## Known limits of this first cut

- `C3a`/`C4a` create the destination project with
  `Application.NewProjectDocument(DefaultProjectTemplate)`; the seed's schemas
  are already in process memory, so this measures destination-*file* schema
  introduction, not a schema-free process (§15.10 caveat).
- Lifecycle rows `S1`, `D1`–`D4`, `U1`/`U2`, `G1`, `P1` and the map-key /
  map-value / nested repetitions (§15.14) are not implemented yet; the
  transition table is a list, so adding a family is a small change.
- Revit 2023 vs 2024 `ElementId` width differences are normalised through
  `eid_value()` (`Value` on 2024+, `IntegerValue` before).
