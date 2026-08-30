# Verification protocol: cross-witness agreement for undocumented formats

Status: **protocol v0 — one recorded edge, one independent witness gate.**
This document defines what "verified" means for rvt-rs and its sibling
[dwg-rs](https://github.com/DrunkOnJava/dwg-rs). It is deliberately small.
It grows only when a new edge or witness is actually recorded, hashed, and
gated in CI.

## Thesis

Closed BIM formats are not a reverse-engineering problem; they are a
verification-graph problem.

- A **node** is a file format (RVT, DWG, IFC, glTF, …).
- An **edge** is a cross-format export path from one node to another,
  produced by a named **authoring witness** (for RVT today: a licensed
  Autodesk Revit session exporting to IFC or DWG).
- A **reading witness** is an independently implemented parser of a node
  (rvt-rs for RVT, dwg-rs for DWG, IfcOpenShell for IFC).
- **Agreement** is a recorded comparison between what two or more
  independent witnesses report about the same underlying model across one
  or more edges.

For a format with no specification, correctness cannot mean "matches the
spec". It means: **agrees with N independent witnesses across M format
boundaries**, where N, M, and the exact artifacts are recorded, hashed, and
re-checked by CI. rvt-rs and dwg-rs are the first two reading witnesses;
Revit's own exporters are the first authoring witness; IfcOpenShell is the
first third-party reading witness.

The research contribution is the protocol and its corpus, not any
individual decoder.

## Vocabulary and where it lives

| Term | Definition | Recorded in |
|---|---|---|
| Golden artifact | One canonical file, identified by SHA-256 and byte length, with license and provenance | `research/witness-registry.json` → `artifacts` |
| Edge | `from` node → `to` node `via` an authoring witness, binding a source artifact to a derived one | `research/witness-registry.json` → `edges` |
| Witness | A reader or author with a stable identity (repo + version policy) | `research/witness-registry.json` → `witnesses` |
| Agreement | A comparison with a named **gate** (a test or CI script), its claim scope, and its status | `research/witness-registry.json` → `agreements` |
| Claim | Any user-facing capability statement | `docs/support-matrix.json`, `docs/status.md` |

`tests/witness_registry.rs` keeps the registry internally consistent and in
sync with the project-count manifests that already carry the artifact hashes.

## The protocol

1. **Record one canonical export.** One authoring session produces the
   derived artifact from the source artifact. Record the authoring witness
   and its build (Revit release/build, exporter settings) — the same way
   the ES-oracle runs record `revit_build`.
2. **Hash it.** SHA-256 + byte length for every artifact, source and
   derived. The registry and the project-count manifests must carry the same
   hashes; the test fails if they drift.
3. **Commit the witness parse.** Each reading witness's report on the
   artifact is committed (counts, geometry summaries, diagnostics) — not
   just "it opened".
4. **Require cross-witness agreement before any semantic claim ships.** A
   capability may move to `verified` in `docs/support-matrix.json` only when
   at least two independent witnesses agree across at least one recorded
   edge, within the tolerance stated in the agreement, and the comparison
   runs in CI. One witness alone (however clever) yields at most `partial`.
5. **Gate regressions.** Every agreement names the test or script that
   enforces it. A decoder change that breaks a recorded agreement fails CI;
   the manifest tolerance is the only place slack is allowed, and changing it
   is a reviewed decision with a written reason.

### How this maps onto existing tiers

| Evidence tier (unified report) | Protocol reading |
|---|---|
| E0–E1 | No independent witness; single-environment observation |
| E2 | Multi-file/multi-release, still one witness |
| E3 | Independently reproduced on redistributable / owned fixtures |
| E4 | Oracle-backed — an authoring witness plus an independent reading witness agree, gated in CI |
| E5 | Promoted with a support-matrix row |

"Verified" in the support matrix therefore requires E4 under this protocol.

## Edges recorded today

| Edge | Authoring witness | Artifacts | Agreement gates | Status |
|---|---|---|---|---|
| RVT → IFC | Autodesk Revit 2024 (magnetar dataset export) | `2024_Core_Interior.rvt` (c805df44…) → `2024_Core_Interior.ifc` (d07c7462…) | `tests/project_count_fixtures.rs` (rvt-rs decode vs manifest counts derived from the export); `tools/ci/witness-ifcopenshell.py` (IfcOpenShell parse of the export vs the same counts) | recorded, gated (tier-2 CI) |
| RVT → IFC (rvt-rs writer) | rvt-rs | rvt-rs output from Einhoven / synthetics | IfcOpenShell validation in `ci.yml` | recorded — validates the **writer**, not the decoder |
| RVT → DWG | Autodesk Revit (pending) | none — no Revit-exported DWG exists in any public corpus | dwg-rs parse vs rvt-rs geometry recovery | **not recorded** |

Honesty note on the first edge: the paired Core Interior IFC in the magnetar
dataset is an element-export fixture (20 KB), not a full project schedule,
so most category expectations there are zero. It is a real edge with an
independent third-party witness, and a weak one. The full-project export
(`2024_Core_Interior_slim.ifc`, bfdf36ff…, Autodesk Revit 24.0.20.20) is
registered as an artifact but has no manifest yet.

## The second edge: RVT → DWG

This is the edge that turns the protocol from "one lucky dataset" into a
method. It needs one licensed Revit session exporting the owned/redistributable
projects to DWG (R2018 and R2013). The plan, in order:

1. **2D plan-view export first.** A plan view flattens to LINE, LWPOLYLINE,
   ARC, CIRCLE, TEXT, MTEXT, INSERT, HATCH and the LAYER/LTYPE/BLOCK tables —
   the subset dwg-rs must parse reliably (measured real-file coverage today:
   20.1% aggregate, 44.2% on R2018). Three independent readers witness it:
   dwg-rs, ACadSharp (MIT, C#, separate process) and jDwgParser (GPL-3.0,
   Java, separate process). Agreement scope: wall location curves, floor
   boundary loops, level elevations, category → layer mapping, text.
2. **3D solid export second.** Revit flattens walls into ACIS 3DSOLID /
   REGION / BODY payloads; ACadSharp reads those since v3.6.51 (2026-07-29),
   MESH is still unimplemented there, and dwg-rs has no ACIS path. This edge
   waits for that coverage.
3. Neither edge can witness identity-level questions (ElementIds, Extensible
   Storage) — those keep the API-runner oracle.

### Witness inventory and independence

`research/witness-registry.json` carries every known witness per node with
its license, language, and — where this repository checked it against the
GitHub API — a `checked` date. Two rules keep the inventory honest:

- **Independence.** Two witnesses count as independent only if neither is
  built on the other's implementation. uncad is FFI over LibreDWG; FreeCAD's
  BIM workbench runs IfcOpenShell; GDAL's DGN driver wraps dgnlib; Ara3D's
  mesh side is web-ifc. Each pair is one witness, not two.
- **Claims stay the project's own.** A coverage or pass-rate figure quoted in
  a candidate's notes (jDwgParser's "100% entity types / 92% samples", for
  example) is that project's claim until an agreement recorded here
  reproduces it.

Copyleft (GPL/LGPL/CDDL) and non-Rust witnesses are adopted only as separate
CI processes — never linked into the workspace. Unlicensed projects (reviter
today) cannot be adopted at all until they carry a license.

The umbrella repository proposed for this protocol (vision, golden corpus,
cross-witness CI gate, protocol spec; decoders linked, never parsing a
byte) is deliberately **not** created yet. It earns its existence the day
the second edge is recorded and gated. Until then this document, the
registry, and the gates live here.

## Non-claims

- Recording an edge does not make a capability verified; the agreement must
  run in CI within a stated tolerance.
- Agreement between rvt-rs's IFC writer and IfcOpenShell says the writer
  emits valid IFC; it says nothing about decode fidelity.
- Nothing here changes any status in `docs/support-matrix.json`.
