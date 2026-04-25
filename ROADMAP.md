# Roadmap

This roadmap is the public, contributor-facing view of where rvt-rs is headed.
The support boundary lives in [`docs/status.md`](docs/status.md). The full task
decomposition lives in [`TODO.md`](TODO.md) and the matching GitHub issues.

## Product Goal

rvt-rs should become a first-class open-source utility for BIM and AEC users who
need to inspect, validate, and exchange Revit files without installing Revit or
uploading private models to a third-party service.

The project is not there yet. It is currently a strong Revit inspection and
reverse-engineering toolkit with a partial IFC/viewer path. The key missing
product capability is reliable typed element recovery from real `.rvt` project
partition streams.

## Current Position

| Area | Current state | Next decision point |
|---|---|---|
| Container, compression, metadata | Shipped | Maintain compatibility and bounds checks. |
| `Formats/Latest` schema | Shipped | Keep 100 percent field classification gated in CI. |
| ADocument/document-level walker | Partial | Expand confidence across project releases and older files. |
| Typed project elements | Research | Decode partition records and link them to `ElemTable` ids. |
| IFC writer | Partial | Keep synthetic validation green while adding real-file diagnostics. |
| Browser viewer | Partial | Show confidence and unsupported-file guidance clearly. |
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
- Typed decoders wired into `iter_elements` without false positives.
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
