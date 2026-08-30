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

## Layout

```text
tools/oracle/
  README.md                       # this file
  runner/pyrevit/                 # pyRevit extension: seed + N1-N4, R1/R2, C1/C2, C3a/C4a
    README.md                     #   how to install, run, and what it writes
    RvtOracle.extension/lib/rvt_oracle.py
    RvtOracle.extension/RvtOracle.tab/ES Remap.panel/Run ES-remap-00.pushbutton/script.py
  out/                            # generated fixtures + observations (git-ignored)
```

The runner is a first cut written **without a Revit install**: it targets the
documented Revit API (Wall.Create, DirectShape, DataStorage, ExtensibleStorage
SchemaBuilder/Entity, ElementTransformUtils.CopyElements, Document.SaveAs) and
writes observations in the `es-observation.schema.json` shape, but it has not
been executed inside Revit yet. Treat the first run as a debugging session and
commit the API fixes back.

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
