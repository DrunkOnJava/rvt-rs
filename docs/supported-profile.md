# Supported MVP Profile

This project is useful today for file inspection, metadata/schema extraction,
diagnostics, and support triage. It is not yet a general Revit model converter.

## Supported Today

| Area | Supported profile |
|---|---|
| File extensions | `.rvt`, `.rfa`, `.rte`, `.rft` containers that use the standard Revit OLE/CFB layout. |
| Revit versions | Metadata/schema inspection is regression-tested against the 2016-2026 family corpus. |
| Safe workflows | `rvt-inspect`, `rvt-info`, `rvt-schema`, previews, stream inventory, document metadata, class schema, and diagnostics sidecars. |
| IFC output | Spec-valid IFC4 scaffold with project/spatial framework; typed real-project output is limited to narrow version-gated evidence such as the 2023 ArcWall path. |
| Browser viewer | Zero-upload inspection, scene/status panels, and explicit export-readiness labels before download. |

## Experimental MVP Target

The first real-model conversion profile is intentionally narrow:

| Dimension | Target |
|---|---|
| File type | `.rvt` project files before `.rfa` family geometry. |
| Versions | Revit 2023 and 2024 project files first, because the current project corpus and ArcWall evidence live there. |
| Discipline | Architectural core before MEP/structure-heavy projects. |
| Classes | Levels, walls, floors/slabs, doors, windows, rooms/spaces, materials, and common parameters. |
| Export quality | `rvt-ifc --mode strict` must reject files that cannot meet the requested quality. |

## Unsupported Or Partial

- Full typed element extraction from arbitrary real `.rvt` files.
- Reliable geometry for real walls/floors/doors/windows outside the narrow
  research profile.
- Semantic Revit editing through the stream writer.
- Revit versions outside the verified corpus without a diagnostics report.
- Files that are corrupt, zero-byte LFS placeholders, encrypted, or not OLE/CFB
  Revit containers.

Use `rvt-inspect <file> --json` or the viewer diagnostics download when a file
falls outside the supported profile. Those reports are designed to be attached
to GitHub issues without exposing raw model bytes.
