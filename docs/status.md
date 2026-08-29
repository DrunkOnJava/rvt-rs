# Project Status

Last reviewed: 2026-08-29

This page is the public source of truth for what rvt-rs can do today. It is
intentionally blunt so users can decide quickly whether the tool fits their
workflow.

The current support boundary is tracked in the
[supported MVP input profile](supported-profile.md). Keep that page, this
status summary, the README, and the viewer support matrix aligned.

## User-Facing Summary

rvt-rs is useful today for inspecting Revit files without Revit, extracting
metadata, reading the embedded schema, auditing stream contents, running a
zero-upload browser viewer, and producing valid IFC/glTF/SVG outputs from the
parts of the model that are actually decoded.

**Generic real-project typed model extraction is not solved yet.** rvt-rs is
not a production RVT-to-IFC converter for arbitrary architectural projects.
Walls, floors, doors, windows, and levels are not recovered as typed elements
from arbitrary `.rvt` project files. Eighty per-class decoder structs exist and
pass synthesized-fixture unit tests; they are not reached by
`walker::iter_elements` on real project streams. A narrow, version-gated 2023
ArcWall research path can emit `IfcWall` when corpus evidence is strong enough
— that exception does not constitute general typed conversion.

## Capability Matrix

| Capability | Status | Evidence | User impact |
|---|---|---|---|
| Open `.rvt`, `.rfa`, `.rte`, `.rft` CFB containers | Full | `reader`, CI matrix | Files can be inspected without Revit. |
| Decode Revit truncated-gzip streams | Full | `compression`, fuzz regressions | Internal streams can be read safely. |
| Extract metadata, PartAtom XML, preview PNG | Full | `basic_file_info`, `part_atom`, tests | Users can identify and audit files. |
| Parse `Formats/Latest` schema | Full | 100 percent field classification over 2016-2026 family corpus | Developers can inspect class and field structure. |
| Read document-level ADocument data | Partial | Reliable on newer samples; older/project bands need more corpus proof | Good for diagnostics, not complete model extraction. |
| Decode typed elements from real project files | **Unsolved** | Production iteration is conservative; diagnostic scans still show parent/proxy candidates, not dependable typed walls/floors/doors | Full model conversion is not ready. |
| Typed decoder structs | Partial (registry only) | `elements::all_decoders()` registers **80** decoders; unit test pins `all_decoders().len() == 80`; not consulted by `iter_elements` | Useful as library building blocks and synthesized-fixture tests. |
| IFC4 writer | Partial | Synthetic fixtures validate in IfcOpenShell; 2023 Einhoven ArcWall records emit `IfcWall` swept solids with recovered ElementId / base elevation / Z-delta height and elevation-derived storeys (partition Level-like names applied when confident; thickness still unresolved — RE-15/#88 falsified trailer inch widths); diagnostic mode can include low-confidence proxy provenance; `rvt-ifc --diagnostics` emits a JSON readiness sidecar; `--mode` gates scaffold/typed/geometry/strict output | Correct writer path exists, but real-file typed inputs are incomplete / unsolved. |
| Browser viewer | Partial | GitHub Pages deployment, no-network WASM import gate, plain-language decode/export confidence panel, and supported-profile matrix | Useful for local inspection; geometry reflects decoded coverage. |
| Stream-level writer | Partial | Family/project corpus patch tests cover identity, grow, shrink, multi-stream, missing-stream, corrupt-gzip verification, and GUID/history preservation | Useful for controlled stream replacement, not semantic Revit editing. |
| Python package | Partial | CI wheel builds and pytest | Useful for metadata/schema automation. |
| User-facing inspect CLI | Partial | `rvt-inspect` reports file health, decoded coverage, IFC export readiness, warnings, next steps, and stable JSON | Useful for support triage without Revit internals. |
| Community corpus open/scaffold check | Partial (executed) | `docs/corpus-hunt-2026-04-21.md`: 222/223 real files pass open → schema → scaffold IFC | Proves container/schema health on public samples; does **not** prove typed element recovery. |

## Roadmap Position

The near-term project is tracked in GitHub milestones:

- `0.2.0: audit-clean alpha` - quality script, honest docs, issue forms, supply-chain checks.
- `0.3.0: real-project wall/floor MVP` - corpus-backed partition scanning and typed element recovery.
- `0.4.0: IFC geometry beta` - trustworthy IFC export modes, diagnostics, and validation.
- `0.5.0: viewer beta` - user-facing viewer guidance, demo gallery, and browser regression tests.
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
