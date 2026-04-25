# Project Status

Last reviewed: 2026-04-25

This page is the public source of truth for what rvt-rs can do today. It is
intentionally blunt so users can decide quickly whether the tool fits their
workflow.

## User-Facing Summary

rvt-rs is useful today for inspecting Revit files without Revit, extracting
metadata, reading the embedded schema, auditing stream contents, running a
zero-upload browser viewer, and producing valid IFC/glTF/SVG outputs from the
parts of the model that are actually decoded.

rvt-rs is not yet a production RVT-to-IFC converter for real architectural
projects. The current blocker is typed element recovery from real `.rvt`
project partition streams: walls, floors, doors, windows, and levels are not
recovered reliably enough from arbitrary project files to claim complete model
conversion.

## Capability Matrix

| Capability | Status | Evidence | User impact |
|---|---|---|---|
| Open `.rvt`, `.rfa`, `.rte`, `.rft` CFB containers | Full | `reader`, CI matrix | Files can be inspected without Revit. |
| Decode Revit truncated-gzip streams | Full | `compression`, fuzz regressions | Internal streams can be read safely. |
| Extract metadata, PartAtom XML, preview PNG | Full | `basic_file_info`, `part_atom`, tests | Users can identify and audit files. |
| Parse `Formats/Latest` schema | Full | 100 percent field classification over 2016-2026 family corpus | Developers can inspect class and field structure. |
| Read document-level ADocument data | Partial | Reliable on newer samples; older/project bands need more corpus proof | Good for diagnostics, not complete model extraction. |
| Decode typed elements from real project files | Research | Production iteration is conservative; diagnostic scans still show parent/proxy candidates, not dependable typed walls/floors/doors | Full model conversion is not ready. |
| Typed decoder structs | Partial | `elements::all_decoders()` has 80 registered decoders | Useful as library building blocks and synthesized-fixture tests. |
| IFC4 writer | Partial | Synthetic fixtures validate in IfcOpenShell | Correct writer path exists, but real-file inputs are incomplete. |
| Browser viewer | Partial | GitHub Pages deployment and no-network WASM import gate | Useful for local inspection; geometry reflects decoded coverage. |
| Stream-level writer | Partial | Byte-preserving patch path tests | Useful for controlled stream replacement, not semantic Revit editing. |
| Python package | Partial | CI wheel builds and pytest | Useful for metadata/schema automation. |

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
