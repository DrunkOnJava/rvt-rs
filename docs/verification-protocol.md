# Verification protocol: cross-witness agreement for undocumented formats

Status: **protocol v0 — two recorded RVT → IFC edges, three independent
reading witnesses.**
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
Revit's own exporters are the first authoring witness; IfcOpenShell and
IFClite are the third-party reading witnesses on the IFC node.

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
| RVT → IFC (element fixture) | Autodesk Revit 2024 (magnetar dataset export) | `2024_Core_Interior.rvt` (c805df44…) → `2024_Core_Interior.ifc` (d07c7462…) | `tests/project_count_fixtures.rs` (rvt-rs decode vs manifest counts derived from the export); `tools/ci/witness-ifcopenshell.py` and `tools/ci/witness-ifc-lite` (two unrelated IFC parsers vs the same counts); `tools/ci/witness-verdict.py` (three-lineage verdict) | recorded, gated (tier-2 CI) |
| RVT → IFC (full project) | Autodesk Revit 24.0.20.20 via ODA SDAI 23.12 | `2024_Core_Interior.rvt` (c805df44…) → `IFC Exports/2024_Core_Interior_slim.ifc` (bfdf36ff…, 19879 entities) | the same four gates, wired to `tests/fixtures/project-counts/2024-core-interior-slim.json`; claimed surface covers `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` at tolerance 0 since RE-21 | recorded, gated (tier-2 CI) |
| RVT → IFC (rvt-rs writer) | rvt-rs | rvt-rs output from Einhoven / synthetics | IfcOpenShell validation in `ci.yml` | recorded — validates the **writer**, not the decoder |
| RVT → DWG | Autodesk Revit (pending) | none — no Revit-exported DWG exists in any public corpus | dwg-rs parse vs rvt-rs geometry recovery | **not recorded** |

Honesty note on the two IFC edges: the paired Core Interior IFC is an
element-export fixture (20 KB), not a full project schedule, so most
category expectations there are zero — a real edge, and a weak one. The
full-project export is the strong half of the same edge and it remains
the place where the distance to Revit is measured: it carries 360
`IfcWall`, 132 `IfcDoor`, 6 `IfcWindow`, 256 `IfcColumn`, 116
`IfcSpace`, 80 `IfcSlab` and 15 `IfcBuildingStorey`, against which
rvt-rs recovers 360 / 132 / 6 / 256 / 18 / 64 / 12 (2026-08-30; walls,
doors and windows were 0 before #211, columns 0 before #204). Five of
its thirteen categories are still `known_gap` or `decoder_baseline` with
a tracking issue (#31, #33, #34, #35, and levels under #33), and the
verdict's claimed surface is eight fields wide — `IFCWALL`, `IFCDOOR`
and `IFCWINDOW` joined `IFCCOLUMN`, `IFCROOF`, `IFCBEAM`,
`IFCFLOWTERMINAL` and `IFCUNITASSIGNMENT` when their **ElementId sets**,
not merely their counts, matched the export at tolerance 0 (RE-21). Four
of the eight are agreements about presence now; the other four are
agreements about absence. The excluded list is still the point: floors
(79 of 80 recoverable, wrong identity key — #212), spaces, materials and
property sets remain the measured distance between this decoder and
Revit's own exporter on a real project, recorded rather than rounded
off.

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

The third adopted reading witness, IFClite, shows the rule applied to a
witness that could have been linked and deliberately is not. It is the
crates.io crate `ifc-lite-core`, pinned at `=7.1.1` (published 2026-08-27),
from `LTplus-AG/ifc-lite` — Rust, and MPL-2.0 in both the crate metadata and
the repository LICENSE, which corrects an earlier registry entry that
recorded `MIT` against a bare `ifc-lite` slug. Its independence is real: the
crate carries its own byte-level STEP scanner and nom tokenizer and links no
IfcOpenShell code. The project *validates its geometry kernel against*
IfcOpenShell, which is a comparison, not a lineage — the same distinction
that makes FreeCAD's BIM workbench (which runs IfcOpenShell) the same
witness as its base. MPL-2.0 is file-level copyleft and would be safe to
link, but `tools/ci/witness-ifc-lite` is still its own workspace root so no
third-party reader ever enters an Apache-2.0 artifact, and
`tests/witness_registry.rs` fails the build if the Cargo pin, the version
the binary stamps into every observation, and the registry entry drift
apart (spec §9.6 forbids silent witness upgrades).

The umbrella repository for this protocol exists:
[DrunkOnJava/octetproof](https://github.com/DrunkOnJava/octetproof) (created
2026-08-30 on the owner's call once the first edge was gated) — protocol,
schemas, witness registry, golden corpus (manifests, observations, verdicts;
bytes fetched by hash), verdict + replay tools, and its own fail-closed CI
gate. It never parses a format byte; the adopted readers run there as
separate processes. This repository stays the working copy of the registry
and the place where decoder-side observations are produced; the umbrella
mirrors them.

## OctetProof alignment

The protocol above is the in-repo instance of
[OctetProof 1.0.0](octetproof-spec.md) (the received draft is kept verbatim at
`docs/octetproof-spec-draft.md`; §19 of the 1.0.0 spec lists the corrections).
The observation and verdict shapes are published as machine-checkable JSON
Schema 2020-12 documents —
[`docs/schemas/witness-observation.schema.json`](schemas/witness-observation.schema.json)
(§6.2) and
[`docs/schemas/witness-verdict.schema.json`](schemas/witness-verdict.schema.json)
(§6.3) — and the committed files under
`research/witness/magnetar-2024-core-interior/` and
`research/witness/magnetar-2024-core-interior-slim/` validate against them.
§5.3.1 of
the spec maps its `registry.yaml` vocabulary onto the fields of
`research/witness-registry.json` and names the four requirements
(`ci_eligible`, coverage declaration, determinism attestation, exact version
pinning) that are enforced in the umbrella repository rather than here.

What exists here today, per layer:

| OctetProof layer | rvt-rs today |
|---|---|
| 1 Protocol | this document + [`docs/octetproof-spec.md`](octetproof-spec.md) (1.0.0) |
| 2 Golden corpus | `research/witness/<artifact>/observations/*.json` + `verdict.json`; artifact hashes in the registry and the project-count manifests (the bytes themselves stay in the magnetar dataset, fetched by hash) |
| 3 Witness registry | `research/witness-registry.json` (`lineage`, `checked`) |
| 4 CI gate | `tools/ci/witness-verdict.py` in the `ifcopenshell-validate` job: fail-closed statuses `PASS` / `DISAGREE` / `INSUFFICIENT_WITNESSES` / `INSUFFICIENT_INDEPENDENT_WITNESSES` / `REJECTED_INPUT` / `MANIFEST_ERROR` / `REPLAY_DRIFT`; observations and verdict published as a build artifact |
| 5 Decoder witness mode | `rvt-ifc --observation PATH --artifact-id ID` (source witness); `tools/ci/witness-ifcopenshell.py --observation` and `tools/ci/witness-ifc-lite --observation` (two independent bridge witnesses) |

Observation payloads are hashed after canonicalization (sorted keys, no
whitespace, UTF-8); `tests/witness_verdict.rs` recomputes those hashes from
the committed files and asserts the committed verdict is `PASS`, so a
decoder change that alters what rvt-rs emits on the golden artifact fails
the build twice — once in replay and once in the verdict.

## Non-claims

- Recording an edge does not make a capability verified; the agreement must
  run in CI within a stated tolerance.
- Agreement between rvt-rs's IFC writer and IfcOpenShell says the writer
  emits valid IFC; it says nothing about decode fidelity.
- Nothing here changes any status in `docs/support-matrix.json`.
