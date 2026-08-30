# Project Status

Last reviewed: 2026-08-29

This page is the public source of truth for what rvt-rs can do today. It is
intentionally blunt so users can decide quickly whether the tool fits their
workflow.

The current support boundary is tracked in the
[supported MVP input profile](supported-profile.md). The machine-readable
[executable support matrix](support-matrix.json) (schema:
[`support-matrix.schema.json`](schemas/support-matrix.schema.json)) is the
checked-in capability ceiling for audit controls COR-001 / TEST-001 /
DOC-001 — statuses there must stay honest and must not claim
converter-grade typed recovery. Keep that matrix, this page, the README,
and the viewer support matrix aligned.

**Verification protocol:** what "verified" means here is defined in
[`docs/verification-protocol.md`](verification-protocol.md) — cross-witness
agreement across recorded format edges, tracked in
[`research/witness-registry.json`](../research/witness-registry.json) and
enforced by `tests/witness_registry.rs` plus the CI gates it names. One
edge (RVT → IFC via Revit's exporter, witnessed by IfcOpenShell) is recorded
today; the RVT → DWG edge with dwg-rs is pending a Revit session.

**OctetProof instance:** the draft protocol spec received 2026-08-30 lives at
`docs/octetproof-spec-draft.md` (reviewer notes in its Appendix A). Its
observation / verdict / replay shapes are implemented in-repo — `rvt-ifc
--observation`, `tools/ci/witness-verdict.py`, committed artifacts under
`research/witness/` — and gated in CI; see the "OctetProof alignment" section
of `docs/verification-protocol.md`. The umbrella repository still waits for the
second (RVT → DWG) edge.

**Research program pointer:** the governing
[unified research report](research/unified-research-report.md) (rev 1.1)
sequences Phase 0 product readiness → Phase 1 identity/evidence contracts →
Phase 2 ES ElementId remapping oracle (H-ES5). Phase 2 fixture generation
requires a Revit-hosted API oracle and is **not** available on Cloud VMs.
ES remapping is **not** a shipped capability; default IFC omits ES edges.
Coordination mirror: [Discussion #112 notes](disc-112-coordination.md).

## User-Facing Summary

rvt-rs is useful today for inspecting Revit files without Revit, extracting
metadata, reading the embedded schema, auditing stream contents, running a
zero-upload browser viewer, and producing valid IFC/glTF/SVG outputs from the
parts of the model that are actually decoded.

**Generic real-project typed model extraction is not solved yet.** rvt-rs is
not a production RVT-to-IFC converter for arbitrary architectural projects.
Production `walker::iter_elements` prefers typed MVP decoders on
`Global/Latest` (fail closed), merges version-gated 2023 ArcWall partition
recovers, and additionally merges fail-closed partition MVP recovers for
`Level` (elevation + name), `Material` / `Room` (display-name candidates),
`Floor` (ArcWall-excluded plan loops), and — on Revit 2024 —
`ArcWallRectOpening` index rows with ElemTable-confirmed related-id
provenance (still not typed `Door`/`Window`). IFC export maps recovered
Levels → storeys, Floors → boundary-annotated slabs, Rooms → spaces, and
Material display names → `IfcMaterial`. Viewer File Status lists recovered
storey names, material name samples, and an honest Parameters row (empty
until AProperty* host joins). The scene tree groups elements under
`IFCBUILDINGSTOREY` nodes (ArcWalls by elevation; Floors/Rooms remain
Unassigned until Level ElementIds exist on both sides — bind plumbing is
fail-closed and corpus-idle today). RE-20 (same corpora) found **no**
recoverable Level ElementId map: `Level` is absent from Formats schema;
LevelAssociationCell / name / elevation proximity scans are
noise-dominated — Floors/Rooms stay Unassigned by evidence, not omission.
RE-19 found **no** reliable Door vs Window discriminator and **no**
schema-field / 2024 ArcWall envelope suitable for fail-closed decode —
typed `Door`/`Window` and non-ArcWall `Wall` stay unsolved. AProperty*
carriers are not present in production `iter_elements` / Global/Latest
candidate scans on these corpora (#35 host joins idle). Floor↔ElemTable
id binding and slab extrusion thickness remain open. Eighty-one per-class
decoder structs remain registered; `MVP_TYPED_CLASSES` are consulted by
`iter_elements`.

## Capability Matrix

| Capability | Status | Evidence | User impact |
|---|---|---|---|
| Open `.rvt`, `.rfa`, `.rte`, `.rft` CFB containers | Full | `reader`, CI matrix | Files can be inspected without Revit. |
| Decode Revit truncated-gzip streams (gated checksum-page strip on Partitions/Global, #151) | Full | `compression`, `checksum_page_framing`, fuzz | Internal streams can be read safely; Formats/Latest stays ungated by default — multipage integrity uncertain (`RVT_FORMATS_MULTIPAGE_UNVERIFIED`). |
| Extract metadata, PartAtom XML, preview PNG | Full | `basic_file_info`, `part_atom`, tests | Users can identify and audit files. |
| Parse `Formats/Latest` schema | Full | 100 percent field classification over 2016-2026 family corpus; multipage integrity diagnostics in inspect/export/viewer | Developers can inspect class and field structure; Formats multipage integrity uncertain while strip stays disabled. |
| Read document-level ADocument data | Partial | Reliable on newer samples; older/project bands need more corpus proof | Good for diagnostics, not complete model extraction. |
| Decode typed elements from real project files | **Partial** | Production `iter_elements`: ArcWall (2023) + partition MVP Levels/Materials/Rooms/Floor plan-loops + 2024 ArcWallRectOpening (ElemTable-confirmed related ids); HostObjAttr filtered; RE-19 negative: no Door/Window discriminator / no schema-field Wall on magnetar corpora | Full model conversion is not ready. |
| Typed decoder structs | Partial | `elements::all_decoders()` registers **81** decoders; `MVP_TYPED_CLASSES` consulted by `iter_elements`; ArcWall uses a separate partition decoder | Library building blocks plus production MVP/ArcWall path. |
| IFC4 writer | Partial | Synthetic fixtures validate in IfcOpenShell; 2023 Einhoven ArcWall `IfcWall` + partition Level storeys / Floor boundary `IfcSlab` / Room `IfcSpace` / Material display names; thickness + Door/Window host IFC still open; `rvt-ifc --diagnostics` JSON readiness sidecar; `--mode` gates scaffold/typed/geometry/strict | Correct writer path exists, but real-file typed inputs are incomplete / unsolved. |
| Browser viewer | Partial | GitHub Pages deployment, no-network WASM import gate, File Status shows production class counts + storey/material totals, supported-profile matrix | Useful for local inspection; geometry reflects decoded coverage. |
| Stream-level writer | Partial | Always-on patch corpus (`gen-fixture` project + MIT `empty.rfa`) covers identity, grow, shrink, multi-stream, missing-stream; optional Autodesk corpora add release-matrix + GUID/history checks; corrupt-gzip verification is unit-tested | Useful for controlled stream replacement, not semantic Revit editing. |
| Python package | Partial | CI wheel builds and pytest | Useful for metadata/schema automation. |
| User-facing inspect CLI | Partial | `rvt-inspect` reports file health, decoded coverage, IFC export readiness, warnings, next steps, and stable JSON | Useful for support triage without Revit internals. |
| Community corpus open/scaffold check | Partial (executed) | `docs/corpus-hunt-2026-04-21.md`: 222/223 real files pass open → schema → scaffold IFC | Proves container/schema health on public samples; does **not** prove typed element recovery. |

## Roadmap Position

The near-term project is tracked in GitHub milestones:

- `0.2.0: audit-clean alpha` — **scope complete** (0 open / 19 closed). Quality
  script, honest docs, issue forms, and supply-chain checks landed on `main`.
  Crate version remains `0.1.x` (latest tag `v0.1.2`); **0.2.0 release hold**
  until a version bump / tag is cut (audit GOV-002).
- `0.3.0: real-project wall/floor MVP` - corpus-backed partition scanning and typed element recovery.
- `0.4.0: IFC geometry beta` - trustworthy IFC export modes, diagnostics, and validation.
- `0.5.0: viewer beta` — **tracked issues complete** (0 open / 6 closed). Viewer
  guidance, demo gallery, and browser regression work is on `main`; **0.5.0
  release hold** until a viewer-beta release is cut (audit GOV-002).
- `1.0.0: first-class utility` - documented non-technical workflow with clear support boundaries.

The detailed task backlog lives in [`TODO.md`](../TODO.md) and the matching
GitHub issues.

## Supported MVP Definition

The first broadly useful release should let a non-technical AEC user:

1. Open a supported Revit file locally without uploading it.
2. See a clear status report that says what was decoded, what was skipped, and
   why.
3. Export IFC only when typed elements and geometry meet the supported profile.
4. Receive actionable diagnostics when a file is outside that profile.
5. Follow docs written for BIM users, not Rust developers.

Until those five conditions hold, rvt-rs should present itself as an
open-source Revit inspection and reverse-engineering toolkit, not as a complete
replacement for production Revit export workflows.
