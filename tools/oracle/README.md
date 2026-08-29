# Revit-hosted oracle runner (scaffold)

Phase 2 of the ES ElementId remapping program
([`docs/research/unified-research-report.md`](../../docs/research/unified-research-report.md))
requires a **Revit API oracle**. The Cursor Cloud environment has **no Revit**.

This directory documents the external runner contract. It does **not** ship
add-in binaries or invented `.rvt` layouts.

## Goals

1. Open / create an owned synthetic project (`S_All` seed).
2. Emit API truth JSON (element ids, unique ids, ES entity field values).
3. Apply **exactly one** semantic mutation per transition.
4. Save before/after files + observation records conforming to
   `docs/schemas/es-observation.schema.json`.
5. Hand fixtures to CI as opaque regression inputs (never discovery oracles).

## Suggested layout (external machine)

```text
tools/oracle/
  README.md          # this file
  runner/            # future: C# / pyRevit / RevitAddIn project (not in Cloud)
  out/               # generated fixtures (git-ignored locally)
```

## Blocked-on-Revit checklist

- [ ] Revit install + API add-in or script host available
- [ ] Generate Phase A `S_All` owned synthetic
- [ ] Capture API truth for ES-held ElementId fields
- [ ] Produce no-op (`N*`) and identity-save baselines
- [ ] Produce remap pairs `R1` / `R2` (one mutation each)
- [ ] Produce copy pairs `C1` / `C2` (+ constrained `C3a` / `C4a` when defined)
- [ ] Validate observations against `es-observation.schema.json`
- [ ] Keep capability `es.elementid_remap` at `research` / E0 until G-E4

## Non-goals here

- Inventing ES on-disk byte layouts
- Claiming remapping works from Cloud-only work
- Scoring ES refs into ElemTable ownership (#152)
- Emitting ES edges on default IFC
