# Roadmap

This roadmap is the public, contributor-facing view of where rvt-rs is headed.
The support boundary lives in [`docs/status.md`](docs/status.md). The full task
decomposition lives in [`TODO.md`](TODO.md) and the matching GitHub issues.

## Product Goal

rvt-rs should become a first-class open-source utility for BIM and AEC users who
need to inspect, validate, and exchange Revit files without installing Revit or
uploading private models to a third-party service.

The project is not there yet. It is currently a strong Revit inspection and
reverse-engineering toolkit with a partial IFC/viewer path. **Generic
real-project typed element extraction remains mostly unsolved**, with
narrow exceptions: production `walker::iter_elements` prefers typed MVP
decoders on `Global/Latest`, merges version-gated **ArcWall** partition
recovers (Revit 2023 standard), and merges fail-closed partition MVP
recovers for **Level** / **Material** / **Room** / **Floor** plan-loops
plus 2024 **ArcWallRectOpening** index rows (ElemTable-confirmed related
ids; never inventing typed `Door`/`Window` success). IFC export wires
partition **Level** → storeys, **Floor** → boundary-annotated `IFCSLAB`,
**Room** → `IFCSPACE`, and **Material** display names → `IfcMaterial`.
**RE-19 (2026-08-29) negative on magnetar corpora:** no reliable Door vs
Window discriminator in the 2024 opening index / nearby partition strings /
ElemTable payloads, and no schema-field `Wall` / fail-closed 2024 ArcWall
envelope — see
[`reports/element-framing/RE-19-door-window-wall-negative.md`](reports/element-framing/RE-19-door-window-wall-negative.md).
Host IFC voids/fills, Floor↔ElemTable id binding, and slab extrusion
thickness remain open. Eighty per-class decoder structs exist in
`elements::all_decoders()`; MVP classes are consulted by `iter_elements`,
while the broader registry remains a library building block (see
[`docs/status.md`](docs/status.md) and
[`docs/compatibility.md`](docs/compatibility.md)).

## Current Position

| Area | Current state | Next decision point |
|---|---|---|
| Container, compression, metadata | Shipped | Maintain compatibility and bounds checks. |
| `Formats/Latest` schema | Shipped | Keep 100 percent field classification gated in CI. |
| ADocument/document-level walker | Partial | Expand confidence across project releases and older files. |
| Typed project elements | **Partial** | MVP typed path + ArcWall + partition Level/Material/Room/Floor plan-loops + 2024 opening index (ElemTable-confirmed ids) in `iter_elements` (fail closed). RE-19: no Door/Window discriminator / no schema-field Wall on magnetar corpora — keep issues open without inventing types. |
| IFC writer | Partial | Levels/Floors/Rooms/Materials from partition MVP emit honestly; ArcWall geometry on 2023; Door/Window host IFC + slab extrusion still open (blocked on RE-19). |
| Browser viewer | Partial | File Status lists recovered storey names + material display-name samples + honest Parameters row (empty until AProperty host joins); scene tree groups under `IFCBUILDINGSTOREY` (ArcWalls by elevation; Floors/Rooms stay Unassigned until Level ElementIds exist on both sides of the bind). |
| Python/CLI surface | Partial | Stabilize JSON schemas and one-shot inspect workflow. |
| Write path | Partial | Keep stream-level writes honest; defer semantic writes until openability can be proven. |

## Milestones

### 0.2.0: Audit-Clean Alpha

Purpose: make the repository easy to trust and easy to contribute to before
deep decoder work accelerates.

- One-command local quality gate.
- Explicit cargo-audit/cargo-deny expectations.
- README, roadmap, compatibility, and status docs aligned.
- GitHub issue forms for decoder work and corpus submissions.
- Contribution map for non-maintainers.
- Release artifact verification documented.

### 0.3.0: Real-Project Wall/Floor MVP

Purpose: prove that rvt-rs can recover meaningful typed building elements from
real project files, not only synthesized fixtures.

- Redistributable project corpus with license metadata.
- Known-count fixtures for levels, walls, floors, doors, and windows.
- Generic partition record scanner.
- `ElemTable` id to partition-record offset linkage.
- Typed MVP decoders + ArcWall + partition Level/Material/Room/Floor plan-loop / 2024 opening-index (ElemTable-confirmed) merge wired into `iter_elements` without false positives; IFC Level/Floor/Room/Material emission (schema-field Wall and typed Door/Window host binding still open).
- Decode confidence and provenance attached to every element.

### 0.4.0: IFC Geometry Beta

Purpose: export useful IFC only when decoded evidence is strong enough.

- Explicit export modes: strict, proxy, and diagnostic.
- No misleading generic proxies in default export.
- IFC diagnostics sidecar describing decoded/skipped elements.
- IfcOpenShell validation for generated outputs.
- Comparison tooling against Revit-exported IFC when fixtures allow it.

### 0.5.0: Viewer Beta

Purpose: make unsupported states clear to non-technical users.

- Decode/export confidence surfaced in the viewer.
- Supported-file guidance before export.
- Demo gallery using redistributable files.
- Browser regression tests across desktop and mobile viewports.
- Accessibility and responsive layout pass.
- Desktop distribution investigation.

### 1.0.0: First-Class Utility

Purpose: ship a complete, honest workflow that gives meaningful value to users
without requiring them to be Rust, Revit API, or reverse-engineering experts.

- Supported input profile documented in user language.
- End-to-end open -> inspect -> diagnose -> export workflow.
- Actionable failure modes for unsupported files.
- Non-technical documentation and screenshots.
- Release artifacts verified and reproducible.

## Contribution Priorities

Start with [`docs/contribution-map.md`](docs/contribution-map.md). The highest
leverage work is:

1. Redistributable corpus files with known counts.
2. Partition-stream probes that turn byte observations into falsifiable decoder
   hypotheses.
3. Tests that prevent `iter_elements` from claiming false positives.
4. Documentation that keeps user-facing support boundaries honest.
5. Viewer diagnostics that explain what the tool could and could not decode.

## Out of Scope

rvt-rs will not:

- Use Autodesk proprietary SDK internals, leaked documents, or decompiled
  proprietary implementation code. See [`CLEANROOM.md`](CLEANROOM.md).
- Claim production RVT-to-IFC conversion before real project typed elements and
  geometry are corpus-proven.
- Provide a Revit API-compatible surface.
- Resolve cloud-worksharing, licensing, or external linked-model semantics in
  the near-term product.
